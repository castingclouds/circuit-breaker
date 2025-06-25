//! Real-time Subscription Demo
//!
//! This example demonstrates the Circuit Breaker SDK's real-time subscription capabilities,
//! showing how to subscribe to various events including resource updates, workflow events,
//! agent executions, LLM streaming, cost updates, and MCP server status changes.

use circuit_breaker_sdk::{Client, Result};
use std::env;
use std::time::Duration;
use tokio;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔔 Circuit Breaker Real-time Subscription Demo");
    println!("===============================================");

    // Initialize the client
    let base_url =
        env::var("CIRCUIT_BREAKER_URL").unwrap_or_else(|_| "http://localhost:4000".to_string());
    let api_key = env::var("CIRCUIT_BREAKER_API_KEY").ok();

    let mut client_builder = Client::builder().base_url(&base_url)?;
    if let Some(key) = api_key {
        client_builder = client_builder.api_key(key);
    }
    let client = client_builder.build()?;

    // Test connection
    match client.ping().await {
        Ok(ping) => println!("✅ Connected to Circuit Breaker server: {}", ping.message),
        Err(e) => {
            println!("❌ Failed to connect to server: {}", e);
            println!("   Note: This demo shows subscription infrastructure even without a running server");
            println!(
                "   In production, ensure the Circuit Breaker server is running at {}",
                base_url
            );
        }
    }

    println!("\n📡 Real-time Subscription Features:");
    println!("===================================");

    // 1. Resource Updates Subscription
    println!("\n1. 📊 Resource Updates Subscription");
    println!("   ---------------------------------");

    let resource_id = "demo_resource_123";
    println!("   Subscribing to updates for resource: {}", resource_id);

    match client
        .subscriptions()
        .resource_updates()
        .resource_id(resource_id)
        .subscribe(|resource| {
            println!("   📦 Resource Update Received:");
            println!("      • ID: {}", resource.id);
            println!("      • Workflow: {}", resource.workflow_id);
            println!("      • State: {}", resource.state);
            println!("      • Updated: {}", resource.updated_at);
        })
        .await
    {
        Ok(subscription_id) => {
            println!(
                "   ✅ Resource subscription active: {}",
                subscription_id.to_string()
            );
        }
        Err(e) => {
            println!("   ⚠️  Resource subscription setup: {}", e);
        }
    }

    // 2. Workflow Events Subscription
    println!("\n2. 🔄 Workflow Events Subscription");
    println!("   --------------------------------");

    let workflow_id = "demo_workflow_456";
    println!("   Subscribing to events for workflow: {}", workflow_id);

    match client
        .subscriptions()
        .workflow_events()
        .workflow_id(workflow_id)
        .subscribe(|event| {
            println!("   🔄 Workflow Event Received:");
            println!("      • ID: {}", event.id);
            println!("      • Type: {}", event.event_type);
            println!("      • Message: {}", event.message);
            println!("      • Timestamp: {}", event.timestamp);
        })
        .await
    {
        Ok(subscription_id) => {
            println!(
                "   ✅ Workflow subscription active: {}",
                subscription_id.to_string()
            );
        }
        Err(e) => {
            println!("   ⚠️  Workflow subscription setup: {}", e);
        }
    }

    // 3. LLM Streaming Subscription
    println!("\n3. 🤖 LLM Streaming Subscription");
    println!("   ------------------------------");

    let request_id = "llm_request_789";
    println!("   Subscribing to LLM stream for request: {}", request_id);

    match client
        .subscriptions()
        .llm_stream(request_id)
        .subscribe(|chunk| {
            println!("   🤖 LLM Chunk Received:");
            println!("      • Request ID: {}", chunk.id);
            println!("      • Content: {}", chunk.content);
            println!("      • Finished: {}", chunk.finished);
            println!("      • Timestamp: {}", chunk.timestamp);
        })
        .await
    {
        Ok(subscription_id) => {
            println!(
                "   ✅ LLM stream subscription active: {}",
                subscription_id.to_string()
            );
        }
        Err(e) => {
            println!("   ⚠️  LLM stream subscription setup: {}", e);
        }
    }

    // 4. Cost Updates Subscription
    println!("\n4. 💰 Cost Updates Subscription");
    println!("   -----------------------------");

    let user_id = "demo_user_123";
    println!("   Subscribing to cost updates for user: {}", user_id);

    match client
        .subscriptions()
        .cost_updates()
        .subscribe(|update| {
            println!("   💰 Cost Update Received:");
            if let Some(uid) = &update.user_id {
                println!("      • User ID: {}", uid);
            }
            if let Some(pid) = &update.project_id {
                println!("      • Project ID: {}", pid);
            }
            println!("      • Cost: ${:.2}", update.cost);
            println!("      • Timestamp: {}", update.timestamp);
        })
        .await
    {
        Ok(subscription_id) => {
            println!(
                "   ✅ Cost updates subscription active: {}",
                subscription_id.to_string()
            );
        }
        Err(e) => {
            println!("   ⚠️  Cost updates subscription setup: {}", e);
        }
    }

    // 5. Agent Execution Stream
    println!("\n5. 🤖 Agent Execution Stream");
    println!("   --------------------------");

    let execution_id = "agent_exec_456";
    println!("   Subscribing to agent execution: {}", execution_id);

    match client
        .subscriptions()
        .agent_execution_stream()
        .subscribe(|event| {
            println!("   🤖 Agent Execution Event:");
            println!("      • ID: {}", event.id);
            println!("      • Agent: {}", event.agent_id);
            println!("      • Status: {}", event.status);
            println!("      • Timestamp: {}", event.timestamp);
        })
        .await
    {
        Ok(subscription_id) => {
            println!(
                "   ✅ Agent execution subscription active: {}",
                subscription_id.to_string()
            );
        }
        Err(e) => {
            println!("   ⚠️  Agent execution subscription setup: {}", e);
        }
    }

    // 6. MCP Server Status Updates
    println!("\n6. 🔌 MCP Server Status Updates");
    println!("   -----------------------------");

    let server_id = "mcp_server_789";
    println!("   Subscribing to MCP server status: {}", server_id);

    match client
        .subscriptions()
        .mcp_server_status_updates()
        .subscribe(|update| {
            println!("   🔌 MCP Server Status Update:");
            println!("      • Server ID: {}", update.server_id);
            println!("      • Status: {}", update.status);
            if let Some(msg) = &update.message {
                println!("      • Message: {}", msg);
            }
            println!("      • Timestamp: {}", update.timestamp);
        })
        .await
    {
        Ok(subscription_id) => {
            println!(
                "   ✅ MCP status subscription active: {}",
                subscription_id.to_string()
            );
        }
        Err(e) => {
            println!("   ⚠️  MCP status subscription setup: {}", e);
        }
    }

    // 7. MCP Session Events
    println!("\n7. 📡 MCP Session Events");
    println!("   ----------------------");

    println!("   Subscribing to MCP session events for user: {}", user_id);

    match client
        .subscriptions()
        .mcp_session_events()
        .subscribe(|event| {
            println!("   📡 MCP Session Event:");
            println!("      • Session ID: {}", event.session_id);
            println!("      • Event: {}", event.event);
            println!("      • Timestamp: {}", event.timestamp);
        })
        .await
    {
        Ok(subscription_id) => {
            println!(
                "   ✅ MCP session subscription active: {}",
                subscription_id.to_string()
            );
        }
        Err(e) => {
            println!("   ⚠️  MCP session subscription setup: {}", e);
        }
    }

    // 8. Subscription Metrics and Monitoring
    println!("\n8. 📈 Subscription Metrics");
    println!("   ------------------------");

    let metrics = client.subscriptions().manager().metrics();
    println!("   Current Subscription Status:");
    println!("   • Active Subscriptions: {}", metrics.active_count());
    println!("   • Messages Received: {}", metrics.messages_count());

    // 9. Convenience Functions Demo
    println!("\n9. 🛠️  Convenience Functions");
    println!("   -------------------------");

    println!("   Using convenience functions for common subscriptions:");

    // Resource updates convenience function
    match circuit_breaker_sdk::subscribe_resource_updates(
        &client,
        "convenience_resource",
        |resource| {
            println!(
                "   📦 Convenience resource update: {} -> {}",
                resource.id, resource.state
            );
        },
    )
    .await
    {
        Ok(subscription_id) => {
            println!(
                "   ✅ Convenience resource subscription: {}",
                subscription_id.to_string()
            );
        }
        Err(e) => {
            println!("   ⚠️  Convenience resource subscription: {}", e);
        }
    }

    // Workflow events convenience function
    match circuit_breaker_sdk::subscribe_workflow_events(&client, "convenience_workflow", |event| {
        println!(
            "   🔄 Convenience workflow event: {} - {}",
            event.event_type, event.message
        );
    })
    .await
    {
        Ok(subscription_id) => {
            println!(
                "   ✅ Convenience workflow subscription: {}",
                subscription_id.to_string()
            );
        }
        Err(e) => {
            println!("   ⚠️  Convenience workflow subscription: {}", e);
        }
    }

    // 10. Advanced Subscription Patterns
    println!("\n10. 🔬 Advanced Subscription Patterns");
    println!("    ----------------------------------");

    // Multiple subscriptions for the same resource
    println!("    Setting up multiple subscriptions for comprehensive monitoring:");

    let resource_id = "monitored_resource_999";

    // State change monitoring
    match client
        .subscriptions()
        .resource_updates()
        .resource_id(resource_id)
        .subscribe(|resource| {
            println!(
                "    🔍 State Monitor: {} is now in state '{}'",
                resource.id, resource.state
            );
        })
        .await
    {
        Ok(subscription_id) => {
            println!(
                "    ✅ State monitor active: {}",
                subscription_id.to_string()
            );
        }
        Err(e) => {
            println!("    ⚠️  State monitor setup: {}", e);
        }
    }

    // Workflow context monitoring
    match client
        .subscriptions()
        .workflow_events()
        .workflow_id("monitored_workflow")
        .subscribe(|event| {
            println!("    🔍 Workflow Monitor: {}", event.message);
        })
        .await
    {
        Ok(subscription_id) => {
            println!(
                "    ✅ Workflow monitor active: {}",
                subscription_id.to_string()
            );
        }
        Err(e) => {
            println!("    ⚠️  Workflow monitor setup: {}", e);
        }
    }

    // 11. Real-time Dashboard Simulation
    println!("\n11. 📊 Real-time Dashboard Simulation");
    println!("    -----------------------------------");

    println!("    Simulating a real-time dashboard with multiple data streams:");

    let dashboard_metrics = client.subscriptions().manager().metrics();
    println!("    📊 Dashboard Metrics:");
    println!(
        "       • Total Active Streams: {}",
        dashboard_metrics.active_count()
    );
    println!(
        "       • Data Points Received: {}",
        dashboard_metrics.messages_count()
    );

    // Simulate dashboard updates
    for i in 1..=5 {
        sleep(Duration::from_secs(1)).await;
        let current_metrics = client.subscriptions().manager().metrics();
        println!(
            "    📊 Dashboard Update #{}: {} active streams, {} total messages",
            i,
            current_metrics.active_count(),
            current_metrics.messages_count()
        );
    }

    // 12. Subscription Lifecycle Management
    println!("\n12. 🔄 Subscription Lifecycle Management");
    println!("    --------------------------------------");

    println!("    Demonstrating subscription lifecycle:");
    println!("    • All subscriptions are automatically managed");
    println!("    • Auto-reconnection on connection loss");
    println!("    • Graceful cleanup on application shutdown");
    println!("    • Message queuing during disconnections");

    // Wait a bit to show subscriptions are active
    println!("\n⏰ Subscriptions are now active and listening for events...");
    println!("   In a real application, this would continue running indefinitely.");
    println!("   Events would be processed as they arrive from the server.");

    sleep(Duration::from_secs(2)).await;

    // Final metrics
    let final_metrics = client.subscriptions().manager().metrics();
    println!("\n📈 Final Subscription Statistics:");
    println!(
        "   • Active Subscriptions: {}",
        final_metrics.active_count()
    );
    println!(
        "   • Total Messages Processed: {}",
        final_metrics.messages_count()
    );

    println!("\n🎉 Subscription Demo Complete!");
    println!("===============================");
    println!("This demo showcased:");
    println!("• Resource state change subscriptions");
    println!("• Workflow event monitoring");
    println!("• Real-time LLM streaming");
    println!("• Cost update notifications");
    println!("• Agent execution tracking");
    println!("• MCP server status monitoring");
    println!("• MCP session event handling");
    println!("• Subscription metrics and monitoring");
    println!("• Convenience functions for common patterns");
    println!("• Advanced multi-stream monitoring");
    println!("• Real-time dashboard simulation");
    println!("• Subscription lifecycle management");
    println!("\nThe Subscription infrastructure provides:");
    println!("• Type-safe event handling");
    println!("• Automatic reconnection and error recovery");
    println!("• Comprehensive monitoring and metrics");
    println!("• Builder patterns for ergonomic APIs");
    println!("• Production-ready WebSocket management");
    println!("• Real-time data streaming capabilities");

    Ok(())
}

// Helper struct to demonstrate structured event data
#[derive(Debug)]
pub struct ResourceUpdate {
    pub id: String,
    pub workflow_id: String,
    pub state: String,
    pub updated_at: String,
}

#[derive(Debug)]
pub struct WorkflowEventData {
    pub id: String,
    pub event_type: String,
    pub message: String,
    pub timestamp: String,
}

#[derive(Debug)]
pub struct LLMStreamChunk {
    pub id: String,
    pub content: String,
    pub finished: bool,
    pub timestamp: String,
}

#[derive(Debug)]
pub struct CostUpdate {
    pub user_id: Option<String>,
    pub project_id: Option<String>,
    pub cost: f64,
    pub timestamp: String,
}

#[derive(Debug)]
pub struct AgentExecutionEvent {
    pub id: String,
    pub agent_id: String,
    pub status: String,
    pub timestamp: String,
}

#[derive(Debug)]
pub struct MCPServerStatusUpdate {
    pub server_id: String,
    pub status: String,
    pub message: Option<String>,
    pub timestamp: String,
}

#[derive(Debug)]
pub struct MCPSessionEvent {
    pub session_id: String,
    pub event: String,
    pub timestamp: String,
}
