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

// Helper function to handle escape characters in streaming content
fn process_escape_characters(content: &str) -> String {
    content
        .replace("\\n", "\n") // Convert \n to actual newlines
        .replace("\\t", "\t") // Convert \t to actual tabs
        .replace("\\r", "\r") // Convert \r to carriage returns
        .replace("\\\"", "\"") // Convert \" to actual quotes
        .replace("\\'", "'") // Convert \' to actual apostrophes
        .replace("\\b", "\x08") // Convert \b to backspace
        .replace("\\f", "\x0C") // Convert \f to form feed
        .replace("\\v", "\x0B") // Convert \v to vertical tab
        .replace("\\0", "\0") // Convert \0 to null character
        .replace("\\\\", "\\") // Convert \\ to actual backslash (must be last)
}

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
                                if event_count == 1 {
                                    print!("   🤔 Thinking... ");
                                    io::stdout().flush().unwrap();
                                }
                            }
                            "complete" => {
                                println!("\n\n   ✅ SSE streaming completed!");
                                return;
                            }
                            "chunk" => {
                                if event_count == 1 {
                                    print!("\n   📝 Response: ");
                                }
                                if let Some(chunk) = event.data.get("content") {
                                    print!("{}", chunk.as_str().unwrap_or(""));
                                } else if let Some(chunk) = event.data.as_str() {
                                    print!("{}", chunk);
                                }
                                io::stdout().flush().unwrap();
                            }
                            "error" => {
                                println!("\n   ❌ Streaming error: {}", event.data);
                                return;
                            }
                            "raw" => {
                                // Parse SSE events from raw content
                                if let Some(raw_content) =
                                    event.data.get("raw_content").and_then(|v| v.as_str())
                                {
                                    let mut event_type = None;
                                    let mut data = None;

                                    for line in raw_content.lines() {
                                        if let Some(evt) = line.strip_prefix("event: ") {
                                            event_type = Some(evt);
                                        } else if let Some(d) = line.strip_prefix("data: ") {
                                            data = Some(d);
                                        }
                                    }

                                    match (event_type, data) {
                                        (Some("thinking"), Some(_)) => {
                                            if event_count == 1 {
                                                print!("   🤔 Thinking... ");
                                                io::stdout().flush().unwrap();
                                            }
                                        }
                                        (Some("chunk"), Some(content)) => {
                                            if event_count == 1 {
                                                print!("\n   📝 Response: ");
                                            }
                                            print!("{}", process_escape_characters(content));
                                            io::stdout().flush().unwrap();
                                        }
                                        (Some("complete"), Some(_)) => {
                                            println!("\n\n   ✅ SSE streaming completed!");
                                            return;
                                        }
                                        _ => {
                                            // Other SSE events - ignore for clean output
                                        }
                                    }
                                }
                            }
                            _ => {
                                // Ignore other event types for clean output
                            }
                        }
                    }
                    Ok(Some(Err(e))) => {
                        println!("   ❌ SSE stream error: {}", e);
                        return;
                    }
                    Ok(None) => {
                        println!("\n   🔚 SSE stream ended");
                        return;
                    }
                    Err(_) => {
                        // Timeout, continue
                        continue;
                    }
                }
            }

            println!("\n   ⏰ SSE stream timeout");
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
                                if message_count == 1 {
                                    print!("   📝 Response: ");
                                }
                                print!("{}", process_escape_characters(&chunk));
                                io::stdout().flush().unwrap();
                            }
                            circuit_breaker_sdk::agents::AgentWebSocketServerMessage::Complete { execution_id, response, .. } => {
                                println!("\n\n   ✅ WebSocket streaming completed!");
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
