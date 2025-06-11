//! vLLM Provider Test Example
//!
//! This example demonstrates how to use vLLM models through the Circuit Breaker server's OpenAI API.
//!
//! Prerequisites:
//! - Circuit Breaker server must be running (cargo run --bin server)
//! - vLLM must be configured and available through the server
//!
//! Usage:
//! ```bash
//! # Start the Circuit Breaker server (in another terminal)
//! cargo run --bin server
//!
//! # Run the example
//! cargo run --example vllm_provider_test
//! ```

use reqwest::Client;
use serde_json::{json, Value};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🚀 Circuit Breaker - vLLM Provider Test");
    println!("=======================================");

    // Configuration
    let base_url =
        env::var("CIRCUIT_BREAKER_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
    let openai_endpoint = format!("{}/v1", base_url);

    // API key (if required by server configuration)
    let api_key = env::var("CIRCUIT_BREAKER_API_KEY").unwrap_or_default();

    println!(
        "🔍 Testing vLLM through Circuit Breaker at: {}",
        openai_endpoint
    );

    let client = Client::new();

    // Test server health
    println!("\n🏥 Testing server health...");
    match test_health(&client, &base_url).await {
        Ok(()) => println!("✅ Server is healthy"),
        Err(e) => {
            eprintln!("❌ Server health check failed: {}", e);
            return Ok(());
        }
    }

    // List available models
    println!("\n📋 Fetching available models...");
    match list_models(&client, &openai_endpoint, &api_key).await {
        Ok(models) => {
            println!("✅ Available models:");
            for model in models {
                if let Some(id) = model.get("id").and_then(|v| v.as_str()) {
                    println!("  - {}", id);
                }
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to list models: {}", e);
            println!("   This might mean vLLM is not properly configured.");
        }
    }

    // Test chat completion with fallback models
    let model = env::var("VLLM_MODEL").unwrap_or_else(|_| "microsoft/DialoGPT-medium".to_string());

    // Test chat completion with fallback models
    let fallback_models = vec![
        model.clone(),
        "microsoft/DialoGPT-medium".to_string(),
        "codellama/CodeLlama-7b-Instruct-hf".to_string(),
        "meta-llama/Llama-2-7b-chat-hf".to_string(),
    ];

    let mut chat_success = false;
    for test_model in &fallback_models {
        println!("\n💬 Testing chat completion with model: {}", test_model);

        let chat_request = json!({
            "model": test_model,
            "messages": [
                {
                    "role": "system",
                    "content": "You are a helpful assistant. Be explicit in your responses."
                },
                {
                    "role": "user",
                    "content": "Write me an elevator pitch for GitLab"
                }
            ],
            "temperature": 0.7,
            "max_tokens": 2048,
            "stream": false
        });

        // Check if this is a streaming request
        let is_streaming = chat_request.get("stream").and_then(|v| v.as_bool()).unwrap_or(false);
        
        if is_streaming {
            match streaming_chat(&client, &openai_endpoint, &api_key, chat_request.clone()).await {
                Ok(()) => {
                    println!("✅ Streaming chat completion successful with model: {}", test_model);
                    chat_success = true;
                    break;
                }
                Err(e) => {
                    eprintln!(
                        "❌ Streaming chat completion failed with model '{}': {}",
                        test_model, e
                    );
                    if test_model == fallback_models.last().unwrap() {
                        eprintln!("   All fallback models failed for streaming.");
                    }
                }
            }
        }

        match chat_completion(&client, &openai_endpoint, &api_key, chat_request).await {
            Ok(response) => {
                println!("✅ Chat completion successful with model: {}", test_model);
                chat_success = true;

                if let Some(usage) = response.get("usage") {
                    if let (Some(prompt_tokens), Some(completion_tokens), Some(total_tokens)) = (
                        usage.get("prompt_tokens").and_then(|v| v.as_u64()),
                        usage.get("completion_tokens").and_then(|v| v.as_u64()),
                        usage.get("total_tokens").and_then(|v| v.as_u64()),
                    ) {
                        println!("📊 Token usage:");
                        println!("  Prompt tokens: {}", prompt_tokens);
                        println!("  Completion tokens: {}", completion_tokens);
                        println!("  Total tokens: {}", total_tokens);
                    }
                }

                if let Some(choices) = response.get("choices").and_then(|v| v.as_array()) {
                    if let Some(choice) = choices.first() {
                        if let Some(message) = choice.get("message") {
                            if let Some(content) = message.get("content").and_then(|v| v.as_str()) {
                                println!("\n🤖 Assistant response:");
                                println!("  {}", content);
                            }
                        }
                    }
                }
                break; // Success, no need to try other models
            }
            Err(e) => {
                eprintln!(
                    "❌ Chat completion failed with model '{}': {}",
                    test_model, e
                );
                if test_model == fallback_models.last().unwrap() {
                    eprintln!("   All fallback models failed. Common issues:");
                    eprintln!("   1. No models available in vLLM server");
                    eprintln!("   2. vLLM server not running or misconfigured");
                    eprintln!("   3. Circuit Breaker routing configuration issue");
                    eprintln!(
                        "   4. Try setting VLLM_MODEL environment variable to a specific model"
                    );
                }
            }
        }
    }

    // Test embeddings (if supported)
    println!("\n🔮 Testing embeddings...");
    let embedding_models = vec![
        env::var("VLLM_EMBEDDING_MODEL")
            .unwrap_or_else(|_| "sentence-transformers/all-MiniLM-L6-v2".to_string()),
        "sentence-transformers/all-MiniLM-L6-v2".to_string(),
        "sentence-transformers/all-mpnet-base-v2".to_string(),
    ];

    let mut embedding_success = false;
    for embedding_model in &embedding_models {
        println!("  Trying embedding model: {}", embedding_model);

        let embedding_request = json!({
            "model": embedding_model,
            "input": "This is a test sentence for embeddings.",
            "encoding_format": "float"
        });

        match embeddings(&client, &openai_endpoint, &api_key, embedding_request).await {
            Ok(response) => {
                println!("✅ Embeddings successful with model: {}", embedding_model);
                embedding_success = true;

                if let Some(data) = response.get("data").and_then(|v| v.as_array()) {
                    if let Some(embedding_obj) = data.first() {
                        if let Some(embedding) =
                            embedding_obj.get("embedding").and_then(|v| v.as_array())
                        {
                            println!("📊 Embedding details:");
                            println!("  Model: {}", embedding_model);
                            println!("  Embedding dimension: {}", embedding.len());

                            // Show first 5 values
                            let preview: Vec<f64> = embedding
                                .iter()
                                .take(5)
                                .filter_map(|v| v.as_f64())
                                .collect();
                            println!("  First 5 values: {:?}", preview);
                        }
                    }
                }
                break; // Success, no need to try other models
            }
            Err(e) => {
                eprintln!(
                    "❌ Embeddings failed with model '{}': {}",
                    embedding_model, e
                );
                if embedding_model == embedding_models.last().unwrap() {
                    eprintln!("   All embedding models failed. This might mean:");
                    eprintln!("   1. No embedding models are available in vLLM");
                    eprintln!("   2. vLLM server doesn't support embeddings endpoint");
                }
            }
        }
    }

    // Test streaming (optional)
    if env::var("TEST_STREAMING").unwrap_or_default() == "true" && chat_success {
        println!("\n🌊 Testing streaming chat completion...");

        // Use the first successful model from the fallback list
        let streaming_model = fallback_models
            .iter()
            .find(|m| **m != model || chat_success)
            .unwrap_or(&fallback_models[0]);

        let streaming_request = json!({
            "model": streaming_model,
            "messages": [
                {
                    "role": "user",
                    "content": "Count from 1 to 5, one number per line."
                }
            ],
            "temperature": 0.3,
            "max_tokens": 50,
            "stream": true
        });

        match streaming_chat(&client, &openai_endpoint, &api_key, streaming_request).await {
            Ok(()) => println!("✅ Streaming completed successfully"),
            Err(e) => eprintln!("❌ Streaming failed: {}", e),
        }
    } else if env::var("TEST_STREAMING").unwrap_or_default() == "true" {
        println!("\n🌊 Skipping streaming test (no successful chat model found)");
    }

    println!("\n🎉 vLLM provider test completed!");

    // Summary
    println!("\n📋 Test Summary:");
    println!(
        "  Chat completion: {}",
        if chat_success {
            "✅ PASSED"
        } else {
            "❌ FAILED"
        }
    );
    println!(
        "  Embeddings: {}",
        if embedding_success {
            "✅ PASSED"
        } else {
            "❌ FAILED"
        }
    );

    println!("\n💡 Tips:");
    println!(
        "  - Set VLLM_MODEL to test specific chat models (default: microsoft/DialoGPT-medium)"
    );
    println!("  - Set VLLM_EMBEDDING_MODEL to test specific embedding models");
    println!("  - Set TEST_STREAMING=true to test streaming responses");
    println!("  - Set CIRCUIT_BREAKER_URL to test remote instances");
    println!("  - Set CIRCUIT_BREAKER_API_KEY if authentication is required");
    println!("  - Available fallback models: microsoft/DialoGPT-medium, codellama/CodeLlama-7b-Instruct-hf");
    println!("  - Make sure vLLM server is running with at least one model loaded");

    Ok(())
}

async fn test_health(client: &Client, base_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let health_url = format!("{}/health", base_url);
    let response = client.get(&health_url).send().await?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!("Health check failed with status: {}", response.status()).into())
    }
}

