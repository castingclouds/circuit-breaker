// Basic workflow example demonstrating GraphQL API usage
// This shows how Rust applications should interact via GraphQL for consistency
// Run with: cargo run --example basic_workflow

use serde_json::json;

// GraphQL endpoint
const GRAPHQL_ENDPOINT: &str = "http://localhost:4000/graphql";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 Circuit Breaker - Basic Workflow Demo (GraphQL)");
    println!("==================================================");
    println!("Demonstrating: Rust → GraphQL → NATS Backend");
    println!("(Same API as TypeScript, Python, etc.)");
    println!();

    // Create HTTP client
    let client = reqwest::Client::new();

    // 1. Create a document review workflow via GraphQL
    println!("📋 Creating Document Review Workflow via GraphQL...");
    let workflow_definition = json!({
        "name": "Document Review Process",
        "states": [
            "init", "processing", "review", "approved", "published", "rejected"
        ],
        "activities": [
            {
                "id": "start_processing",
                "fromStates": ["init"],
                "toState": "processing",
                "conditions": []
            },
            {
                "id": "submit_for_review",
                "fromStates": ["processing"],
                "toState": "review",
                "conditions": []
            },
            {
                "id": "approve",
                "fromStates": ["review"],
                "toState": "approved",
                "conditions": []
            },
            {
                "id": "reject",
                "fromStates": ["review"],
                "toState": "rejected",
                "conditions": []
            },
            {
                "id": "publish",
                "fromStates": ["approved"],
                "toState": "published",
                "conditions": []
            }
        ],
        "initialState": "init"
    });

    let create_workflow_query = json!({
        "query": r#"
            mutation CreateWorkflow($input: WorkflowDefinitionInput!) {
                createWorkflow(input: $input) {
                    id
                    name
                    states
                    activities {
                        id
                        fromStates
                        toState
                    }
                    initialState
                }
            }
        "#,
        "variables": {
            "input": workflow_definition
        }
    });

    let workflow_result: serde_json::Value = client
        .post(GRAPHQL_ENDPOINT)
        .json(&create_workflow_query)
        .send()
        .await?
        .json()
        .await?;

    if let Some(errors) = workflow_result.get("errors") {
        println!("❌ Failed to create workflow: {}", errors);
        return Ok(());
    }

    let workflow = &workflow_result["data"]["createWorkflow"];
    let workflow_id = workflow["id"].as_str().unwrap();
    println!(
        "✅ Created workflow: {} ({})",
        workflow["name"], workflow_id
    );
    println!("   States: {:?}", workflow["states"]);
    println!(
        "   Activities: {}",
        workflow["activities"].as_array().unwrap().len()
    );
    println!();

    // 2. Create a document resource via GraphQL
    println!("📄 Creating document resource...");
    let resource_data = json!({
        "title": "Technical Documentation",
        "author": "Engineering Team",
        "document_type": "specification",
        "priority": "high",
        "content": "This is a technical document that needs review.",
        "word_count": 150
    });

    let create_resource_query = json!({
        "query": r#"
            mutation CreateResource($input: ResourceCreateInput!) {
                createResource(input: $input) {
                    id
                    workflowId
                    state
                    data
                    metadata
                    createdAt
                }
            }
        "#,
        "variables": {
            "input": {
                "workflowId": workflow_id,
                "initialState": "init",
                "data": resource_data,
                "metadata": {
                    "created_by": "rust-demo",
                    "department": "engineering"
                }
            }
        }
    });

    let resource_result: serde_json::Value = client
        .post(GRAPHQL_ENDPOINT)
        .json(&create_resource_query)
        .send()
        .await?
        .json()
        .await?;

    if let Some(errors) = resource_result.get("errors") {
        println!("❌ Failed to create resource: {}", errors);
        return Ok(());
    }

    let resource = &resource_result["data"]["createResource"];
    let resource_id = resource["id"].as_str().unwrap();
    println!("✅ Created resource: {}", resource_id);
    println!("   Title: {}", resource["data"]["title"]);
    println!("   Current state: {}", resource["state"]);
    println!();

    // 3. Execute activities via GraphQL
    println!("⚡ Executing document review workflow...");
    let activities = vec![
        ("start_processing", "Start Processing"),
        ("submit_for_review", "Submit for Review"),
        ("approve", "Approve Document"),
        ("publish", "Publish Document"),
    ];

    let mut current_resource_id = resource_id.to_string();

    for (activity_id, description) in activities {
        println!("   ➡️  {} ({})", description, activity_id);

        let execute_activity_query = json!({
            "query": r#"
                mutation ExecuteActivity($input: ActivityExecuteInput!) {
                    executeActivity(input: $input) {
                        id
                        state
                        workflowId
                        data
                        history {
                            timestamp
                            activity
                            fromState
                            toState
                        }
                    }
                }
            "#,
            "variables": {
                "input": {
                    "resourceId": current_resource_id,
                    "activityId": activity_id,
                    "data": {
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                        "performed_by": "rust-demo"
                    }
                }
            }
        });

        match client
            .post(GRAPHQL_ENDPOINT)
            .json(&execute_activity_query)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?
        {
            activity_result => {
                if let Some(data) = activity_result.get("data") {
                    let updated_resource = &data["executeActivity"];
                    println!("   ✅ New state: {}", updated_resource["state"]);

                    // Simulate processing time
                    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                } else if let Some(errors) = activity_result.get("errors") {
                    println!("   ❌ Activity failed: {}", errors[0]["message"]);
                    break;
                }
            }
        }
    }

    // 4. Query final resource state
    println!();
    println!("📊 Querying final resource state...");
    let get_resource_query = json!({
        "query": r#"
            query GetResource($id: String!) {
                resource(id: $id) {
                    id
                    state
                    data
                    history {
                        timestamp
                        activity
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

    let final_result: serde_json::Value = client
        .post(GRAPHQL_ENDPOINT)
        .json(&get_resource_query)
        .send()
        .await?
        .json()
        .await?;

    if let Some(final_resource) = final_result["data"]["resource"].as_object() {
        println!("✅ Final resource state:");
        println!("   ID: {}", final_resource["id"]);
        println!("   State: {}", final_resource["state"]);
        println!("   Title: {}", final_resource["data"]["title"]);

        if let Some(history) = final_resource["history"].as_array() {
            println!("   History ({} events):", history.len());
            for (i, event) in history.iter().enumerate() {
                println!(
                    "     {}. {} → {} via {}",
                    i + 1,
                    event["fromState"],
                    event["toState"],
                    event["activity"]
                );
            }
        }
    }

    println!();
    println!("🎯 Basic Workflow Demo demonstrates:");
    println!("   • Workflow creation via GraphQL API");
    println!("   • Resource lifecycle management");
    println!("   • Activity execution with state transitions");
    println!("   • Audit trail and history tracking");
    println!("   • Consistent API across all languages");
    println!("   • NATS-backed persistent storage");

    Ok(())
}
