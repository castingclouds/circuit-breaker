// NATS-based agent storage implementation
// This module provides persistent storage for agents using NATS JetStream

use async_nats::{self, jetstream, Client};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::{future::join_all, stream::StreamExt};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::engine::AgentStorage;
use crate::models::{
    AgentDefinition, AgentExecution, AgentExecutionStatus, AgentId, AgentStreamEvent,
};
use crate::{CircuitBreakerError, Result};

use super::tenant_storage::TenantId;

// NATS-specific storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatsStorageConfig {
    // NATS connection settings
    pub url: String,
    pub connection_timeout: Duration,
    pub reconnect_attempts: usize,
    pub client_name: Option<String>,

    // JetStream settings
    pub stream_name: String,
    pub agents_bucket: String,
    pub executions_bucket: String,

    // Performance settings
    pub max_batch_size: usize,
    pub max_age: Duration,
    pub replicas: usize,
    pub max_bytes: Option<i64>,
}

impl Default for NatsStorageConfig {
    fn default() -> Self {
        Self {
            url: "nats://localhost:4222".to_string(),
            connection_timeout: Duration::from_secs(10),
            reconnect_attempts: 5,
            client_name: Some("circuit-breaker-agent-storage".to_string()),
            stream_name: "AGENTS".to_string(),
            agents_bucket: "agents".to_string(),
            executions_bucket: "executions".to_string(),
            max_batch_size: 100,
            max_age: Duration::from_secs(60 * 60 * 24 * 30), // 30 days
            replicas: 1,                                     // Single replica for development
            max_bytes: Some(1024 * 1024 * 1024),             // 1GB limit
        }
    }
}

// Key prefixes and separators
const AGENT_PREFIX: &str = "agent";
const EXECUTION_PREFIX: &str = "exec";
const TENANT_PREFIX: &str = "tenant";
const KEY_SEPARATOR: &str = ":";

// NATS agent storage implementation
pub struct NatsAgentStorage {
    // NATS connection
    client: Client,
    // JetStream context
    js: jetstream::Context,
    // Storage configuration
    config: NatsStorageConfig,
    // Key-value store for agents
    agents_kv: Arc<Mutex<jetstream::kv::Store>>,
    // Key-value store for executions
    executions_kv: Arc<Mutex<jetstream::kv::Store>>,
    // Connection status
    connected: Arc<RwLock<bool>>,
}

impl NatsAgentStorage {
    /// Create a new NATS-based agent storage
    pub async fn new(config: NatsStorageConfig) -> Result<Self> {
        // Create NATS connection
        info!("Connecting to NATS server at {}", config.url);
        let client = match Self::connect_with_retry(&config).await {
            Ok(client) => client,
            Err(e) => {
                error!("Failed to connect to NATS: {}", e);
                return Err(CircuitBreakerError::Storage(anyhow::Error::msg(format!(
                    "Failed to connect to NATS: {}",
                    e
                ))));
            }
        };

        // Create JetStream context
        let js = jetstream::new(client.clone());

        // Ensure streams and buckets exist
        let agents_kv = Self::ensure_kv_bucket(&js, &config.agents_bucket, &config).await?;
        let executions_kv = Self::ensure_kv_bucket(&js, &config.executions_bucket, &config).await?;

        info!("NATS agent storage initialized successfully");
        Ok(Self {
            client,
            js,
            config,
            agents_kv: Arc::new(Mutex::new(agents_kv)),
            executions_kv: Arc::new(Mutex::new(executions_kv)),
            connected: Arc::new(RwLock::new(true)),
        })
    }