async fn list_models(
    client: &Client,
    base_url: &str,
    api_key: &str,
) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    let url = format!("{}/models", base_url);

    let mut request = client.get(&url);

    if !api_key.is_empty() {
        request = request.header("Authorization", format!("Bearer {}", api_key));
    }

    let response = request.send().await?;

    if !response.status().is_success() {
        return Err(format!("Models request failed with status: {}", response.status()).into());
    }

    let json: Value = response.json().await?;

    Ok(json
        .get("data")
        .and_then(|v| v.as_array())
        .unwrap_or(&vec![])
        .clone())
}

async fn chat_completion(
    client: &Client,
    base_url: &str,
    api_key: &str,
    request_body: Value,
) -> Result<Value, Box<dyn std::error::Error>> {
    let url = format!("{}/chat/completions", base_url);

    let mut request = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&request_body);

    if !api_key.is_empty() {
        request = request.header("Authorization", format!("Bearer {}", api_key));
    }

    let response = request.send().await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(format!("Chat completion failed: {}", error_text).into());
    }

    let json: Value = response.json().await?;
    Ok(json)
}

async fn embeddings(
    client: &Client,
    base_url: &str,
    api_key: &str,
    request_body: Value,
) -> Result<Value, Box<dyn std::error::Error>> {
    let url = format!("{}/embeddings", base_url);

    let mut request = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&request_body);

    if !api_key.is_empty() {
        request = request.header("Authorization", format!("Bearer {}", api_key));
    }

    let response = request.send().await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(format!("Embeddings failed: {}", error_text).into());
    }

    let json: Value = response.json().await?;
    Ok(json)
}

