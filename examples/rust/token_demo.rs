// Example demonstrating the new Generic Rust Token implementation
// This shows how the backend engine is generic and states are defined by GraphQL clients
// Run with: cargo run --example token_demo

use circuit_breaker::{Token, PlaceId, TransitionId, WorkflowDefinition, TransitionDefinition};
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Circuit Breaker Token Lifecycle Demo");
    println!("=======================================\n");

    // Define a simple e-commerce order workflow
    let workflow = WorkflowDefinition {
        id: "order_fulfillment".to_string(),
        name: "E-commerce Order Fulfillment".to_string(),
        places: vec![
            PlaceId::from("cart"),
            PlaceId::from("payment_pending"),
            PlaceId::from("paid"),
            PlaceId::from("shipped"),
            PlaceId::from("delivered"),
            PlaceId::from("cancelled"),
        ],
        transitions: vec![
            TransitionDefinition::new("checkout", vec!["cart"], "payment_pending"),
            TransitionDefinition::new("pay", vec!["payment_pending"], "paid"),
            TransitionDefinition::new("ship", vec!["paid"], "shipped"),
            TransitionDefinition::new("deliver", vec!["shipped"], "delivered"),
            TransitionDefinition::new("cancel", vec!["cart", "payment_pending"], "cancelled"),
        ],
        initial_place: PlaceId::from("cart"),
    };

    // Create a token representing an order
    let mut token = Token::new(&workflow.id, workflow.initial_place.clone());
    
    // Add order data
    token.data = json!({
        "order_id": "ORD-12345",
        "customer_id": "CUST-789",
        "items": [
            {"product": "Laptop", "price": 999.99, "quantity": 1},
            {"product": "Mouse", "price": 29.99, "quantity": 2}
        ],
        "total": 1059.97
    });

    // Add metadata
    token.set_metadata("customer_tier", json!("premium"));
    token.set_metadata("sales_channel", json!("web"));
    token.set_metadata("region", json!("US-West"));

    println!("🛒 Order Created:");
    println!("   ID: {}", token.id);
    println!("   Current Place: {}", token.current_place());
    println!("   Total: ${}", token.data["total"]);
    println!("   Customer Tier: {}", token.get_metadata("customer_tier").unwrap());
    println!();

    // Simulate checkout
    if let Some(target) = workflow.can_transition(&token.place, &TransitionId::from("checkout")) {
        token.transition_to(target.clone(), TransitionId::from("checkout"));
        println!("💳 Checkout initiated");
        println!("   Current Place: {}", token.current_place());
        
        // Add payment method metadata
        token.set_metadata("payment_method", json!("credit_card"));
        token.set_metadata("payment_processor", json!("stripe"));
        println!();
    }

    // Simulate payment
    if let Some(target) = workflow.can_transition(&token.place, &TransitionId::from("pay")) {
        token.transition_to(target.clone(), TransitionId::from("pay"));
        println!("✅ Payment processed");
        println!("   Current Place: {}", token.current_place());
        
        // Add payment confirmation data
        token.data["payment"] = json!({
            "transaction_id": "TXN-ABC123",
            "timestamp": "2024-01-15T10:30:00Z",
            "amount": 1059.97
        });
        println!();
    }

    // Simulate shipping
    if let Some(target) = workflow.can_transition(&token.place, &TransitionId::from("ship")) {
        token.transition_to(target.clone(), TransitionId::from("ship"));
        println!("📦 Order shipped");
        println!("   Current Place: {}", token.current_place());
        
        // Add shipping metadata
        token.set_metadata("carrier", json!("FedEx"));
        token.set_metadata("tracking_number", json!("1234567890"));
        token.data["shipping"] = json!({
            "address": "123 Main St, Seattle, WA 98101",
            "estimated_delivery": "2024-01-18",
            "shipping_cost": 15.99
        });
        println!();
    }

    // Simulate delivery
    if let Some(target) = workflow.can_transition(&token.place, &TransitionId::from("deliver")) {
        token.transition_to(target.clone(), TransitionId::from("deliver"));
        println!("🏠 Order delivered");
        println!("   Current Place: {}", token.current_place());
        
        // Add delivery confirmation
        token.data["delivery"] = json!({
            "delivered_at": "2024-01-17T14:30:00Z",
            "signature": "John Doe",
            "location": "Front door"
        });
        println!();
    }

    // Show final token state
    println!("📊 Final Token State:");
    println!("   Workflow: {}", token.workflow_id);
    println!("   Final Place: {}", token.current_place());
    println!("   Created: {}", token.created_at.format("%Y-%m-%d %H:%M:%S"));
    println!("   Updated: {}", token.updated_at.format("%Y-%m-%d %H:%M:%S"));
    println!();

    // Show transition history
    println!("📚 Transition History:");
    for (i, event) in token.history.iter().enumerate() {
        println!("   {}. {} → {} via {} at {}", 
            i + 1,
            event.from.as_str(),
            event.to.as_str(),
            event.transition.as_str(),
            event.timestamp.format("%H:%M:%S")
        );
    }
    println!();

    // Show metadata
    println!("🏷️  Token Metadata:");
    for (key, value) in &token.metadata {
        println!("   {}: {}", key, value);
    }
    println!();

    // Show data evolution
    println!("📋 Token Data (JSON):");
    println!("{}", serde_json::to_string_pretty(&token.data)?);

    println!("\n✨ Token demo completed!");
    println!("💡 Notice how the token carries rich data and metadata through each state transition,");
    println!("   providing a complete audit trail and context for the entire order lifecycle.");

    Ok(())
} 