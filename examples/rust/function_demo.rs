// Function system demonstration
// Shows how to create event-driven Docker functions with chaining

use circuit_breaker::models::{
    FunctionDefinition, FunctionId, ContainerConfig, EventTrigger,
    FunctionSchema, InputMapping, ChainCondition, FunctionChain, PlaceId,
    TransitionId, Token, TriggerEvent
};
use circuit_breaker::engine::{
    FunctionEngine, InMemoryFunctionStorage, EventBus, TokenEvents
};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Circuit Breaker Function System Demo");
    println!("========================================");

    // Create function storage and engine
    let storage = Box::new(InMemoryFunctionStorage::new());
    let function_engine = FunctionEngine::new(storage);
    let event_bus = EventBus::new();

    // Create a simple data processing function
    let data_processor = create_data_processor_function();
    let processor_id = data_processor.id.clone();
    
    // Create a notification function that chains after the processor
    let notifier = create_notification_function(&processor_id);
    
    // Register functions
    function_engine.create_function(data_processor).await?;
    function_engine.create_function(notifier).await?;

    println!("\n📋 Registered Functions:");
    let functions = function_engine.list_functions().await?;
    for func in &functions {
        println!("  • {} ({})", func.name, func.id);
        println!("    Triggers: {} events", func.triggers.len());
        println!("    Chains: {} functions", func.chains.len());
    }

    // Simulate workflow events
    println!("\n🎯 Simulating Workflow Events:");
    
    // Create a token and emit events
    let mut token = Token::new("demo-workflow", PlaceId::from("start"));
    token.data = serde_json::json!({
        "user_id": "user123",
        "order_id": "order456",
        "amount": 99.99
    });

    // Emit token created event
    event_bus.emit_token_created(&token).await?;

    // Transition token and emit transition event using the combined method
    token.transition_to_with_events(
        PlaceId::from("processing"), 
        TransitionId::from("process"),
        &event_bus
    ).await?;

    // Process events with function engine
    println!("\n⚡ Processing Events:");
    let event = TriggerEvent::token_created(
        "demo-workflow",
        token.id,
        PlaceId::from("processing"),
        token.data.clone(),
        token.metadata.clone(),
    );

    let execution_ids = function_engine.process_event(event).await?;
    println!("  Triggered {} function executions", execution_ids.len());
    
    for execution_id in execution_ids {
        println!("  • Execution ID: {}", execution_id);
        
        if let Some(execution) = function_engine.get_execution(&execution_id).await? {
            println!("    Function: {}", execution.function_id);
            println!("    Status: {:?}", execution.status);
            println!("    Input: {}", execution.input_data);
        }
    }

    println!("\n✅ Demo completed successfully!");
    println!("\nNext steps:");
    println!("  1. Implement actual Docker execution");
    println!("  2. Add function chaining with proper async handling");
    println!("  3. Integrate with rules engine for conditional execution");
    println!("  4. Add GraphQL API for function management");

    Ok(())
}

/// Create a data processing function that triggers on token creation
fn create_data_processor_function() -> FunctionDefinition {
    let container = ContainerConfig::new("node:18-alpine")
        .with_working_dir("/app")
        .with_env_var("NODE_ENV", "production")
        .with_exec(vec![
            "node".to_string(),
            "-e".to_string(),
            "console.log(JSON.stringify({processed: true, timestamp: new Date().toISOString()}))".to_string()
        ]);

    let trigger = EventTrigger::on_token_created("token_created", Some(PlaceId::from("processing")))
        .with_input_mapping(InputMapping::MergedData);

    let input_schema = FunctionSchema::new(serde_json::json!({
        "type": "object",
        "properties": {
            "user_id": {"type": "string"},
            "order_id": {"type": "string"},
            "amount": {"type": "number"}
        },
        "required": ["user_id", "order_id", "amount"]
    }))
    .with_description("Order data for processing")
    .with_example(serde_json::json!({
        "user_id": "user123",
        "order_id": "order456", 
        "amount": 99.99
    }));

    let output_schema = FunctionSchema::new(serde_json::json!({
        "type": "object",
        "properties": {
            "processed": {"type": "boolean"},
            "timestamp": {"type": "string"},
            "result": {"type": "object"}
        },
        "required": ["processed", "timestamp"]
    }))
    .with_description("Processing result");

    let mut function = FunctionDefinition::new(
        FunctionId::from("data-processor"),
        "Data Processor",
        container,
    )
    .with_input_schema(input_schema)
    .with_output_schema(output_schema);

    function.add_trigger(trigger);
    function.description = Some("Processes order data and prepares it for downstream systems".to_string());
    function.tags = vec!["data".to_string(), "processing".to_string(), "orders".to_string()];

    function
}

/// Create a notification function that chains after the data processor
fn create_notification_function(processor_id: &FunctionId) -> FunctionDefinition {
    let container = ContainerConfig::new("alpine:3.17")
        .with_exec(vec![
            "sh".to_string(),
            "-c".to_string(),
            "echo '{\"notification_sent\": true, \"timestamp\": \"'$(date -Iseconds)'\"}'".to_string()
        ]);

    let trigger = EventTrigger::on_function_completed(
        "processor_completed",
        processor_id.clone(),
        true // Only trigger on success
    )
    .with_input_mapping(InputMapping::FieldMapping({
        let mut mapping = HashMap::new();
        mapping.insert("result".to_string(), "data".to_string());
        mapping.insert("timestamp".to_string(), "processed_at".to_string());
        mapping
    }));

    let chain = FunctionChain {
        target_function: FunctionId::from("audit-logger"),
        condition: ChainCondition::Always,
        input_mapping: InputMapping::FullOutput,
        delay: Some(chrono::Duration::seconds(5)),
        description: Some("Log notification for audit trail".to_string()),
    };

    let mut function = FunctionDefinition::new(
        FunctionId::from("notifier"),
        "Order Notification Service",
        container,
    );

    function.add_trigger(trigger);
    function.add_chain(chain);
    function.description = Some("Sends notifications when orders are processed".to_string());
    function.tags = vec!["notification".to_string(), "messaging".to_string()];

    function
} 