async fn streaming_chat(
    client: &Client,
    base_url: &str,
    api_key: &str,
    request_body: Value,
) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!("{}/chat/completions", base_url);

    let mut request = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&request_body);

    if !api_key.is_empty() {
        request = request.header("Authorization", format!("Bearer {}", api_key));
    }

    let response = request.send().await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(format!("Streaming chat failed: {}", error_text).into());
    }

    // For streaming, we get the raw text and parse SSE format
    let text = response.text().await?;
    
    println!("\n🤖 Streaming response:");

    // Parse SSE format: "data: {json}\n\n"
    for line in text.lines() {
        if line.starts_with("data: ") {
            let json_str = &line[6..]; // Remove "data: " prefix
            if json_str == "[DONE]" {
                break;
            }

            if let Ok(chunk) = serde_json::from_str::<Value>(json_str) {
                if let Some(choices) = chunk.get("choices").and_then(|v| v.as_array()) {
                    if let Some(choice) = choices.first() {
                        if let Some(delta) = choice.get("delta") {
                            if let Some(content) = delta.get("content").and_then(|v| v.as_str()) {
                                print!("{}", content);
                                std::io::Write::flush(&mut std::io::stdout()).unwrap();
                            }
                        }
                    }
                }
            }
        }
    }

    println!(); // New line after streaming
    Ok(())
}
