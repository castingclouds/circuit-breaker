// NATS storage implementation for Circuit Breaker workflows and resources
// This provides distributed, persistent storage using NATS JetStream

//! # NATS Storage Implementation
//!
//! This module provides a NATS JetStream-based implementation of the WorkflowStorage trait.
//! It enables distributed, persistent storage of workflows and resources with streaming capabilities.
//!
//! ## Key Features
//!
//! - **Distributed Storage**: NATS JetStream provides distributed, replicated storage
//! - **Stream-based Architecture**: Resources are stored as messages in workflow-specific streams
//! - **Real-time Updates**: Resource activities are published as streaming events
//! - **Automatic Stream Management**: Streams are created and configured automatically
//! - **Subject Hierarchy**: Organized subject structure for efficient querying
//!
//! ## Subject Hierarchy
//!
//! The NATS storage uses a hierarchical subject structure:
//! - `workflows.{workflow_id}.definition` - Workflow definitions
//! - `workflows.{workflow_id}.states.{state_id}.resources` - Resources in specific states
//! - `workflows.{workflow_id}.events.activities` - Activity events
//! - `workflows.{workflow_id}.events.lifecycle` - Workflow lifecycle events
//!
//! ## Stream Configuration
//!
//! Each workflow gets its own NATS stream with:
//! - **Retention Policy**: Interest-based (messages kept until acknowledged)
//! - **Storage Type**: File-based for persistence
//! - **Replication**: Configurable based on NATS cluster setup
//! - **Deduplication**: Based on message ID to prevent duplicates

use async_nats::jetstream::{self, consumer, stream, Context};
use async_nats::Client;
use chrono::Utc;
use futures::StreamExt;
use serde_json;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::{sleep, timeout};
use tracing::{error, warn};
use uuid::Uuid;

