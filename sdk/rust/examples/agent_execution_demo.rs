//! Agent Execution Demo - Comprehensive Example
//!
//! This example demonstrates the new multi-tenant agent execution capabilities
//! of the Circuit Breaker SDK, including:
//!
//! 1. Simple agent execution with context
//! 2. Streaming execution with Server-Sent Events (SSE)
//! 3. WebSocket streaming for real-time communication
//! 4. Tenant isolation and multi-tenancy
//! 5. Error handling and recovery
//! 6. Multiple concurrent executions
//!
//! Prerequisites:
//! 1. Circuit Breaker server running with multi-tenant agent support
//! 2. Agents configured in the system
//! 3. Environment variables set for configuration

use circuit_breaker_sdk::{
    agents::{AgentExecutionRequest, AgentWebSocketServerMessage, ListExecutionsRequest},
    Client, Result,
};
use futures::{future, StreamExt};
use serde_json::json;
use std::collections::HashMap;
use std::env;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🤖 Circuit Breaker Agent Execution Demo");
    println!("========================================");

    // Initialize the client
    let base_url =
        env::var("CIRCUIT_BREAKER_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
    let api_key = env::var("CIRCUIT_BREAKER_API_KEY").ok();

    let mut client_builder = Client::builder().base_url(&base_url)?;
    if let Some(key) = api_key {
        client_builder = client_builder.api_key(key);
    }
    let client = client_builder.build()?;

    // Test connection
    match client.ping().await {
        Ok(ping) => println!("✅ Connected to Circuit Breaker server: {}", ping.status),
        Err(e) => {
            println!("❌ Failed to connect to server: {}", e);
            println!(
                "   Make sure the Circuit Breaker server is running at {}",
                base_url
            );
            return Ok(());
        }
    }

    // Demo configuration
    let tenant_id = "demo-tenant-001";
    let agent_id = "customer-support-agent"; // This should exist in your system

    println!("\n🎯 Demo Configuration:");
    println!("   Tenant ID: {}", tenant_id);
    println!("   Agent ID: {}", agent_id);
    println!("   Base URL: {}", base_url);

    // Run all demo scenarios
    demo_simple_execution(&client, agent_id, tenant_id).await?;
    demo_streaming_execution(&client, agent_id, tenant_id).await?;
    demo_websocket_execution(&client, agent_id, tenant_id).await?;
    demo_execution_management(&client, agent_id, tenant_id).await?;
    demo_concurrent_executions(&client, agent_id, tenant_id).await?;
    demo_tenant_isolation(&client, agent_id).await?;

    println!("\n🎉 Agent Execution Demo Complete!");
    println!("   All scenarios have been successfully demonstrated.");

    Ok(())
}

/// Demo 1: Simple Agent Execution
async fn demo_simple_execution(client: &Client, agent_id: &str, tenant_id: &str) -> Result<()> {
    println!("\n1. 🚀 Simple Agent Execution");
    println!("   ===========================");

    let context = json!({
        "message": "Hello! I need help with my order status.",
        "user_id": "user123",
        "session_id": "session456",
        "metadata": {
            "channel": "web",
            "language": "en",
            "priority": "normal"
        }
    });

    let request = AgentExecutionRequest {
        context: context.clone(),
        mapping: Some(json!({
            "input_field": "message",
            "output_field": "response"
        })),
        tenant_id: Some(tenant_id.to_string()),
        stream: Some(false),
    };

    println!(
        "   Executing agent with context: {}",
        serde_json::to_string_pretty(&context)?
    );

    match client.agents().execute(agent_id, request).await {
        Ok(response) => {
            println!("   ✅ Execution completed successfully!");
            println!("      Execution ID: {}", response.execution_id);
            println!("      Status: {:?}", response.status);
            if let Some(output) = response.output {
                println!("      Output: {}", serde_json::to_string_pretty(&output)?);
            }
        }
        Err(e) => {
            println!("   ❌ Execution failed: {}", e);
        }
    }

    Ok(())
}

