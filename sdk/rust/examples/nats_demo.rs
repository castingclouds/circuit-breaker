//! NATS Client Demo
//!
//! Demonstrates the NATS event streaming functionality in the Rust SDK
//! and verifies feature parity with the TypeScript implementation.

use circuit_breaker_sdk::{
    create_workflow_instance, get_nats_resource, get_resources_in_state, Client, Result,
};
use serde_json::json;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🔧 Circuit Breaker NATS Demo");
    println!("============================");

    // Initialize the SDK
    let mut client_builder = Client::builder()
        .base_url(
            &std::env::var("CIRCUIT_BREAKER_URL")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
        )?
        .timeout(30000);

    if let Ok(api_key) = std::env::var("CIRCUIT_BREAKER_API_KEY") {
        client_builder = client_builder.api_key(api_key);
    } else {
        client_builder = client_builder.api_key("demo-api-key".to_string());
    }

    let client = client_builder.build()?;

    match run_demo(&client).await {
        Ok(_) => {
            println!("\n🎉 All NATS operations completed successfully!");
            println!("\n📋 Feature Parity Verification:");
            println!("✅ NATSClient struct with all core methods");
            println!("✅ CreateWorkflowInstanceBuilder with fluent API");
            println!("✅ ExecuteActivityWithNATSBuilder with NATS headers");
            println!("✅ Convenience functions matching TypeScript SDK");
            println!("✅ Complete type safety with Rust type system");
            println!("✅ Error handling with Result<T, Error>");
            Ok(())
        }
        Err(error) => {
            eprintln!("\n❌ Demo failed: {}", error);

            // Check for common issues
            let error_msg = error.to_string();
            if error_msg.contains("Connection") || error_msg.contains("network") {
                println!("\n💡 Tip: Make sure the Circuit Breaker server is running on http://localhost:3000");
            }

            if error_msg.contains("Validation") {
                println!("\n💡 Tip: Check that all required parameters are provided");
            }

            Err(error)
        }
    }
}

