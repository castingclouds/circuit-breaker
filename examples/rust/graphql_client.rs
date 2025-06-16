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
        "states": [
            "design", "implementation", "testing", "benchmarking",
            "optimization", "documentation", "release", "maintenance"
        ],
        "activities": [
            {
                "id": "start_implementation",
                "fromStates": ["design"],
                "toState": "implementation",
                "conditions": ["architecture_approved", "dependencies_resolved"]
            },
            {
                "id": "start_testing",
                "fromStates": ["implementation"],
                "toState": "testing",
                "conditions": ["code_complete", "unit_tests_written"]
            },
            {
                "id": "benchmark",
                "fromStates": ["testing"],
                "toState": "benchmarking",
                "conditions": ["tests_passing", "integration_complete"]
            },
            {
                "id": "optimize",
                "fromStates": ["benchmarking"],
                "toState": "optimization",
                "conditions": ["performance_baseline_established"]
            },
            {
                "id": "needs_optimization",
                "fromStates": ["optimization"],
                "toState": "implementation",
                "conditions": ["performance_below_target"] // Cycle back!
            },
            {
                "id": "document",
                "fromStates": ["optimization"],
                "toState": "documentation",
                "conditions": ["performance_acceptable"]
            },
            {
                "id": "release",
                "fromStates": ["documentation"],
                "toState": "release",
                "conditions": ["docs_complete", "changelog_ready"]
            },
            {
                "id": "maintain",
                "fromStates": ["release"],
                "toState": "maintenance",
                "conditions": ["version_published"]
            },
            {
                "id": "next_version",
                "fromStates": ["maintenance"],
                "toState": "design",
                "conditions": ["new_features_requested"] // Full cycle!
            }
        ],
        "initialState": "design",
    });

    // 2. Send GraphQL mutation to create workflow
    let create_workflow_query = json!({
        "query": r#"
            mutation CreateWorkflow($input: WorkflowDefinitionInput!) {
                createWorkflow(input: $input) {
                    id
                    name
                    states
                    initialState
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
    println!("   States: {:?}", workflow["states"]);
    println!();

    // 3. Create a resource with Rust-specific data
    println!("🎯 Creating resource with Rust project data...");
    let resource_data = json!({
        "project_name": "hyper-fast-json-parser",
        "language": "Rust",
        "target_performance": "10x faster than serde_json",
        "memory_safety": "guaranteed",
        "features": ["zero-copy", "simd-optimized", "async-ready"],
        "dependencies": ["tokio", "serde", "criterion"],
        "estimated_completion": "2 months"
    });

    let create_resource_query = json!({
        "query": r#"
            mutation CreateResource($input: ResourceCreateInput!) {
                createResource(input: $input) {
                    id
                    state
                    workflowId
                    data
                }
            }
        "#,
        "variables": {
            "input": {
                "workflowId": workflow["id"].as_str().unwrap(),
                "data": resource_data
            }
        }
    });

    let response = client
        .post(GRAPHQL_ENDPOINT)
        .json(&create_resource_query)
        .send()
        .await?;

    let resource_result: serde_json::Value = response.json().await?;

    if let Some(errors) = resource_result.get("errors") {
        println!("❌ GraphQL Error creating resource: {}", errors);
        return Ok(());
    }

    let resource = &resource_result["data"]["createResource"];

    println!("✅ Resource created: {}", resource["id"]);
    println!("   Initial state: {}", resource["state"]);
    println!("   Project: {}", resource_data["project_name"]);
    println!();

    // 4. Execute Rust development workflow transitions
    println!("🔄 Executing Rust development workflow...");
    let transitions = vec![
        ("start_implementation", "Begin Rust implementation"),
        ("start_testing", "Write comprehensive tests"),
        ("benchmark", "Run performance benchmarks"),
        ("optimize", "Apply Rust-specific optimizations"),
        ("document", "Write API documentation"),
        ("release", "Publish to crates.io"),
    ];

    let current_resource_id = resource["id"].as_str().unwrap().to_string();

    for (activity_id, description) in transitions {
        println!("   ➡️  {} ({})", description, activity_id);

        let execute_activity_query = json!({
            "query": r#"
                mutation ExecuteActivity($input: ActivityExecuteInput!) {
                    executeActivity(input: $input) {
                        id
                        state
                        workflowId
                    }
                }
            "#,
            "variables": {
                "input": {
                    "resourceId": current_resource_id,
                    "activityId": activity_id
                }
            }
        });

        match client
            .post(GRAPHQL_ENDPOINT)
            .json(&execute_activity_query)
            .send()
            .await
        {
            Ok(response) => {
                let activity_result: serde_json::Value = response.json().await?;
                if let Some(data) = activity_result.get("data") {
                    let updated_resource = &data["executeActivity"];
                    println!("   ✅ New state: {}", updated_resource["state"]);

                    // Simulate Rust-specific processing time
                    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                } else if let Some(errors) = activity_result.get("errors") {
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
    // 5. Query final resource state with history via GraphQL
    println!("\n📊 Querying final resource state via GraphQL...");
    let resource_query = json!({
        "query": r#"
            query GetResource($id: String!) {
                resource(id: $id) {
                    id
                    state
                    workflowId
                    data
                    metadata
                    createdAt
                    updatedAt
                    history {
                        timestamp
                        activityId
                        fromState
                        toState
                    }
                }
            }
        "#,
        "variables": {
            "id": current_resource_id
        }
    });

    let response = client
        .post(GRAPHQL_ENDPOINT)
        .json(&resource_query)
        .send()
        .await?;

    let history_result: serde_json::Value = response.json().await?;
    let final_resource = &history_result["data"]["resource"];

    // 6. Show complete workflow history via GraphQL
    println!("📈 Complete Workflow History:");
    if let Some(history) = final_resource["history"].as_array() {
        for (i, event) in history.iter().enumerate() {
            println!(
                "   {}. {} → {} via {} at {}",
                i + 1,
                event["fromState"],
                event["toState"],
                event["activityId"],
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
