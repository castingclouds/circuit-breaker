//! Basic usage example for the Circuit Breaker Rust SDK
//!
//! This example demonstrates the core functionality of the SDK,
//! showing how to create workflows, resources, agents, and more.

use circuit_breaker_sdk::{
    create_agent, create_chat, create_resource, create_workflow,
    rules::{evaluate_rule, RuleBuilderStandalone},
    Client, PaginationOptions, Result, COMMON_MODELS,
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🔧 Circuit Breaker Rust SDK Example");
    println!("===================================");

    // ============================================================================
    // 1. Create Client
    // ============================================================================

    println!("\n1. Creating client...");

    let mut client_builder = Client::builder()
        .base_url(
            &std::env::var("CIRCUIT_BREAKER_URL")
                .unwrap_or_else(|_| "http://localhost:4000".to_string()),
        )?
        .timeout(30000);

    if let Ok(api_key) = std::env::var("CIRCUIT_BREAKER_API_KEY") {
        client_builder = client_builder.api_key(api_key);
    }

    let client = client_builder.build()?;

    // Test connection
    match client.ping().await {
        Ok(response) => {
            println!("✅ Connected to Circuit Breaker v{}", response.version);

            match client.info().await {
                Ok(info) => {
                    println!("📊 Server: {}, Features: {:?}", info.name, info.features);
                }
                Err(_) => println!("📊 Server info not available"),
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to connect: {}", e);
            return Err(e);
        }
    }

    // ============================================================================
    // 2. Create Workflow
    // ============================================================================

    println!("\n2. Creating workflow...");

    // Using the builder pattern
    let workflow_definition = create_workflow("Order Processing Example")
        .set_description("A simple order processing workflow")
        .add_state("pending", "normal")
        .add_state("validating", "normal")
        .add_state("processing", "normal")
        .add_state("completed", "final")
        .add_state("cancelled", "final")
        .add_transition("pending", "validating", "validate")
        .add_transition("validating", "processing", "approve")
        .add_transition("validating", "cancelled", "reject")
        .add_transition("processing", "completed", "complete")
        .add_transition("processing", "cancelled", "cancel")
        .set_initial_state("pending")
        .build();

    let workflow = client
        .workflows()
        .create()
        .name(workflow_definition.name)
        .description(workflow_definition.description.unwrap_or_default())
        .build()
        .await?;

    println!(
        "✅ Created workflow: {} ({})",
        workflow.name(),
        workflow.id()
    );

    // ============================================================================
    // 3. Create Resource
    // ============================================================================

    println!("\n3. Creating resource...");

    let resource_definition = create_resource(workflow.id())
        .add_data("orderId", json!("ORD-2024-001"))
        .add_data("customerId", json!("CUST-123"))
        .add_data("amount", json!(299.99))
        .add_data(
            "items",
            json!([
                {"id": "ITEM-1", "name": "Product A", "price": 199.99},
                {"id": "ITEM-2", "name": "Product B", "price": 100.0}
            ]),
        )
        .set_initial_state("pending")
        .build();

    let resource = client
        .resources()
        .create()
        .set_workflow_id(resource_definition.workflow_id)
        .add_data("orderId", json!("ORD-2024-001"))
        .add_data("customerId", json!("CUST-123"))
        .add_data("amount", json!(299.99))
        .set_initial_state("pending")
        .build()
        .await?;

    println!(
        "✅ Created resource: {} in state '{}'",
        resource.id(),
        resource.state().unwrap_or("unknown")
    );

    // ============================================================================
    // 4. Create Function (Skipped - handled via activities)
    // ============================================================================

    println!("\n4. Function creation (skipped - functions are handled via workflow activities)");

    // ============================================================================
    // 5. Server-Side Rule Storage & Evaluation
    // ============================================================================

    println!("\n5. Testing server-side rule storage...");

    // Try server-side rule creation and evaluation
    let rule_result = async {
        // Create a rule on the server (if available)
        let rule = client
            .rules()
            .create()
            .name("High Value Order Rule")
            .description("Detects orders over $1000 for approval workflow")
            .field_greater_than("amount", json!(1000))
            .add_tag("e-commerce")
            .add_tag("validation")
            .add_tag("high-value")
            .build()
            .await?;

        println!("✅ Created server rule: {} ({})", rule.name(), rule.id());

        // Test rule evaluation on the server
        let evaluation = client
            .rules()
            .evaluate(
                rule.id(),
                json!({
                    "orderId": "ORD-2024-001",
                    "customerId": "CUST-123",
                    "amount": 1500.0
                }),
            )
            .await?;

        let status = if evaluation.passed { "✅" } else { "❌" };
        println!("{} Server rule evaluation: {}", status, evaluation.reason);

        // List all rules
        let rules = client.rules().list().await?;
        println!("📋 Found {} rules on server:", rules.len());
        for rule in rules {
            println!(
                "  - {} ({}): {}",
                rule.name(),
                rule.id(),
                rule.description()
            );
        }

        Ok::<(), circuit_breaker_sdk::Error>(())
    }
    .await;

    if let Err(e) = rule_result {
        println!("⚠️ Server-side rules not available: {}", e);

        // Fallback to client-side rule evaluation
        println!("\n📋 Falling back to client-side rule evaluation...");

        let high_value_rule = RuleBuilderStandalone::field_greater_than(
            "high_value",
            "High value order",
            "amount",
            json!(1000),
        )
        .build();

        let resource_data = json!({
            "orderId": "ORD-2024-001",
            "customerId": "CUST-123",
            "amount": 299.99
        });

        let result = evaluate_rule(&high_value_rule, &resource_data);
        let status = if result.passed { "✅" } else { "❌" };
        println!("{} Client-side evaluation: {}", status, result.reason);
    }

    // ============================================================================
    // 6. Create Agent (if LLM is available)
    // ============================================================================

    println!("\n6. Creating agent...");

    let agent_result = async {
        let agent_definition = create_agent("Order Support Agent")
            .set_description("AI agent for customer order support")
            .set_type("conversational")
            .set_llm_provider("openai")
            .set_model(COMMON_MODELS::GPT_4O_MINI)
            .set_temperature(0.7)
            .set_max_tokens(500)
            .set_system_prompt(
                "You are a helpful customer service agent for an e-commerce platform. \
                Help customers with their orders, returns, and general inquiries. \
                Be polite, professional, and concise.",
            )
            .add_tool(
                "lookup_order",
                "Look up order details by order ID",
                json!({
                    "orderId": {"type": "string", "description": "The order ID to lookup"}
                }),
            )
            .set_memory("short_term", json!({"max_entries": 10}))
            .build();

        let agent = client
            .agents()
            .create()
            .name(agent_definition.name)
            .description(agent_definition.description.unwrap_or_default())
            .set_type("conversational")
            .set_llm_provider("openai")
            .set_model(COMMON_MODELS::GPT_4O_MINI)
            .set_temperature(0.7)
            .set_max_tokens(500)
            .set_system_prompt(
                "You are a helpful customer service agent for an e-commerce platform.",
            )
            .build()
            .await?;

        println!("✅ Created agent: {} ({})", agent.name(), agent.id());

        // Chat with the agent
        let chat_response = agent
            .send_message("Hi, I have a question about my order ORD-2024-001")
            .await?;
        println!("🤖 Agent response: {}", chat_response);

        Ok::<circuit_breaker_sdk::Agent, circuit_breaker_sdk::Error>(agent)
    }
    .await;

    match agent_result {
        Ok(_agent) => {
            println!("✅ Agent creation and testing successful");
        }
        Err(e) => {
            println!("⚠️ Agent creation skipped (LLM not available): {}", e);
        }
    }

    // ============================================================================
    // 7. Multi-Provider LLM Usage (if available)
    // ============================================================================

    println!("\n7. Multi-Provider LLM capabilities...");

    let llm_result = async {
        let llm_client = client.llm();

        // Test different providers
        println!("\n🧪 Testing multiple providers:");

        let test_models = vec![
            ("OpenAI GPT-4", COMMON_MODELS::GPT_4O_MINI),
            ("Claude Haiku", COMMON_MODELS::CLAUDE_3_HAIKU),
            ("Gemini Flash", COMMON_MODELS::GEMINI_FLASH),
        ];

        for (name, model) in test_models {
            let start_time = std::time::Instant::now();
            match llm_client
                .chat(
                    model,
                    "Explain workflow automation benefits in one sentence.",
                )
                .await
            {
                Ok(response) => {
                    let latency = start_time.elapsed().as_millis();
                    println!("✅ {} ({}ms): {}", name, latency, response);
                }
                Err(e) => {
                    println!("⚠️ {} unavailable: {}", name, e);
                }
            }
        }

        // Test virtual models for smart routing
        println!("\n🎯 Testing smart routing with virtual models:");

        let virtual_models = vec![
            ("Auto-Route", "auto"),
            ("Cost-Optimal", "cb:cost-optimal"),
            ("Fastest", "cb:fastest"),
        ];

        for (name, model) in virtual_models {
            match llm_client
                .chat(model, "What is the capital of France?")
                .await
            {
                Ok(response) => {
                    println!("🎯 {}: {}", name, response);
                }
                Err(e) => {
                    println!("⚠️ {} routing failed: {}", name, e);
                }
            }
        }

        // Cost comparison demonstration
        println!("\n💰 Provider cost comparison:");
        let cost_tests = vec![
            ("OpenAI", COMMON_MODELS::GPT_4O_MINI, 0.003),
            ("Anthropic", COMMON_MODELS::CLAUDE_3_HAIKU, 0.00025),
            ("Google", COMMON_MODELS::GEMINI_FLASH, 0.0000375),
        ];

        println!("Provider    | Model              | Est. Cost/1K tokens");
        println!("------------|--------------------|-----------------");

        for (provider, model, cost) in cost_tests {
            println!(
                "{:<11}| {:<18} | ${:.6}",
                provider,
                &model[..18.min(model.len())],
                cost
            );
        }

        // Using chat builder with different providers
        println!("\n🔧 Chat builder with provider selection:");
        let chat_builder = create_chat(COMMON_MODELS::CLAUDE_3_HAIKU)
            .set_system_prompt("You are a helpful assistant specializing in workflow automation.")
            .add_user_message("List 3 key benefits of workflow automation")
            .set_temperature(0.2)
            .set_max_tokens(150);

        let chat_result = chat_builder.execute(&llm_client).await?;
        if let Some(choice) = chat_result.choices.first() {
            println!("📝 Claude response: {}", choice.message.content);
        }

        Ok::<(), circuit_breaker_sdk::Error>(())
    }
    .await;

    if let Err(e) = llm_result {
        println!("⚠️ Multi-provider LLM testing skipped: {}", e);

        // Fallback to basic LLM usage
        let llm_client = client.llm();
        {
            match llm_client
                .chat(
                    COMMON_MODELS::GPT_4O_MINI,
                    "Explain the benefits of workflow automation in 2 sentences.",
                )
                .await
            {
                Ok(response) => {
                    println!("🤖 Fallback LLM response: {}", response);
                }
                Err(e) => {
                    println!("⚠️ All LLM usage skipped: {}", e);
                }
            }
        }
    }

    // ============================================================================
    // 8. Execute Workflow (Create Workflow Instance)
    // ============================================================================

    println!("\n8. Creating workflow instance...");

    let workflow_instance = workflow
        .execute_with_input(json!({
            "initialData": {
                "orderId": "ORD-2024-001",
                "customerId": "CUST-123",
                "amount": 299.99
            },
            "metadata": {
                "source": "api",
                "timestamp": chrono::Utc::now().to_rfc3339()
            }
        }))
        .await?;

    println!("✅ Created workflow instance: {}", workflow_instance.id());
    println!("📊 Current status: {:?}", workflow_instance.status());

    // Check instance status after a delay
    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        // In a real implementation, you would refresh and check status
        println!("📊 Instance status check completed");
    });

    // ============================================================================
    // 9. Resource Operations
    // ============================================================================

    println!("\n9. Performing resource operations...");

    // Execute activity on resource using the workflow instance
    match client
        .resources()
        .execute_activity(
            resource.id().to_string(),
            "activity_0",
            Some(json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "performedBy": "sdk-user",
                "notes": "Executed validate activity"
            })),
        )
        .await
    {
        Ok(updated_resource) => {
            println!(
                "✅ Activity executed, resource state: {}",
                updated_resource.state().unwrap_or("unknown")
            );

            // Get resource history
            match client
                .resources()
                .get_history(
                    updated_resource.id().to_string(),
                    Some(PaginationOptions {
                        limit: Some(5),
                        offset: None,
                    }),
                )
                .await
            {
                Ok(history) => {
                    println!("📜 Resource history ({} events):", history.data.len());
                    for event in history.data {
                        println!(
                            "  - {}: {} → {} at {}",
                            event.activity,
                            event.from_state,
                            event.to_state,
                            event.timestamp.format("%H:%M:%S")
                        );
                    }
                }
                Err(e) => {
                    println!("⚠️ Could not fetch resource history: {}", e);
                }
            }

            // Update the workflow instance status
            println!(
                "📊 Instance update - State: {}",
                updated_resource.state().unwrap_or("unknown")
            );
        }
        Err(e) => {
            println!("⚠️ Activity execution failed: {}", e);
        }
    }

    println!("\n✨ Example completed successfully!");
    println!("\n🚀 For comprehensive multi-provider LLM testing, see:");
    println!("   cargo run --example multi_provider_demo");
    println!("\n📖 This demonstrates:");
    println!("   • Provider discovery and health monitoring");
    println!("   • Cost optimization across providers");
    println!("   • Real-time streaming from multiple providers");
    println!("   • Smart routing and virtual models");
    println!("   • Advanced features like function calling");

    Ok(())
}
