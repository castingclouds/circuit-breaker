// NATS Integration Demo using Circuit Breaker Server Engine
// This demonstrates using NATS-enhanced storage through the GraphQL API

use circuit_breaker::{
    NATSStorage, NATSStorageConfig,
    create_schema_with_nats
};
use axum::{
    extract::Extension,
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use async_graphql::http::GraphQLPlaygroundConfig;
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use reqwest;
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Circuit Breaker NATS Integration Demo");
    println!("=========================================");

    // Start the demo server with NATS storage
    let _server_task = start_nats_server().await?;
    
    // Give the server time to start
    sleep(Duration::from_secs(2)).await;
    
    // Run the demo workflow
    run_nats_workflow_demo().await?;
    
    println!("\n✅ NATS integration demo completed successfully!");
    
    // Note: In a real application, you'd want to gracefully shutdown the server
    // For this demo, we'll just let it run
    
    Ok(())
}

/// Start the Circuit Breaker server with NATS storage
async fn start_nats_server() -> Result<tokio::task::JoinHandle<()>, Box<dyn std::error::Error>> {
    println!("🔧 Starting Circuit Breaker server with NATS storage...");
    
    let task = tokio::spawn(async move {
        // Create NATS storage configuration
        let nats_config = NATSStorageConfig {
            nats_urls: vec!["nats://localhost:4222".to_string()],
            default_max_messages: 100_000,
            default_max_bytes: 512 * 1024 * 1024, // 512MB
            default_max_age: Duration::from_secs(7 * 24 * 60 * 60), // 7 days
            consumer_timeout: Duration::from_secs(30),
            max_deliver: 3,
            connection_timeout: Duration::from_secs(10),
            reconnect_buffer_size: 4 * 1024 * 1024, // 4MB
        };
        
        // Create NATS storage
        let nats_storage = match NATSStorage::new(nats_config).await {
            Ok(storage) => storage,
            Err(e) => {
                eprintln!("❌ Failed to create NATS storage: {}", e);
                eprintln!("💡 Make sure NATS server is running on localhost:4222");
                eprintln!("   You can start NATS with: nats-server --jetstream");
                return;
            }
        };
        
        println!("✅ Connected to NATS JetStream");
        
        // Create GraphQL schema with NATS storage
        let schema = create_schema_with_nats(std::sync::Arc::new(nats_storage));
        
        // GraphQL handler functions
        async fn graphql_handler(
            schema: Extension<circuit_breaker::CircuitBreakerSchema>,
            req: GraphQLRequest,
        ) -> GraphQLResponse {
            schema.execute(req.into_inner()).await.into()
        }

        async fn graphql_playground() -> impl IntoResponse {
            Html(async_graphql::http::playground_source(
                GraphQLPlaygroundConfig::new("/graphql")
            ))
        }

        // Start the GraphQL server
        let app = Router::new()
            .route("/graphql", post(graphql_handler))
            .route("/", get(graphql_playground))
            .layer(Extension(schema));
        
        println!("🌐 GraphQL server starting on http://localhost:8080/graphql");
        
        axum::Server::bind(&"127.0.0.1:8080".parse().unwrap())
            .serve(app.into_make_service())
            .await
            .unwrap();
    });
    
    Ok(task)
}