    /// Connect to NATS with retry
    async fn connect_with_retry(
        config: &NatsStorageConfig,
    ) -> std::result::Result<Client, async_nats::Error> {
        // Connect with retry
        for attempt in 1..=config.reconnect_attempts {
            let mut connect_options = async_nats::ConnectOptions::new();

            // Apply configuration
            connect_options = connect_options
                .connection_timeout(config.connection_timeout)
                .request_timeout(Some(
                    config.connection_timeout * config.reconnect_attempts as u32,
                ));

            if let Some(client_name) = &config.client_name {
                connect_options = connect_options.name(client_name);
            }

            match connect_options.connect(&config.url).await {
                Ok(client) => {
                    info!("Connected to NATS server after {} attempt(s)", attempt);
                    return Ok(client);
                }
                Err(e) => {
                    if attempt < config.reconnect_attempts {
                        warn!("NATS connection attempt {} failed: {}", attempt, e);
                        tokio::time::sleep(Duration::from_millis(500 * attempt as u64)).await;
                    } else {
                        error!(
                            "Failed to connect to NATS after {} attempts: {}",
                            attempt, e
                        );
                        return Err(Box::new(e));
                    }
                }
            }
        }

        // This should never be reached due to the for loop above
        Err(anyhow::Error::msg("Failed to connect to NATS server").into())
    }

    /// Ensure KV bucket exists
    async fn ensure_kv_bucket(
        js: &jetstream::Context,
        bucket_name: &str,
        config: &NatsStorageConfig,
    ) -> Result<jetstream::kv::Store> {
        // Check if bucket already exists
        match js.get_key_value(bucket_name).await {
            Ok(kv) => {
                debug!("Using existing KV bucket: {}", bucket_name);
                Ok(kv)
            }
            Err(_) => {
                // Create new bucket
                info!("Creating new KV bucket: {}", bucket_name);
                let kv_config = jetstream::kv::Config {
                    bucket: bucket_name.to_string(),
                    history: 5,
                    max_age: config.max_age,
                    storage: jetstream::stream::StorageType::File,
                    num_replicas: config.replicas as usize,
                    max_bytes: config.max_bytes.unwrap_or(1024 * 1024 * 1024),
                    description: format!("Circuit Breaker Agent Storage - {}", bucket_name),
                    ..Default::default()
                };

                match js.create_key_value(kv_config).await {
                    Ok(kv) => {
                        info!("Created KV bucket: {}", bucket_name);
                        Ok(kv)
                    }
                    Err(e) => {
                        error!("Failed to create KV bucket {}: {}", bucket_name, e);
                        Err(CircuitBreakerError::Storage(anyhow::Error::msg(format!(
                            "Failed to create KV bucket {}: {}",
                            bucket_name, e
                        ))))
                    }
                }
            }
        }
    }

    /// Generate key for agent
    fn agent_key(id: &AgentId, tenant_id: Option<&TenantId>) -> String {
        if let Some(tenant) = tenant_id {
            format!(
                "{}{}{}{}{}",
                TENANT_PREFIX,
                KEY_SEPARATOR,
                tenant.as_str(),
                KEY_SEPARATOR,
                id.as_str()
            )
        } else {
            format!("{}{}{}", AGENT_PREFIX, KEY_SEPARATOR, id.as_str())
        }
    }

    /// Generate key for execution
    fn execution_key(id: &Uuid, tenant_id: Option<&TenantId>) -> String {
        if let Some(tenant) = tenant_id {
            format!(
                "{}{}{}{}{}",
                TENANT_PREFIX,
                KEY_SEPARATOR,
                tenant.as_str(),
                KEY_SEPARATOR,
                id.to_string()
            )
        } else {
            format!("{}{}{}", EXECUTION_PREFIX, KEY_SEPARATOR, id.to_string())
        }
    }

    /// Parse tenant ID from key
    fn parse_tenant_from_key(key: &str) -> Option<TenantId> {
        let parts: Vec<&str> = key.split(KEY_SEPARATOR).collect();
        if parts.len() >= 3 && parts[0] == TENANT_PREFIX {
            Some(TenantId::new(parts[1]))
        } else {
            None
        }
    }

    /// Extract tenant ID from agent definition
    fn extract_tenant_from_agent(&self, agent: &AgentDefinition) -> Option<TenantId> {
        // Check system prompt for tenant ID
        if agent.prompts.system.contains("tenant") {
            // This is a simple heuristic, in a real implementation you would
            // have a more robust way to extract tenant information
            let parts: Vec<&str> = agent.prompts.system.split_whitespace().collect();
            for (i, part) in parts.iter().enumerate() {
                if part.to_lowercase() == "tenant" && i + 1 < parts.len() {
                    return Some(TenantId::new(
                        parts[i + 1].trim_matches(|c: char| !c.is_alphanumeric()),
                    ));
                }
            }
        }

        None
    }

