// NATS storage implementation for Circuit Breaker workflows and tokens
// This provides distributed, persistent storage using NATS JetStream

//! # NATS Storage Implementation
//! 
//! This module provides a NATS JetStream-based implementation of the WorkflowStorage trait.
//! It enables distributed, persistent storage of workflows and tokens with streaming capabilities.
//! 
//! ## Key Features
//! 
//! - **Distributed Storage**: NATS JetStream provides distributed, replicated storage
//! - **Stream-based Architecture**: Tokens are stored as messages in workflow-specific streams
//! - **Real-time Updates**: Token transitions are published as streaming events
//! - **Automatic Stream Management**: Streams are created and configured automatically
//! - **Subject Hierarchy**: Organized subject structure for efficient querying
//! 
//! ## Subject Hierarchy
//! 
//! The NATS storage uses a hierarchical subject structure:
//! - `workflows.{workflow_id}.definition` - Workflow definitions
//! - `workflows.{workflow_id}.places.{place_id}.tokens` - Tokens in specific places
//! - `workflows.{workflow_id}.events.transitions` - Transition events
//! - `workflows.{workflow_id}.events.lifecycle` - Workflow lifecycle events
//! 
//! ## Stream Configuration
//! 
//! Each workflow gets its own NATS stream with:
//! - **Retention Policy**: Interest-based (messages kept until acknowledged)
//! - **Storage Type**: File-based for persistence
//! - **Replication**: Configurable based on NATS cluster setup
//! - **Deduplication**: Based on message ID to prevent duplicates

use std::collections::HashMap;
use std::time::Duration;
use async_nats::jetstream::{self, stream, consumer, Context};
use async_nats::Client;
use chrono::Utc;
use serde_json;
use uuid::Uuid;
use futures::StreamExt;

use crate::models::{Token, WorkflowDefinition, TransitionRecord};
use crate::engine::storage::WorkflowStorage;
use crate::Result;

/// Wrapper to use Arc<NATSStorage> as WorkflowStorage
pub struct NATSStorageWrapper {
    storage: std::sync::Arc<NATSStorage>,
}

impl NATSStorageWrapper {
    pub fn new(storage: std::sync::Arc<NATSStorage>) -> Self {
        Self { storage }
    }
}

#[async_trait::async_trait]
impl WorkflowStorage for NATSStorageWrapper {
    async fn create_workflow(&self, definition: WorkflowDefinition) -> Result<WorkflowDefinition> {
        self.storage.create_workflow(definition).await
    }

    async fn get_workflow(&self, id: &str) -> Result<Option<WorkflowDefinition>> {
        self.storage.get_workflow(id).await
    }

    async fn list_workflows(&self) -> Result<Vec<WorkflowDefinition>> {
        self.storage.list_workflows().await
    }

    async fn create_token(&self, token: Token) -> Result<Token> {
        self.storage.create_token(token).await
    }

    async fn get_token(&self, id: &uuid::Uuid) -> Result<Option<Token>> {
        self.storage.get_token(id).await
    }

    async fn update_token(&self, token: Token) -> Result<Token> {
        self.storage.update_token(token).await
    }

    async fn list_tokens(&self, workflow_id: Option<&str>) -> Result<Vec<Token>> {
        self.storage.list_tokens(workflow_id).await
    }
}

/// Configuration for NATS storage
#[derive(Debug, Clone)]
pub struct NATSStorageConfig {
    /// NATS server URLs
    pub nats_urls: Vec<String>,
    
    /// Default stream configuration
    pub default_max_messages: i64,
    pub default_max_bytes: i64,
    pub default_max_age: Duration,
    
    /// Consumer configuration
    pub consumer_timeout: Duration,
    pub max_deliver: i64,
    
    /// Connection configuration
    pub connection_timeout: Duration,
    pub reconnect_buffer_size: usize,
}

impl Default for NATSStorageConfig {
    fn default() -> Self {
        Self {
            nats_urls: vec!["nats://localhost:4222".to_string()],
            default_max_messages: 1_000_000,
            default_max_bytes: 1024 * 1024 * 1024, // 1GB
            default_max_age: Duration::from_secs(30 * 24 * 60 * 60), // 30 days
            consumer_timeout: Duration::from_secs(30),
            max_deliver: 5,
            connection_timeout: Duration::from_secs(10),
            reconnect_buffer_size: 8 * 1024 * 1024, // 8MB
        }
    }
}