/// Run the NATS workflow demonstration
async fn run_nats_workflow_demo() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📋 Creating workflow with NATS storage...");
    
    let client = reqwest::Client::new();
    let graphql_url = "http://localhost:8080/graphql";
    
    // Step 1: Create a workflow definition
    let workflow_query = json!({
        "query": r#"
            mutation CreateWorkflow($input: WorkflowDefinitionInput!) {
                createWorkflow(input: $input) {
                    id
                    name
                    places
                    initialPlace
                    transitions {
                        id
                        fromPlaces
                        toPlace
                    }
                }
            }
        "#,
        "variables": {
            "input": {
                "name": "NATS Document Review Process",
                "description": "A document review workflow using NATS streaming",
                "places": ["draft", "review", "approved", "published", "rejected"],
                "initialPlace": "draft",
                "transitions": [
                    {
                        "id": "submit_for_review",
                        "fromPlaces": ["draft"],
                        "toPlace": "review",
                        "conditions": [],
                        "description": "Submit document for review"
                    },
                    {
                        "id": "approve",
                        "fromPlaces": ["review"],
                        "toPlace": "approved",
                        "conditions": [],
                        "description": "Approve the document"
                    },
                    {
                        "id": "reject",
                        "fromPlaces": ["review"],
                        "toPlace": "rejected",
                        "conditions": [],
                        "description": "Reject the document"
                    },
                    {
                        "id": "publish",
                        "fromPlaces": ["approved"],
                        "toPlace": "published",
                        "conditions": [],
                        "description": "Publish the document"
                    }
                ]
            }
        }
    });
    
    let response = client
        .post(graphql_url)
        .json(&workflow_query)
        .send()
        .await?;
    
    let workflow_result: serde_json::Value = response.json().await?;
    
    if let Some(errors) = workflow_result.get("errors") {
        println!("❌ Failed to create workflow: {}", errors);
        return Ok(());
    }
    
    let workflow_data = workflow_result["data"]["createWorkflow"].clone();
    let workflow_id = workflow_data["id"].as_str().unwrap();
    
    println!("✅ Created workflow: {} (ID: {})", workflow_data["name"], workflow_id);
    
    // Step 2: Create workflow instances using NATS-enhanced mutations
    println!("\n📄 Creating workflow instances with NATS tracking...");
    
    let instances = vec![
        ("Technical Specification", "engineering"),
        ("Marketing Proposal", "marketing"),
        ("Legal Contract", "legal")
    ];
    
    let mut token_ids = Vec::new();
    
    for (doc_title, department) in instances {
        let instance_query = json!({
            "query": r#"
                mutation CreateWorkflowInstance($input: CreateWorkflowInstanceInput!) {
                    createWorkflowInstance(input: $input) {
                        id
                        workflowId
                        place
                        natsSequence
                        natsTimestamp
                        natsSubject
                        transitionHistory {
                            fromPlace
                            toPlace
                            transitionId
                            timestamp
                            triggeredBy
                            natsSequence
                        }
                    }
                }
            "#,
            "variables": {
                "input": {
                    "workflowId": workflow_id,
                    "initialData": {
                        "title": doc_title,
                        "content": format!("This is the content for {}", doc_title),
                        "priority": "medium"
                    },
                    "metadata": {
                        "department": department,
                        "created_by": "demo_user",
                        "urgency": "normal"
                    },
                    "triggeredBy": "nats_demo"
                }
            }
        });
        
        let response = client
            .post(graphql_url)
            .json(&instance_query)
            .send()
            .await?;
        
        let instance_result: serde_json::Value = response.json().await?;
        
        if let Some(errors) = instance_result.get("errors") {
            println!("❌ Failed to create instance for {}: {}", doc_title, errors);
            continue;
        }
        
        let token_data = instance_result["data"]["createWorkflowInstance"].clone();
        let token_id = token_data["id"].as_str().unwrap().to_string();
        token_ids.push(token_id.clone());
        
        println!("📝 Created instance: {} (Token: {})", doc_title, token_id);
        println!("   📍 Place: {}", token_data["place"]);
        println!("   🔗 NATS Subject: {}", token_data["natsSubject"].as_str().unwrap_or("N/A"));
        
        if let Some(sequence) = token_data["natsSequence"].as_str() {
            println!("   📊 NATS Sequence: {}", sequence);
        }
    }
    
    // Step 3: Query tokens in specific places using NATS-optimized queries
    println!("\n🔍 Querying tokens in 'draft' place using NATS...");
    
    let place_query = json!({
        "query": r#"
            query TokensInPlace($workflowId: String!, $placeId: String!) {
                tokensInPlace(workflowId: $workflowId, placeId: $placeId) {
                    id
                    place
                    data
                    natsSequence
                    natsSubject
                    transitionHistory {
                        fromPlace
                        toPlace
                        timestamp
                        triggeredBy
                    }
                }
            }
        "#,
        "variables": {
            "workflowId": workflow_id,
            "placeId": "draft"
        }
    });
    
    let response = client
        .post(graphql_url)
        .json(&place_query)
        .send()
        .await?;
    
    let place_result: serde_json::Value = response.json().await?;
    
    if let Some(errors) = place_result.get("errors") {
        println!("❌ Failed to query tokens in place: {}", errors);
    } else {
        let tokens = place_result["data"]["tokensInPlace"].as_array().unwrap();
        println!("📊 Found {} tokens in 'draft' place", tokens.len());
        
        for token in tokens {
            println!("   🎫 Token {}: {}", 
                token["id"].as_str().unwrap(),
                token["data"]["title"].as_str().unwrap_or("Unknown")
            );
        }
    }
    
    // Step 4: Perform transitions with NATS event tracking
    println!("\n⚡ Performing transitions with NATS event tracking...");
    
    if let Some(first_token_id) = token_ids.first() {
        let transition_query = json!({
            "query": r#"
                mutation TransitionTokenWithNATS($input: TransitionTokenWithNATSInput!) {
                    transitionTokenWithNats(input: $input) {
                        id
                        place
                        natsSequence
                        natsTimestamp
                        transitionHistory {
                            fromPlace
                            toPlace
                            transitionId
                            timestamp
                            triggeredBy
                            natsSequence
                        }
                    }
                }
            "#,
            "variables": {
                "input": {
                    "tokenId": first_token_id,
                    "transitionId": "submit_for_review",
                    "newPlace": "review",
                    "triggeredBy": "nats_demo_transition",
                    "data": {
                        "reviewed_by": "demo_reviewer",
                        "review_notes": "Ready for review"
                    }
                }
            }
        });
        
        let response = client
            .post(graphql_url)
            .json(&transition_query)
            .send()
            .await?;
        
        let transition_result: serde_json::Value = response.json().await?;
        
        if let Some(errors) = transition_result.get("errors") {
            println!("❌ Failed to perform transition: {}", errors);
        } else {
            let token_data = transition_result["data"]["transitionTokenWithNats"].clone();
            println!("✅ Transitioned token {} to place: {}", 
                first_token_id, 
                token_data["place"].as_str().unwrap()
            );
            
            let history = token_data["transitionHistory"].as_array().unwrap();
            if let Some(last_transition) = history.last() {
                println!("   📈 Transition: {} → {}", 
                    last_transition["fromPlace"].as_str().unwrap(),
                    last_transition["toPlace"].as_str().unwrap()
                );
                println!("   👤 Triggered by: {}", 
                    last_transition["triggeredBy"].as_str().unwrap_or("Unknown")
                );
                if let Some(sequence) = last_transition["natsSequence"].as_str() {
                    println!("   📊 NATS Sequence: {}", sequence);
                }
            }
        }
    }
    
    // Step 5: Demonstrate NATS-enhanced token retrieval
    println!("\n🔎 Retrieving token with NATS metadata...");
    
    if let Some(token_id) = token_ids.first() {
        let nats_token_query = json!({
            "query": r#"
                query GetNATSToken($id: String!) {
                    natsToken(id: $id) {
                        id
                        workflowId
                        place
                        data
                        natsSequence
                        natsTimestamp
                        natsSubject
                        transitionHistory {
                            fromPlace
                            toPlace
                            transitionId
                            timestamp
                            triggeredBy
                            natsSequence
                            metadata
                        }
                    }
                }
            "#,
            "variables": {
                "id": token_id
            }
        });
        
        let response = client
            .post(graphql_url)
            .json(&nats_token_query)
            .send()
            .await?;
        
        let nats_result: serde_json::Value = response.json().await?;
        
        if let Some(errors) = nats_result.get("errors") {
            println!("❌ Failed to get NATS token: {}", errors);
        } else if let Some(token_data) = nats_result["data"]["natsToken"].as_object() {
            println!("🎫 NATS Token Details:");
            println!("   📋 ID: {}", token_data["id"].as_str().unwrap());
            println!("   📍 Current Place: {}", token_data["place"].as_str().unwrap());
            println!("   🔗 NATS Subject: {}", token_data["natsSubject"].as_str().unwrap_or("N/A"));
            
            if let Some(sequence) = token_data["natsSequence"].as_str() {
                println!("   📊 NATS Sequence: {}", sequence);
            }
            
            if let Some(timestamp) = token_data["natsTimestamp"].as_str() {
                println!("   ⏰ NATS Timestamp: {}", timestamp);
            }
            
            let history = token_data["transitionHistory"].as_array().unwrap();
            println!("   📈 Transition History ({} events):", history.len());
            
            for (i, transition) in history.iter().enumerate() {
                println!("      {}. {} → {} ({})", 
                    i + 1,
                    transition["fromPlace"].as_str().unwrap(),
                    transition["toPlace"].as_str().unwrap(),
                    transition["transitionId"].as_str().unwrap()
                );
                if let Some(triggered_by) = transition["triggeredBy"].as_str() {
                    println!("         👤 Triggered by: {}", triggered_by);
                }
            }
        }
    }
    
    println!("\n🎉 NATS Integration Demo Features Demonstrated:");
    println!("   ✅ NATS JetStream storage backend");
    println!("   ✅ Automatic stream creation per workflow");
    println!("   ✅ Enhanced token tracking with NATS metadata");
    println!("   ✅ Event-driven transition recording");
    println!("   ✅ Efficient place-based token queries");
    println!("   ✅ Real-time transition history with NATS sequences");
    println!("   ✅ GraphQL API integration with NATS storage");
    
    Ok(())
}