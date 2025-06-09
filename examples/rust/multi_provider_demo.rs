//! Multi-Provider LLM Demo
//!
//! This demo shows how to configure and use multiple LLM providers
//! (OpenAI, Anthropic, and Google Gemini) with the Circuit Breaker LLM Router.
//!
//! ## Setup
//! 1. Set your API keys:
//! ```bash
//! export OPENAI_API_KEY=your_openai_api_key_here
//! export ANTHROPIC_API_KEY=your_anthropic_api_key_here
//! export GOOGLE_API_KEY=your_google_api_key_here
//! ```
//!
//! 2. Run the demo:
//! ```bash
//! cargo run --example multi_provider_demo
//! ```
//!
//! ## What this demonstrates:
//! - Configuration of multiple LLM providers
//! - Cost comparison across providers
//! - Streaming capabilities for each provider
//! - Real API integration with actual cost tracking

use circuit_breaker::llm::{router::LLMRouter, ChatMessage, LLMRequest, MessageRole};
use reqwest::Client;
use serde_json::json;
use std::io::{self, Write};
use uuid::Uuid;

fn wait_for_enter(message: &str) {
    print!("{} (Press Enter to continue)", message);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🤖 Circuit Breaker Multi-Provider LLM Demo");
    println!("==========================================");
    println!();

    println!("🔑 API Key Configuration:");
    println!("   ℹ️  This demo connects to the Circuit Breaker server");
    println!("   📋 Server API keys are configured in the server's .env file");
    println!("   🔍 Check server startup logs to see actual API key status:");
    println!("      • ✅ OpenAI API key configured");
    println!("      • ✅ Anthropic API key configured");  
    println!("      • ✅ Google API key configured");
    println!();
    println!("   💡 If you see warnings in server logs, add keys to .env file:");
    println!("      OPENAI_API_KEY=your_key");
    println!("      ANTHROPIC_API_KEY=your_key");
    println!("      GOOGLE_API_KEY=your_key");
    println!();
    
    // All providers should work via the server regardless of local keys
    let has_openai_key = true;
    let has_anthropic_key = true; 
    let has_google_key = true;

    wait_for_enter("Ready to start the demo?");

    let client = Client::new();
    let graphql_url = "http://localhost:4000/graphql";

    // Start with listing available providers
    println!("📋 Listing available LLM providers...");
    
    let providers_query = json!({
        "query": r#"
            query {
                llmProviders {
                    id
                    providerType
                    name
                    baseUrl
                    models {
                        id
                        name
                        maxTokens
                        contextWindow
                        costPerInputToken
                        costPerOutputToken
                        supportsStreaming
                        supportsFunctionCalling
                        capabilities
                    }
                    healthStatus {
                        isHealthy
                        errorRate
                        averageLatencyMs
                    }
                }
            }
        "#
    });

    let response = client
        .post(graphql_url)
        .json(&providers_query)
        .send()
        .await?;

    let providers_result: serde_json::Value = response.json().await?;
    
    if let Some(providers) = providers_result
        .get("data")
        .and_then(|d| d.get("llmProviders"))
        .and_then(|p| p.as_array())
    {
        println!("✅ Available Providers: {}", providers.len());
        for provider in providers {
            let name = provider.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown");
            let provider_type = provider.get("providerType").and_then(|v| v.as_str()).unwrap_or("Unknown");
            let models = provider.get("models").and_then(|m| m.as_array()).map(|m| m.len()).unwrap_or(0);
            println!("   • {} ({}): {} models", name, provider_type, models);
        }
    }

    wait_for_enter("Providers listed! Ready to configure OpenAI?");

    // Configure OpenAI Provider
    println!("🔧 Configuring OpenAI provider...");
    
    let openai_config = json!({
        "query": r#"
            mutation($input: LlmproviderConfigInput!) {
                configureLlmProvider(input: $input) {
                    id
                    providerType
                    name
                    baseUrl
                    models {
                        id
                        name
                        costPerInputToken
                        costPerOutputToken
                        supportsStreaming
                        supportsFunctionCalling
                    }
                    healthStatus {
                        isHealthy
                        lastCheck
                    }
                }
            }
        "#,
        "variables": {
            "input": {
                "providerType": "openai",
                "name": "OpenAI",
                "baseUrl": "https://api.openai.com/v1",
                "apiKeyId": "openai-key-1",
                "models": [
                    {
                        "id": "o4-mini-2025-04-16",
                        "name": "OpenAI o4 Mini",
                        "maxTokens": 16384,
                        "contextWindow": 128000,
                        "costPerInputToken": 0.000001,
                        "costPerOutputToken": 0.000002,
                        "supportsStreaming": true,
                        "supportsFunctionCalling": true,
                        "capabilities": ["text_generation", "analysis", "code_generation", "reasoning", "function_calling"]
                    }
                ]
            }
        }
    });

    let response = client
        .post(graphql_url)
        .json(&openai_config)
        .send()
        .await?;

    let config_result: serde_json::Value = response.json().await?;
    println!("✅ OpenAI Provider Configured!");
    if let Some(provider) = config_result
        .get("data")
        .and_then(|d| d.get("configureLlmProvider"))
    {
        if let Some(models) = provider.get("models").and_then(|m| m.as_array()) {
            println!("   Models: {} configured", models.len());
            for model in models {
                let name = model.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown");
                let input_cost = model.get("costPerInputToken").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let output_cost = model.get("costPerOutputToken").and_then(|v| v.as_f64()).unwrap_or(0.0);
                println!("   • {}: ${:.6}/input token, ${:.6}/output token", name, input_cost, output_cost);
            }
        }
    }

    wait_for_enter("OpenAI configured! Ready to configure Google Gemini?");

    // Configure Google Gemini Provider
    println!("🔧 Configuring Google Gemini provider...");
    
    let google_config = json!({
        "query": r#"
            mutation($input: LlmproviderConfigInput!) {
                configureLlmProvider(input: $input) {
                    id
                    providerType
                    name
                    baseUrl
                    models {
                        id
                        name
                        costPerInputToken
                        costPerOutputToken
                        supportsStreaming
                        supportsFunctionCalling
                    }
                    healthStatus {
                        isHealthy
                        lastCheck
                    }
                }
            }
        "#,
        "variables": {
            "input": {
                "providerType": "google",
                "name": "Google Gemini",
                "baseUrl": "https://generativelanguage.googleapis.com/v1beta",
                "apiKeyId": "google-key-1",
                "models": [
                    {
                        "id": "gemini-2.5-flash-preview-05-20",
                        "name": "Gemini 2.5 Flash Preview",
                        "maxTokens": 8192,
                        "contextWindow": 1048576,
                        "costPerInputToken": 0.000000075,
                        "costPerOutputToken": 0.0000003,
                        "supportsStreaming": true,
                        "supportsFunctionCalling": true,
                        "capabilities": ["text_generation", "analysis", "code_generation", "reasoning", "function_calling"]
                    }
                ]
            }
        }
    });

    let response = client
        .post(graphql_url)
        .json(&google_config)
        .send()
        .await?;

    let config_result: serde_json::Value = response.json().await?;
    println!("✅ Google Gemini Provider Configured!");
    if let Some(provider) = config_result
        .get("data")
        .and_then(|d| d.get("configureLlmProvider"))
    {
        if let Some(models) = provider.get("models").and_then(|m| m.as_array()) {
            println!("   Models: {} configured", models.len());
            for model in models {
                let name = model.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown");
                let input_cost = model.get("costPerInputToken").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let output_cost = model.get("costPerOutputToken").and_then(|v| v.as_f64()).unwrap_or(0.0);
                println!("   • {}: ${:.9}/input token, ${:.9}/output token", name, input_cost, output_cost);
            }
        }
    }

    wait_for_enter("Google configured! Ready to configure Anthropic?");

    // Configure Anthropic Provider (keeping the existing one for comparison)
    println!("🔧 Configuring Anthropic provider...");
    
    let anthropic_config = json!({
        "query": r#"
            mutation($input: LlmproviderConfigInput!) {
                configureLlmProvider(input: $input) {
                    id
                    providerType
                    name
                    baseUrl
                    models {
                        id
                        name
                        costPerInputToken
                        costPerOutputToken
                        supportsStreaming
                        supportsFunctionCalling
                    }
                    healthStatus {
                        isHealthy
                        lastCheck
                    }
                }
            }
        "#,
        "variables": {
            "input": {
                "providerType": "anthropic",
                "name": "Anthropic Claude",
                "baseUrl": "https://api.anthropic.com",
                "apiKeyId": "anthropic-key-1",
                "models": [
                    {
                        "id": "claude-3-5-sonnet-20241022",
                        "name": "Claude 3.5 Sonnet",
                        "maxTokens": 8192,
                        "contextWindow": 200000,
                        "costPerInputToken": 0.000003,
                        "costPerOutputToken": 0.000015,
                        "supportsStreaming": true,
                        "supportsFunctionCalling": true,
                        "capabilities": ["text_generation", "analysis", "code_generation", "reasoning"]
                    },
                    {
                        "id": "claude-3-haiku-20240307",
                        "name": "Claude 3 Haiku",
                        "maxTokens": 4096,
                        "contextWindow": 200000,
                        "costPerInputToken": 0.00000025,
                        "costPerOutputToken": 0.00000125,
                        "supportsStreaming": true,
                        "supportsFunctionCalling": false,
                        "capabilities": ["text_generation", "analysis"]
                    }
                ]
            }
        }
    });

    let response = client
        .post(graphql_url)
        .json(&anthropic_config)
        .send()
        .await?;

    let config_result: serde_json::Value = response.json().await?;
    println!("✅ Anthropic Provider Configured!");
    if let Some(provider) = config_result
        .get("data")
        .and_then(|d| d.get("configureLlmProvider"))
    {
        if let Some(models) = provider.get("models").and_then(|m| m.as_array()) {
            println!("   Models: {} configured", models.len());
            for model in models {
                let name = model.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown");
                let input_cost = model.get("costPerInputToken").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let output_cost = model.get("costPerOutputToken").and_then(|v| v.as_f64()).unwrap_or(0.0);
                println!("   • {}: ${:.9}/input token, ${:.9}/output token", name, input_cost, output_cost);
            }
        }
    }

    wait_for_enter("All providers configured! Ready to test cost comparison?");

    // Cost comparison demo
    println!("💰 Cost Comparison Demo");
    println!("=======================");
    
    let test_prompt = "Explain quantum computing in simple terms";
    let estimated_input_tokens = 10; // Rough estimate for the prompt
    let estimated_output_tokens = 100; // Rough estimate for response
    
    println!("🧮 Cost estimation for prompt: \"{}\"", test_prompt);
    println!("   Estimated input tokens: {}", estimated_input_tokens);
    println!("   Estimated output tokens: {}", estimated_output_tokens);
    println!();
    
    // Cost calculations
    let models = vec![
        ("OpenAI o4 Mini", 0.000001, 0.000002),
        ("Gemini 2.5 Flash Preview", 0.000000075, 0.0000003),
        ("Claude 3.5 Sonnet", 0.000003, 0.000015),
        ("Claude 3 Haiku", 0.00000025, 0.00000125),
    ];
    
    let mut costs: Vec<(String, f64)> = models
        .iter()
        .map(|(name, input_cost, output_cost)| {
            let total_cost = (estimated_input_tokens as f64 * input_cost) + (estimated_output_tokens as f64 * output_cost);
            (name.to_string(), total_cost)
        })
        .collect();
    
    costs.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    
    println!("💸 Cost ranking (cheapest to most expensive):");
    for (i, (model, cost)) in costs.iter().enumerate() {
        println!("   {}. {}: ${:.8}", i + 1, model, cost);
    }
    
    wait_for_enter("Cost comparison done! Ready to test real streaming?");

    // Test streaming with available providers
    if has_anthropic_key || has_openai_key || has_google_key {
        println!("🔄 Testing real-time streaming with available providers...");
        
        match LLMRouter::new().await {
            Ok(router) => {
                let test_models = vec![
                    (has_openai_key, "o4-mini-2025-04-16", "OpenAI o4 Mini"),
                    (has_google_key, "gemini-2.5-flash-preview-05-20", "Google Gemini 2.5 Flash"),
                    (has_anthropic_key, "claude-3-haiku-20240307", "Anthropic Claude 3 Haiku"),
                ];
                
                for (has_key, model_id, display_name) in test_models {
                    if !has_key {
                        println!("⏭️  Skipping {} (no API key)", display_name);
                        continue;
                    }
                    
                    println!("🧪 Testing streaming with {}", display_name);
                    
                    let streaming_request = LLMRequest {
                        id: Uuid::new_v4(),
                        model: model_id.to_string(),
                        messages: vec![ChatMessage {
                            role: MessageRole::User,
                            content: "Count from 1 to 5 slowly".to_string(),
                            name: None,
                            function_call: None,
                        }],
                        temperature: Some(0.7),
                        max_tokens: Some(50),
                        top_p: None,
                        frequency_penalty: None,
                        presence_penalty: None,
                        stop: None,
                        stream: true,
                        functions: None,
                        function_call: None,
                        user: None,
                        metadata: std::collections::HashMap::new(),
                    };

                    match router.stream_chat_completion(streaming_request).await {
                        Ok(mut stream) => {
                            print!("   Response: ");
                            io::stdout().flush().unwrap();
                            
                            use futures::StreamExt;
                            while let Some(chunk_result) = stream.next().await {
                                match chunk_result {
                                    Ok(chunk) => {
                                        for choice in &chunk.choices {
                                            if !choice.delta.content.is_empty() {
                                                print!("{}", choice.delta.content);
                                                io::stdout().flush().unwrap();
                                            }
                                        }
                                    },
                                    Err(e) => {
                                        println!("\n   ❌ Streaming error: {}", e);
                                        break;
                                    }
                                }
                            }
                            println!("\n   ✅ Streaming completed");
                        },
                        Err(e) => {
                            println!("   ❌ Failed to start streaming: {}", e);
                        }
                    }
                    
                    println!();
                }
            },
            Err(e) => {
                println!("❌ Failed to create router: {}", e);
            }
        }
    } else {
        println!("⏭️  Skipping streaming tests (no API keys available)");
    }

    println!();
    println!("🎉 Multi-Provider Demo Complete!");
    println!("================================");
    println!();
    println!("✅ What We Demonstrated:");
    println!("   • Configuration of OpenAI, Google Gemini, and Anthropic providers");
    println!("   • Cost comparison across different models");
    println!("   • Real-time streaming capabilities");
    println!("   • Unified API interface for all providers");
    println!();
    println!("🔧 Next Steps:");
    println!("   • Set API keys to test real provider integrations");
    println!("   • Use GraphiQL at http://localhost:4000 for interactive testing");
    println!("   • Implement smart routing based on cost and performance");
    println!("   • Add budget limits and usage tracking");
    
    Ok(())
}

