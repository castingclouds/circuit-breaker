// Resource operations demonstration - Rust GraphQL Client
// Shows detailed resource lifecycle operations using GraphQL API
// Run with: cargo run --example token_demo

use serde_json::json;

// GraphQL endpoint
const GRAPHQL_ENDPOINT: &str = "http://localhost:4000/graphql";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Circuit Breaker Resource Operations Demo - Rust GraphQL Client");
    println!("================================================================");
    println!("Demonstrating: Rust → GraphQL → NATS Backend");
    println!("(Same API as TypeScript, Python, etc.)");
    println!();

    // Create HTTP client
    let client = reqwest::Client::new();

    // 1. Create e-commerce order fulfillment workflow
    println!("📋 Creating E-commerce Order Fulfillment Workflow...");
    let workflow_definition = json!({
        "name": "E-commerce Order Fulfillment",
        "states": [
            "cart", "payment_pending", "paid", "shipped", "delivered", "cancelled"
        ],
        "activities": [
            {
                "id": "checkout",
                "fromStates": ["cart"],
                "toState": "payment_pending",
                "conditions": []
            },
            {
                "id": "pay",
                "fromStates": ["payment_pending"],
                "toState": "paid",
                "conditions": []
            },
            {
                "id": "ship",
                "fromStates": ["paid"],
                "toState": "shipped",
                "conditions": []
            },
            {
                "id": "deliver",
                "fromStates": ["shipped"],
                "toState": "delivered",
                "conditions": []
            },
            {
                "id": "cancel",
                "fromStates": ["cart", "payment_pending"],
                "toState": "cancelled",
                "conditions": []
            }
        ],
        "initialState": "cart"
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
    println!();

    // 2. Create multiple order resources
    println!("📦 Creating order resources...");
    let orders = vec![
        json!({
            "order_id": "ORD-12345",
            "customer_id": "CUST-789",
            "customer_name": "Alice Johnson",
            "items": [
                {"product": "Laptop", "price": 999.99, "quantity": 1},
                {"product": "Mouse", "price": 29.99, "quantity": 2}
            ],
            "total": 1059.97,
            "payment_method": "credit_card"
        }),
        json!({
            "order_id": "ORD-12346",
            "customer_id": "CUST-790",
            "customer_name": "Bob Smith",
            "items": [
                {"product": "Keyboard", "price": 149.99, "quantity": 1}
            ],
            "total": 149.99,
            "payment_method": "paypal"
        }),
        json!({
            "order_id": "ORD-12347",
            "customer_id": "CUST-791",
            "customer_name": "Carol Davis",
            "items": [
                {"product": "Monitor", "price": 299.99, "quantity": 2},
                {"product": "Cables", "price": 19.99, "quantity": 3}
            ],
            "total": 659.95,
            "payment_method": "bank_transfer"
        }),
    ];

    let mut resource_ids = Vec::new();

    for (i, order_data) in orders.iter().enumerate() {
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
                    "initialState": "cart",
                    "data": order_data,
                    "metadata": {
                        "created_by": "rust-demo",
                        "priority": if i == 0 { "high" } else { "normal" },
                        "source": "website"
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
            continue;
        }

        let resource = &resource_result["data"]["createResource"];
        let resource_id = resource["id"].as_str().unwrap();
        resource_ids.push(resource_id.to_string());

        println!(
            "✅ Created order: {} ({})",
            resource["data"]["order_id"], resource_id
        );
    }

    println!("📊 Created {} order resources", resource_ids.len());
    println!();

    // 3. Process orders through different lifecycle paths
    for (i, resource_id) in resource_ids.iter().enumerate() {
        println!("⚡ Processing order {} ({})...", i + 1, resource_id);

        // Get current resource to show order details
        let get_resource_query = json!({
            "query": r#"
                query GetResource($id: String!) {
                    resource(id: $id) {
                        id
                        state
                        data
                    }
                }
            "#,
            "variables": {
                "id": resource_id
            }
        });

        let resource_result: serde_json::Value = client
            .post(GRAPHQL_ENDPOINT)
            .json(&get_resource_query)
            .send()
            .await?
            .json()
            .await?;

        if let Some(resource) = resource_result["data"]["resource"].as_object() {
            println!("   📋 Order ID: {}", resource["data"]["order_id"]);
            println!("   👤 Customer: {}", resource["data"]["customer_name"]);
            println!("   💰 Total: ${}", resource["data"]["total"]);
            println!("   📍 Current state: {}", resource["state"]);
        }

        // Determine order path based on index
        let activities = if i == 2 {
            // Third order gets cancelled
            vec![("cancel", "Cancel Order")]
        } else {
            // Normal fulfillment path
            vec![
                ("checkout", "Proceed to Checkout"),
                ("pay", "Process Payment"),
                ("ship", "Ship Order"),
                ("deliver", "Deliver Order"),
            ]
        };

        // Execute activities
        for (activity_id, description) in activities {
            println!("   ➡️  {}", description);

            let execute_activity_query = json!({
                "query": r#"
                    mutation ExecuteActivity($input: ActivityExecuteInput!) {
                        executeActivity(input: $input) {
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
                    "input": {
                        "resourceId": resource_id,
                        "activityId": activity_id,
                        "data": {
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                            "processed_by": "rust-demo",
                            "activity": activity_id
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
                        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                    } else if let Some(errors) = activity_result.get("errors") {
                        println!("   ❌ Activity failed: {}", errors[0]["message"]);
                        break;
                    }
                }
            }
        }

        println!();
    }

    // 4. Query final states and show summary
    println!("📊 Final Order Summary:");
    println!("=======================");

    for (i, resource_id) in resource_ids.iter().enumerate() {
        let get_final_resource_query = json!({
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
                "id": resource_id
            }
        });

        let final_result: serde_json::Value = client
            .post(GRAPHQL_ENDPOINT)
            .json(&get_final_resource_query)
            .send()
            .await?
            .json()
            .await?;

        if let Some(resource) = final_result["data"]["resource"].as_object() {
            println!("Order {} - {}:", i + 1, resource["data"]["order_id"]);
            println!("   Customer: {}", resource["data"]["customer_name"]);
            println!("   Final State: {}", resource["state"]);
            println!("   Total: ${}", resource["data"]["total"]);

            if let Some(history) = resource["history"].as_array() {
                println!("   Lifecycle ({} steps):", history.len());
                for (j, event) in history.iter().enumerate() {
                    let timestamp = event["timestamp"].as_str().unwrap_or("");
                    let time_part = if let Some(t_pos) = timestamp.find('T') {
                        &timestamp[t_pos + 1..t_pos + 9]
                    } else {
                        timestamp
                    };

                    println!(
                        "     {}. {} → {} via {} ({})",
                        j + 1,
                        event["fromState"],
                        event["toState"],
                        event["activity"],
                        time_part
                    );
                }
            }
            println!();
        }
    }

    println!("🎯 Resource Operations Demo demonstrates:");
    println!("   • Multiple resource creation with different data");
    println!("   • Parallel resource processing");
    println!("   • Different lifecycle paths (normal vs cancelled)");
    println!("   • Complete audit trail for each resource");
    println!("   • GraphQL API consistency across languages");
    println!("   • NATS-backed persistent state management");
    println!();

    println!("💡 This shows the same patterns as:");
    println!("   • TypeScript: npm run demo:token");
    println!("   • Python: python examples/python/token_demo.py");
    println!("   • Any language with GraphQL support!");

    Ok(())
}
