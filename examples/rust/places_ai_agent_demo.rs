use dotenv::dotenv;
use serde_json::json;
use std::env;
use tokio::time::{sleep, Duration};

use circuit_breaker::{
    engine::{
        AgentEngine, AgentEngineConfig, AgentStorage, InMemoryAgentStorage, InMemoryStorage,
        RulesEngine, WorkflowStorage,
    },
    models::{
        ActivityDefinition, AgentDefinition, AgentId, AgentPrompts, AgentRetryConfig, LLMConfig,
        LLMProvider, Resource, Rule, StateAgentConfig, StateAgentSchedule, StateId,
        WorkflowDefinition,
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    if let Err(e) = dotenv() {
        eprintln!("Warning: Could not load .env file: {}", e);
        eprintln!("Make sure to copy .env.example to .env and configure your API keys");
    }

    println!("🤖 State AI Agent Demo");
    println!("=======================");

    // Initialize storage and engines
    let workflow_storage = std::sync::Arc::new(InMemoryStorage::default());
    let agent_storage = std::sync::Arc::new(InMemoryAgentStorage::default());
    let rules_engine = std::sync::Arc::new(RulesEngine::new());

    let agent_engine = AgentEngine::new(
        agent_storage.clone(),
        rules_engine.clone(),
        AgentEngineConfig::default(),
    );

    // Create a demo workflow
    println!("\n📋 Creating demo workflow...");
    let workflow = create_demo_workflow();
    workflow_storage.create_workflow(workflow).await?;

    // Create demo agents
    println!("🤖 Creating demo agents...");
    let classification_agent = create_classification_agent();
    let review_agent = create_review_agent();

    agent_storage.store_agent(&classification_agent).await?;
    agent_storage.store_agent(&review_agent).await?;

    // Configure Places AI Agents
    println!("⚙️  Configuring State AI Agents...");
    let classification_config = create_classification_state_config();
    let review_config = create_review_state_config();

    agent_storage
        .store_state_agent_config(&classification_config)
        .await?;
    agent_storage
        .store_state_agent_config(&review_config)
        .await?;

    // Subscribe to agent execution stream
    let mut stream_receiver = agent_engine.subscribe_to_stream();

    // Spawn task to handle stream events
    let stream_handle = tokio::spawn(async move {
        while let Ok(event) = stream_receiver.recv().await {
            match event {
                circuit_breaker::models::AgentStreamEvent::ThinkingStatus {
                    execution_id,
                    status,
                } => {
                    println!("🧠 Agent {}: {}", execution_id, status);
                }
                circuit_breaker::models::AgentStreamEvent::ContentChunk {
                    execution_id,
                    chunk,
                    ..
                } => {
                    println!("📝 Agent {}: {}", execution_id, chunk);
                }
                circuit_breaker::models::AgentStreamEvent::Completed {
                    execution_id,
                    final_response,
                    ..
                } => {
                    println!("✅ Agent {} completed: {}", execution_id, final_response);
                }
                circuit_breaker::models::AgentStreamEvent::Failed {
                    execution_id,
                    error,
                } => {
                    println!("❌ Agent {} failed: {}", execution_id, error);
                }
                _ => {}
            }
        }
    });

    // Create and process demo resources
    println!("\n🎯 Creating demo resources...");

    // Resource 1: Document for classification
    let mut doc_resource =
        Resource::new("document_workflow", StateId::from("pending_classification"));
    if let serde_json::Value::Object(ref mut map) = &mut doc_resource.data {
        map.insert(
            "content".to_string(),
            json!("This is a technical document about Rust programming and async/await patterns."),
        );
    }
    doc_resource
        .metadata
        .insert("type".to_string(), json!("document"));
    doc_resource
        .metadata
        .insert("status".to_string(), json!("unclassified"));

    println!("📄 Created document resource: {}", doc_resource.id);
    workflow_storage
        .create_resource(doc_resource.clone())
        .await?;

    // Execute state agents for the document resource
    println!("\n🚀 Executing state agents for document resource...");
    let executions = agent_engine.execute_state_agents(&doc_resource).await?;
    println!("📊 Started {} agent executions", executions.len());

    // Wait for executions to complete
    sleep(Duration::from_secs(2)).await;

    // Resource 2: Content for review
    let mut content_resource = Resource::new("document_workflow", StateId::from("pending_review"));
    if let serde_json::Value::Object(ref mut map) = &mut content_resource.data {
        map.insert(
            "content".to_string(),
            json!("This blog post discusses the benefits of using Rust for systems programming."),
        );
        map.insert("classification".to_string(), json!("technical_article"));
    }
    content_resource
        .metadata
        .insert("type".to_string(), json!("blog_post"));
    content_resource
        .metadata
        .insert("priority".to_string(), json!("high"));

    println!("\n📝 Created content resource: {}", content_resource.id);
    workflow_storage
        .create_resource(content_resource.clone())
        .await?;

    // Execute state agents for the content resource
    println!("🚀 Executing state agents for content resource...");
    let content_executions = agent_engine.execute_state_agents(&content_resource).await?;
    println!("📊 Started {} agent executions", content_executions.len());

    // Wait for executions to complete
    sleep(Duration::from_secs(2)).await;

    // Resource 3: Resource that doesn't meet conditions
    let mut excluded_resource =
        Resource::new("document_workflow", StateId::from("pending_classification"));
    if let serde_json::Value::Object(ref mut map) = &mut excluded_resource.data {
        map.insert("content".to_string(), json!("Short text"));
    }
    excluded_resource
        .metadata
        .insert("type".to_string(), json!("note"));
    excluded_resource
        .metadata
        .insert("status".to_string(), json!("classified")); // Already classified

    println!("\n📝 Created excluded resource: {}", excluded_resource.id);
    workflow_storage
        .create_resource(excluded_resource.clone())
        .await?;

    // This should not trigger agents due to conditions
    println!("🚀 Executing state agents for excluded resource...");
    let excluded_executions = agent_engine
        .execute_state_agents(&excluded_resource)
        .await?;
    println!(
        "📊 Started {} agent executions (should be 0)",
        excluded_executions.len()
    );

    // Display execution statistics
    println!("\n📈 Agent Execution Statistics:");

    let classification_stats = agent_engine
        .get_execution_stats(&AgentId::from("content-classifier"))
        .await?;
    println!("🏷️  Classification Agent:");
    println!(
        "   Total: {}, Completed: {}, Failed: {}, Running: {}",
        classification_stats.total,
        classification_stats.completed,
        classification_stats.failed,
        classification_stats.running
    );
    if let Some(avg_duration) = classification_stats.avg_duration_ms {
        println!("   Average Duration: {}ms", avg_duration);
    }

    let review_stats = agent_engine
        .get_execution_stats(&AgentId::from("content-reviewer"))
        .await?;
    println!("📋 Review Agent:");
    println!(
        "   Total: {}, Completed: {}, Failed: {}, Running: {}",
        review_stats.total, review_stats.completed, review_stats.failed, review_stats.running
    );
    if let Some(avg_duration) = review_stats.avg_duration_ms {
        println!("   Average Duration: {}ms", avg_duration);
    }

    // List all executions for the document resource
    println!("\n📋 Executions for document resource:");
    let doc_executions = agent_storage
        .list_executions_for_resource(&doc_resource.id)
        .await?;
    for execution in doc_executions {
        println!(
            "   🤖 Agent: {}, Status: {:?}, Duration: {:?}ms",
            execution.agent_id.as_str(),
            execution.status,
            execution.duration_ms
        );
    }

    // Cleanup
    stream_handle.abort();

    println!("\n✨ Demo completed successfully!");
    Ok(())
}

fn create_demo_workflow() -> WorkflowDefinition {
    WorkflowDefinition {
        id: "document_workflow".to_string(),
        name: "Document Processing Workflow".to_string(),
        initial_state: StateId::from("pending_classification"),
        states: vec![
            StateId::from("pending_classification"),
            StateId::from("classified"),
            StateId::from("pending_review"),
            StateId::from("reviewed"),
            StateId::from("published"),
        ],
        activities: vec![
            ActivityDefinition {
                id: "classify".into(),
                from_states: vec![StateId::from("pending_classification")],
                to_state: StateId::from("classified"),
                conditions: vec![],
                rules: vec![],
            },
            ActivityDefinition {
                id: "review".into(),
                from_states: vec![StateId::from("classified")],
                to_state: StateId::from("pending_review"),
                conditions: vec![],
                rules: vec![],
            },
            ActivityDefinition {
                id: "publish".into(),
                from_states: vec![StateId::from("reviewed")],
                to_state: StateId::from("published"),
                conditions: vec![],
                rules: vec![],
            },
        ],
    }
}

fn create_classification_agent() -> AgentDefinition {
    use chrono::Utc;

    AgentDefinition {
        id: AgentId::from("content-classifier"),
        name: "Content Classification Agent".to_string(),
        description: "Classifies content into categories".to_string(),
        // Using Anthropic as default (requires ANTHROPIC_API_KEY in .env)
        llm_provider: LLMProvider::Anthropic {
            api_key: env::var("ANTHROPIC_API_KEY").unwrap_or_else(|_| "demo-key".to_string()),
            model: env::var("ANTHROPIC_DEFAULT_MODEL").unwrap_or_else(|_| "claude-3-5-sonnet-20241022".to_string()),
            base_url: env::var("ANTHROPIC_BASE_URL").ok(),
        },
        // Alternative providers (uncomment to use):
        // OpenAI:
        // llm_provider: LLMProvider::OpenAI {
        //     api_key: env::var("OPENAI_API_KEY").unwrap_or_else(|_| "demo-key".to_string()),
        //     model: env::var("OPENAI_DEFAULT_MODEL").unwrap_or_else(|_| "gpt-4".to_string()),
        //     base_url: env::var("OPENAI_BASE_URL").ok(),
        // },
        // Google Gemini:
        // llm_provider: LLMProvider::Google {
        //     api_key: env::var("GOOGLE_API_KEY").unwrap_or_else(|_| "demo-key".to_string()),
        //     model: env::var("GOOGLE_DEFAULT_MODEL").unwrap_or_else(|_| "gemini-pro".to_string()),
        // },
        // Ollama (local):
        // llm_provider: LLMProvider::Ollama {
        //     base_url: env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://localhost:11434".to_string()),
        //     model: env::var("OLLAMA_DEFAULT_MODEL").unwrap_or_else(|_| "llama2".to_string()),
        // },
        llm_config: LLMConfig {
            temperature: 0.2,  // Lower temperature for consistent classification
            max_tokens: Some(200),
            top_p: Some(0.9),
            frequency_penalty: Some(0.0),
            presence_penalty: Some(0.0),
            stop_sequences: vec!["CLASSIFICATION COMPLETE".to_string()],
        },
        prompts: AgentPrompts {
            system: "You are a content classification expert. Analyze the provided content and classify it into one of these categories: technical_article, blog_post, documentation, tutorial, news, other.".to_string(),
            user_template: "Please classify this content: {content}\n\nContent type: {content_type}".to_string(),
            context_instructions: Some("Focus on the technical depth and intended audience.".to_string()),
        },
        capabilities: vec!["content_analysis".to_string(), "categorization".to_string()],
        tools: vec![],
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn create_review_agent() -> AgentDefinition {
    use chrono::Utc;

    AgentDefinition {
        id: AgentId::from("content-reviewer"),
        name: "Content Review Agent".to_string(),
        description: "Reviews content for quality and accuracy".to_string(),
        // Using Anthropic as default (requires ANTHROPIC_API_KEY in .env)
        llm_provider: LLMProvider::Anthropic {
            api_key: env::var("ANTHROPIC_API_KEY").unwrap_or_else(|_| "demo-key".to_string()),
            model: env::var("ANTHROPIC_DEFAULT_MODEL").unwrap_or_else(|_| "claude-3-5-sonnet-20241022".to_string()),
            base_url: env::var("ANTHROPIC_BASE_URL").ok(),
        },
        // Alternative providers (uncomment to use):
        // OpenAI:
        // llm_provider: LLMProvider::OpenAI {
        //     api_key: env::var("OPENAI_API_KEY").unwrap_or_else(|_| "demo-key".to_string()),
        //     model: env::var("OPENAI_DEFAULT_MODEL").unwrap_or_else(|_| "gpt-4".to_string()),
        //     base_url: env::var("OPENAI_BASE_URL").ok(),
        // },
        // Google Gemini:
        // llm_provider: LLMProvider::Google {
        //     api_key: env::var("GOOGLE_API_KEY").unwrap_or_else(|_| "demo-key".to_string()),
        //     model: env::var("GOOGLE_DEFAULT_MODEL").unwrap_or_else(|_| "gemini-pro".to_string()),
        // },
        // Ollama (local):
        // llm_provider: LLMProvider::Ollama {
        //     base_url: env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://localhost:11434".to_string()),
        //     model: env::var("OLLAMA_DEFAULT_MODEL").unwrap_or_else(|_| "llama2".to_string()),
        // },
        llm_config: LLMConfig {
            temperature: 0.3,
            max_tokens: Some(500),
            top_p: Some(0.9),
            frequency_penalty: Some(0.0),
            presence_penalty: Some(0.0),
            stop_sequences: vec!["REVIEW COMPLETE".to_string()],
        },
        prompts: AgentPrompts {
            system: "You are a content quality reviewer. Analyze content for accuracy, clarity, and completeness. Provide a quality score from 1-10 and specific feedback.".to_string(),
            user_template: "Please review this {content_type} content:\n\n{content}\n\nClassification: {classification}\nPriority: {priority}".to_string(),
            context_instructions: Some("Focus on technical accuracy and readability.".to_string()),
        },
        capabilities: vec!["content_review".to_string(), "quality_assessment".to_string()],
        tools: vec![],
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn create_classification_state_config() -> StateAgentConfig {
    let mut config = StateAgentConfig::new(
        StateId::from("pending_classification"),
        AgentId::from("content-classifier"),
    );

    // Override LLM config for classification-specific needs
    config.llm_config = Some(LLMConfig {
        temperature: 0.1, // Very low temperature for consistent classification with Anthropic
        max_tokens: Some(100),
        top_p: Some(0.9),
        frequency_penalty: Some(0.0),
        presence_penalty: Some(0.0),
        stop_sequences: vec!["CLASSIFICATION COMPLETE".to_string()],
    });

    // Only trigger for unclassified content with actual content
    config.trigger_conditions = vec![
        Rule::field_exists("has_content", "data.content"),
        Rule::field_equals("unclassified", "metadata.status", json!("unclassified")),
    ];

    config.input_mapping = [
        ("content".to_string(), "data.content".to_string()),
        ("content_type".to_string(), "metadata.type".to_string()),
    ]
    .iter()
    .cloned()
    .collect();

    config.output_mapping = [
        ("data.classification".to_string(), "category".to_string()),
        (
            "data.confidence".to_string(),
            "confidence_score".to_string(),
        ),
        ("metadata.classifier".to_string(), "agent_id".to_string()),
        (
            "metadata.classified_at".to_string(),
            "timestamp".to_string(),
        ),
    ]
    .iter()
    .cloned()
    .collect();

    config.schedule = Some(StateAgentSchedule {
        initial_delay_seconds: Some(2),
        interval_seconds: None,
        max_executions: Some(1),
    });

    config.retry_config = Some(AgentRetryConfig {
        max_attempts: 2,
        backoff_seconds: 5,
        retry_on_errors: vec!["timeout".to_string(), "rate_limit".to_string()],
    });

    config
}

fn create_review_state_config() -> StateAgentConfig {
    let mut config = StateAgentConfig::new(
        StateId::from("pending_review"),
        AgentId::from("content-reviewer"),
    );

    // Trigger for content that has classification and priority
    config.trigger_conditions = vec![
        Rule::field_exists("has_content", "data.content"),
        Rule::field_exists("has_classification", "data.classification"),
        Rule::field_exists("has_priority", "metadata.priority"),
    ];

    config.input_mapping = [
        ("content".to_string(), "data.content".to_string()),
        ("content_type".to_string(), "metadata.type".to_string()),
        (
            "classification".to_string(),
            "data.classification".to_string(),
        ),
        ("priority".to_string(), "metadata.priority".to_string()),
    ]
    .iter()
    .cloned()
    .collect();

    config.output_mapping = [
        ("data.review_result".to_string(), "assessment".to_string()),
        ("data.review_score".to_string(), "quality_score".to_string()),
        ("metadata.reviewer".to_string(), "agent_id".to_string()),
        (
            "metadata.review_timestamp".to_string(),
            "timestamp".to_string(),
        ),
    ]
    .iter()
    .cloned()
    .collect();

    config.schedule = Some(StateAgentSchedule {
        initial_delay_seconds: Some(1),
        interval_seconds: None,
        max_executions: Some(1),
    });

    config.retry_config = Some(AgentRetryConfig::default());

    config
}