/// NATS JetStream storage implementation
pub struct NATSStorage {
    client: Client,
    jetstream: Context,
    config: NATSStorageConfig,
    stream_cache: std::sync::RwLock<HashMap<String, bool>>,
}

/// Stream manager for workflow-specific streams
pub struct WorkflowStreamManager {
    jetstream: Context,
    config: NATSStorageConfig,
}

impl WorkflowStreamManager {
    pub fn new(jetstream: Context, config: NATSStorageConfig) -> Self {
        Self { jetstream, config }
    }

    /// Ensure global stream exists for all workflows
    pub async fn ensure_global_stream(&self) -> Result<()> {
        let stream_name = "CIRCUIT_BREAKER_GLOBAL";
        let subjects = vec![
            "cb.workflows.*.definition".to_string(),
            "cb.workflows.*.places.*.tokens".to_string(),
            "cb.workflows.*.events.transitions".to_string(),
            "cb.workflows.*.events.lifecycle".to_string(),
        ];

        // Check if stream already exists and has correct configuration
        if let Ok(mut stream) = self.jetstream.get_stream(stream_name).await {
            let info = stream.info().await.map_err(|e| anyhow::anyhow!("Failed to get stream info: {}", e))?;
            
            // Check if retention policy is correct
            if matches!(info.config.retention, stream::RetentionPolicy::Interest) {
                println!("🔧 Deleting stream with incorrect retention policy...");
                self.jetstream.delete_stream(stream_name).await
                    .map_err(|e| anyhow::anyhow!("Failed to delete stream: {}", e))?;
                println!("✅ Deleted old stream with Interest retention policy");
            } else {
                println!("✅ Stream already exists with correct configuration");
                return Ok(());
            }
        }

        // Create new stream configuration
        let stream_config = stream::Config {
            name: stream_name.to_string(),
            subjects,
            max_messages: self.config.default_max_messages,
            max_bytes: self.config.default_max_bytes,
            max_age: self.config.default_max_age,
            storage: stream::StorageType::File,
            num_replicas: 1,
            retention: stream::RetentionPolicy::Limits,
            discard: stream::DiscardPolicy::Old,
            duplicate_window: Duration::from_secs(120),
            ..Default::default()
        };

        println!("🔧 Creating new stream with Limits retention policy...");
        self.jetstream.create_stream(stream_config).await
            .map_err(|e| anyhow::anyhow!("Failed to create NATS stream: {}", e))?;
        println!("✅ Created stream with correct retention policy");
        Ok(())
    }

    /// Get global stream name
    pub fn stream_name(&self) -> String {
        "CIRCUIT_BREAKER_GLOBAL".to_string()
    }
}

impl NATSStorage {
    /// Create a new NATS storage instance
    pub async fn new(config: NATSStorageConfig) -> Result<Self> {
        let client = async_nats::connect(&config.nats_urls.join(","))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to connect to NATS: {}", e))?;

        let jetstream = jetstream::new(client.clone());

        Ok(Self {
            client,
            jetstream,
            config,
            stream_cache: std::sync::RwLock::new(HashMap::new()),
        })
    }

    /// Create with default configuration
    pub async fn with_default_config() -> Result<Self> {
        Self::new(NATSStorageConfig::default()).await
    }

    /// Get stream manager for workflow operations
    fn stream_manager(&self) -> WorkflowStreamManager {
        WorkflowStreamManager::new(self.jetstream.clone(), self.config.clone())
    }

    /// Ensure global stream exists (with caching)
    async fn ensure_stream(&self) -> Result<()> {
        // Check cache first
        {
            let cache = self.stream_cache.read().unwrap();
            if cache.contains_key("global") {
                return Ok(());
            }
        }

        // Create stream if not cached
        self.stream_manager().ensure_global_stream().await?;

        // Update cache
        {
            let mut cache = self.stream_cache.write().unwrap();
            cache.insert("global".to_string(), true);
        }

        Ok(())
    }

