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
    llm::common_models,
    Client, Error, Result,
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

    println!("\n🎯 Demo Configuration:");
    println!("   Tenant ID: {}", tenant_id);
    println!("   Base URL: {}", base_url);

    println!("\n🔑 Environment Variables Status (Server-Side):");
    let openai_key = env::var("OPENAI_API_KEY").ok();
    let anthropic_key = env::var("ANTHROPIC_API_KEY").ok();
    let google_key = env::var("GOOGLE_API_KEY").ok();
    let cb_url = env::var("CIRCUIT_BREAKER_URL").ok();
    let cb_key = env::var("CIRCUIT_BREAKER_API_KEY").ok();

    println!(
        "   • OPENAI_API_KEY: {} (used by server)",
        if openai_key.is_some() {
            "✅ Set"
        } else {
            "❌ Not set"
        }
    );
    println!(
        "   • ANTHROPIC_API_KEY: {} (used by server)",
        if anthropic_key.is_some() {
            "✅ Set"
        } else {
            "❌ Not set"
        }
    );
    println!(
        "   • GOOGLE_API_KEY: {} (used by server)",
        if google_key.is_some() {
            "✅ Set"
        } else {
            "❌ Not set"
        }
    );
    println!(
        "   • CIRCUIT_BREAKER_URL: {}",
        if cb_url.is_some() {
            "✅ Set"
        } else {
            "❌ Not set"
        }
    );
    println!(
        "   • CIRCUIT_BREAKER_API_KEY: {}",
        if cb_key.is_some() {
            "✅ Set"
        } else {
            "❌ Not set"
        }
    );

    if openai_key.is_none() && anthropic_key.is_none() && google_key.is_none() {
        println!("   ⚠️  WARNING: No LLM API keys found! Agent execution may fail.");
        println!("   📝 NOTE: Keys must be set on the Circuit Breaker SERVER, not client");
    }

    println!("   🔒 SECURITY: API keys are read by server from environment variables");
    println!("   ℹ️  Virtual models automatically route to available providers");

    // Create demo agents first
    println!("\n🏗️  Creating Demo Agents:");
    println!("   ========================");
    let agent_ids = match create_demo_agents(&client, tenant_id).await {
        Ok(ids) => {
            println!("   ✅ Successfully created {} agents", ids.len());
            ids
        }
        Err(e) => {
            println!("   ❌ Failed to create agents: {}", e);
            println!("   💡 Make sure the GraphQL server is running at http://localhost:4000");
            return Ok(());
        }
    };

    let primary_agent_id = &agent_ids[0];
    println!("\n   Using primary agent: {}", primary_agent_id);

    // Run all demo scenarios
    demo_simple_execution(&client, primary_agent_id, tenant_id).await?;
    demo_streaming_execution(&client, primary_agent_id, tenant_id).await?;
    demo_websocket_execution(&client, primary_agent_id, tenant_id).await?;
    demo_execution_management(&client, primary_agent_id, tenant_id).await?;
    demo_concurrent_executions(&client, primary_agent_id, tenant_id).await?;
    demo_tenant_isolation(&client, primary_agent_id).await?;
    demo_virtual_model_examples(&client, tenant_id).await?;

    println!("\n🎉 Agent Execution Demo Complete!");
    println!("   All scenarios have been successfully demonstrated.");

    Ok(())
}

/// Create demo agents via GraphQL for testing
async fn create_demo_agents(client: &Client, tenant_id: &str) -> Result<Vec<String>> {
    println!("   📝 Creating customer support agent...");
    // Create a customer support agent
    let customer_support_agent = create_agent_via_graphql(
        client,
        "customer-support-agent",
        "Customer Support Assistant",
        "AI assistant specialized in handling customer inquiries, order status, and support requests",
        common_models::SMART_FAST,
        tenant_id,
    )
    .await?;

    println!("   📝 Creating sales assistant agent...");
    // Create a sales assistant agent
    let sales_assistant_agent = create_agent_via_graphql(
        client,
        "sales-assistant-agent",
        "Sales Assistant",
        "AI assistant specialized in product recommendations, pricing inquiries, and sales support",
        common_models::SMART_BALANCED,
        tenant_id,
    )
    .await?;

    Ok(vec![customer_support_agent, sales_assistant_agent])
}

