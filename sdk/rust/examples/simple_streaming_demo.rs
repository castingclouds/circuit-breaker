use anyhow::Result;
use circuit_breaker_sdk::{agents::AgentExecutionRequest, Client};
use futures::StreamExt;
use serde_json::json;
use std::{
    env,
    io::{self, Write},
    time::Duration,
};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🧪 Simple Streaming Demo");
    println!("========================");

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
        Ok(ping) => println!("✅ Connected to server: {}", ping.status),
        Err(e) => {
            println!("❌ Failed to connect: {}", e);
            return Ok(());
        }
    }

    let tenant_id = "test-tenant";

    println!("\n📝 Creating test agent...");
    let agent_id = create_test_agent(&client, tenant_id).await?;
    println!("✅ Created agent: {}", agent_id);

    // Test 1: SSE Streaming
    println!("\n🌊 Testing SSE Streaming...");
    test_sse_streaming(&client, &agent_id, tenant_id).await;

    // Test 2: WebSocket
    println!("\n🔌 Testing WebSocket...");
    test_websocket(&client, &agent_id, tenant_id).await;

    println!("\n✅ Demo completed!");
    Ok(())
}

async fn create_test_agent(client: &Client, tenant_id: &str) -> Result<String> {
    let agent = client
        .agents()
        .create()
        .name("test-streaming-agent")
        .description("Simple agent for testing streaming")
        .conversational()
        .set_llm_provider("openai")
        .set_model("cb:fastest")
        .set_temperature(0.7)
        .set_system_prompt("You are a helpful assistant for testing streaming functionality. Always provide detailed, comprehensive responses with multiple paragraphs to properly test streaming chunks. Write thorough explanations that demonstrate real-time streaming capabilities.")
        .build()
        .await?;

    Ok(agent.id())
}

async fn test_sse_streaming(client: &Client, agent_id: &str, tenant_id: &str) {
    let request = AgentExecutionRequest {
        context: json!({
            "message": "Please write a detailed explanation about how streaming works in distributed systems. Include multiple paragraphs to test streaming chunks.",
            "tenant_id": tenant_id
        }),
        mapping: Some(json!({
            "message": "message"
        })),
        tenant_id: Some(tenant_id.to_string()),
        stream: Some(true),
    };

    println!("   Starting SSE stream...");

    match client.agents().execute_stream(agent_id, request).await {
        Ok(stream) => {
            println!("   ✅ SSE stream connected!");

            let mut stream = Box::pin(stream);
            let mut event_count = 0;
            let timeout = Duration::from_secs(30);
            let start_time = std::time::Instant::now();

            while start_time.elapsed() < timeout {
                match tokio::time::timeout(Duration::from_secs(30), stream.next()).await {
                    Ok(Some(Ok(event))) => {
                        event_count += 1;

                        match event.event_type.as_str() {
                            "thinking" => {
                                println!(
                                    "      🤔 Event {}: Thinking - {}",
                                    event_count,
                                    event
                                        .data
                                        .get("status")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("processing")
                                );
                            }
                            "complete" => {
                                println!("      ✅ Event {}: Complete", event_count);
                                if let Some(response) = event.data.get("response") {
                                    println!(
                                        "      📝 Response: {}",
                                        response.as_str().unwrap_or("No response")
                                    );
                                } else if let Some(content) = event.data.as_str() {
                                    println!("      📝 Content: {}", content);
                                } else {
                                    println!("      📝 Data: {}", event.data);
                                }
                                println!("   ✅ SSE stream completed with {} events", event_count);
                                return;
                            }
                            "chunk" => {
                                if let Some(chunk) = event.data.get("content") {
                                    print!("{}", chunk.as_str().unwrap_or(""));
                                } else if let Some(chunk) = event.data.as_str() {
                                    print!("{}", chunk);
                                }
                                // Flush stdout to ensure immediate display
                                io::stdout().flush().unwrap();
                            }
                            "error" => {
                                println!("      ❌ Event {}: Error - {}", event_count, event.data);
                                return;
                            }
                            _ => {
                                println!(
                                    "      📡 Event {}: {} - {}",
                                    event_count, event.event_type, event.data
                                );
                            }
                        }
                    }
                    Ok(Some(Err(e))) => {
                        println!("   ❌ SSE stream error: {}", e);
                        return;
                    }
                    Ok(None) => {
                        println!("   🔚 SSE stream ended");
                        return;
                    }
                    Err(_) => {
                        // Timeout, continue
                        continue;
                    }
                }
            }

            println!("   ⏰ SSE stream timeout");
        }
        Err(e) => {
            println!("   ❌ SSE stream failed: {}", e);
        }
    }
}