    /// Publish workflow definition to NATS
    async fn publish_workflow(&self, definition: &WorkflowDefinition) -> Result<()> {
        self.ensure_stream().await?;

        let subject = format!("cb.workflows.{}.definition", definition.id);
        let payload = serde_json::to_vec(definition)?;

        println!("🔧 Publishing workflow to NATS subject: {}", subject);
        println!("🔧 Workflow payload size: {} bytes", payload.len());

        let publish_ack = self.jetstream
            .publish(subject.clone(), payload.into())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to publish workflow to NATS: {}", e))?;

        // Wait for acknowledgment and log details
        let ack_result = publish_ack.await
            .map_err(|e| anyhow::anyhow!("Failed to get NATS publish acknowledgment: {}", e))?;
        
        println!("✅ NATS publish ACK received:");
        println!("   📊 Stream: {}", ack_result.stream);
        println!("   📊 Sequence: {:?}", ack_result.sequence);
        println!("   📍 Subject: {}", subject);
        println!("✅ Successfully published workflow {} to NATS", definition.id);
        Ok(())
    }

    /// Get workflow definition from NATS
    async fn get_workflow_from_nats(&self, workflow_id: &str) -> Result<Option<WorkflowDefinition>> {
        let stream_name = self.stream_manager().stream_name();
        
        println!("🔍 [STACK TRACE] get_workflow_from_nats called for workflow: {}", workflow_id);
        println!("🔍 [BACKTRACE] Call stack: {:?}", std::backtrace::Backtrace::capture());
        println!("🔍 Looking for workflow {} in NATS stream: {}", workflow_id, stream_name);
        
        // Try to get the stream
        let mut stream = match self.jetstream.get_stream(&stream_name).await {
            Ok(stream) => {
                println!("✅ Found NATS stream: {}", stream_name);
                stream
            },
            Err(e) => {
                println!("❌ NATS stream not found: {} (error: {})", stream_name, e);
                return Ok(None); // Stream doesn't exist, so workflow doesn't exist
            }
        };

        // Create consumer for workflow definition
        let filter_subject = format!("cb.workflows.{}.definition", workflow_id);
        println!("🔍 Creating consumer for subject: {}", filter_subject);
        
        let consumer_config = consumer::pull::Config {
            durable_name: None, // Use ephemeral consumer for immediate retrieval
            filter_subject: filter_subject.clone(),
            deliver_policy: consumer::DeliverPolicy::All,
            ..Default::default()
        };

        let consumer = stream.create_consumer(consumer_config).await
            .map_err(|e| anyhow::anyhow!("Failed to create NATS consumer: {}", e))?;
        
        println!("✅ Created NATS consumer for subject: {}", filter_subject);
        
        // Add a small delay to allow for message propagation
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // Debug: Check stream info
        match stream.info().await {
            Ok(info) => {
                println!("📊 Stream info:");
                println!("   💾 Messages: {}", info.state.messages);
                println!("   📝 Subjects: {:?}", info.config.subjects);
                println!("   🔢 First sequence: {}", info.state.first_sequence);
                println!("   🔢 Last sequence: {}", info.state.last_sequence);
            },
            Err(e) => println!("❌ Failed to get stream info: {}", e),
        }
        
        // Try to get all messages first to see what's in the stream
        println!("🔍 Attempting to fetch messages from stream...");
        let mut messages = consumer.fetch().max_messages(10).messages().await
            .map_err(|e| anyhow::anyhow!("Failed to fetch NATS messages: {}", e))?;
        
        // Get the last (most recent) workflow definition message
        let mut latest_workflow: Option<WorkflowDefinition> = None;
        let mut message_count = 0;
        
        while let Some(message) = messages.next().await {
            let message = message.map_err(|e| anyhow::anyhow!("Failed to receive NATS message: {}", e))?;
            message_count += 1;
            println!("📨 Received NATS message {} for workflow {}", message_count, workflow_id);
            println!("📨 Message subject: {}", message.subject);
            println!("📨 Message payload size: {} bytes", message.payload.len());
            
            if let Ok(workflow) = serde_json::from_slice::<WorkflowDefinition>(&message.payload) {
                println!("✅ Successfully parsed workflow: {}", workflow.id);
                latest_workflow = Some(workflow);
            } else {
                println!("❌ Failed to parse workflow from message payload");
                // Print first 100 chars of payload for debugging
                let payload_str = String::from_utf8_lossy(&message.payload);
                println!("❌ Payload preview: {}", &payload_str[..std::cmp::min(100, payload_str.len())]);
            }
            message.ack().await.map_err(|e| anyhow::anyhow!("Failed to ack NATS message: {}", e))?;
        }
        
        println!("📊 Total messages processed: {}", message_count);
        
        match &latest_workflow {
            Some(workflow) => println!("✅ Retrieved workflow {} from NATS", workflow.id),
            None => println!("❌ No workflow found in NATS for ID: {}", workflow_id),
        }
        
        Ok(latest_workflow)
    }



