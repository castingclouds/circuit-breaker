use std::env;
use tokio::time::{sleep, Duration};
use serde_json::json;
use dotenv::dotenv;

use circuit_breaker::{
    models::{
        AgentId, AgentDefinition, LLMProvider, LLMConfig, AgentPrompts,
        PlaceAgentConfig, PlaceAgentSchedule, AgentRetryConfig, Token, 
        WorkflowDefinition, PlaceId, TransitionDefinition, Rule
    },
    engine::{
        AgentEngine, AgentStorage, InMemoryAgentStorage, AgentEngineConfig,
        InMemoryStorage, WorkflowStorage, RulesEngine
    }
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    if let Err(e) = dotenv() {
        eprintln!("Warning: Could not load .env file: {}", e);
        eprintln!("Make sure to copy .env.example to .env and configure your API keys");
    }

    println!("🤖 Places AI Agent Demo");
    println!("========================");

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
    println!("⚙️  Configuring Places AI Agents...");
    let classification_config = create_classification_place_config();
    let review_config = create_review_place_config();
    
    agent_storage.store_place_agent_config(&classification_config).await?;
    agent_storage.store_place_agent_config(&review_config).await?;

    // Subscribe to agent execution stream
    let mut stream_receiver = agent_engine.subscribe_to_stream();
    
    // Spawn task to handle stream events
    let stream_handle = tokio::spawn(async move {
        while let Ok(event) = stream_receiver.recv().await {
            match event {
                circuit_breaker::models::AgentStreamEvent::ThinkingStatus { execution_id, status } => {
                    println!("🧠 Agent {}: {}", execution_id, status);
                }
                circuit_breaker::models::AgentStreamEvent::ContentChunk { execution_id, chunk, .. } => {
                    println!("📝 Agent {}: {}", execution_id, chunk);
                }
                circuit_breaker::models::AgentStreamEvent::Completed { execution_id, final_response, .. } => {
                    println!("✅ Agent {} completed: {}", execution_id, final_response);
                }
                circuit_breaker::models::AgentStreamEvent::Failed { execution_id, error } => {
                    println!("❌ Agent {} failed: {}", execution_id, error);
                }
                _ => {}
            }
        }
    });

    // Create and process demo tokens
    println!("\n🎯 Creating demo tokens...");
    
    // Token 1: Document for classification
    let mut doc_token = Token::new("document_workflow", PlaceId::from("pending_classification"));
    if let serde_json::Value::Object(ref mut map) = &mut doc_token.data {
        map.insert("content".to_string(), json!("This is a technical document about Rust programming and async/await patterns."));
    }
    doc_token.metadata.insert("type".to_string(), json!("document"));
    doc_token.metadata.insert("status".to_string(), json!("unclassified"));
    
    println!("📄 Created document token: {}", doc_token.id);
    workflow_storage.create_token(doc_token.clone()).await?;

    // Execute place agents for the document token
    println!("\n🚀 Executing place agents for document token...");
    let executions = agent_engine.execute_place_agents(&doc_token).await?;
    println!("📊 Started {} agent executions", executions.len());

    // Wait for executions to complete
    sleep(Duration::from_secs(2)).await;

    // Token 2: Content for review
    let mut content_token = Token::new("document_workflow", PlaceId::from("pending_review"));
    if let serde_json::Value::Object(ref mut map) = &mut content_token.data {
        map.insert("content".to_string(), json!("This blog post discusses the benefits of using Rust for systems programming."));
        map.insert("classification".to_string(), json!("technical_article"));
    }
    content_token.metadata.insert("type".to_string(), json!("blog_post"));
    content_token.metadata.insert("priority".to_string(), json!("high"));
    
    println!("\n📝 Created content token: {}", content_token.id);
    workflow_storage.create_token(content_token.clone()).await?;

    // Execute place agents for the content token
    println!("🚀 Executing place agents for content token...");
    let review_executions = agent_engine.execute_place_agents(&content_token).await?;
    println!("📊 Started {} agent executions", review_executions.len());

    // Wait for executions to complete
    sleep(Duration::from_secs(2)).await;

    // Token 3: Token that doesn't meet conditions
    let mut excluded_token = Token::new("document_workflow", PlaceId::from("pending_classification"));
    if let serde_json::Value::Object(ref mut map) = &mut excluded_token.data {
        map.insert("content".to_string(), json!("Short text"));
    }
    excluded_token.metadata.insert("type".to_string(), json!("note"));
    excluded_token.metadata.insert("status".to_string(), json!("classified")); // Already classified
    
    println!("\n📝 Created excluded token: {}", excluded_token.id);
    workflow_storage.create_token(excluded_token.clone()).await?;

    // This should not trigger agents due to conditions
    println!("🚀 Executing place agents for excluded token...");
    let excluded_executions = agent_engine.execute_place_agents(&excluded_token).await?;
    println!("📊 Started {} agent executions (should be 0)", excluded_executions.len());

    // Display execution statistics
    println!("\n📈 Agent Execution Statistics:");
    
    let classification_stats = agent_engine.get_execution_stats(&AgentId::from("content-classifier")).await?;
    println!("🏷️  Classification Agent:");
    println!("   Total: {}, Completed: {}, Failed: {}, Running: {}", 
             classification_stats.total, classification_stats.completed, 
             classification_stats.failed, classification_stats.running);
    if let Some(avg_duration) = classification_stats.avg_duration_ms {
        println!("   Average Duration: {}ms", avg_duration);
    }

    let review_stats = agent_engine.get_execution_stats(&AgentId::from("content-reviewer")).await?;
    println!("📋 Review Agent:");
    println!("   Total: {}, Completed: {}, Failed: {}, Running: {}", 
             review_stats.total, review_stats.completed, 
             review_stats.failed, review_stats.running);
    if let Some(avg_duration) = review_stats.avg_duration_ms {
        println!("   Average Duration: {}ms", avg_duration);
    }

    // List all executions for the document token
    println!("\n📋 Executions for document token:");
    let doc_executions = agent_storage.list_executions_for_token(&doc_token.id).await?;
    for execution in doc_executions {
        println!("   🤖 Agent: {}, Status: {:?}, Duration: {:?}ms", 
                 execution.agent_id.as_str(), execution.status, execution.duration_ms);
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
        initial_place: PlaceId::from("pending_classification"),
        places: vec![
            PlaceId::from("pending_classification"),
            PlaceId::from("classified"),
            PlaceId::from("pending_review"),
            PlaceId::from("reviewed"),
            PlaceId::from("published"),
        ],
        transitions: vec![
            TransitionDefinition {
                id: "classify".into(),
                from_places: vec![PlaceId::from("pending_classification")],
                to_place: PlaceId::from("classified"),
                conditions: vec![],
                rules: vec![],
            },
            TransitionDefinition {
                id: "review".into(),
                from_places: vec![PlaceId::from("classified")],
                to_place: PlaceId::from("pending_review"),
                conditions: vec![],
                rules: vec![],
            },
            TransitionDefinition {
                id: "approve".into(),
                from_places: vec![PlaceId::from("pending_review")],
                to_place: PlaceId::from("reviewed"),
                conditions: vec![],
                rules: vec![],
            },
            TransitionDefinition {
                id: "publish".into(),
                from_places: vec![PlaceId::from("reviewed")],
                to_place: PlaceId::from("published"),
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

fn create_classification_place_config() -> PlaceAgentConfig {
    let mut config = PlaceAgentConfig::new(
        PlaceId::from("pending_classification"),
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
    ].iter().cloned().collect();

    config.output_mapping = [
        ("data.classification".to_string(), "category".to_string()),
        ("data.confidence".to_string(), "confidence_score".to_string()),
        ("metadata.classifier".to_string(), "agent_id".to_string()),
        ("metadata.classified_at".to_string(), "timestamp".to_string()),
    ].iter().cloned().collect();

    config.schedule = Some(PlaceAgentSchedule {
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

fn create_review_place_config() -> PlaceAgentConfig {
    let mut config = PlaceAgentConfig::new(
        PlaceId::from("pending_review"),
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
        ("classification".to_string(), "data.classification".to_string()),
        ("priority".to_string(), "metadata.priority".to_string()),
    ].iter().cloned().collect();

    config.output_mapping = [
        ("data.review_result".to_string(), "assessment".to_string()),
        ("data.review_score".to_string(), "quality_score".to_string()),
        ("metadata.reviewer".to_string(), "agent_id".to_string()),
        ("metadata.review_timestamp".to_string(), "timestamp".to_string()),
    ].iter().cloned().collect();

    config.schedule = Some(PlaceAgentSchedule {
        initial_delay_seconds: Some(1),
        interval_seconds: None,
        max_executions: Some(1),
    });

    config.retry_config = Some(AgentRetryConfig::default());

    config
}