/// Demo 2: Streaming Agent Execution with SSE
async fn demo_streaming_execution(client: &Client, agent_id: &str, tenant_id: &str) -> Result<()> {
    println!("\n2. 📡 Streaming Agent Execution (SSE)");
    println!("   ===================================");

    let context = json!({
        "message": "Can you explain the return policy for electronics?",
        "user_id": "user456",
        "session_id": "session789",
        "metadata": {
            "channel": "chat",
            "language": "en",
            "priority": "high"
        }
    });

    let request = AgentExecutionRequest {
        context: context.clone(),
        mapping: None,
        tenant_id: Some(tenant_id.to_string()),
        stream: Some(true),
    };

    println!("   Starting streaming execution...");

    match client.agents().execute_stream(agent_id, request).await {
        Ok(stream) => {
            println!("   ✅ Stream started successfully!");
            println!("   📊 Streaming events:");

            let mut stream = Box::pin(stream);
            let mut event_count = 0;

            while let Some(event_result) = stream.next().await {
                match event_result {
                    Ok(event) => {
                        event_count += 1;
                        println!(
                            "      Event {}: {} - {}",
                            event_count,
                            event.event_type,
                            event
                                .data
                                .get("message")
                                .unwrap_or(&json!("No message"))
                                .as_str()
                                .unwrap_or("")
                        );

                        if event.event_type == "completed" || event.event_type == "failed" {
                            println!("   ✅ Stream completed with {} events", event_count);
                            break;
                        }
                    }
                    Err(e) => {
                        if e.to_string().contains("Stream completed") {
                            println!("   ✅ Stream completed normally");
                            break;
                        }
                        println!("   ⚠️  Stream error: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            println!("   ❌ Streaming failed: {}", e);
        }
    }

    Ok(())
}

/// Demo 3: WebSocket Agent Execution
async fn demo_websocket_execution(client: &Client, agent_id: &str, tenant_id: &str) -> Result<()> {
    println!("\n3. 🔌 WebSocket Agent Execution");
    println!("   ==============================");

    let context = json!({
        "message": "I have a technical question about your API integration.",
        "user_id": "user789",
        "session_id": "session101",
        "metadata": {
            "channel": "developer_portal",
            "language": "en",
            "priority": "urgent"
        }
    });

    let request = AgentExecutionRequest {
        context: context.clone(),
        mapping: None,
        tenant_id: Some(tenant_id.to_string()),
        stream: Some(true),
    };

    println!("   Connecting to WebSocket...");

    match client.agents().execute_websocket(agent_id, request).await {
        Ok(mut ws_stream) => {
            println!("   ✅ WebSocket connected successfully!");

            // Agent execution and subscription are handled automatically
            println!("   📤 Agent execution initiated via WebSocket...");

            // Listen for messages
            println!("   📥 Listening for WebSocket messages...");
            let mut message_count = 0;
            let timeout = Duration::from_secs(30);
            let start_time = std::time::Instant::now();

            while start_time.elapsed() < timeout {
                match tokio::time::timeout(Duration::from_secs(5), ws_stream.receive_message())
                    .await
                {
                    Ok(Ok(Some(message))) => {
                        message_count += 1;
                        match message {
                            AgentWebSocketServerMessage::ExecutionStarted {
                                execution_id,
                                agent_id,
                                timestamp: _,
                            } => {
                                println!(
                                    "      🚀 Execution started: {} (agent: {})",
                                    execution_id, agent_id
                                );
                            }
                            AgentWebSocketServerMessage::Thinking {
                                execution_id: _,
                                status,
                                timestamp: _,
                            } => {
                                println!("      🤔 Thinking: {}", status);
                            }
                            AgentWebSocketServerMessage::ContentChunk {
                                execution_id: _,
                                chunk,
                                sequence,
                                timestamp: _,
                            } => {
                                println!("      📝 Content chunk {}: {}", sequence, chunk);
                            }
                            AgentWebSocketServerMessage::Complete {
                                execution_id,
                                response,
                                usage: _,
                                timestamp: _,
                            } => {
                                println!(
                                    "      ✅ Execution completed: {} - {}",
                                    execution_id, response
                                );
                                break;
                            }
                            AgentWebSocketServerMessage::Error {
                                execution_id,
                                error,
                                timestamp: _,
                            } => {
                                println!(
                                    "      ❌ Error in execution {:?}: {}",
                                    execution_id.unwrap_or_else(|| "unknown".to_string()),
                                    error
                                );
                                break;
                            }
                            AgentWebSocketServerMessage::AuthSuccess { tenant_id } => {
                                println!("      🔐 Authenticated for tenant: {}", tenant_id);
                            }
                            AgentWebSocketServerMessage::AuthFailure { error } => {
                                println!("      ❌ Authentication failed: {}", error);
                                break;
                            }
                            AgentWebSocketServerMessage::Pong { timestamp: _ } => {
                                println!("      🏓 Pong received");
                            }
                        }
                    }
                    Ok(Ok(None)) => {
                        println!("   🔌 WebSocket connection closed");
                        break;
                    }
                    Ok(Err(e)) => {
                        println!("   ❌ WebSocket error: {}", e);
                        break;
                    }
                    Err(_) => {
                        // Timeout - continue listening
                        continue;
                    }
                }
            }

            println!(
                "   ✅ WebSocket demo completed with {} messages",
                message_count
            );

            // Close the connection
            if let Err(e) = ws_stream.close().await {
                println!("   ⚠️  Failed to close WebSocket: {}", e);
            }
        }
        Err(e) => {
            println!("   ❌ WebSocket connection failed: {}", e);
        }
    }

    Ok(())
}

/// Demo 4: Execution Management
async fn demo_execution_management(client: &Client, agent_id: &str, tenant_id: &str) -> Result<()> {
    println!("\n4. 📋 Execution Management");
    println!("   =========================");

    // Create a few executions first
    println!("   Creating sample executions...");
    let mut execution_ids = Vec::new();

    for i in 1..=3 {
        let context = json!({
            "message": format!("Test message {}", i),
            "user_id": format!("user{}", i),
            "session_id": format!("session{}", i)
        });

        let request = AgentExecutionRequest {
            context,
            mapping: None,
            tenant_id: Some(tenant_id.to_string()),
            stream: Some(false),
        };

        match client.agents().execute(agent_id, request).await {
            Ok(response) => {
                execution_ids.push(response.execution_id.clone());
                println!(
                    "      ✅ Created execution {}: {}",
                    i, response.execution_id
                );
            }
            Err(e) => {
                println!("      ❌ Failed to create execution {}: {}", i, e);
            }
        }

        sleep(Duration::from_millis(100)).await;
    }

    // List executions
    println!("   📊 Listing executions...");
    let list_request = ListExecutionsRequest {
        limit: Some(10),
        offset: Some(0),
        status: None,
        tenant_id: Some(tenant_id.to_string()),
    };

    match client
        .agents()
        .list_executions(agent_id, list_request)
        .await
    {
        Ok(response) => {
            println!(
                "      ✅ Found {} executions (total: {})",
                response.executions.len(),
                response.total
            );
            for execution in response.executions {
                println!(
                    "         • {}: {:?} ({})",
                    execution.execution_id,
                    execution.status,
                    execution.created_at.format("%H:%M:%S")
                );
            }
        }
        Err(e) => {
            println!("      ❌ Failed to list executions: {}", e);
        }
    }

    // Get specific execution details
    if let Some(execution_id) = execution_ids.first() {
        println!("   🔍 Getting execution details for: {}", execution_id);
        match client
            .agents()
            .get_execution(agent_id, execution_id, Some(tenant_id))
            .await
        {
            Ok(response) => {
                println!("      ✅ Execution details:");
                println!("         Status: {:?}", response.status);
                println!(
                    "         Created: {}",
                    response.created_at.format("%Y-%m-%d %H:%M:%S")
                );
                if let Some(output) = response.output {
                    println!(
                        "         Output: {}",
                        serde_json::to_string_pretty(&output)?
                    );
                }
            }
            Err(e) => {
                println!("      ❌ Failed to get execution details: {}", e);
            }
        }
    }

    Ok(())
}

/// Demo 5: Concurrent Executions
async fn demo_concurrent_executions(
    client: &Client,
    agent_id: &str,
    tenant_id: &str,
) -> Result<()> {
    println!("\n5. 🚀 Concurrent Executions");
    println!("   ===========================");

    let concurrent_count = 3;
    println!("   Starting {} concurrent executions...", concurrent_count);

    let mut tasks = Vec::new();

    for i in 1..=concurrent_count {
        let client_clone = client.clone();
        let agent_id_clone = agent_id.to_string();
        let tenant_id_clone = tenant_id.to_string();

        let task = tokio::spawn(async move {
            let context = json!({
                "message": format!("Concurrent message {}", i),
                "user_id": format!("concurrent_user{}", i),
                "session_id": format!("concurrent_session{}", i),
                "metadata": {
                    "execution_number": i,
                    "test_type": "concurrent"
                }
            });

            let request = AgentExecutionRequest {
                context,
                mapping: None,
                tenant_id: Some(tenant_id_clone),
                stream: Some(false),
            };

            let start_time = std::time::Instant::now();
            match client_clone
                .agents()
                .execute(&agent_id_clone, request)
                .await
            {
                Ok(response) => {
                    let duration = start_time.elapsed();
                    (i, Ok(response), duration)
                }
                Err(e) => {
                    let duration = start_time.elapsed();
                    (i, Err(e), duration)
                }
            }
        });

        tasks.push(task);
    }

    // Wait for all tasks to complete
    let results = future::join_all(tasks).await;
    let mut success_count = 0;
    let mut total_duration = Duration::from_secs(0);

    for result in results {
        match result {
            Ok((i, Ok(response), duration)) => {
                success_count += 1;
                total_duration += duration;
                println!(
                    "      ✅ Execution {} completed in {:?}: {}",
                    i, duration, response.execution_id
                );
            }
            Ok((i, Err(e), duration)) => {
                total_duration += duration;
                println!("      ❌ Execution {} failed in {:?}: {}", i, duration, e);
            }
            Err(e) => {
                println!("      ❌ Task {} panicked: {}", 0, e);
            }
        }
    }

    let avg_duration = total_duration / concurrent_count;
    println!("   📊 Concurrent execution results:");
    println!("      Success rate: {}/{}", success_count, concurrent_count);
    println!("      Average duration: {:?}", avg_duration);
    println!("      Total duration: {:?}", total_duration);

    Ok(())
}

/// Demo 6: Tenant Isolation
async fn demo_tenant_isolation(client: &Client, agent_id: &str) -> Result<()> {
    println!("\n6. 🏢 Tenant Isolation");
    println!("   ====================");

    let tenants = vec!["tenant-a", "tenant-b", "tenant-c"];
    println!("   Testing isolation across {} tenants...", tenants.len());

    let mut tenant_executions = HashMap::new();

    // Create executions for each tenant
    for tenant in &tenants {
        println!("   🏢 Creating execution for tenant: {}", tenant);

        let context = json!({
            "message": format!("Message from {}", tenant),
            "user_id": format!("{}_user", tenant),
            "session_id": format!("{}_session", tenant),
            "tenant_data": {
                "tenant_id": tenant,
                "environment": "production"
            }
        });

        let request = AgentExecutionRequest {
            context,
            mapping: None,
            tenant_id: Some(tenant.to_string()),
            stream: Some(false),
        };

        match client.agents().execute(agent_id, request).await {
            Ok(response) => {
                tenant_executions.insert(tenant.to_string(), response.execution_id.clone());
                println!("      ✅ Created execution: {}", response.execution_id);
            }
            Err(e) => {
                println!("      ❌ Failed to create execution: {}", e);
            }
        }
    }

    // Verify each tenant can only see their own executions
    for tenant in &tenants {
        println!("   🔍 Listing executions for tenant: {}", tenant);

        let list_request = ListExecutionsRequest {
            limit: Some(10),
            offset: Some(0),
            status: None,
            tenant_id: Some(tenant.to_string()),
        };

        match client
            .agents()
            .list_executions(agent_id, list_request)
            .await
        {
            Ok(response) => {
                println!(
                    "      ✅ Found {} executions for {}",
                    response.executions.len(),
                    tenant
                );

                // Verify all executions belong to this tenant
                let mut isolation_verified = true;
                for execution in &response.executions {
                    if let Some(tenant_in_context) = execution
                        .context
                        .get("tenant_data")
                        .and_then(|td| td.get("tenant_id"))
                        .and_then(|tid| tid.as_str())
                    {
                        if tenant_in_context != *tenant {
                            isolation_verified = false;
                            println!(
                                "      ❌ ISOLATION BREACH: Found execution from {} in {} results",
                                tenant_in_context, tenant
                            );
                        }
                    }
                }

                if isolation_verified {
                    println!("      ✅ Tenant isolation verified for {}", tenant);
                } else {
                    println!("      ❌ Tenant isolation FAILED for {}", tenant);
                }
            }
            Err(e) => {
                println!("      ❌ Failed to list executions for {}: {}", tenant, e);
            }
        }
    }

    Ok(())
}