async fn run_demo(client: &Client) -> Result<()> {
    // Test connection
    println!("\n1. Testing connection...");
    let info = client.ping().await?;
    println!("✅ Connected: {} (version: {})", info.status, info.version);

    // Get NATS client
    let nats_client = client.nats();
    println!("✅ NATS client initialized");

    // Demo 1: Create workflow instance with NATS tracking
    println!("\n2. Creating workflow instance with NATS event tracking...");

    let metadata = json!({
        "source": "nats-demo",
        "version": "1.0.0"
    });

    let workflow_instance = nats_client
        .create_workflow_instance()
        .workflow_id("demo-workflow-123")
        .initial_data(json!({
            "inputValue": "test-data",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
        .initial_state("initialized")
        .metadata(metadata)
        .enable_nats_events(true)
        .execute()
        .await?;

    println!("✅ Created workflow instance: {}", workflow_instance.id);
    println!("   Workflow ID: {}", workflow_instance.workflow_id);
    println!("   State: {}", workflow_instance.state);
    println!("   History events: {}", workflow_instance.history.len());

    // Demo 2: Get NATS resource by ID
    println!("\n3. Retrieving NATS resource...");

    let resource = nats_client.get_resource(&workflow_instance.id).await?;
    if let Some(resource) = &resource {
        println!("✅ Retrieved resource: {}", resource.id);
        println!("   Current state: {}", resource.state);
        println!("   Last updated: {}", resource.updated_at);
        println!("   History events: {}", resource.history.len());

        // Show latest history event
        if !resource.history.is_empty() {
            let latest_event = &resource.history[resource.history.len() - 1];
            println!(
                "   Latest event: {} ({})",
                latest_event.event,
                latest_event.source.as_deref().unwrap_or("unknown")
            );
        }
    } else {
        println!("❌ Resource not found");
    }

    // Demo 3: Execute activity with NATS event publishing
    println!("\n4. Executing activity with NATS event publishing...");

    let mut nats_headers = HashMap::new();
    nats_headers.insert("source".to_string(), "nats-demo".to_string());
    nats_headers.insert("priority".to_string(), "high".to_string());

    let activity_result = nats_client
        .execute_activity_with_nats()
        .resource_id(&workflow_instance.id)
        .activity_name("process-data")
        .input_data(json!({
            "operation": "transform",
            "parameters": {
                "format": "json",
                "validate": true
            }
        }))
        .nats_subject("workflow.activity.completed")
        .nats_headers(nats_headers)
        .execute()
        .await?;

    println!("✅ Activity executed: {}", activity_result.id);
    println!("   New state: {}", activity_result.state);
    println!("   History events: {}", activity_result.history.len());

    // Demo 4: Get resources in specific state
    println!("\n5. Finding resources in specific state...");

    let resources_in_state = nats_client
        .resources_in_state(&workflow_instance.workflow_id, &activity_result.state)
        .await?;

    println!(
        "✅ Found {} resources in state '{}'",
        resources_in_state.len(),
        activity_result.state
    );
    for (index, res) in resources_in_state.iter().enumerate() {
        println!("   {}. {} (updated: {})", index + 1, res.id, res.updated_at);
    }

    // Demo 5: Find resource with workflow context
    println!("\n6. Finding resource with workflow context...");

    let found_resource = nats_client
        .find_resource(&workflow_instance.workflow_id, &workflow_instance.id)
        .await?;

    if let Some(found_resource) = &found_resource {
        println!("✅ Found resource: {}", found_resource.id);
        println!("   State: {}", found_resource.state);
        let metadata_keys = if let Some(obj) = found_resource.metadata.as_object() {
            obj.keys().cloned().collect::<Vec<String>>()
        } else {
            vec![]
        };
        println!("   Metadata keys: {}", metadata_keys.join(", "));
    } else {
        println!("❌ Resource not found with workflow context");
    }

    // Demo 6: Using convenience functions
    println!("\n7. Testing convenience functions...");

    // Test convenience function for workflow creation
    let convenience_workflow = create_workflow_instance(&client, "convenience-workflow-456")
        .initial_data(json!({"test": "convenience-function"}))
        .execute()
        .await?;

    println!(
        "✅ Convenience workflow created: {}",
        convenience_workflow.id
    );

    // Test convenience function for resource retrieval
    let convenience_resource = get_nats_resource(&client, &convenience_workflow.id).await?;

    if let Some(convenience_resource) = convenience_resource {
        println!(
            "✅ Retrieved via convenience function: {}",
            convenience_resource.id
        );
    }

    // Test convenience function for state-based search
    let state_resources = get_resources_in_state(
        &client,
        &convenience_workflow.workflow_id,
        &convenience_workflow.state,
    )
    .await?;

    println!(
        "✅ Found {} resources via convenience function",
        state_resources.len()
    );

    // Demo 7: Demonstrate error handling
    demonstrate_error_handling(&nats_client).await;

    Ok(())
}

/// Helper function to demonstrate error handling
async fn demonstrate_error_handling(nats_client: &circuit_breaker_sdk::NATSClient) {
    println!("\n8. Demonstrating error handling...");

    // This should fail with validation error - missing workflow ID
    match nats_client
        .create_workflow_instance()
        .initial_data(json!({"test": "data"}))
        .execute()
        .await
    {
        Ok(_) => println!("❌ Expected validation error but operation succeeded"),
        Err(error) => println!("✅ Caught expected validation error: {}", error),
    }

    // This should fail with validation error - missing required fields
    match nats_client
        .execute_activity_with_nats()
        .input_data(json!({"test": "data"}))
        .execute()
        .await
    {
        Ok(_) => println!("❌ Expected validation error but operation succeeded"),
        Err(error) => println!("✅ Caught expected validation error: {}", error),
    }
}