    /// Extract tenant ID from agent execution
    fn extract_tenant_from_execution(&self, execution: &AgentExecution) -> Option<TenantId> {
        execution
            .context
            .get("tenant_id")
            .and_then(|v| v.as_str())
            .map(TenantId::new)
    }

    /// Serialize object to JSON bytes
    fn serialize<T: Serialize>(obj: &T) -> Result<Vec<u8>> {
        serde_json::to_vec(obj).map_err(|e| {
            CircuitBreakerError::Storage(anyhow::anyhow!("Failed to serialize object: {}", e))
        })
    }

    /// Deserialize object from JSON bytes
    fn deserialize<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
        serde_json::from_slice(bytes).map_err(|e| {
            CircuitBreakerError::Storage(anyhow::anyhow!("Failed to deserialize object: {}", e))
        })
    }

    /// Get all keys with a specific prefix
    async fn get_keys_with_prefix(
        &self,
        kv: &jetstream::kv::Store,
        prefix: &str,
    ) -> Result<Vec<String>> {
        let mut keys = Vec::new();

        let mut entries = kv.watch_all().await.map_err(|e| {
            CircuitBreakerError::Storage(anyhow::Error::msg(format!(
                "Failed to watch KV store: {}",
                e
            )))
        })?;

        while let Some(Ok(entry)) = entries.next().await {
            let key = entry.key;
            if key.starts_with(prefix) {
                keys.push(key);
            }
        }

        Ok(keys)
    }

    /// Check connection status and reconnect if needed
    async fn ensure_connected(&self) -> Result<()> {
        if !*self.connected.read().await {
            // Try to reconnect
            match Self::connect_with_retry(&self.config).await {
                Ok(client) => {
                    let js = jetstream::new(client.clone());

                    // Re-create KV stores
                    let agents_kv =
                        Self::ensure_kv_bucket(&js, &self.config.agents_bucket, &self.config)
                            .await?;
                    let executions_kv =
                        Self::ensure_kv_bucket(&js, &self.config.executions_bucket, &self.config)
                            .await?;

                    // Update internal state
                    *self.agents_kv.lock().await = agents_kv;
                    *self.executions_kv.lock().await = executions_kv;
                    *self.connected.write().await = true;

                    info!("Successfully reconnected to NATS server");
                }
                Err(e) => {
                    error!("Failed to reconnect to NATS: {}", e);
                    return Err(CircuitBreakerError::Storage(anyhow::Error::msg(format!(
                        "Failed to reconnect to NATS: {}",
                        e
                    ))));
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl AgentStorage for NatsAgentStorage {
    async fn store_agent(&self, agent: &AgentDefinition) -> Result<()> {
        self.ensure_connected().await?;

        // Extract tenant ID if present
        let tenant_id = self.extract_tenant_from_agent(agent);

        // Generate key
        let key = Self::agent_key(&agent.id, tenant_id.as_ref());

        // Serialize agent
        let agent_bytes = Self::serialize(agent)?;

        // Store in KV
        let agents_kv = self.agents_kv.lock().await;
        agents_kv.put(&key, agent_bytes.into()).await.map_err(|e| {
            *self.connected.blocking_write() = false;
            CircuitBreakerError::Storage(anyhow::anyhow!("Failed to store agent: {}", e))
        })?;

        debug!("Stored agent {} with key {}", agent.id, key);
        Ok(())
    }

    async fn get_agent(&self, id: &AgentId) -> Result<Option<AgentDefinition>> {
        self.ensure_connected().await?;

        // Try to get agent without tenant ID first
        let key = Self::agent_key(id, None);

        let agents_kv = self.agents_kv.lock().await;
        match agents_kv.get(&key).await {
            Ok(entry) => {
                debug!("Found agent {} with key {}", id, key);
                Ok(Some(Self::deserialize(&entry)?))
            }
            Err(async_nats::Error::KeyNotFound { .. }) => {
                // Try to find the agent with any tenant prefix
                // In a production system with many tenants, this would be inefficient
                // and you would need to know the tenant ID in advance
                let tenant_prefix = format!("{}{}", TENANT_PREFIX, KEY_SEPARATOR);
                let all_keys = self
                    .get_keys_with_prefix(&agents_kv, &tenant_prefix)
                    .await?;

                for agent_key in all_keys {
                    if agent_key.ends_with(&format!("{}{}", KEY_SEPARATOR, id.as_str())) {
                        match agents_kv.get(&agent_key).await {
                            Ok(entry) => {
                                debug!("Found agent {} with tenant key {}", id, agent_key);
                                return Ok(Some(Self::deserialize(&entry)?));
                            }
                            Err(_) => continue,
                        }
                    }
                }

                Ok(None)
            }
            Err(e) => {
                *self.connected.blocking_write() = false;
                Err(CircuitBreakerError::Storage(anyhow::anyhow!(
                    "Failed to get agent: {}",
                    e
                )))
            }
        }
    }

    async fn list_agents(&self) -> Result<Vec<AgentDefinition>> {
        self.ensure_connected().await?;

        let agents_kv = self.agents_kv.lock().await;
        let mut agents = Vec::new();

        // Watch all keys (this is a NATS way to list all keys/values)
        let mut entries = agents_kv.watch_all().await.map_err(|e| {
            *self.connected.blocking_write() = false;
            CircuitBreakerError::Storage(anyhow::anyhow!("Failed to list agents: {}", e))
        })?;

        while let Some(Ok(entry)) = entries.next().await {
            match Self::deserialize::<AgentDefinition>(&entry.value) {
                Ok(agent) => agents.push(agent),
                Err(e) => {
                    warn!("Failed to deserialize agent from key {}: {}", entry.key, e);
                    continue;
                }
            }
        }

        debug!("Listed {} agents", agents.len());
        Ok(agents)
    }

    async fn delete_agent(&self, id: &AgentId) -> Result<bool> {
        self.ensure_connected().await?;

        // Try to delete without tenant ID first
        let key = Self::agent_key(id, None);

        let agents_kv = self.agents_kv.lock().await;
        match agents_kv.delete(&key).await {
            Ok(_) => {
                debug!("Deleted agent {} with key {}", id, key);
                Ok(true)
            }
            Err(async_nats::Error::KeyNotFound { .. }) => {
                // Try to find and delete with any tenant prefix
                let tenant_prefix = format!("{}{}", TENANT_PREFIX, KEY_SEPARATOR);
                let all_keys = self
                    .get_keys_with_prefix(&agents_kv, &tenant_prefix)
                    .await?;

                for agent_key in all_keys {
                    if agent_key.ends_with(&format!("{}{}", KEY_SEPARATOR, id.as_str())) {
                        match agents_kv.delete(&agent_key).await {
                            Ok(_) => {
                                debug!("Deleted agent {} with tenant key {}", id, agent_key);
                                return Ok(true);
                            }
                            Err(_) => continue,
                        }
                    }
                }

                Ok(false)
            }
            Err(e) => {
                *self.connected.blocking_write() = false;
                Err(CircuitBreakerError::Storage(anyhow::anyhow!(
                    "Failed to delete agent: {}",
                    e
                )))
            }
        }
    }

    async fn store_execution(&self, execution: &AgentExecution) -> Result<()> {
        self.ensure_connected().await?;

        // Extract tenant ID if present
        let tenant_id = self.extract_tenant_from_execution(execution);

        // Generate key
        let key = Self::execution_key(&execution.id, tenant_id.as_ref());

        // Serialize execution
        let execution_bytes = Self::serialize(execution)?;

        // Store in KV
        let executions_kv = self.executions_kv.lock().await;
        executions_kv
            .put(&key, execution_bytes.into())
            .await
            .map_err(|e| {
                *self.connected.blocking_write() = false;
                CircuitBreakerError::Storage(anyhow::anyhow!("Failed to store execution: {}", e))
            })?;

        debug!("Stored execution {} with key {}", execution.id, key);
        Ok(())
    }

    async fn get_execution(&self, id: &Uuid) -> Result<Option<AgentExecution>> {
        self.ensure_connected().await?;

        // Try to get execution without tenant ID first
        let key = Self::execution_key(id, None);

        let executions_kv = self.executions_kv.lock().await;
        match executions_kv.get(&key).await {
            Ok(entry) => {
                debug!("Found execution {} with key {}", id, key);
                Ok(Some(Self::deserialize(&entry)?))
            }
            Err(async_nats::Error::KeyNotFound { .. }) => {
                // Try to find the execution with any tenant prefix
                let tenant_prefix = format!("{}{}", TENANT_PREFIX, KEY_SEPARATOR);
                let all_keys = self
                    .get_keys_with_prefix(&executions_kv, &tenant_prefix)
                    .await?;

                for exec_key in all_keys {
                    if exec_key.ends_with(&format!("{}{}", KEY_SEPARATOR, id.to_string())) {
                        match executions_kv.get(&exec_key).await {
                            Ok(entry) => {
                                debug!("Found execution {} with tenant key {}", id, exec_key);
                                return Ok(Some(Self::deserialize(&entry)?));
                            }
                            Err(_) => continue,
                        }
                    }
                }

                Ok(None)
            }
            Err(e) => {
                *self.connected.blocking_write() = false;
                Err(CircuitBreakerError::Storage(anyhow::anyhow!(
                    "Failed to get execution: {}",
                    e
                )))
            }
        }
    }

    async fn list_executions_by_context(
        &self,
        context_key: &str,
        context_value: &str,
    ) -> Result<Vec<AgentExecution>> {
        self.ensure_connected().await?;

        // Special case for tenant_id to optimize lookup
        if context_key == "tenant_id" {
            let tenant_id = TenantId::new(context_value);
            let tenant_prefix = format!(
                "{}{}{}{}",
                TENANT_PREFIX,
                KEY_SEPARATOR,
                tenant_id.as_str(),
                KEY_SEPARATOR
            );

            let executions_kv = self.executions_kv.lock().await;
            let all_keys = self
                .get_keys_with_prefix(&executions_kv, &tenant_prefix)
                .await?;

            let mut executions = Vec::with_capacity(all_keys.len());
            for key in all_keys {
                match executions_kv.get(&key).await {
                    Ok(entry) => match Self::deserialize::<AgentExecution>(&entry) {
                        Ok(execution) => executions.push(execution),
                        Err(e) => warn!("Failed to deserialize execution from key {}: {}", key, e),
                    },
                    Err(e) => warn!("Failed to get execution for key {}: {}", key, e),
                }
            }

            debug!(
                "Listed {} executions for tenant {}",
                executions.len(),
                tenant_id
            );
            return Ok(executions);
        }

        // For other contexts, we need to fetch all executions and filter
        // This is inefficient and should be optimized in a production system
        // with secondary indexes or a more suitable database
        let executions_kv = self.executions_kv.lock().await;
        let mut entries = executions_kv.watch_all().await.map_err(|e| {
            *self.connected.blocking_write() = false;
            CircuitBreakerError::Storage(anyhow::anyhow!("Failed to list executions: {}", e))
        })?;

        let mut executions = Vec::new();
        while let Some(Ok(entry)) = entries.next().await {
            match Self::deserialize::<AgentExecution>(&entry) {
                Ok(execution) => {
                    // Check if context matches
                    if let Some(value) = execution.context.get(context_key) {
                        if let Some(value_str) = value.as_str() {
                            if value_str == context_value {
                                executions.push(execution);
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to deserialize execution from key {}: {}",
                        entry.key, e
                    );
                    continue;
                }
            }
        }

        debug!(
            "Listed {} executions with context {}={}",
            executions.len(),
            context_key,
            context_value
        );
        Ok(executions)
    }

    // Implement the remaining methods of the AgentStorage trait...
    // For brevity, I'll implement a few key methods and stub the rest

    async fn list_executions_by_context_filters(
        &self,
        filters: &[(&str, &str)],
    ) -> Result<Vec<AgentExecution>> {
        // Get all executions and filter
        let executions_kv = self.executions_kv.lock().await;
        let mut entries = executions_kv.watch_all().await.map_err(|e| {
            *self.connected.blocking_write() = false;
            CircuitBreakerError::Storage(anyhow::anyhow!("Failed to list executions: {}", e))
        })?;

        let mut executions = Vec::new();
        while let Some(Ok(entry)) = entries.next().await {
            match Self::deserialize::<AgentExecution>(&entry.value) {
                Ok(execution) => {
                    // Check if all filters match
                    let matches = filters.iter().all(|(key, value)| {
                        if let Some(ctx_value) = execution.context.get(*key) {
                            if let Some(ctx_value_str) = ctx_value.as_str() {
                                return ctx_value_str == *value;
                            }
                        }
                        false
                    });

                    if matches {
                        executions.push(execution);
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to deserialize execution from key {}: {}",
                        entry.key, e
                    );
                    continue;
                }
            }
        }

        Ok(executions)
    }

    async fn list_executions_by_nested_context(
        &self,
        context_path: &str,
        context_value: &str,
    ) -> Result<Vec<AgentExecution>> {
        // Get all executions and filter by nested context
        let executions_kv = self.executions_kv.lock().await;
        let mut entries = executions_kv.watch_all().await.map_err(|e| {
            *self.connected.blocking_write() = false;
            CircuitBreakerError::Storage(anyhow::anyhow!("Failed to list executions: {}", e))
        })?;

        let path_parts: Vec<&str> = context_path.split('.').collect();

        let mut executions = Vec::new();
        while let Some(Ok(entry)) = entries.next().await {
            match Self::deserialize::<AgentExecution>(&entry.value) {
                Ok(execution) => {
                    // Navigate nested context
                    let mut current_value = &execution.context;
                    let mut matches = true;

                    for part in &path_parts {
                        if let Some(next_value) = current_value.get(*part) {
                            current_value = next_value;
                        } else {
                            matches = false;
                            break;
                        }
                    }

                    // Check if final value matches
                    if matches {
                        if let Some(value_str) = current_value.as_str() {
                            if value_str == context_value {
                                executions.push(execution);
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to deserialize execution from key {}: {}",
                        entry.key, e
                    );
                    continue;
                }
            }
        }

        Ok(executions)
    }

    async fn count_executions_by_context(
        &self,
        context_key: &str,
        context_value: &str,
    ) -> Result<usize> {
        // For efficiency, just get the count without loading all executions
        let executions = self
            .list_executions_by_context(context_key, context_value)
            .await?;
        Ok(executions.len())
    }

    async fn list_executions_for_agent(&self, agent_id: &AgentId) -> Result<Vec<AgentExecution>> {
        // Get all executions and filter by agent ID
        let executions_kv = self.executions_kv.lock().await;
        let mut entries = executions_kv.watch_all().await.map_err(|e| {
            *self.connected.blocking_write() = false;
            CircuitBreakerError::Storage(anyhow::anyhow!("Failed to list executions: {}", e))
        })?;

        let mut executions = Vec::new();
        while let Some(Ok(entry)) = entries.next().await {
            match Self::deserialize::<AgentExecution>(&entry.value) {
                Ok(execution) => {
                    if execution.agent_id == *agent_id {
                        executions.push(execution);
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to deserialize execution from key {}: {}",
                        entry.key, e
                    );
                    continue;
                }
            }
        }

        Ok(executions)
    }

    async fn list_executions_by_status(
        &self,
        status: &AgentExecutionStatus,
    ) -> Result<Vec<AgentExecution>> {
        // Get all executions and filter by status
        let executions_kv = self.executions_kv.lock().await;
        let mut entries = executions_kv.watch_all().await.map_err(|e| {
            *self.connected.blocking_write() = false;
            CircuitBreakerError::Storage(anyhow::anyhow!("Failed to list executions: {}", e))
        })?;

        let mut executions = Vec::new();
        while let Some(Ok(entry)) = entries.next().await {
            match Self::deserialize::<AgentExecution>(&entry.value) {
                Ok(execution) => {
                    if &execution.status == status {
                        executions.push(execution);
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to deserialize execution from key {}: {}",
                        entry.key, e
                    );
                    continue;
                }
            }
        }

        Ok(executions)
    }

    async fn list_recent_executions(&self, limit: usize) -> Result<Vec<AgentExecution>> {
        // Get all executions
        let executions_kv = self.executions_kv.lock().await;
        let mut entries = executions_kv.watch_all().await.map_err(|e| {
            *self.connected.blocking_write() = false;
            CircuitBreakerError::Storage(anyhow::anyhow!("Failed to list executions: {}", e))
        })?;

        // Collect and sort executions by start time
        let mut executions = Vec::new();
        while let Some(Ok(entry)) = entries.next().await {
            match Self::deserialize::<AgentExecution>(&entry.value) {
                Ok(execution) => {
                    executions.push(execution);
                }
                Err(e) => {
                    warn!(
                        "Failed to deserialize execution from key {}: {}",
                        entry.key, e
                    );
                    continue;
                }
            }
        }

        // Sort by start time descending
        executions.sort_by(|a, b| b.started_at.cmp(&a.started_at));

        // Take only the requested number of executions
        if executions.len() > limit {
            executions.truncate(limit);
        }

        Ok(executions)
    }

    async fn list_executions_for_agent_with_context(
        &self,
        agent_id: &AgentId,
        context_key: &str,
        context_value: &str,
    ) -> Result<Vec<AgentExecution>> {
        // Get all executions and filter by agent ID and context
        let executions_kv = self.executions_kv.lock().await;
        let mut entries = executions_kv.watch_all().await.map_err(|e| {
            *self.connected.blocking_write() = false;
            CircuitBreakerError::Storage(anyhow::anyhow!("Failed to list executions: {}", e))
        })?;

        let mut executions = Vec::new();
        while let Some(Ok(entry)) = entries.next().await {
            match Self::deserialize::<AgentExecution>(&entry.value) {
                Ok(execution) => {
                    if execution.agent_id == *agent_id {
                        // Check if context matches
                        if let Some(value) = execution.context.get(context_key) {
                            if let Some(value_str) = value.as_str() {
                                if value_str == context_value {
                                    executions.push(execution);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to deserialize execution from key {}: {}",
                        entry.key, e
                    );
                    continue;
                }
            }
        }

        Ok(executions)
    }

    async fn list_executions_for_resource(
        &self,
        resource_id: &Uuid,
    ) -> Result<Vec<AgentExecution>> {
        self.ensure_connected().await?;

        let resource_id_str = resource_id.to_string();
        let executions_kv = self.executions_kv.lock().await;
        let mut entries = executions_kv.watch_all().await.map_err(|e| {
            *self.connected.blocking_write() = false;
            CircuitBreakerError::Storage(anyhow::anyhow!("Failed to list executions: {}", e))
        })?;

        let mut executions = Vec::new();
        while let Some(Ok(entry)) = entries.next().await {
            match Self::deserialize::<AgentExecution>(&entry) {
                Ok(execution) => {
                    // Check for resource_id in various context locations
                    if execution
                        .get_context_value("resource_id")
                        .and_then(|v| v.as_str())
                        .map(|value| value == resource_id_str)
                        .unwrap_or(false)
                        || execution
                            .get_context_value("workflow")
                            .and_then(|w| w.get("resource_id"))
                            .and_then(|v| v.as_str())
                            .map(|value| value == resource_id_str)
                            .unwrap_or(false)
                    {
                        executions.push(execution);
                    }
                }
                Err(e) => warn!("Failed to deserialize execution: {}", e),
            }
        }

        Ok(executions)
    }
}

// Integration with tenant storage
pub async fn create_tenant_aware_nats_storage(
    nats_url: &str,
    backup_dir: Option<&str>,
) -> Result<Arc<crate::api::agents::tenant_storage::TenantAgentStorage>> {
    // Create NATS configuration
    let config = NatsStorageConfig {
        url: nats_url.to_string(),
        ..Default::default()
    };

    // Create NATS storage
    let nats_storage = NatsAgentStorage::new(config).await?;
    let nats_storage_arc = Arc::new(nats_storage);

    // Create tenant-aware storage wrapper
    let tenant_storage =
        crate::api::agents::tenant_storage::TenantAgentStorage::new(nats_storage_arc);

    // Configure backup if directory is provided
    let tenant_storage = if let Some(backup_path) = backup_dir {
        tenant_storage.with_backup_manager(backup_path).await?
    } else {
        tenant_storage
    };

    // Return wrapped storage
    Ok(Arc::new(tenant_storage))
}
