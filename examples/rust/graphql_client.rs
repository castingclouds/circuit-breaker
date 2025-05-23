// Rust GraphQL client example demonstrating polyglot architecture
// This shows how even Rust applications can interact via GraphQL instead of direct model usage
// Useful for distributed architectures, microservices, or when you want API consistency
// Run with: cargo run --example graphql_client

use serde_json::json;

// GraphQL endpoint 
const GRAPHQL_ENDPOINT: &str = "http://localhost:4000/graphql";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🦀 Circuit Breaker - Rust GraphQL Client");
    println!("=========================================");
    println!("Demonstrating: Rust → GraphQL → Rust Backend");
    println!("(Same API as TypeScript, Python, etc.)");
    println!();

    // Create HTTP client
    let client = reqwest::Client::new();

    // 1. Define a Rust-specific workflow via GraphQL
    println!("📋 Creating Rust System Workflow via GraphQL...");
    let workflow_definition = json!({
        "name": "Rust System Development",
        "places": [
            "design", "implementation", "testing", "benchmarking", 
            "optimization", "documentation", "release", "maintenance"
        ],
        "transitions": [
            {
                "id": "start_implementation", 
                "fromPlaces": ["design"], 
                "toPlace": "implementation",
                "conditions": ["architecture_approved", "dependencies_resolved"]
            },
            {
                "id": "start_testing",
                "fromPlaces": ["implementation"],
                "toPlace": "testing", 
                "conditions": ["code_complete", "unit_tests_written"]
            },
            {
                "id": "benchmark",
                "fromPlaces": ["testing"],
                "toPlace": "benchmarking",
                "conditions": ["tests_passing", "integration_complete"]
            },
            {
                "id": "optimize",
                "fromPlaces": ["benchmarking"],
                "toPlace": "optimization",
                "conditions": ["performance_baseline_established"]
            },
            {
                "id": "needs_optimization",
                "fromPlaces": ["optimization"],
                "toPlace": "implementation",
                "conditions": ["performance_below_target"] // Cycle back!
            },
            {
                "id": "document",
                "fromPlaces": ["optimization"],
                "toPlace": "documentation",
                "conditions": ["performance_acceptable"]
            },
            {
                "id": "release",
                "fromPlaces": ["documentation"],
                "toPlace": "release",
                "conditions": ["docs_complete", "changelog_ready"]
            },
            {
                "id": "maintain",
                "fromPlaces": ["release"],
                "toPlace": "maintenance",
                "conditions": ["version_published"]
            },
            {
                "id": "next_version",
                "fromPlaces": ["maintenance"],
                "toPlace": "design",
                "conditions": ["new_features_requested"] // Full cycle!
            }
        ],
        "initialPlace": "design"
    });

    // 2. Send GraphQL mutation to create workflow
    let create_workflow_query = json!({
        "query": r#"
            mutation CreateWorkflow($input: WorkflowDefinitionInput!) {
                createWorkflow(input: $input) {
                    id
                    name
                    places
                    initialPlace
                }
            }
        "#,
        "variables": {
            "input": workflow_definition
        }
    });

    println!("🚀 Sending workflow to generic backend via GraphQL...");
    let response = client
        .post(GRAPHQL_ENDPOINT)
        .json(&create_workflow_query)
        .send()
        .await?;

    let workflow_result: serde_json::Value = response.json().await?;
    
    if let Some(errors) = workflow_result.get("errors") {
        println!("❌ GraphQL Error: {}", errors);
        return Ok(());
    }
    
    let workflow = &workflow_result["data"]["createWorkflow"];
    println!("✅ Workflow created: {}", workflow["id"]);
    println!("   Places: {:?}", workflow["places"]);
    println!();

    // 3. Create a token with Rust-specific data
    println!("🎯 Creating token with Rust project data...");
    let token_data = json!({
        "project_name": "hyper-fast-json-parser",
        "language": "Rust",
        "target_performance": "10x faster than serde_json",
        "memory_safety": "guaranteed",
        "features": ["zero-copy", "simd-optimized", "async-ready"],
        "dependencies": ["tokio", "serde", "criterion"],
        "estimated_completion": "2 months"
    });

    let create_token_query = json!({
        "query": r#"
            mutation CreateToken($input: TokenCreateInput!) {
                createToken(input: $input) {
                    id
                    place
                    workflowId
                    createdAt
                }
            }
        "#,
        "variables": {
            "input": {
                "workflowId": workflow["id"].as_str().unwrap(),
                "data": token_data
            }
        }
    });

    let response = client
        .post(GRAPHQL_ENDPOINT)
        .json(&create_token_query)
        .send()
        .await?;

    let token_result: serde_json::Value = response.json().await?;
    let token = &token_result["data"]["createToken"];
    
    println!("✅ Token created: {}", token["id"]);
    println!("   Initial place: {}", token["place"]);
    println!("   Project: {}", token_data["project_name"]);
    println!();

    // 4. Execute Rust development workflow transitions
    println!("🔄 Executing Rust development workflow...");
    let transitions = vec![
        ("start_implementation", "Begin Rust implementation"),
        ("start_testing", "Write comprehensive tests"),
        ("benchmark", "Run performance benchmarks"),
        ("optimize", "Apply Rust-specific optimizations"),
        ("document", "Write API documentation"),
        ("release", "Publish to crates.io")
    ];

    let current_token_id = token["id"].as_str().unwrap().to_string();

    for (transition_id, description) in transitions {
        println!("   ➡️  {} ({})", description, transition_id);
        
        let fire_transition_query = json!({
            "query": r#"
                mutation FireTransition($input: TransitionFireInput!) {
                    fireTransition(input: $input) {
                        id
                        place
                        updatedAt
                    }
                }
            "#,
            "variables": {
                "input": {
                    "tokenId": current_token_id,
                    "transitionId": transition_id
                }
            }
        });

        match client.post(GRAPHQL_ENDPOINT).json(&fire_transition_query).send().await {
            Ok(response) => {
                let transition_result: serde_json::Value = response.json().await?;
                if let Some(data) = transition_result.get("data") {
                    let updated_token = &data["fireTransition"];
                    println!("   ✅ New place: {}", updated_token["place"]);
                    
                    // Simulate Rust-specific processing time
                    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                } else if let Some(errors) = transition_result.get("errors") {
                    println!("   ❌ Transition failed: {}", errors[0]["message"]);
                }
            }
            Err(e) => {
                println!("   ❌ Network error: {}", e);
                break;
            }
        }
    }

    println!();

    // 5. Query final token state via GraphQL
    println!("📚 Fetching final token state...");
    let get_token_query = json!({
        "query": r#"
            query GetToken($id: String!) {
                token(id: $id) {
                    id
                    workflowId
                    place
                    data
                    history {
                        timestamp
                        transition
                        fromPlace
                        toPlace
                    }
                }
            }
        "#,
        "variables": {
            "id": current_token_id
        }
    });

    let response = client
        .post(GRAPHQL_ENDPOINT)
        .json(&get_token_query)
        .send()
        .await?;

    let history_result: serde_json::Value = response.json().await?;
    let final_token = &history_result["data"]["token"];

    println!("📈 Complete Workflow History:");
    if let Some(history) = final_token["history"].as_array() {
        for (i, event) in history.iter().enumerate() {
            println!("   {}. {} → {} via {} at {}", 
                i + 1,
                event["fromPlace"], 
                event["toPlace"], 
                event["transition"],
                event["timestamp"]
            );
        }
    }

    println!();

    // 6. Demonstrate the key architectural point
    println!("🏗️  Architecture Demonstration:");
    println!("   🦀 Rust Client: Uses GraphQL API (just like TypeScript)");
    println!("   🌐 GraphQL API: Language-agnostic interface");  
    println!("   🦀 Rust Backend: Generic engine, no client knowledge");
    println!("   🔄 Same API: TypeScript, Python, Go, Java all use identical GraphQL");
    println!();
    
    println!("💡 Why use GraphQL from Rust?");
    println!("   • Microservices: Rust service → Circuit Breaker service");
    println!("   • API Consistency: Same interface as all other languages");
    println!("   • Distributed Systems: Network boundaries require serialization anyway");
    println!("   • Multi-team: Backend team owns Circuit Breaker, client teams use GraphQL");
    println!();

    println!("🎉 Rust GraphQL client demo complete!");
    println!("The same backend serves Rust, TypeScript, Python, Go, etc. identically!");

    Ok(())
} 