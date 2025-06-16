// Rules Engine Demo - showing complex rule evaluation for resource activities

use circuit_breaker::engine::RulesEngine;
use circuit_breaker::models::{ActivityDefinition, Resource, Rule, StateId, WorkflowDefinition};
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
        json!("article"),
    ));

    rules_engine.register_rule(Rule::field_greater_than(
        "word_count_sufficient",
        "word_count",
        500.0,
    ));

    println!(
        "📋 Registered {} rules in the engine",
        rules_engine.list_rule_ids().len()
    );
    println!("Rules available: {:?}\n", rules_engine.list_rule_ids());

    // Create a complex workflow with sophisticated rules
    let workflow = create_publishing_workflow();
    println!("📄 Created workflow: {}", workflow.name);
    println!(
        "States: {:?}",
        workflow
            .states
            .iter()
            .map(|p| p.as_str())
            .collect::<Vec<_>>()
    );
    println!("Activities: {}\n", workflow.activities.len());

    // Create test resources with different scenarios
    println!("🎯 Creating test resources with different scenarios...\n");

    // Scenario 1: Ready to publish
    let ready_resource = create_ready_resource();
    demo_resource_evaluation(&rules_engine, &ready_resource, &workflow, "Ready Article")?;

    // Scenario 2: Incomplete article
    let incomplete_resource = create_incomplete_resource();
    demo_resource_evaluation(
        &rules_engine,
        &incomplete_resource,
        &workflow,
        "Incomplete Article",
    )?;

    // Scenario 3: Emergency override
    let emergency_resource = create_emergency_resource();
    demo_resource_evaluation(
        &rules_engine,
        &emergency_resource,
        &workflow,
        "Emergency Override",
    )?;

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
                ],
            ),
            // Emergency override
            Rule::field_equals("emergency_flag", "emergency", json!(true)),
        ],
    );

    // Rule for starting review: must have basic content
    let review_rule = Rule::and(
        "review_ready",
        "Ready for review",
        vec![
            Rule::field_exists("has_content", "content"),
            Rule::field_exists("has_title", "title"),
            Rule::field_greater_than("word_count_sufficient", "word_count", 100.0), // Lower threshold for review
        ],
    );

    WorkflowDefinition {
        id: "article_publishing".to_string(),
        name: "Article Publishing Workflow".to_string(),
        states: vec![
            StateId::from("draft"),
            StateId::from("review"),
            StateId::from("approved"),
            StateId::from("published"),
            StateId::from("rejected"),
        ],
        activities: vec![
            ActivityDefinition {
                id: "submit_for_review".into(),
                from_states: vec![StateId::from("draft")],
                to_state: StateId::from("review"),
                conditions: vec![],
                rules: vec![review_rule],
            },
            ActivityDefinition {
                id: "approve_article".into(),
                from_states: vec![StateId::from("review")],
                to_state: StateId::from("approved"),
                conditions: vec![],
                rules: vec![Rule::field_exists("has_reviewer", "reviewer")],
            },
            ActivityDefinition {
                id: "publish_article".into(),
                from_states: vec![StateId::from("approved")],
                to_state: StateId::from("published"),
                conditions: vec![],
                rules: vec![publish_rule],
            },
            ActivityDefinition {
                id: "reject_article".into(),
                from_states: vec![StateId::from("review")],
                to_state: StateId::from("rejected"),
                conditions: vec![],
                rules: vec![Rule::field_exists("has_reviewer", "reviewer")],
            },
            ActivityDefinition {
                id: "revise_article".into(),
                from_states: vec![StateId::from("rejected")],
                to_state: StateId::from("draft"),
                conditions: vec![],
                rules: vec![], // No rules - can always revise
            },
        ],
        initial_state: StateId::from("draft"),
    }
}

fn create_ready_resource() -> Resource {
    let mut resource = Resource::new("article_publishing", StateId::from("approved"));

    // Set up a complete, high-quality article
    resource.data = json!({
        "content": "This is a comprehensive article about the new features in our platform. ".repeat(50),
        "title": "Exciting New Features Released!",
        "document_type": "article",
        "word_count": 750
    });

    resource
        .metadata
        .insert("status".to_string(), json!("approved"));
    resource
        .metadata
        .insert("reviewer".to_string(), json!("senior_editor"));
    resource.metadata.insert("priority".to_string(), json!(8));

    resource
}

fn create_incomplete_resource() -> Resource {
    let mut resource = Resource::new("article_publishing", StateId::from("draft"));

    // Set up an incomplete article
    resource.data = json!({
        "content": "Just a short draft...",
        "title": "Draft Article",
        "document_type": "article",
        "word_count": 50
    });

    resource
        .metadata
        .insert("status".to_string(), json!("draft"));
    // Missing reviewer

    resource
}

fn create_emergency_resource() -> Resource {
    let mut resource = Resource::new("article_publishing", StateId::from("approved"));

    // Set up an article with emergency override
    resource.data = json!({
        "content": "Emergency security announcement.",
        "title": "Critical Security Update",
        "document_type": "article",
        "word_count": 100
    });

    resource
        .metadata
        .insert("emergency".to_string(), json!(true)); // Emergency override
    resource
        .metadata
        .insert("status".to_string(), json!("pending"));

    resource
}

fn demo_resource_evaluation(
    _engine: &RulesEngine,
    resource: &Resource,
    workflow: &WorkflowDefinition,
    scenario_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Scenario: {}", scenario_name);
    println!("Resource is in state: '{}'", resource.current_state());

    // Get available activities for this resource
    let current_state = StateId::from(resource.current_state());
    let available_activities = workflow.available_activities(&current_state);
    println!("Available activities: {}", available_activities.len());

    for activity in &available_activities {
        println!(
            "  ✅ Can execute: '{}' -> '{}'",
            activity.id.as_str(),
            activity.to_state.as_str()
        );
    }

    // Show rule evaluation for activities with rules
    println!("\nRule evaluation:");
    for activity in &available_activities {
        if !activity.rules.is_empty() {
            println!("  Activity '{}' rules:", activity.id.as_str());
            for rule in &activity.rules {
                let passed = rule.evaluate(&resource.metadata, &resource.data);
                let status = if passed { "✅" } else { "❌" };
                let result = rule.evaluate_detailed(&resource.metadata, &resource.data);
                println!("    {} {}: {}", status, rule.id, result.explanation);
            }
        } else {
            println!(
                "  Activity '{}': No rules (always available)",
                activity.id.as_str()
            );
        }
    }

    println!("{}", format!("\n{}\n", "─".repeat(60)));
    Ok(())
}