/// Map virtual models to standard fallback models
fn get_fallback_model(virtual_model: &str) -> (&str, &str) {
    match virtual_model {
        common_models::SMART_CHEAP => ("openai", "gpt-3.5-turbo"),
        common_models::SMART_FAST => ("openai", "gpt-4"),
        common_models::SMART_BALANCED => ("openai", "gpt-4"),
        common_models::SMART_CREATIVE => ("anthropic", "claude-3-sonnet-20240229"),
        common_models::SMART_CODING => ("openai", "gpt-4"),
        common_models::SMART_ANALYSIS => ("openai", "gpt-4"),
        _ => ("openai", "gpt-3.5-turbo"),
    }
}

/// Create a single agent via GraphQL
async fn create_agent_via_graphql(
    _client: &Client,
    agent_id: &str,
    name: &str,
    description: &str,
    virtual_model: &str,
    tenant_id: &str,
) -> Result<String> {
    println!(
        "      🔧 Preparing GraphQL mutation for agent: {}",
        agent_id
    );

    let mutation = format!(
        r#"
        mutation CreateAgent($input: AgentDefinitionInput!) {{
            createAgent(input: $input) {{
                id
                name
                description
            }}
        }}
        "#
    );

    // Debug: Print what we're trying to create
    println!(
        "      🔍 Attempting to create agent with virtual model: {}",
        virtual_model
    );

    // First, try with the virtual model using OpenAI provider
    let provider_type = "openai";
    let _api_key = env::var("OPENAI_API_KEY").unwrap_or_else(|_| "sk-placeholder".to_string());
    let model = virtual_model;
    let base_url_option: Option<String> = None;

    println!(
        "      🔍 Trying virtual model - Provider: {}, Model: {}",
        provider_type, model
    );

    let variables = json!({
        "input": {
            "name": name,
            "description": description,
            "llmProvider": {
                "providerType": provider_type,
                "model": model,
                "baseUrl": base_url_option
            },
            "llmConfig": {
                "temperature": 0.7,
                "maxTokens": 1000,
                "topP": null,
                "frequencyPenalty": null,
                "presencePenalty": null,
                "stopSequences": []
            },
            "prompts": {
                "system": format!("You are a helpful {}. {}", name, description),
                "userTemplate": "User: {{message}}\n\nContext: {{context}}",
                "contextInstructions": "Always be helpful, professional, and provide accurate information."
            },
            "capabilities": ["text-generation", "conversation"],
            "tools": []
        }
    });

    // Use GraphQL endpoint (port 4000) for agent creation
    let graphql_client = Client::builder()
        .base_url("http://localhost:4000")?
        .build()?;

    // Create request body
    let request_body = json!({
        "query": mutation,
        "variables": variables
    });

    // Try creating the agent with virtual model first
    match try_create_agent(&graphql_client, &request_body, tenant_id).await {
        Ok(agent_id) => {
            println!("      ✅ Agent created with virtual model: {}", agent_id);
            println!(
                "      📝 Virtual model '{}' will be used for execution",
                virtual_model
            );
            return Ok(agent_id);
        }
        Err(e) => {
            println!("      ⚠️  Virtual model '{}' failed: {}", virtual_model, e);
            println!("      🔄 Trying fallback to standard model...");
        }
    }

    // Fallback to standard model
    let (fallback_provider, fallback_model) = get_fallback_model(virtual_model);
    // NOTE: No API keys sent - server pulls from environment variables

    println!(
        "      🔍 Trying fallback - Provider: {}, Model: {}",
        fallback_provider, fallback_model
    );

    let fallback_variables = json!({
        "input": {
            "name": name,
            "description": description,
            "llmProvider": {
                "providerType": fallback_provider,
                "model": fallback_model,
                "baseUrl": base_url_option
            },
            "llmConfig": {
                "temperature": 0.7,
                "maxTokens": 1000,
                "topP": null,
                "frequencyPenalty": null,
                "presencePenalty": null,
                "stopSequences": []
            },
            "prompts": {
                "system": format!("You are a helpful {}. {}", name, description),
                "userTemplate": "User: {{message}}\n\nContext: {{context}}",
                "contextInstructions": "Always be helpful, professional, and provide accurate information."
            },
            "capabilities": ["text-generation", "conversation"],
            "tools": []
        }
    });

    let fallback_request_body = json!({
        "query": mutation,
        "variables": fallback_variables
    });

    match try_create_agent(&graphql_client, &fallback_request_body, tenant_id).await {
        Ok(agent_id) => {
            println!("      ✅ Agent created with fallback model: {}", agent_id);
            println!(
                "      📝 Fallback model '{}' ({}) will be used for execution",
                fallback_model, fallback_provider
            );
            println!("      ⚠️  Note: This agent may not use Circuit Breaker routing");
            Ok(agent_id)
        }
        Err(e) => {
            println!("      ❌ Fallback also failed: {}", e);
            println!("      💡 Possible issues:");
            println!("         • GraphQL server doesn't support the provider type");
            println!("         • API key format is incorrect");
            println!("         • Server configuration issue");
            Err(e)
        }
    }
}