use crate::engine::storage::WorkflowStorage;
use crate::models::{ActivityRecord, Resource, WorkflowDefinition};
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

    async fn create_resource(&self, resource: Resource) -> Result<Resource> {
        self.storage.create_resource(resource).await
    }

    async fn get_resource(&self, id: &Uuid) -> Result<Option<Resource>> {
        self.storage.get_resource(id).await
    }

    async fn update_resource(&self, resource: Resource) -> Result<Resource> {
        self.storage.update_resource(resource).await
    }

    async fn list_resources(&self, workflow_id: Option<&str>) -> Result<Vec<Resource>> {
        self.storage.list_resources(workflow_id).await
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
    stream_cache: std::sync::Mutex<HashMap<String, bool>>,
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
            "cb.workflows.*.states.*.resources.*".to_string(),
            "cb.workflows.*.events.activities".to_string(),
            "cb.workflows.*.events.lifecycle".to_string(),
        ];

        // Check if stream already exists and has correct configuration
        if let Ok(mut stream) = self.jetstream.get_stream(stream_name).await {
            let info = stream
                .info()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to get stream info: {}", e))?;

            // Check if retention policy is correct
            if matches!(info.config.retention, stream::RetentionPolicy::Interest) {
                println!("🔧 Deleting stream with incorrect retention policy...");
                self.jetstream
                    .delete_stream(stream_name)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to delete stream: {}", e))?;
                println!("✅ Deleted old stream with Interest retention policy");
            } else if info.config.subjects != subjects {
                println!("🔧 Deleting stream with outdated subject configuration...");
                println!("   Current subjects: {:?}", info.config.subjects);
                println!("   Required subjects: {:?}", subjects);
                self.jetstream
                    .delete_stream(stream_name)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to delete stream: {}", e))?;
                println!("✅ Deleted old stream with outdated subject configuration");
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
        self.jetstream
            .create_stream(stream_config)
            .await
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
            stream_cache: std::sync::Mutex::new(HashMap::new()),
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
        // Check if we've already ensured this stream exists
        {
            let cache = self.stream_cache.lock().unwrap();
            if cache.contains_key("global") {
                return Ok(());
            }
        }

        // Create stream if not cached
        self.stream_manager().ensure_global_stream().await?;

        // Update cache
        // Mark stream as created
        {
            let mut cache = self.stream_cache.lock().unwrap();
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

        let publish_ack = self
            .jetstream
            .publish(subject.clone(), payload.into())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to publish workflow to NATS: {}", e))?;

        // Wait for acknowledgment and log details
        let ack_result = publish_ack
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get NATS publish acknowledgment: {}", e))?;

        println!("✅ NATS publish ACK received:");
        println!("   📊 Stream: {}", ack_result.stream);
        println!("   📊 Sequence: {:?}", ack_result.sequence);
        println!("   📍 Subject: {}", subject);
        println!(
            "✅ Successfully published workflow {} to NATS",
            definition.id
        );
        Ok(())
    }

    /// Get workflow definition from NATS
    async fn get_workflow_from_nats(
        &self,
        workflow_id: &str,
    ) -> Result<Option<WorkflowDefinition>> {
        let stream_name = self.stream_manager().stream_name();

        println!(
            "🔍 [STACK TRACE] get_workflow_from_nats called for workflow: {}",
            workflow_id
        );
        println!(
            "🔍 [BACKTRACE] Call stack: {:?}",
            std::backtrace::Backtrace::capture()
        );
        println!(
            "🔍 Looking for workflow {} in NATS stream: {}",
            workflow_id, stream_name
        );

        // Try to get the stream
        let mut stream = match self.jetstream.get_stream(&stream_name).await {
            Ok(stream) => {
                println!("✅ Found NATS stream: {}", stream_name);
                stream
            }
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
            deliver_policy: consumer::DeliverPolicy::LastPerSubject,
            ack_policy: consumer::AckPolicy::None, // Read-only access for workflow queries
            ..Default::default()
        };

        let consumer = stream
            .create_consumer(consumer_config)
            .await
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
            }
            Err(e) => println!("❌ Failed to get stream info: {}", e),
        }

        // Try to get all messages first to see what's in the stream
        println!("🔍 Attempting to fetch messages from stream...");
        let mut messages = consumer
            .fetch()
            .max_messages(10)
            .messages()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch NATS messages: {}", e))?;

        // Get the last (most recent) workflow definition message
        let mut latest_workflow: Option<WorkflowDefinition> = None;
        let mut message_count = 0;

        while let Some(message) = messages.next().await {
            let message =
                message.map_err(|e| anyhow::anyhow!("Failed to receive NATS message: {}", e))?;
            message_count += 1;
            println!(
                "📨 Received NATS message {} for workflow {}",
                message_count, workflow_id
            );
            println!("📨 Message subject: {}", message.subject);
            println!("📨 Message payload size: {} bytes", message.payload.len());

            if let Ok(workflow) = serde_json::from_slice::<WorkflowDefinition>(&message.payload) {
                println!("✅ Successfully parsed workflow: {}", workflow.id);
                latest_workflow = Some(workflow);
            } else {
                println!("❌ Failed to parse workflow from message payload");
                // Print first 100 chars of payload for debugging
                let payload_str = String::from_utf8_lossy(&message.payload);
                println!(
                    "❌ Payload preview: {}",
                    &payload_str[..std::cmp::min(100, payload_str.len())]
                );
            }
            // No acknowledgment needed with AckPolicy::None
        }

        println!("📊 Total messages processed: {}", message_count);

        match &latest_workflow {
            Some(workflow) => println!("✅ Retrieved workflow {} from NATS", workflow.id),
            None => println!("❌ No workflow found in NATS for ID: {}", workflow_id),
        }

        Ok(latest_workflow)
    }

    /// Publish resource to appropriate NATS subject
    async fn publish_resource(&self, resource: &Resource) -> Result<u64> {
        self.ensure_stream().await?;

        let subject = resource.nats_subject_for_state();
        let payload = serde_json::to_vec(resource)?;

        // Publish and wait for acknowledgment with sequence number
        let ack = self
            .jetstream
            .publish(subject.clone(), payload.into())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to publish token to NATS: {}", e))?;

        // Wait for the acknowledgment and get the sequence number
        let pub_ack = ack
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get publish acknowledgment: {}", e))?;

        // Small delay to ensure message is available for consumers
        sleep(Duration::from_millis(50)).await;

        Ok(pub_ack.sequence)
    }

    /// Get resource from NATS by ID
    async fn get_resource_from_nats(
        &self,
        resource_id: &Uuid,
        workflow_id: Option<&str>,
    ) -> Result<Option<Resource>> {
        // If we have workflow_id, we can be more efficient
        if let Some(wid) = workflow_id {
            return self.get_resource_from_workflow(resource_id, wid).await;
        }

        // Otherwise search all workflows - this is more expensive
        // but still efficient with unique resource subjects
        self.search_resource_across_workflows(resource_id).await
    }

    /// Get resource from a specific workflow with retry logic
    pub async fn get_resource_from_workflow(
        &self,
        resource_id: &Uuid,
        workflow_id: &str,
    ) -> Result<Option<Resource>> {
        // Use the same proven approach as get_tokens_in_place, but search for specific token
        let workflow_def = match self.get_workflow_from_nats(workflow_id).await? {
            Some(workflow) => workflow,
            None => return Ok(None),
        };

        // Search through each state using the same logic as get_resources_in_state
        for state in &workflow_def.states {
            let resources_in_state = self
                .get_resources_in_state(workflow_id, state.as_str())
                .await?;

            // Look for our specific resource in this state
            for resource in resources_in_state {
                if resource.id == *resource_id {
                    return Ok(Some(resource));
                }
            }
        }

        Ok(None)
    }

    /// Search for resource across all workflows - now efficient with unique subjects
    async fn search_resource_across_workflows(
        &self,
        resource_id: &Uuid,
    ) -> Result<Option<Resource>> {
        let stream_name = self.stream_manager().stream_name();

        let stream = match self.jetstream.get_stream(&stream_name).await {
            Ok(stream) => stream,
            Err(_) => return Ok(None),
        };

        // Create consumer for the specific resource subject pattern
        // Each resource has unique subject: cb.workflows.*.states.*.resources.{resource_id}
        let consumer_config = consumer::pull::Config {
            durable_name: None, // Use ephemeral consumer
            filter_subject: format!("cb.workflows.*.states.*.resources.{}", resource_id),
            deliver_policy: consumer::DeliverPolicy::All, // Get all versions of this resource
            ack_policy: consumer::AckPolicy::None,        // Read-only access
            max_deliver: self.config.max_deliver,
            ack_wait: Duration::from_secs(30),
            ..Default::default()
        };

        let consumer = match stream.create_consumer(consumer_config).await {
            Ok(consumer) => consumer,
            Err(_) => return Ok(None),
        };

        // Find the most recent version of the token across all places
        let search_future = async {
            let mut batch = consumer
                .batch()
                .max_messages(100) // Get all versions of this token
                .max_bytes(1024 * 1024) // 1MB limit
                .expires(Duration::from_secs(2))
                .messages()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to get resource batch: {}", e))?;

            let mut latest_resource: Option<Resource> = None;
            let mut latest_timestamp =
                chrono::DateTime::<chrono::Utc>::from_timestamp(0, 0).unwrap();

            while let Some(message) = batch.next().await {
                let message = message
                    .map_err(|e| anyhow::anyhow!("Failed to receive resource message: {}", e))?;

                if let Ok(resource) = serde_json::from_slice::<Resource>(&message.payload) {
                    if resource.id == *resource_id {
                        // Use NATS timestamp if available, otherwise updated_at
                        let msg_timestamp = resource.nats_timestamp.unwrap_or(resource.updated_at);
                        if msg_timestamp > latest_timestamp {
                            latest_timestamp = msg_timestamp;
                            latest_resource = Some(resource);
                        }
                    }
                }
            }

            Ok::<Option<Resource>, anyhow::Error>(latest_resource)
        };

        match timeout(Duration::from_secs(5), search_future).await {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(_)) => Ok(None),
            Err(_) => Ok(None),
        }
    }

    /// List resources in a specific workflow
    async fn list_resources_in_workflow(&self, workflow_id: &str) -> Result<Vec<Resource>> {
        let stream_name = self.stream_manager().stream_name();

        let stream = match self.jetstream.get_stream(&stream_name).await {
            Ok(stream) => stream,
            Err(_) => return Ok(vec![]),
        };

        let consumer_config = consumer::pull::Config {
            durable_name: None, // Use ephemeral consumer
            filter_subject: format!("cb.workflows.{}.states.*.resources.*", workflow_id),
            deliver_policy: consumer::DeliverPolicy::LastPerSubject,
            ack_policy: consumer::AckPolicy::None, // Read-only access for resource listing
            ..Default::default()
        };

        let consumer = stream
            .create_consumer(consumer_config)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create workflow resources consumer: {}", e))?;
        let mut resources = Vec::new();
        let mut batch = consumer
            .batch()
            .max_messages(1000)
            .messages()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get workflow resources batch: {}", e))?;

        while let Some(message) = batch.next().await {
            let message = message.map_err(|e| {
                anyhow::anyhow!("Failed to receive workflow resource message: {}", e)
            })?;
            if let Ok(resource) = serde_json::from_slice::<Resource>(&message.payload) {
                resources.push(resource);
            }
        }
        Ok(resources)
    }

    /// List all workflows by scanning streams
    async fn list_all_workflows(&self) -> Result<Vec<WorkflowDefinition>> {
        let mut workflows = Vec::new();
        let stream_name = self.stream_manager().stream_name();

        // Get the stream and scan for workflow definitions
        match self.jetstream.get_stream(&stream_name).await {
            Ok(stream) => {
                // Create consumer to scan all workflow definition messages
                let consumer_config = async_nats::jetstream::consumer::pull::Config {
                    durable_name: None, // Use ephemeral consumer
                    filter_subject: "cb.workflows.*.definition".to_string(),
                    deliver_policy: async_nats::jetstream::consumer::DeliverPolicy::All,
                    ack_policy: async_nats::jetstream::consumer::AckPolicy::None, // Read-only access
                    max_deliver: self.config.max_deliver,
                    ack_wait: std::time::Duration::from_secs(30),
                    ..Default::default()
                };

                match stream.create_consumer(consumer_config).await {
                    Ok(consumer) => {
                        // Fetch messages in batches
                        match consumer.fetch().max_messages(100).messages().await {
                            Ok(mut messages) => {
                                while let Some(message) = messages.next().await {
                                    match message {
                                        Ok(msg) => {
                                            // Try to parse the workflow definition
                                            match serde_json::from_slice::<WorkflowDefinition>(
                                                &msg.payload,
                                            ) {
                                                Ok(workflow) => {
                                                    workflows.push(workflow);
                                                }
                                                Err(e) => {
                                                    warn!(
                                                        "Failed to parse workflow definition: {}",
                                                        e
                                                    );
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            warn!("Error reading message from stream: {}", e);
                                            break;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Failed to fetch messages from stream: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to create consumer for workflow listing: {}", e);
                    }
                }
            }
            Err(e) => {
                warn!("Failed to get stream for workflow listing: {}", e);
            }
        }

        Ok(workflows)
    }

    /// Create resource with NATS activity event
    pub async fn create_resource_with_event(
        &self,
        mut resource: Resource,
        triggered_by: Option<String>,
    ) -> Result<Resource> {
        self.ensure_stream().await?;

        let now = Utc::now();

        // Publish the initial resource and get the sequence number
        let sequence = self.publish_resource(&resource).await?;

        // Add creation event to activity history with actual sequence
        let creation_record = ActivityRecord {
            from_state: resource.state.clone(), // Same state since it's creation
            to_state: resource.state.clone(),
            activity_id: "create".into(),
            timestamp: now,
            triggered_by: triggered_by.clone(),
            nats_sequence: Some(sequence),
            metadata: Some(serde_json::json!({
                "event_type": "creation",
                "workflow_id": resource.workflow_id,
                "created_at": now
            })),
        };

        resource.add_activity_record(creation_record);

        // Update resource with NATS metadata
        resource.set_nats_metadata(sequence, now, resource.nats_subject_for_state());

        // Publish the complete resource with metadata and history
        let _final_sequence = self.publish_resource(&resource).await?;

        // Publish creation event
        let event_subject = format!("cb.workflows.{}.events.lifecycle", resource.workflow_id);
        let event_payload = serde_json::json!({
            "event_type": "resource_created",
            "resource_id": resource.id,
            "workflow_id": resource.workflow_id,
            "state": resource.state.as_str(),
            "timestamp": now,
            "triggered_by": triggered_by,
            "nats_sequence": sequence
        });

        let event_ack = self
            .jetstream
            .publish(event_subject, serde_json::to_vec(&event_payload)?.into())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to publish creation event: {}", e))?;

        // Wait for event acknowledgment
        let _event_pub_ack = event_ack
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get event publish acknowledgment: {}", e))?;

        Ok(resource)
    }

    /// Execute activity with NATS event tracking
    pub async fn execute_activity_with_nats(
        &self,
        mut resource: Resource,
        new_state: crate::models::StateId,
        activity_id: crate::models::ActivityId,
        triggered_by: Option<String>,
    ) -> Result<Resource> {
        let old_state = resource.state.clone();
        let now = Utc::now();

        // Perform the activity with NATS tracking
        resource.execute_activity_with_nats(
            new_state.clone(),
            activity_id.clone(),
            triggered_by.clone(),
            None, // Will be set after publishing
        );

        // Publish the resource to its new state and get sequence
        let sequence = self.publish_resource(&resource).await?;

        // Update the resource's NATS metadata with the actual sequence
        resource.set_nats_metadata(sequence, now, resource.nats_subject_for_state());

        // Ensure the activity record has the correct sequence
        if let Some(last_record) = resource.activity_history.last_mut() {
            last_record.nats_sequence = Some(sequence);
        }

        // Publish the final resource with complete sequence information
        let _final_sequence = self.publish_resource(&resource).await?;

        // Publish transition event with sequence information
        // Publish activity event
        let event_subject = format!("cb.workflows.{}.events.activities", resource.workflow_id);
        let event_payload = serde_json::json!({
            "event_type": "resource_activity_executed",
            "resource_id": resource.id,
            "workflow_id": resource.workflow_id,
            "from_state": old_state.as_str(),
            "to_state": new_state.as_str(),
            "activity_id": activity_id.as_str(),
            "timestamp": now,
            "triggered_by": triggered_by,
            "nats_sequence": sequence
        });

        let _event_ack = self
            .jetstream
            .publish(event_subject, serde_json::to_vec(&event_payload)?.into())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to publish activity event: {}", e))?;

        Ok(resource)
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

    async fn create_resource(&self, resource: Resource) -> Result<Resource> {
        self.create_resource_with_event(resource, Some("api".to_string()))
            .await
    }

    async fn get_resource(&self, id: &Uuid) -> Result<Option<Resource>> {
        self.get_resource_from_nats(id, None).await
    }

    async fn update_resource(&self, mut resource: Resource) -> Result<Resource> {
        // For updates with state changes, we need to ensure proper NATS metadata
        let now = Utc::now();
        let sequence = self.publish_resource(&resource).await?;

        // Update NATS metadata with the new subject and sequence
        resource.set_nats_metadata(sequence, now, resource.nats_subject_for_state());

        Ok(resource)
    }

    async fn list_resources(&self, workflow_id: Option<&str>) -> Result<Vec<Resource>> {
        match workflow_id {
            Some(wid) => {
                // List resources for specific workflow
                self.list_resources_in_workflow(wid).await
            }
            None => {
                // List resources across all workflows
                let workflows = self.list_workflows().await?;
                let mut all_resources = Vec::new();

                for workflow in workflows {
                    let resources = self.list_resources_in_workflow(&workflow.id).await?;
                    all_resources.extend(resources);
                }

                Ok(all_resources)
            }
        }
    }
}

/// Utility functions for NATS resource operations
impl NATSStorage {
    /// Get resources currently in a specific state with retry logic
    pub async fn get_resources_in_state(
        &self,
        workflow_id: &str,
        state_id: &str,
    ) -> Result<Vec<Resource>> {
        let stream_name = self.stream_manager().stream_name();

        let stream = match self.jetstream.get_stream(&stream_name).await {
            Ok(stream) => stream,
            Err(_) => return Ok(vec![]),
        };

        // Try with retry logic for timing issues
        for attempt in 0..2 {
            if attempt > 0 {
                sleep(Duration::from_millis(100)).await;
            }

            let consumer_config = consumer::pull::Config {
                durable_name: None, // Use ephemeral consumer
                filter_subject: format!(
                    "cb.workflows.{}.states.{}.resources.*",
                    workflow_id, state_id
                ),
                deliver_policy: consumer::DeliverPolicy::LastPerSubject,
                ack_policy: consumer::AckPolicy::None, // Read-only access for state queries
                max_deliver: self.config.max_deliver,
                ack_wait: Duration::from_secs(30),
                ..Default::default()
            };

            let consumer = match stream.create_consumer(consumer_config).await {
                Ok(consumer) => consumer,
                Err(e) => {
                    error!(
                        "Failed to create state consumer on attempt {}: {}",
                        attempt + 1,
                        e
                    );
                    continue;
                }
            };

            let mut resources = Vec::new();

            // Use timeout to prevent hanging
            let fetch_future = async {
                let mut batch = consumer
                    .batch()
                    .max_messages(100)
                    .expires(Duration::from_secs(5))
                    .messages()
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to get state batch: {}", e))?;

                while let Some(message) = batch.next().await {
                    let message = message
                        .map_err(|e| anyhow::anyhow!("Failed to receive state message: {}", e))?;
                    if let Ok(resource) = serde_json::from_slice::<Resource>(&message.payload) {
                        resources.push(resource);
                        // No acknowledgment needed with AckPolicy::None
                    }
                    // Invalid messages are ignored without acknowledgment
                }

                Ok::<Vec<Resource>, anyhow::Error>(resources)
            };

            match timeout(Duration::from_secs(10), fetch_future).await {
                Ok(Ok(resources)) => return Ok(resources),
                Ok(Err(e)) => {
                    warn!("Fetch error on attempt {}: {}", attempt + 1, e);
                }
                Err(_) => {
                    warn!("Fetch timeout on attempt {}", attempt + 1);
                }
            }
        }

        // Return empty if all attempts failed
        Ok(vec![])
    }

    /// Subscribe to resource events for real-time updates
    pub async fn subscribe_to_resource_events(
        &self,
        workflow_id: &str,
    ) -> Result<consumer::pull::Stream> {
        let stream_name = self.stream_manager().stream_name();
        let stream = self
            .jetstream
            .get_stream(&stream_name)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get NATS stream: {}", e))?;

        let consumer_config = consumer::pull::Config {
            durable_name: Some(format!("resource-events-{}", workflow_id)),
            filter_subject: format!("cb.workflows.{}.events.*", workflow_id),
            deliver_policy: consumer::DeliverPolicy::New,
            ..Default::default()
        };

        let consumer = stream
            .create_consumer(consumer_config)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create events consumer: {}", e))?;
        let stream = consumer
            .messages()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get events stream: {}", e))?;
        Ok(stream)
    }

    /// Find resource by ID with known workflow (more efficient)
    pub async fn find_resource(
        &self,
        workflow_id: &str,
        resource_id: &Uuid,
    ) -> Result<Option<Resource>> {
        self.get_resource_from_workflow(resource_id, workflow_id)
            .await
    }
}
