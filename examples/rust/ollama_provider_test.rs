//! Ollama Provider Test Example
//!
//! This example demonstrates how to use the Ollama provider with Circuit Breaker.
//!
//! Prerequisites:
//! - Ollama must be running locally (typically on http://localhost:11434)
//! - At least one model must be pulled (e.g., `ollama pull llama2`)
//!
//! Usage:
//! ```bash
//! # Start Ollama (if not already running)
//! ollama serve
//!
//! # Pull a model (if not already available)
//! ollama pull llama2
//!
//! # Run the example
//! cargo run --example ollama_provider_test
//! ```

use circuit_breaker::llm::{
    providers::ollama::{check_availability, create_client_from_env},
    traits::LLMProviderClient,
    ChatMessage, EmbeddingsInput, EmbeddingsRequest, LLMRequest, MessageRole,
};
use std::env;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🦙 Circuit Breaker - Ollama Provider Test");
    println!("==========================================");

    // Check if Ollama is available
    let base_url =
        env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://localhost:11434".to_string());
    println!("🔍 Checking Ollama availability at: {}", base_url);

    if !check_availability(&base_url).await {
        eprintln!("❌ Ollama is not available at {}. Please ensure:", base_url);
        eprintln!("   1. Ollama is installed and running");
        eprintln!("   2. The base URL is correct");
        eprintln!("   3. At least one model is pulled (e.g., 'ollama pull llama2')");
        return Ok(());
    }

    println!("✅ Ollama is available!");

    // Create Ollama client
    let client = match create_client_from_env() {
        Ok(client) => client,
        Err(e) => {
            eprintln!("❌ Failed to create Ollama client: {}", e);
            return Ok(());
        }
    };

    println!("✅ Ollama client created successfully");

    // Test health check
    println!("\n🏥 Testing health check...");
    match client.health_check("").await {
        Ok(is_healthy) => {
            if is_healthy {
                println!("✅ Health check passed");
            } else {
                println!("⚠️  Health check failed");
            }
        }
        Err(e) => {
            eprintln!("❌ Health check error: {}", e);
        }
    }

    // Get available models
    println!("\n📋 Available models:");
    let models = client.get_available_models();
    for model in models {
        println!("  - {} ({})", model.name, model.id);
        println!(
            "    Context: {} tokens, Max output: {} tokens",
            model.context_window, model.max_output_tokens
        );
        println!("    Capabilities: {:?}", model.capabilities);
    }

    // Test model fetching from Ollama instance
    println!("\n🔍 Fetching models from Ollama instance...");
    match client.fetch_available_models().await {
        Ok(fetched_models) => {
            println!(
                "✅ Successfully fetched {} models from Ollama:",
                fetched_models.len()
            );
            for model in fetched_models.iter().take(5) {
                // Show first 5
                println!("  - {}", model.name);
            }
            if fetched_models.len() > 5 {
                println!("  ... and {} more", fetched_models.len() - 5);
            }
        }
        Err(e) => {
            println!("⚠️  Could not fetch models from Ollama: {}", e);
            println!("   This might mean no models are currently pulled.");
        }
    }

    // Test chat completion
    let model = env::var("OLLAMA_DEFAULT_MODEL").unwrap_or_else(|_| "qwen2.5-coder:3b".to_string());
    println!("\n💬 Testing chat completion with model: {}", model);

    let request = LLMRequest {
        id: Uuid::new_v4(),
        model: model.clone(),
        messages: vec![
            ChatMessage {
                role: MessageRole::System,
                content: "You are a helpful assistant. Be concise in your responses.".to_string(),
                name: None,
                function_call: None,
            },
            ChatMessage {
                role: MessageRole::User,
                content: "Count from 1 to 5 slowly".to_string(),
                name: None,
                function_call: None,
            },
        ],
        temperature: Some(0.7),
        max_tokens: Some(100),
        top_p: None,
        frequency_penalty: None,
        presence_penalty: None,
        stop: None,
        stream: Some(false),
        functions: None,
        function_call: None,
        user: None,
        metadata: std::collections::HashMap::new(),
    };

    match client.chat_completion(&request, "").await {
        Ok(response) => {
            println!("✅ Chat completion successful!");
            println!("📊 Response details:");
            println!("  Model: {}", response.model);
            println!("  Provider: {:?}", response.provider);
            println!("  Prompt tokens: {}", response.usage.prompt_tokens);
            println!("  Completion tokens: {}", response.usage.completion_tokens);
            println!("  Total tokens: {}", response.usage.total_tokens);
            println!("  Estimated cost: ${:.6}", response.usage.estimated_cost);
            println!("  Latency: {}ms", response.routing_info.latency_ms);

            if let Some(choice) = response.choices.first() {
                println!("\n🤖 Assistant response:");
                println!("  {}", choice.message.content);
            }
        }
        Err(e) => {
            eprintln!("❌ Chat completion failed: {}", e);
            eprintln!("   Common issues:");
            eprintln!(
                "   1. Model '{}' not found. Try 'ollama pull {}'",
                model, model
            );
            eprintln!("   2. Ollama server overloaded or out of memory");
            eprintln!("   3. Model name incorrect (check 'ollama list')");
        }
    }

    // Test embeddings (if embedding model is available)
    let embedding_model = env::var("OLLAMA_EMBEDDING_MODEL")
        .unwrap_or_else(|_| "nomic-embed-text:latest".to_string());
    println!("\n🔮 Testing embeddings with model: {}", embedding_model);

    // Test single text embedding
    let embedding_request = EmbeddingsRequest {
        id: uuid::Uuid::new_v4(),
        model: embedding_model.clone(),
        input: EmbeddingsInput::Text("Hello, this is a test sentence for embeddings.".to_string()),
        user: None,
        metadata: std::collections::HashMap::new(),
    };

    match client.embeddings(&embedding_request, "").await {
        Ok(embedding_response) => {
            println!("✅ Single text embedding successful!");
            println!("📊 Embedding details:");
            println!("  Model: {}", embedding_response.model);
            println!("  Provider: {:?}", embedding_response.provider);
            println!(
                "  Prompt tokens: {}",
                embedding_response.usage.prompt_tokens
            );
            println!("  Total tokens: {}", embedding_response.usage.total_tokens);
            println!(
                "  Estimated cost: ${:.6}",
                embedding_response.usage.estimated_cost
            );

            if let Some(embedding_data) = embedding_response.data.first() {
                println!("  Embedding dimension: {}", embedding_data.embedding.len());
                println!(
                    "  First 5 values: {:?}",
                    &embedding_data.embedding[..5.min(embedding_data.embedding.len())]
                );
            }
        }
        Err(e) => {
            eprintln!("❌ Single text embedding failed: {}", e);
            eprintln!("   Common issues:");
            eprintln!(
                "   1. Model '{}' not found. Try 'ollama pull {}'",
                embedding_model, embedding_model
            );
            eprintln!("   2. Model is not an embedding model");
        }
    }

    // Test batch embeddings
    println!("\n📚 Testing batch embeddings...");
    let batch_embedding_request = EmbeddingsRequest {
        id: uuid::Uuid::new_v4(),
        model: embedding_model.clone(),
        input: EmbeddingsInput::TextArray(vec![
            "This is the first sentence.".to_string(),
            "This is the second sentence.".to_string(),
            "This is the third sentence.".to_string(),
        ]),
        user: None,
        metadata: std::collections::HashMap::new(),
    };

    match client.embeddings(&batch_embedding_request, "").await {
        Ok(batch_response) => {
            println!("✅ Batch embeddings successful!");
            println!("📊 Batch details:");
            println!("  Number of embeddings: {}", batch_response.data.len());
            println!("  Total tokens: {}", batch_response.usage.total_tokens);

            for (i, embedding_data) in batch_response.data.iter().enumerate() {
                println!(
                    "  Embedding {}: {} dimensions",
                    i + 1,
                    embedding_data.embedding.len()
                );
            }
        }
        Err(e) => {
            eprintln!("❌ Batch embeddings failed: {}", e);
        }
    }

    // Test streaming (optional)
    if env::var("TEST_STREAMING").unwrap_or_default() == "true" {
        println!("\n🌊 Testing streaming chat completion...");

        let mut streaming_request = request.clone();
        streaming_request.stream = Some(true);
        streaming_request.messages = vec![ChatMessage {
            role: MessageRole::User,
            content: "Create me an elevator pitch for selling GitLab".to_string(),
            name: None,
            function_call: None,
        }];

        match client
            .chat_completion_stream(streaming_request, "".to_string())
            .await
        {
            Ok(mut stream) => {
                println!("✅ Streaming started...");
                use futures::StreamExt;

                let mut chunk_count = 0;
                while let Some(chunk_result) = stream.next().await {
                    match chunk_result {
                        Ok(chunk) => {
                            chunk_count += 1;
                            if let Some(choice) = chunk.choices.first() {
                                print!("{}", choice.delta.content);
                                std::io::Write::flush(&mut std::io::stdout()).unwrap();
                            }

                            if let Some(finish_reason) =
                                chunk.choices.first().and_then(|c| c.finish_reason.as_ref())
                            {
                                if finish_reason == "stop" {
                                    println!("\n✅ Streaming completed ({} chunks)", chunk_count);
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("\n❌ Streaming error: {}", e);
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("❌ Streaming failed: {}", e);
            }
        }
    }

    println!("\n🎉 Ollama provider test completed!");
    println!("\n💡 Tips:");
    println!(
        "  - Set OLLAMA_DEFAULT_MODEL to test different chat models (current: {})",
        model
    );
    println!(
        "  - Set OLLAMA_EMBEDDING_MODEL to test different embedding models (current: {})",
        embedding_model
    );
    println!("  - Set TEST_STREAMING=true to test streaming responses");
    println!("  - Set OLLAMA_BASE_URL to test remote Ollama instances");
    println!("\n📋 Model recommendations:");
    println!("  - Chat: qwen2.5-coder:3b, gemma3:4b");
    println!("  - Code: qwen2.5-coder:3b");
    println!("  - Embeddings: nomic-embed-text:latest");

    Ok(())
}