/// Helper function to try creating an agent with given request body
async fn try_create_agent(
    graphql_client: &Client,
    request_body: &serde_json::Value,
    tenant_id: &str,
) -> Result<String> {
    println!("      📡 Sending GraphQL request to create agent...");
    println!("      🌐 URL: {}/graphql", graphql_client.base_url());

    // Extract and log key details from request
    if let Some(input) = request_body.get("variables").and_then(|v| v.get("input")) {
        if let Some(provider) = input.get("llmProvider") {
            println!("      🔍 LLM Provider Config:");
            println!(
                "         Type: {}",
                provider
                    .get("providerType")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
            );
            println!(
                "         Model: {}",
                provider
                    .get("model")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
            );
            println!(
                "         API Key: {}",
                if provider.get("apiKey").is_some() {
                    "Provided (SECURITY RISK!)"
                } else {
                    "Not provided (server uses environment)"
                }
            );
        }
    }

    // Make GraphQL request
    let response = graphql_client
        .http_client()
        .post(&format!(
            "{}/graphql",
            graphql_client.base_url().as_str().trim_end_matches('/')
        ))
        .header("Content-Type", "application/json")
        .header("X-Tenant-ID", tenant_id)
        .json(request_body)
        .send()
        .await?;

    let status = response.status();
    println!("      📊 GraphQL response status: {}", status);

    if !status.is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        println!("      📋 GraphQL response body: {}", error_text);
        return Err(Error::Server {
            status: status.as_u16(),
            message: format!("GraphQL request failed: {}", error_text),
        });
    }

    let response_json: serde_json::Value = response.json().await?;
    println!(
        "      📋 GraphQL response JSON: {}",
        serde_json::to_string_pretty(&response_json).unwrap_or_else(|_| "Invalid JSON".to_string())
    );

    // Check for GraphQL errors
    if let Some(errors) = response_json.get("errors") {
        return Err(Error::Server {
            status: 400,
            message: format!("GraphQL errors: {}", errors),
        });
    }

    // Extract the created agent ID
    let created_agent = response_json
        .get("data")
        .and_then(|data| data.get("createAgent"))
        .and_then(|agent| agent.get("id"))
        .and_then(|id| id.as_str())
        .ok_or_else(|| Error::Server {
            status: 500,
            message: "Failed to extract agent ID from GraphQL response".to_string(),
        })?;

    Ok(created_agent.to_string())
}

