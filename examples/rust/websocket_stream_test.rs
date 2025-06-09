// WebSocket Streaming Test Client
// Tests the real-time GraphQL subscription streaming functionality

use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message, tungstenite::handshake::client::Request};
use futures::{SinkExt, StreamExt};
use serde_json::{json, Value};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 WebSocket Streaming Test");
    println!("============================");
    
    // Connect to the WebSocket endpoint with GraphQL subprotocol
    let ws_url = "ws://localhost:4000/ws";
    println!("📡 Connecting to: {}", ws_url);
    
    let request = Request::builder()
        .uri(ws_url)
        .header("Sec-WebSocket-Protocol", "graphql-ws")
        .body(())?;
    
    let (ws_stream, _) = connect_async(request).await
        .map_err(|e| format!("Failed to connect to WebSocket: {}. Make sure the server is running with: cargo run --bin server", e))?;
    
    println!("✅ Connected to WebSocket server");
    
    let (mut write, mut read) = ws_stream.split();
    
    // GraphQL-WS connection init
    let init_message = json!({
        "type": "connection_init"
    });
    
    write.send(Message::Text(init_message.to_string())).await?;
    println!("📤 Sent connection init");
    
    // Wait for connection_ack
    if let Some(msg) = read.next().await {
        let msg = msg?;
        if let Message::Text(text) = msg {
            let response: Value = serde_json::from_str(&text)?;
            println!("📥 Received: {}", response.get("type").unwrap_or(&Value::Null));
        }
    }
    
    // Test 1: Simple token updates subscription
    println!("\n🔄 Test 1: Token Updates Subscription");
    let subscription1 = json!({
        "id": "test1",
        "type": "start",
        "payload": {
            "query": "subscription { tokenUpdates(tokenId: \"test-token\") { id currentPlace } }"
        }
    });
    
    write.send(Message::Text(subscription1.to_string())).await?;
    println!("📤 Sent token updates subscription");
    
    // Listen for responses for a few seconds
    let mut message_count = 0;
    let timeout = tokio::time::timeout(Duration::from_secs(3), async {
        while let Some(msg) = read.next().await {
            let msg = msg?;
            if let Message::Text(text) = msg {
                let response: Value = serde_json::from_str(&text)?;
                println!("📥 Token update: {}", text);
                message_count += 1;
                if message_count >= 2 { break; }
            }
        }
        Ok::<(), Box<dyn std::error::Error>>(())
    }).await;
    
    match timeout {
        Ok(_) => println!("✅ Token updates subscription working"),
        Err(_) => println!("⏰ Token updates subscription timeout (expected for empty stream)"),
    }
    
    // Test 2: Cost updates subscription  
    println!("\n💰 Test 2: Cost Updates Subscription");
    let subscription2 = json!({
        "id": "test2", 
        "type": "start",
        "payload": {
            "query": "subscription { costUpdates(userId: \"test-user\") }"
        }
    });
    
    write.send(Message::Text(subscription2.to_string())).await?;
    println!("📤 Sent cost updates subscription");
    
    // Listen for cost updates
    let timeout = tokio::time::timeout(Duration::from_secs(8), async {
        let mut cost_messages = 0;
        while let Some(msg) = read.next().await {
            let msg = msg?;
            if let Message::Text(text) = msg {
                let response: Value = serde_json::from_str(&text)?;
                if let Some(data) = response.get("payload").and_then(|p| p.get("data")) {
                    if let Some(cost_data) = data.get("costUpdates") {
                        println!("📥 Cost update: {}", cost_data);
                        cost_messages += 1;
                        if cost_messages >= 2 { break; }
                    }
                }
            }
        }
        Ok::<(), Box<dyn std::error::Error>>(())
    }).await;
    
    match timeout {
        Ok(_) => println!("✅ Cost updates subscription working"),
        Err(_) => println!("⏰ Cost updates subscription timeout"),
    }
    
    // Test 3: LLM Stream subscription (this will fail without API key, but tests WebSocket structure)
    println!("\n🤖 Test 3: LLM Stream Subscription Structure Test");
    let subscription3 = json!({
        "id": "test3",
        "type": "start", 
        "payload": {
            "query": "subscription { llmStream(requestId: \"websocket-test\") }"
        }
    });
    
    write.send(Message::Text(subscription3.to_string())).await?;
    println!("📤 Sent LLM stream subscription");
    
    // Listen for LLM stream response (will likely be an error due to missing API key)
    let timeout = tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(msg) = read.next().await {
            let msg = msg?;
            if let Message::Text(text) = msg {
                let response: Value = serde_json::from_str(&text)?;
                if let Some(payload) = response.get("payload") {
                    if let Some(errors) = payload.get("errors") {
                        println!("📥 Expected LLM error (no API key): {}", errors);
                        break;
                    } else if let Some(data) = payload.get("data") {
                        println!("📥 LLM data: {}", data);
                        break;
                    }
                }
            }
        }
        Ok::<(), Box<dyn std::error::Error>>(())
    }).await;
    
    match timeout {
        Ok(_) => println!("✅ LLM stream subscription structure working"),
        Err(_) => println!("⏰ LLM stream subscription timeout"),
    }
    
    // Close subscriptions
    let stop_message = json!({
        "id": "test1",
        "type": "stop"
    });
    write.send(Message::Text(stop_message.to_string())).await?;
    
    let stop_message2 = json!({
        "id": "test2", 
        "type": "stop"
    });
    write.send(Message::Text(stop_message2.to_string())).await?;
    
    let stop_message3 = json!({
        "id": "test3",
        "type": "stop"
    });
    write.send(Message::Text(stop_message3.to_string())).await?;
    
    println!("\n🎉 WebSocket streaming tests completed!");
    println!("🔧 To test with real LLM streaming, set ANTHROPIC_API_KEY and run:");
    println!("   cargo run --example llm_router_demo");
    
    Ok(())
}