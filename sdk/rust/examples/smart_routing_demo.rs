//! LLM Router Demo
//!
//! This example demonstrates how to use the Circuit Breaker SDK's LLM client
//! to make OpenAI-compatible API calls through the Circuit Breaker router.
//! The router handles routing to different providers while maintaining a
//! consistent OpenAI-compatible interface.

use circuit_breaker_sdk::{
    create_balanced_chat, create_chat, create_cost_optimized_chat, create_fast_chat,
    create_smart_chat, ChatCompletionRequest, ChatMessage, ChatRole, CircuitBreakerOptions, Client,
    Result, RoutingStrategy, SmartCompletionRequest, TaskType, COMMON_MODELS,
};
use std::env;
use tokio;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🤖 Circuit Breaker LLM Router Demo");
    println!("==================================");
    println!("🎯 Showcasing Smart Routing & Virtual Models");

    // Initialize the client
    let base_url =
        env::var("CIRCUIT_BREAKER_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
    let api_key = env::var("CIRCUIT_BREAKER_API_KEY").ok();

    let mut client_builder = Client::builder().base_url(&base_url)?;
    if let Some(key) = api_key {
        client_builder = client_builder.api_key(key);
    }
    let client = client_builder.build()?;

    println!("🔗 Connected to Circuit Breaker router at: {}", base_url);
    println!("📡 All LLM calls will be routed through the Circuit Breaker");

    // Test connection
    match client.ping().await {
        Ok(ping) => println!("✅ Circuit Breaker server: {}", ping.message),
        Err(e) => {
            println!("❌ Failed to connect: {}", e);
            println!("   Make sure the Circuit Breaker server is running");
            return Err(e);
        }
    }

    let llm_client = client.llm();

    // ============================================================================
    // 1. List Available Models
    // ============================================================================
    println!("\n1. 📋 Available Models");
    println!("   ------------------");

    match llm_client.list_models().await {
        Ok(models) => {
            println!("   Available models through Circuit Breaker router:");
            for model in models {
                println!("   • {} ({})", model.id, model.owned_by);
            }
        }
        Err(e) => {
            println!("   ⚠️  Could not fetch models: {}", e);
            println!("   Using predefined common models...");
        }
    }

    // ============================================================================
    // 2. Virtual Model Demonstration
    // ============================================================================
    println!("\n2. 🎯 Virtual Model Smart Routing");
    println!("   --------------------------------");

    // Test different virtual models
    let virtual_models = vec![
        ("💰 Cost-Optimized", COMMON_MODELS::SMART_CHEAP),
        ("⚡ Performance-First", COMMON_MODELS::SMART_FAST),
        ("⚖️  Balanced", COMMON_MODELS::SMART_BALANCED),
        ("🎨 Creative", COMMON_MODELS::SMART_CREATIVE),
        ("💻 Coding", COMMON_MODELS::SMART_CODING),
    ];

    for (name, virtual_model) in virtual_models {
        println!("   Testing {}", name);
        let start_time = std::time::Instant::now();

        match llm_client
            .chat(
                virtual_model,
                "Explain circuit breaker pattern in software in one sentence.",
            )
            .await
        {
            Ok(response) => {
                let duration = start_time.elapsed();
                println!("   ✅ {} ({:?}): {}", name, duration, response);
            }
            Err(e) => {
                println!("   ❌ {} failed: {}", name, e);
            }
        }
    }

    // ============================================================================
    // 3. Smart Completion with Circuit Breaker Options
    // ============================================================================
    println!("\n3. 🧠 Smart Completion with Routing Options");
    println!("   ------------------------------------------");

    let smart_request = SmartCompletionRequest {
        model: COMMON_MODELS::SMART_CHEAP.to_string(),
        messages: vec![
            ChatMessage {
                role: ChatRole::System,
                content: "You are a cost-conscious AI assistant.".to_string(),
                name: None,
            },
            ChatMessage {
                role: ChatRole::User,
                content: "Write a short explanation of microservices architecture.".to_string(),
                name: None,
            },
        ],
        temperature: Some(0.7),
        max_tokens: Some(150),
        stream: Some(false),
        circuit_breaker: Some(CircuitBreakerOptions {
            routing_strategy: Some(RoutingStrategy::CostOptimized),
            max_cost_per_1k_tokens: Some(0.01),
            task_type: Some(TaskType::General),
            fallback_models: Some(vec![
                "gpt-3.5-turbo".to_string(),
                "claude-3-haiku-20240307".to_string(),
            ]),
            max_latency_ms: Some(5000),
            require_streaming: Some(false),
            budget_constraint: None,
        }),
    };

    match llm_client.smart_completion(smart_request).await {
        Ok(response) => {
            if let Some(choice) = response.choices.first() {
                println!("   🧠 Smart routed response: {}", choice.message.content);
                if let Some(usage) = response.usage {
                    println!("   💰 Cost-optimized tokens: {} total", usage.total_tokens);
                }
            }
        }
        Err(e) => {
            println!("   ⚠️  Smart completion failed: {}", e);
        }
    }

    // ============================================================================
    // 4. Task-Specific Optimization
    // ============================================================================
    println!("\n4. 🎯 Task-Specific Smart Routing");
    println!("   --------------------------------");

    // Code generation task
    let code_request = create_smart_chat(COMMON_MODELS::SMART_CODING)
        .set_system_prompt("You are an expert programmer.")
        .add_user_message("Write a Python function to sort a list using quicksort")
        .set_task_type(TaskType::CodeGeneration)
        .set_routing_strategy(RoutingStrategy::PerformanceFirst)
        .set_max_cost_per_1k_tokens(0.05)
        .build();

    match llm_client.chat_completion(code_request).await {
        Ok(response) => {
            if let Some(choice) = response.choices.first() {
                println!("   💻 Code generation: {}", choice.message.content);
            }
        }
        Err(e) => {
            println!("   ⚠️  Code generation failed: {}", e);
        }
    }

    // Creative writing task
    let creative_request = create_smart_chat(COMMON_MODELS::SMART_CREATIVE)
        .add_user_message("Write a haiku about distributed systems")
        .set_task_type(TaskType::CreativeWriting)
        .set_temperature(0.9)
        .build();

    match llm_client.chat_completion(creative_request).await {
        Ok(response) => {
            if let Some(choice) = response.choices.first() {
                println!("   🎨 Creative writing: {}", choice.message.content);
            }
        }
        Err(e) => {
            println!("   ⚠️  Creative writing failed: {}", e);
        }
    }

    // ============================================================================
    // 5. Convenience Builders for Different Strategies
    // ============================================================================
    println!("\n5. 🛠️  Convenience Builder Functions");
    println!("   ----------------------------------");

    // Cost-optimized builder
    let cost_response = create_cost_optimized_chat()
        .add_user_message("Summarize the benefits of serverless computing")
        .set_max_cost_per_1k_tokens(0.005)
        .execute(&llm_client)
        .await;

    match cost_response {
        Ok(response) => {
            if let Some(choice) = response.choices.first() {
                println!("   💰 Cost-optimized: {}", choice.message.content);
            }
        }
        Err(e) => {
            println!("   ⚠️  Cost-optimized failed: {}", e);
        }
    }

    // Performance-first builder
    let fast_response = create_fast_chat()
        .add_user_message("Quick: What is Docker?")
        .execute(&llm_client)
        .await;

    match fast_response {
        Ok(response) => {
            if let Some(choice) = response.choices.first() {
                println!("   ⚡ Performance-first: {}", choice.message.content);
            }
        }
        Err(e) => {
            println!("   ⚠️  Performance-first failed: {}", e);
        }
    }

    // Balanced approach
    let balanced_response = create_balanced_chat()
        .add_user_message(
            "Explain the trade-offs between monolithic and microservices architectures",
        )
        .set_fallback_models(vec![
            "gpt-4".to_string(),
            "claude-3-sonnet-20240229".to_string(),
        ])
        .execute(&llm_client)
        .await;

    match balanced_response {
        Ok(response) => {
            if let Some(choice) = response.choices.first() {
                println!("   ⚖️  Balanced: {}", choice.message.content);
            }
        }
        Err(e) => {
            println!("   ⚠️  Balanced failed: {}", e);
        }
    }

    // ============================================================================
    // 6. Smart Streaming with Virtual Models
    // ============================================================================
    println!("\n6. 🌊 Smart Streaming with Virtual Models");
    println!("   ----------------------------------------");

    let streaming_request = ChatCompletionRequest {
        model: COMMON_MODELS::SMART_CREATIVE.to_string(),
        messages: vec![ChatMessage {
            role: ChatRole::User,
            content: "Write a short story about a circuit breaker in distributed systems."
                .to_string(),
            name: None,
        }],
        temperature: Some(0.8),
        max_tokens: Some(300),
        stream: Some(true),
        circuit_breaker: Some(CircuitBreakerOptions {
            routing_strategy: Some(RoutingStrategy::LoadBalanced),
            task_type: Some(TaskType::CreativeWriting),
            require_streaming: Some(true),
            max_latency_ms: Some(3000),
            fallback_models: Some(vec![
                "gpt-4".to_string(),
                "claude-3-opus-20240229".to_string(),
            ]),
            max_cost_per_1k_tokens: None,
            budget_constraint: None,
        }),
        ..Default::default()
    };

    match llm_client.chat_completion_stream(streaming_request).await {
        Ok(mut stream) => {
            use futures::StreamExt;

            print!("   🌊 Streaming response: ");
            std::io::Write::flush(&mut std::io::stdout()).unwrap();

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        if let Some(choice) = chunk.choices.first() {
                            if let Some(content) = &choice.delta.content {
                                print!("{}", content);
                                std::io::Write::flush(&mut std::io::stdout()).unwrap();
                            }
                        }
                    }
                    Err(e) => {
                        if e.to_string().contains("Stream completed") {
                            break;
                        }
                        println!("\n   ⚠️  Stream chunk error: {}", e);
                    }
                }
            }
            println!("\n   ✅ Streaming completed");
        }
        Err(e) => {
            println!("   ⚠️  Streaming failed: {}", e);
        }
    }

    // ============================================================================
    // 7. Budget-Constrained Routing
    // ============================================================================
    println!("\n7. 💰 Budget-Constrained Smart Routing");
    println!("   -------------------------------------");

    use circuit_breaker_sdk::BudgetConstraint;

    let budget_request = create_smart_chat(COMMON_MODELS::SMART_BALANCED)
        .set_system_prompt("You are a budget-conscious technical writer.")
        .add_user_message("Explain Circuit Breaker pattern benefits in 2 sentences")
        .set_circuit_breaker(CircuitBreakerOptions {
            routing_strategy: Some(RoutingStrategy::CostOptimized),
            max_cost_per_1k_tokens: Some(0.01),
            task_type: Some(TaskType::General),
            budget_constraint: Some(BudgetConstraint {
                daily_limit: Some(10.0),
                monthly_limit: Some(100.0),
                per_request_limit: Some(0.05),
            }),
            fallback_models: Some(vec![
                "gpt-3.5-turbo".to_string(),
                "claude-3-haiku-20240307".to_string(),
            ]),
            max_latency_ms: Some(4000),
            require_streaming: Some(false),
        })
        .build();

    match llm_client.chat_completion(budget_request).await {
        Ok(response) => {
            if let Some(choice) = response.choices.first() {
                println!("   💰 Budget-aware response: {}", choice.message.content);
                println!("   📋 Response ID: {}", response.id);
                println!("   🤖 Model selected: {}", response.model);
                if let Some(usage) = response.usage {
                    println!("   📊 Tokens used: {}", usage.total_tokens);
                }
            }
        }
        Err(e) => {
            println!("   ⚠️  Budget-constrained request failed: {}", e);
        }
    }

    // ============================================================================
    // 8. Function Calling (if supported)
    // ============================================================================
    println!("\n8. 🔧 Function Calling Capabilities");
    println!("   ---------------------------------");

    use circuit_breaker_sdk::llm::ChatFunction;

    let function_request = create_chat(COMMON_MODELS::GPT_4)
        .add_user_message("What's the weather like in San Francisco?")
        .add_function(ChatFunction {
            name: "get_weather".to_string(),
            description: Some("Get current weather for a location".to_string()),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "The city and state, e.g. San Francisco, CA"
                    }
                },
                "required": ["location"]
            }),
        })
        .build();

    match llm_client.chat_completion(function_request).await {
        Ok(response) => {
            if let Some(choice) = response.choices.first() {
                println!(
                    "   🔧 Function calling response: {}",
                    choice.message.content
                );
            }
        }
        Err(e) => {
            println!("   ⚠️  Function calling not available: {}", e);
        }
    }

    // ============================================================================
    // Summary
    // ============================================================================
    println!("\n✨ Circuit Breaker Smart Router Demo Complete!");
    println!("===============================================");
    println!("🎯 Key Features Demonstrated:");
    println!("   • 🎯 Virtual Models (smart-fast, smart-cheap, smart-balanced, etc.)");
    println!("   • 🧠 Smart Routing with cost/performance optimization");
    println!("   • 🎨 Task-specific routing (code, creative, analysis)");
    println!("   • 💰 Budget-constrained routing with cost limits");
    println!("   • ⚡ Performance-first vs cost-optimized strategies");
    println!("   • 🌊 Smart streaming with fallback models");
    println!("   • 🛠️  Convenience builders for common patterns");
    println!("   • 🔄 Automatic failover and load balancing");
    println!("\n🚀 Circuit Breaker goes beyond simple API routing!");
    println!("   It provides intelligent model selection, cost optimization,");
    println!("   and task-aware routing while maintaining OpenAI compatibility!");

    Ok(())
}
