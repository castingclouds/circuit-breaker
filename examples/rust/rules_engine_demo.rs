// Rules Engine Demo - showing complex rule evaluation for token transitions

use circuit_breaker::models::{
    Token, WorkflowDefinition, TransitionDefinition, PlaceId, Rule
};
use circuit_breaker::engine::RulesEngine;
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🤖 Circuit Breaker Rules Engine Demo");
    println!("=====================================\n");
    
    // Create a rules engine with common rules
    let mut rules_engine = RulesEngine::with_common_rules();
    
    // Add some custom rules for our demo
    rules_engine.register_rule(Rule::field_equals(
        "document_type_article", 
        "document_type", 
        json!("article")
    ));
    
    rules_engine.register_rule(Rule::field_greater_than(
        "word_count_sufficient", 
        "word_count", 
        500.0
    ));
    
    println!("📋 Registered {} rules in the engine", rules_engine.list_rule_ids().len());
    println!("Rules available: {:?}\n", rules_engine.list_rule_ids());
    
    // Create a complex workflow with sophisticated rules
    let workflow = create_publishing_workflow();
    println!("📄 Created workflow: {}", workflow.name);
    println!("Places: {:?}", workflow.places.iter().map(|p| p.as_str()).collect::<Vec<_>>());
    println!("Transitions: {}\n", workflow.transitions.len());
    
    // Create test tokens with different scenarios
    println!("🎯 Creating test tokens with different scenarios...\n");
    
    // Scenario 1: Ready to publish
    let ready_token = create_ready_token();
    demo_token_evaluation(&rules_engine, &ready_token, &workflow, "Ready Article")?;
    
    // Scenario 2: Incomplete article
    let incomplete_token = create_incomplete_token();
    demo_token_evaluation(&rules_engine, &incomplete_token, &workflow, "Incomplete Article")?;
    
    // Scenario 3: Emergency override
    let emergency_token = create_emergency_token();
    demo_token_evaluation(&rules_engine, &emergency_token, &workflow, "Emergency Override")?;
    
    println!("✅ Rules engine demo completed successfully!");
    Ok(())
}

fn create_publishing_workflow() -> WorkflowDefinition {
    // Complex rule: Ready to publish if (high quality AND sufficient length) OR emergency override
    let publish_rule = Rule::or(
        "publish_ready",
        "Ready to publish",
        vec![
            // Normal publishing criteria
            Rule::and(
                "quality_criteria", 
                "High quality article with sufficient content",
                vec![
                    Rule::field_exists("has_content", "content"),
                    Rule::field_exists("has_title", "title"),
                    Rule::field_exists("has_reviewer", "reviewer"),
                    Rule::field_equals("status_approved", "status", json!("approved")),
                    Rule::field_equals("document_type_article", "document_type", json!("article")),
                    Rule::field_greater_than("word_count_sufficient", "word_count", 500.0),
                ]
            ),
            // Emergency override
            Rule::field_equals("emergency_flag", "emergency", json!(true)),
        ]
    );
    
    // Rule for starting review: must have basic content
    let review_rule = Rule::and(
        "review_ready",
        "Ready for review",
        vec![
            Rule::field_exists("has_content", "content"),
            Rule::field_exists("has_title", "title"),
            Rule::field_greater_than("word_count_sufficient", "word_count", 100.0), // Lower threshold for review
        ]
    );
    
    WorkflowDefinition::new(
        "article_publishing",
        "Article Publishing Workflow",
        vec![
            PlaceId::from("draft"),
            PlaceId::from("review"), 
            PlaceId::from("approved"),
            PlaceId::from("published"),
            PlaceId::from("rejected")
        ],
        vec![
            TransitionDefinition::with_rules(
                "submit_for_review",
                vec!["draft"],
                "review",
                vec![review_rule]
            ),
            TransitionDefinition::with_rules(
                "approve_article",
                vec!["review"],
                "approved", 
                vec![Rule::field_exists("has_reviewer", "reviewer")]
            ),
            TransitionDefinition::with_rules(
                "publish_article",
                vec!["approved"],
                "published",
                vec![publish_rule]
            ),
            TransitionDefinition::with_rules(
                "reject_article",
                vec!["review"],
                "rejected",
                vec![Rule::field_exists("has_reviewer", "reviewer")]
            ),
            TransitionDefinition::with_rules(
                "revise_article",
                vec!["rejected"],
                "draft",
                vec![] // No rules - can always revise
            ),
        ],
        "draft"
    )
}

fn create_ready_token() -> Token {
    let mut token = Token::new("article_publishing", PlaceId::from("approved"));
    
    // Set up a complete, high-quality article
    token.data = json!({
        "content": "This is a comprehensive article about the new features in our platform. ".repeat(50),
        "title": "New Platform Features: A Comprehensive Guide",
        "document_type": "article",
        "word_count": 750
    });
    
    token.set_metadata("status", json!("approved"));
    token.set_metadata("reviewer", json!("senior_editor"));
    token.set_metadata("priority", json!(8));
    
    token
}

fn create_incomplete_token() -> Token {
    let mut token = Token::new("article_publishing", PlaceId::from("draft"));
    
    // Set up an incomplete article
    token.data = json!({
        "content": "Just a short draft...",
        "title": "Draft Article",
        "document_type": "article", 
        "word_count": 50
    });
    
    token.set_metadata("status", json!("draft"));
    // Missing reviewer
    
    token
}

fn create_emergency_token() -> Token {
    let mut token = Token::new("article_publishing", PlaceId::from("approved"));
    
    // Set up an article with emergency override
    token.data = json!({
        "content": "Emergency security announcement.",
        "title": "URGENT: Security Update Required",
        "document_type": "article",
        "word_count": 100
    });
    
    token.set_metadata("emergency", json!(true)); // Emergency override
    token.set_metadata("status", json!("pending"));
    
    token
}

fn demo_token_evaluation(
    engine: &RulesEngine,
    token: &Token, 
    workflow: &WorkflowDefinition,
    scenario_name: &str
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Scenario: {}", scenario_name);
    println!("Token is in place: '{}'", token.current_place());
    
    // Get available transitions
    let available = engine.available_transitions(token, workflow);
    println!("Available transitions: {}", available.len());
    
    for transition in &available {
        println!("  ✅ Can fire: '{}' -> '{}'", transition.id.as_str(), transition.to_place.as_str());
    }
    
    // Get detailed evaluation for all transitions
    let detailed = engine.evaluate_all_transitions(token, workflow);
    println!("\nDetailed evaluation:");
    println!("  Available: {}, Blocked: {}", detailed.available_count, detailed.blocked_count);
    
    for result in &detailed.transition_results {
        let status = if result.can_fire { "✅" } else { "❌" };
        println!("  {} {} ({}): {}", 
            status,
            result.transition_id.as_str(),
            if result.place_compatible { "place ok" } else { "wrong place" },
            result.explanation
        );
        
        // Show rule details for complex transitions
        if result.transition_id.as_str() == "publish_article" && !result.rule_results.is_empty() {
            println!("    Rule details:");
            for rule_result in &result.rule_results {
                let rule_status = if rule_result.passed { "✅" } else { "❌" };
                println!("      {} {}: {}", rule_status, rule_result.rule_id, rule_result.explanation);
                
                // Show sub-rule details for complex rules
                for (sub_rule_id, sub_passed) in &rule_result.sub_results {
                    let sub_status = if *sub_passed { "✅" } else { "❌" };
                    println!("        {} {}", sub_status, sub_rule_id);
                }
            }
        }
    }
    
    println!("{}", format!("\n{}\n", "─".repeat(60)));
    Ok(())
} 