/// Demo 1: Simple Agent Execution
async fn demo_simple_execution(client: &Client, agent_id: &str, tenant_id: &str) -> Result<()> {
    println!("\n1. 🚀 Simple Agent Execution");
    println!("   ===========================");

    let context = json!({
        "message": "What is the weather like in Hilton Head, SC",
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
    println!(
        "   Connecting to URL: {}/agents/{}/execute",
        client.get_endpoint_url("rest").trim_end_matches('/'),
        agent_id
    );

    // Add detailed logging for debugging
    println!("   🔍 Request details:");
    println!("      Agent ID: {}", agent_id);
    println!(
        "      Tenant ID: {}",
        request.tenant_id.as_ref().unwrap_or(&"None".to_string())
    );
    println!(
        "      Context keys: {:?}",
        context
            .as_object()
            .map(|obj| obj.keys().collect::<Vec<_>>())
    );
    println!("      Stream: {}", request.stream.unwrap_or(false));

    println!("   ⏳ Sending execution request...");
    let start_time = std::time::Instant::now();

    match client.agents().execute(agent_id, request).await {
        Ok(response) => {
            let duration = start_time.elapsed();
            println!("   ✅ Execution successful ({:?}):", duration);
            println!("      Agent ID: {}", response.agent_id);
            println!("      Execution ID: {}", response.execution_id);
            println!("      Status: {:?}", response.status);
            println!(
                "      Created: {}",
                response.created_at.format("%Y-%m-%d %H:%M:%S")
            );
            if let Some(output) = response.output {
                println!("      Output: {}", serde_json::to_string_pretty(&output)?);
            } else {
                println!("      Output: None (execution may be pending)");
            }
        }
        Err(e) => {
            let duration = start_time.elapsed();
            println!("   ❌ Execution failed after {:?}: {}", duration, e);
            println!("      This could indicate:");
            println!("      • Agent model/provider configuration issues");
            println!("      • Missing or invalid API keys");
            println!("      • Backend routing problems");
            println!("      • Network connectivity issues");
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

/// Demo 7: Virtual Model Examples
async fn demo_virtual_model_examples(client: &Client, tenant_id: &str) -> Result<()> {
    println!("\n7. 🎯 Virtual Model Examples");
    println!("   ===========================");
    println!("   Demonstrating different Circuit Breaker virtual models");
    println!("   These models use intelligent routing to select the best provider");
    println!("   🔒 SECURITY: Circuit Breaker server reads API keys from environment:");
    println!("   • Server environment: OPENAI_API_KEY, ANTHROPIC_API_KEY, GOOGLE_API_KEY, etc.");
    println!("   • Clients NEVER send API keys - server handles all provider authentication");

    // Virtual model configurations to test
    let virtual_model_configs = vec![
        (
            "💰 Cost-Optimized Agent",
            common_models::SMART_CHEAP,
            "Customer Support",
            "Handle customer inquiries cost-effectively",
        ),
        (
            "⚡ Performance-First Agent",
            common_models::SMART_FAST,
            "Emergency Response",
            "Provide rapid responses for urgent queries",
        ),
        (
            "⚖️ Balanced Agent",
            common_models::SMART_BALANCED,
            "General Assistant",
            "Balanced performance and cost for general tasks",
        ),
        (
            "🎨 Creative Agent",
            common_models::SMART_CREATIVE,
            "Content Creator",
            "Generate creative content and marketing materials",
        ),
        (
            "💻 Coding Agent",
            common_models::SMART_CODING,
            "Code Assistant",
            "Help with programming and technical questions",
        ),
        (
            "📊 Analysis Agent",
            common_models::SMART_ANALYSIS,
            "Data Analyst",
            "Analyze data and provide insights",
        ),
    ];

    let mut created_agents = Vec::new();

    // Create agents with different virtual models
    for (display_name, virtual_model, agent_name, description) in virtual_model_configs {
        println!(
            "   📝 Creating {} with model: {}",
            display_name, virtual_model
        );

        let agent_id = format!(
            "virtual-{}",
            virtual_model.replace(":", "-").replace("cb-", "")
        );

        match create_agent_via_graphql(
            client,
            &agent_id,
            agent_name,
            description,
            virtual_model,
            tenant_id,
        )
        .await
        {
            Ok(created_agent_id) => {
                created_agents.push((display_name, virtual_model, created_agent_id.clone()));
                println!("      ✅ Created agent: {}", created_agent_id);
            }
            Err(e) => {
                println!("      ❌ Failed to create {}: {}", display_name, e);
            }
        }
    }

    // Test each virtual model agent
    for (display_name, virtual_model, agent_id) in created_agents {
        println!("   🧪 Testing {} ({})", display_name, virtual_model);

        let context = match virtual_model {
            common_models::SMART_CHEAP => json!({
                "message": "Help me understand the basics of cloud computing",
                "priority": "normal",
                "cost_constraint": "low"
            }),
            common_models::SMART_FAST => json!({
                "message": "I need a quick explanation of microservices",
                "priority": "urgent",
                "speed_requirement": "fast"
            }),
            common_models::SMART_BALANCED => json!({
                "message": "Explain the pros and cons of different database types",
                "priority": "normal",
                "balance_requirement": "optimal"
            }),
            common_models::SMART_CREATIVE => json!({
                "message": "Write a creative product description for a new AI-powered tool",
                "priority": "normal",
                "creativity_level": "high"
            }),
            common_models::SMART_CODING => json!({
                "message": "Show me how to implement a simple REST API in Python",
                "priority": "normal",
                "code_type": "web_development"
            }),
            common_models::SMART_ANALYSIS => json!({
                "message": "Analyze the key trends in cloud computing for 2024",
                "priority": "normal",
                "analysis_depth": "comprehensive"
            }),
            _ => json!({
                "message": "This is a test message",
                "priority": "normal"
            }),
        };

        let request = AgentExecutionRequest {
            context,
            mapping: Some(json!({
                "input_field": "message",
                "output_field": "response"
            })),
            tenant_id: Some(tenant_id.to_string()),
            stream: Some(false),
        };

        println!(
            "      🌐 Executing with virtual model: {} -> {}",
            virtual_model,
            client.get_endpoint_url("rest").trim_end_matches('/'),
        );

        match client.agents().execute(&agent_id, request).await {
            Ok(response) => {
                println!("      ✅ Execution successful:");
                println!("         Execution ID: {}", response.execution_id);
                println!("         Status: {:?}", response.status);
                if let Some(output) = response.output {
                    // Truncate long responses for demo
                    let output_str = output.to_string();
                    if output_str.len() > 200 {
                        println!("         Output: {}...", &output_str[..200]);
                    } else {
                        println!("         Output: {}", output_str);
                    }
                }
                println!(
                    "         Virtual Model: {} routes to optimal provider",
                    virtual_model
                );
            }
            Err(e) => {
                println!("      ❌ Execution failed: {}", e);
            }
        }

        // Small delay between requests
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    println!("\n   🎯 Virtual Model Benefits:");
    println!(
        "   • {} - Routes to cheapest available models",
        common_models::SMART_CHEAP
    );
    println!(
        "   • {} - Routes to fastest available models",
        common_models::SMART_FAST
    );
    println!(
        "   • {} - Balances cost and performance",
        common_models::SMART_BALANCED
    );
    println!(
        "   • {} - Routes to models optimized for creativity",
        common_models::SMART_CREATIVE
    );
    println!(
        "   • {} - Routes to models optimized for coding",
        common_models::SMART_CODING
    );
    println!(
        "   • {} - Routes to models optimized for analysis",
        common_models::SMART_ANALYSIS
    );
    println!("   • Circuit Breaker handles provider selection, API keys, and fallbacks");
    println!("   • Agents don't need to know about specific providers or models");
    println!(
        "   • 🔒 SECURE: Server environment variables (OPENAI_API_KEY, etc.) used automatically"
    );
    println!("   • Router selects optimal provider based on cost, speed, and availability");
    println!("   • 🚫 NEVER send API keys from client - major security vulnerability!");

    Ok(())
}
