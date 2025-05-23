// Basic example showing the refactored Circuit Breaker architecture
// This demonstrates the separation between models, engine, and server

use circuit_breaker::{
    // Core domain models - completely language-agnostic
    Token, PlaceId, TransitionId, WorkflowDefinition, TransitionDefinition,
    // Engine types - GraphQL execution layer
    InMemoryStorage, create_schema_with_storage,
    // Server types - deployable server implementations
    GraphQLServerBuilder,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 Circuit Breaker - Refactored Architecture Demo");
    println!("==================================================");
    println!("📁 src/models/     → Domain-agnostic workflow state management");
    println!("🚀 src/engine/     → GraphQL API for polyglot clients");  
    println!("🖥️  src/server/     → Deployable server implementations");
    println!();

    // 1. Create a generic workflow using core models (src/models)
    let workflow = WorkflowDefinition {
        id: "document_review".to_string(),
        name: "Document Review Process".to_string(),
        places: vec![
            PlaceId::from("init"),
            PlaceId::from("processing"),
            PlaceId::from("review"),
            PlaceId::from("complete"),
            PlaceId::from("failed"),
        ],
        transitions: vec![
            TransitionDefinition {
                id: TransitionId::from("start_processing"),
                from_places: vec![PlaceId::from("init")],
                to_place: PlaceId::from("processing"),
                conditions: vec![],
                rules: vec![],
            },
            TransitionDefinition {
                id: TransitionId::from("submit_for_review"),
                from_places: vec![PlaceId::from("processing")],
                to_place: PlaceId::from("review"),
                conditions: vec![],
                rules: vec![],
            },
            TransitionDefinition {
                id: TransitionId::from("approve"),
                from_places: vec![PlaceId::from("review")],
                to_place: PlaceId::from("complete"),
                conditions: vec![],
                rules: vec![],
            },
            TransitionDefinition {
                id: TransitionId::from("request_changes"),
                from_places: vec![PlaceId::from("review")],
                to_place: PlaceId::from("processing"),
                conditions: vec![],
                rules: vec![],
            },
            TransitionDefinition {
                id: TransitionId::from("fail"),
                from_places: vec![PlaceId::from("processing"), PlaceId::from("review")],
                to_place: PlaceId::from("failed"),
                conditions: vec![],
                rules: vec![],
            },
        ],
        initial_place: PlaceId::from("init"),
    };

    println!("✅ Created workflow using src/models/: {}", workflow.name);
    println!("📊 Places: {:?}", workflow.places.iter().map(|s| s.as_str()).collect::<Vec<_>>());
    println!("🔄 Transitions: {}", workflow.transitions.len());
    println!();

    // 2. Create a token using core models (src/models)
    let mut token = Token::new(&workflow.id, workflow.initial_place.clone());
    token.set_metadata("creator", serde_json::json!("system"));
    token.set_metadata("priority", serde_json::json!("high"));
    
    println!("🎯 Created token using src/models/: {}", token.id);
    println!("🏁 Initial place: {}", token.current_place());
    println!();

    // 3. Execute transitions using core models (src/models)
    println!("🔄 Executing transitions using src/models/...");
    
    // Start processing
    if let Some(target) = workflow.can_transition(&token.place, &TransitionId::from("start_processing")) {
        token.transition_to(target.clone(), TransitionId::from("start_processing"));
        println!("   ➡️  {} -> {}", "init", token.current_place());
    }

    // Submit for review
    if let Some(target) = workflow.can_transition(&token.place, &TransitionId::from("submit_for_review")) {
        token.transition_to(target.clone(), TransitionId::from("submit_for_review"));
        println!("   ➡️  {} -> {}", "processing", token.current_place());
    }

    // Approve
    if let Some(target) = workflow.can_transition(&token.place, &TransitionId::from("approve")) {
        token.transition_to(target.clone(), TransitionId::from("approve"));
        println!("   ➡️  {} -> {}", "review", token.current_place());
    }
    
    println!();
    println!("📈 Transition history:");
    for (i, event) in token.history.iter().enumerate() {
        println!("   {}. {} -> {} via {} ({})", 
            i + 1,
            event.from.as_str(), 
            event.to.as_str(), 
            event.transition.as_str(),
            event.timestamp.format("%H:%M:%S")
        );
    }
    println!();

    // 4. Demonstrate engine layer (src/engine)
    println!("🚀 Creating GraphQL engine using src/engine/...");
    let storage = Box::new(InMemoryStorage::default());
    let _schema = create_schema_with_storage(storage);
    println!("   ✅ GraphQL schema ready for polyglot clients");
    println!("   📋 Query: workflows, tokens, availableTransitions");
    println!("   ✏️  Mutation: createWorkflow, createToken, fireTransition");
    println!("   📡 Subscription: tokenUpdates, workflowEvents (TODO)");
    println!();

    // 5. Demonstrate server layer (src/server)
    println!("🖥️  Creating server using src/server/...");
    let _server_builder = GraphQLServerBuilder::new()
        .with_port(4000);
    println!("   ✅ GraphQLServer configured and ready to deploy");
    println!("   🌐 Would serve GraphQL API at http://localhost:4000/graphql");
    println!("   📊 Includes GraphQL Playground for interactive testing");
    println!();

    // 6. Show the complete architecture benefits
    println!("🏗️  Complete Architecture Benefits:");
    println!("   📦 src/models/  → Pure domain logic, zero external dependencies");
    println!("   🚀 src/engine/  → GraphQL interface, swappable for gRPC, REST, etc.");
    println!("   🖥️  src/server/  → Production-ready servers with config, logging, CORS");
    println!("   🌍 Polyglot     → Any language can define workflows via GraphQL");
    println!("   🔌 Pluggable    → Different storage backends (NATS, PostgreSQL, etc.)");
    println!("   📚 Organized    → Standard Rust project structure for teams");
    println!();
    
    println!("💡 Next steps:");
    println!("   → Run: cargo run --bin server");
    println!("   → Visit: http://localhost:4000/graphql");
    println!("   → Try the example GraphQL queries in the playground!");

    Ok(())
} 