    /// Publish token to appropriate NATS subject
    async fn publish_token(&self, token: &Token) -> Result<()> {
        self.ensure_stream().await?;

        let subject = token.nats_subject_for_place();
        let mut token_with_nats = token.clone();
        
        // Set NATS metadata
        let now = Utc::now();
        token_with_nats.set_nats_metadata(0, now, subject.clone()); // Sequence will be set by NATS
        
        let payload = serde_json::to_vec(&token_with_nats)?;

        let _ack = self.jetstream
            .publish(subject, payload.into())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to publish token to NATS: {}", e))?;

        Ok(())
    }

    /// Get token from NATS by ID
    async fn get_token_from_nats(&self, token_id: &Uuid, workflow_id: Option<&str>) -> Result<Option<Token>> {
        // If we have workflow_id, we can be more efficient
        if let Some(wid) = workflow_id {
            return self.get_token_from_workflow(token_id, wid).await;
        }

        // Otherwise, we need to search across all workflows
        // This is less efficient but necessary when we only have token ID
        self.search_token_across_workflows(token_id).await
    }

    /// Get token from a specific workflow
    async fn get_token_from_workflow(&self, token_id: &Uuid, workflow_id: &str) -> Result<Option<Token>> {
        let stream_name = self.stream_manager().stream_name();
        
        let stream = match self.jetstream.get_stream(&stream_name).await {
            Ok(stream) => stream,
            Err(_) => return Ok(None),
        };

        // Create consumer for all token places in this workflow
        let consumer_config = consumer::pull::Config {
            durable_name: Some(format!("token_search_consumer_{}_{}", workflow_id, token_id)),
            filter_subject: format!("cb.workflows.{}.places.*.tokens", workflow_id),
            ..Default::default()
        };

        let consumer = stream.create_consumer(consumer_config).await
            .map_err(|e| anyhow::anyhow!("Failed to create token search consumer: {}", e))?;
        
        // Search through messages to find our token
        let mut batch = consumer.batch().max_messages(1000).messages().await
            .map_err(|e| anyhow::anyhow!("Failed to get token search batch: {}", e))?;
        
        while let Some(message) = batch.next().await {
            let message = message.map_err(|e| anyhow::anyhow!("Failed to receive token message: {}", e))?;
            if let Ok(token) = serde_json::from_slice::<Token>(&message.payload) {
                if token.id == *token_id {
                    message.ack().await.map_err(|e| anyhow::anyhow!("Failed to ack token message: {}", e))?;
                    return Ok(Some(token));
                }
            }
            message.ack().await.map_err(|e| anyhow::anyhow!("Failed to ack token message: {}", e))?;
        }

        Ok(None)
    }

    /// Search for token across all workflows (inefficient, use sparingly)
    async fn search_token_across_workflows(&self, _token_id: &Uuid) -> Result<Option<Token>> {
        // This would require iterating through all streams
        // For now, return None as this operation is very expensive
        // In production, you'd want to maintain a separate index
        Ok(None)
    }

