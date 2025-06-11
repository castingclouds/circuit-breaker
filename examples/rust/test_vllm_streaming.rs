//! Simple vLLM Streaming Test
//!
//! This is a focused test for vLLM streaming functionality through Circuit Breaker.
//! Run with: cargo run --bin test_vllm_streaming

use reqwest::Client;
use serde_json::{json, Value};
use std::env;
use futures::StreamExt;
use std::time::{Duration, Instant};
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 vLLM Streaming Test");
    println!("=====================");

    // Configuration
    let base_url =
        env::var("CIRCUIT_BREAKER_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
    let openai_endpoint = format!("{}/v1", base_url);
    let api_key = env::var("CIRCUIT_BREAKER_API_KEY").unwrap_or_default();

    println!("🌐 Testing against: {}", openai_endpoint);

    let client = Client::new();

    // Test models to try
    let test_models = vec![
        "codellama/CodeLlama-7b-Instruct-hf",
        "microsoft/DialoGPT-medium",
        "meta-llama/Llama-2-7b-chat-hf",
    ];

    // Test 1: Non-streaming first
    println!("\n1️⃣  Testing NON-STREAMING chat completion...");
    for model in &test_models {
        println!("   Trying model: {}", model);

        let request = json!({
            "model": model,
            "messages": [
                {
                    "role": "user",
                    "content": "Say hello in exactly 5 words."
                }
            ],
            "temperature": 0.7,
            "max_tokens": 50,
            "stream": false
        });

        match test_chat_completion(&client, &openai_endpoint, &api_key, request).await {
            Ok(response) => {
                println!("   ✅ SUCCESS with model: {}", model);
                if let Some(choices) = response.get("choices").and_then(|v| v.as_array()) {
                    if let Some(choice) = choices.first() {
                        if let Some(message) = choice.get("message") {
                            if let Some(content) = message.get("content").and_then(|v| v.as_str()) {
                                println!("   🤖 Response: {}", content.trim());
                            }
                        }
                    }
                }
                break;
            }
            Err(e) => {
                println!("   ❌ Failed: {}", e);
                continue;
            }
        }
    }

    // Test 2: Streaming
    println!("\n2️⃣  Testing STREAMING chat completion...");
    for model in &test_models {
        println!("   Trying streaming with model: {}", model);

        let request = json!({
            "model": model,
            "messages": [
                {
                    "role": "user",
                    "content": "Write a slow elevator pitch for GitLab. Use exactly 3 short sentences, each on a new line."
                }
            ],
            "temperature": 0.1,
            "max_tokens": 150,
            "stream": true
        });

        match test_streaming_chat(&client, &openai_endpoint, &api_key, request).await {
            Ok(()) => {
                println!("\n   ✅ STREAMING SUCCESS with model: {}", model);
                break;
            }
            Err(e) => {
                println!("   ❌ Streaming failed: {}", e);
                continue;
            }
        }
    }

    println!("\n🎉 Test completed!");
    Ok(())
}

async fn test_chat_completion(
    client: &Client,
    endpoint: &str,
    api_key: &str,
    request_body: Value,
) -> Result<Value, String> {
    let url = format!("{}/chat/completions", endpoint);

    let mut request = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&request_body);

    if !api_key.is_empty() {
        request = request.header("Authorization", format!("Bearer {}", api_key));
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("HTTP {}: {}", status, error_text));
    }

    let json: Value = response
        .json()
        .await
        .map_err(|e| format!("JSON parse error: {}", e))?;
    Ok(json)
}

async fn test_streaming_chat(
    client: &Client,
    endpoint: &str,
    api_key: &str,
    request_body: Value,
) -> Result<(), String> {
    let url = format!("{}/chat/completions", endpoint);
    
    let mut request = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Accept", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .json(&request_body);

    if !api_key.is_empty() {
        request = request.header("Authorization", format!("Bearer {}", api_key));
    }

    let response = request.send().await.map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("HTTP {}: {}", status, error_text));
    }

    println!("   🌊 Real-time streaming output:");
    print!("   ");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();

    let start_time = Instant::now();
    let mut chunk_count = 0;
    let mut total_chars = 0;
    
    // Stream the response byte by byte
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    
    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                let chunk_str = String::from_utf8_lossy(&chunk);
                buffer.push_str(&chunk_str);
                
                // Process complete lines from buffer
                while let Some(line_end) = buffer.find('\n') {
                    let line = buffer[..line_end].trim().to_string();
                    buffer = buffer[line_end + 1..].to_string();
                    
                    if line.starts_with("data: ") {
                        let data = &line[6..]; // Remove "data: " prefix
                        
                        if data == "[DONE]" {
                            println!();
                            let elapsed = start_time.elapsed();
                            println!("   📊 Streaming stats: {} chunks, {} chars in {:?}", 
                                chunk_count, total_chars, elapsed);
                            return Ok(());
                        }

                        // Try to parse as JSON
                        if let Ok(json_chunk) = serde_json::from_str::<Value>(data) {
                            if let Some(choices) = json_chunk.get("choices").and_then(|v| v.as_array()) {
                                if let Some(choice) = choices.first() {
                                    if let Some(delta) = choice.get("delta") {
                                        if let Some(content) = delta.get("content").and_then(|v| v.as_str()) {
                                            // Display each character with a small delay for visualization
                                            for ch in content.chars() {
                                                print!("{}", ch);
                                                std::io::Write::flush(&mut std::io::stdout()).unwrap();
                                                sleep(Duration::from_millis(25)).await;
                                            }
                                            
                                            chunk_count += 1;
                                            total_chars += content.len();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                return Err(format!("Stream error: {}", e));
            }
        }
    }

    println!(); // Add newline after streaming
    Ok(())
}