async fn test_websocket(client: &Client, agent_id: &str, tenant_id: &str) {
    let request = AgentExecutionRequest {
        context: json!({
            "message": "Via WebSocket, please explain the benefits of real-time streaming in agent communication. Write several sentences to test streaming.",
            "tenant_id": tenant_id
        }),
        mapping: Some(json!({
            "message": "message"
        })),
        tenant_id: Some(tenant_id.to_string()),
        stream: Some(true),
    };

    println!("   Connecting to WebSocket...");

    match client.agents().execute_websocket(agent_id, request).await {
        Ok(mut ws_stream) => {
            println!("   ✅ WebSocket connected!");

            let mut message_count = 0;
            let timeout = Duration::from_secs(30);
            let start_time = std::time::Instant::now();

            while start_time.elapsed() < timeout {
                match tokio::time::timeout(Duration::from_secs(5), ws_stream.receive_message())
                    .await
                {
                    Ok(Ok(Some(message))) => {
                        message_count += 1;

                        match &message {
                            circuit_breaker_sdk::agents::AgentWebSocketServerMessage::AuthSuccess { tenant_id } => {
                                println!("      🔐 Message {}: Auth Success for tenant: {}", message_count, tenant_id);
                            }
                            circuit_breaker_sdk::agents::AgentWebSocketServerMessage::AuthFailure { error } => {
                                println!("      🔒 Message {}: Auth Failed - {}", message_count, error);
                            }
                            circuit_breaker_sdk::agents::AgentWebSocketServerMessage::ExecutionStarted { execution_id, agent_id, .. } => {
                                println!("      🚀 Message {}: Execution Started - Agent: {}, Execution: {}", message_count, agent_id, execution_id);
                            }
                            circuit_breaker_sdk::agents::AgentWebSocketServerMessage::Thinking { execution_id, status, .. } => {
                                println!("      🤔 Message {}: Thinking - Execution: {}, Status: {}", message_count, execution_id, status);
                            }
                            circuit_breaker_sdk::agents::AgentWebSocketServerMessage::ContentChunk { execution_id, chunk, sequence, .. } => {
                                println!("      📝 Message {}: Content Chunk #{} - {}", message_count, sequence, chunk);
                            }
                            circuit_breaker_sdk::agents::AgentWebSocketServerMessage::Complete { execution_id, response, .. } => {
                                println!("      ✅ Message {}: Complete - Execution: {}", message_count, execution_id);
                                println!("      📝 Final Response: {}", response);
                                println!("   ✅ WebSocket completed with {} messages", message_count);
                                break;
                            }
                            circuit_breaker_sdk::agents::AgentWebSocketServerMessage::Error { execution_id, error, .. } => {
                                println!("      ❌ Message {}: Error - Execution: {}, Error: {}", message_count, execution_id.as_ref().map(|id| id.to_string()).unwrap_or_else(|| "Unknown".to_string()), error);
                                println!("   ❌ WebSocket error message received");
                                break;
                            }
                            circuit_breaker_sdk::agents::AgentWebSocketServerMessage::Pong { .. } => {
                                println!("      🏓 Message {}: Pong", message_count);
                            }
                        }
                    }
                    Ok(Ok(None)) => {
                        println!("   🔚 WebSocket connection closed");
                        break;
                    }
                    Ok(Err(e)) => {
                        println!("   ❌ WebSocket error: {}", e);
                        break;
                    }
                    Err(_) => {
                        // Timeout, continue
                        continue;
                    }
                }
            }

            if start_time.elapsed() >= timeout {
                println!("   ⏰ WebSocket timeout");
            }

            // Close connection
            if let Err(e) = ws_stream.close().await {
                println!("   ⚠️  Failed to close WebSocket: {}", e);
            }
        }
        Err(e) => {
            println!("   ❌ WebSocket failed: {}", e);
        }
    }
}