    /// List tokens in a specific workflow
    async fn list_tokens_in_workflow(&self, workflow_id: &str) -> Result<Vec<Token>> {
        let stream_name = self.stream_manager().stream_name();
        
        let stream = match self.jetstream.get_stream(&stream_name).await {
            Ok(stream) => stream,
            Err(_) => return Ok(vec![]),
        };

        let consumer_config = consumer::pull::Config {
            durable_name: Some(format!("token_list_consumer_{}", workflow_id)),
            filter_subject: format!("cb.workflows.{}.places.*.tokens", workflow_id),
            ..Default::default()
        };

        let consumer = stream.create_consumer(consumer_config).await
            .map_err(|e| anyhow::anyhow!("Failed to create workflow tokens consumer: {}", e))?;
        let mut tokens = Vec::new();
        let mut batch = consumer.batch().max_messages(1000).messages().await
            .map_err(|e| anyhow::anyhow!("Failed to get workflow tokens batch: {}", e))?;
        
        while let Some(message) = batch.next().await {
            let message = message.map_err(|e| anyhow::anyhow!("Failed to receive workflow token message: {}", e))?;
            if let Ok(token) = serde_json::from_slice::<Token>(&message.payload) {
                tokens.push(token);
            }
            message.ack().await.map_err(|e| anyhow::anyhow!("Failed to ack workflow token message: {}", e))?;
        }

        Ok(tokens)
    }

    /// List all workflows by scanning streams
    async fn list_all_workflows(&self) -> Result<Vec<WorkflowDefinition>> {
        let workflows = Vec::new();
        
        // Get all streams (simplified implementation)
        // Note: In the current async-nats version, we'd need to use a different approach
        // For now, return empty list as this is a complex operation
        let _streams: Vec<String> = Vec::new();
        
        // Simplified implementation - in production you'd iterate through actual streams

        Ok(workflows)
    }

    /// Create token with NATS transition event
    pub async fn create_token_with_event(&self, mut token: Token, triggered_by: Option<String>) -> Result<Token> {
        self.ensure_stream().await?;

        let now = Utc::now();
        let sequence = 0; // Will be set by NATS
        
        // Add creation event to transition history
        let creation_record = TransitionRecord {
            from_place: token.place.clone(), // Same place since it's creation
            to_place: token.place.clone(),
            transition_id: crate::models::TransitionId::from("create"),
            timestamp: now,
            triggered_by: triggered_by.clone(),
            nats_sequence: Some(sequence),
            metadata: Some(serde_json::json!({
                "event_type": "token_created",
                "workflow_id": token.workflow_id
            })),
        };

        token.add_transition_record(creation_record);
        
        // Publish creation event
        let event_subject = format!("cb.workflows.{}.events.lifecycle", token.workflow_id);
        let event_payload = serde_json::json!({
            "event_type": "token_created",
            "token_id": token.id,
            "workflow_id": token.workflow_id,
            "place": token.place.as_str(),
            "timestamp": now,
            "triggered_by": triggered_by
        });

        self.jetstream
            .publish(event_subject, serde_json::to_vec(&event_payload)?.into())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to publish creation event: {}", e))?;

        // Publish the token itself
        self.publish_token(&token).await?;

        Ok(token)
    }

    /// Transition token with NATS event publishing
    pub async fn transition_token_with_event(
        &self,
        mut token: Token,
        new_place: crate::models::PlaceId,
        transition_id: crate::models::TransitionId,
        triggered_by: Option<String>,
    ) -> Result<Token> {
        let old_place = token.place.clone();
        
        // Perform the transition with NATS tracking
        token.transition_to_with_nats(
            new_place.clone(),
            transition_id.clone(),
            triggered_by.clone(),
            None, // Sequence will be set by NATS
        );

        // Publish transition event
        let event_subject = format!("cb.workflows.{}.events.transitions", token.workflow_id);
        let event_payload = serde_json::json!({
            "event_type": "token_transitioned",
            "token_id": token.id,
            "workflow_id": token.workflow_id,
            "from_place": old_place.as_str(),
            "to_place": new_place.as_str(),
            "transition_id": transition_id.as_str(),
            "timestamp": Utc::now(),
            "triggered_by": triggered_by
        });

        self.jetstream
            .publish(event_subject, serde_json::to_vec(&event_payload)?.into())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to publish transition event: {}", e))?;

        // Republish the token to its new place
        self.publish_token(&token).await?;

        Ok(token)
    }
}

#[async_trait::async_trait]
impl WorkflowStorage for NATSStorage {
    async fn create_workflow(&self, definition: WorkflowDefinition) -> Result<WorkflowDefinition> {
        self.publish_workflow(&definition).await?;
        Ok(definition)
    }

    async fn get_workflow(&self, id: &str) -> Result<Option<WorkflowDefinition>> {
        self.get_workflow_from_nats(id).await
    }

    async fn list_workflows(&self) -> Result<Vec<WorkflowDefinition>> {
        self.list_all_workflows().await
    }

    async fn create_token(&self, token: Token) -> Result<Token> {
        self.create_token_with_event(token, Some("api".to_string())).await
    }

    async fn get_token(&self, id: &Uuid) -> Result<Option<Token>> {
        self.get_token_from_nats(id, None).await
    }

    async fn update_token(&self, token: Token) -> Result<Token> {
        // For updates, we republish the token
        self.publish_token(&token).await?;
        Ok(token)
    }

    async fn list_tokens(&self, workflow_id: Option<&str>) -> Result<Vec<Token>> {
        match workflow_id {
            Some(wid) => self.list_tokens_in_workflow(wid).await,
            None => {
                // List tokens across all workflows
                let workflows = self.list_workflows().await?;
                let mut all_tokens = Vec::new();
                
                for workflow in workflows {
                    let tokens = self.list_tokens_in_workflow(&workflow.id).await?;
                    all_tokens.extend(tokens);
                }
                
                Ok(all_tokens)
            }
        }
    }
}

/// Utility functions for NATS token operations
impl NATSStorage {
    /// Get tokens currently in a specific place
    pub async fn get_tokens_in_place(&self, workflow_id: &str, place_id: &str) -> Result<Vec<Token>> {
        let stream_name = self.stream_manager().stream_name();
        
        let stream = match self.jetstream.get_stream(&stream_name).await {
            Ok(stream) => stream,
            Err(_) => return Ok(vec![]),
        };

        let consumer_config = consumer::pull::Config {
            durable_name: Some(format!("place_tokens_consumer_{}_{}", workflow_id, place_id)),
            filter_subject: format!("cb.workflows.{}.places.{}.tokens", workflow_id, place_id),
            ..Default::default()
        };

        let consumer = stream.create_consumer(consumer_config).await
            .map_err(|e| anyhow::anyhow!("Failed to create place tokens consumer: {}", e))?;
        let mut tokens = Vec::new();
        let mut batch = consumer.batch().max_messages(1000).messages().await
            .map_err(|e| anyhow::anyhow!("Failed to get place tokens batch: {}", e))?;
        
        while let Some(message) = batch.next().await {
            let message = message.map_err(|e| anyhow::anyhow!("Failed to receive place token message: {}", e))?;
            if let Ok(token) = serde_json::from_slice::<Token>(&message.payload) {
                tokens.push(token);
            }
            message.ack().await.map_err(|e| anyhow::anyhow!("Failed to ack place token message: {}", e))?;
        }

        Ok(tokens)
    }

    /// Subscribe to token events for real-time updates
    pub async fn subscribe_to_token_events(&self, workflow_id: &str) -> Result<consumer::pull::Stream> {
        let stream_name = self.stream_manager().stream_name();
        let stream = self.jetstream.get_stream(&stream_name).await
            .map_err(|e| anyhow::anyhow!("Failed to get NATS stream: {}", e))?;

        let consumer_config = consumer::pull::Config {
            durable_name: Some(format!("events_subscriber_{}", workflow_id)),
            filter_subject: format!("cb.workflows.{}.events.*", workflow_id),
            deliver_policy: consumer::DeliverPolicy::New,
            ..Default::default()
        };

        let consumer = stream.create_consumer(consumer_config).await
            .map_err(|e| anyhow::anyhow!("Failed to create events consumer: {}", e))?;
        let stream = consumer.messages().await
            .map_err(|e| anyhow::anyhow!("Failed to get events stream: {}", e))?;
        Ok(stream)
    }

    /// Find token by ID with known workflow (more efficient)
    pub async fn find_token(&self, workflow_id: &str, token_id: &Uuid) -> Result<Option<Token>> {
        self.get_token_from_workflow(token_id, workflow_id).await
    }
}