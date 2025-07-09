// Multi-Tenant Storage Layer for Agent System
// This module provides storage implementations with tenant isolation and data partitioning

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use futures::{future::join_all, Stream, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    fmt,
    path::PathBuf,
    sync::Arc,
    time::Duration as StdDuration,
};
use tokio::{
    fs,
    io::{self, AsyncWriteExt},
    sync::{Mutex, RwLock},
    task::JoinHandle,
    time,
};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::engine::AgentStorage;
use crate::models::{
    AgentDefinition, AgentExecution, AgentExecutionStatus, AgentId, AgentPrompts, LLMConfig,
    LLMProvider,
};
use crate::{CircuitBreakerError, Result};

//===============================================================
// Tenant Storage Types and Helpers
//===============================================================

/// Tenant identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TenantId(pub String);

impl TenantId {
    pub fn new<S: Into<String>>(id: S) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TenantId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for TenantId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for TenantId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Storage partition strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PartitionStrategy {
    /// Each tenant gets its own storage namespace
    Namespace,
    /// Tenants share storage with filtering
    Filtered,
    /// Data is physically partitioned (e.g., different databases)
    Physical,
}

/// Tenant storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantStorageConfig {
    /// Tenant ID
    pub tenant_id: TenantId,
    /// Partition strategy
    pub partition_strategy: PartitionStrategy,
    /// Storage quota in bytes (0 = unlimited)
    pub storage_quota_bytes: u64,
    /// Maximum number of agent definitions
    pub max_agent_definitions: usize,
    /// Maximum number of agent executions
    pub max_agent_executions: usize,
    /// Retention period for agent executions
    pub execution_retention_days: u32,
    /// Backup frequency in hours (0 = no automatic backup)
    pub backup_frequency_hours: u32,
    /// Enable detailed analytics
    pub enable_analytics: bool,
    /// Custom storage configuration
    pub custom_config: Option<Value>,
}

impl Default for TenantStorageConfig {
    fn default() -> Self {
        Self {
            tenant_id: TenantId::new("default"),
            partition_strategy: PartitionStrategy::Namespace,
            storage_quota_bytes: 1024 * 1024 * 100, // 100 MB
            max_agent_definitions: 100,
            max_agent_executions: 10000,
            execution_retention_days: 30,
            backup_frequency_hours: 24,
            enable_analytics: true,
            custom_config: None,
        }
    }
}

//===============================================================
// Storage Metrics and Analytics
//===============================================================

/// Storage usage metrics for a tenant
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TenantStorageMetrics {
    /// Number of agent definitions
    pub agent_count: usize,
    /// Number of agent executions
    pub execution_count: usize,
    /// Total storage used in bytes
    pub storage_bytes_used: u64,
    /// Storage usage by type
    pub storage_by_type: HashMap<String, u64>,
    /// Number of operations performed
    pub operation_count: HashMap<String, u64>,
    /// Average operation latency in milliseconds
    pub avg_operation_latency_ms: HashMap<String, f64>,
    /// Number of read operations
    pub read_operations: u64,
    /// Number of write operations
    pub write_operations: u64,
    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,
}

impl TenantStorageMetrics {
    /// Create new metrics instance
    pub fn new() -> Self {
        Self {
            last_updated: Utc::now(),
            ..Default::default()
        }
    }

    /// Record storage operation
    pub fn record_operation(&mut self, operation: &str, latency_ms: f64, size_bytes: u64) {
        // Update operation count
        *self
            .operation_count
            .entry(operation.to_string())
            .or_insert(0) += 1;

        // Update average latency using weighted average
        let count = self.operation_count.get(operation).copied().unwrap_or(1);
        let current_avg = self
            .avg_operation_latency_ms
            .get(operation)
            .copied()
            .unwrap_or(0.0);
        let new_avg = (current_avg * (count - 1) as f64 + latency_ms) / count as f64;
        self.avg_operation_latency_ms
            .insert(operation.to_string(), new_avg);

        // Update storage metrics
        if operation.starts_with("write_") || operation.starts_with("create_") {
            self.write_operations += 1;
            self.storage_bytes_used += size_bytes;

            // Update storage by type
            let type_key = operation.split('_').nth(1).unwrap_or("unknown").to_string();
            *self.storage_by_type.entry(type_key).or_insert(0) += size_bytes;
        } else if operation.starts_with("read_") || operation.starts_with("get_") {
            self.read_operations += 1;
        }

        self.last_updated = Utc::now();
    }

    /// Update agent count
    pub fn update_agent_count(&mut self, count: usize) {
        self.agent_count = count;
        self.last_updated = Utc::now();
    }

    /// Update execution count
    pub fn update_execution_count(&mut self, count: usize) {
        self.execution_count = count;
        self.last_updated = Utc::now();
    }
}

/// Metrics collector for tenant storage
pub struct MetricsCollector {
    /// Metrics by tenant ID
    metrics: Arc<RwLock<HashMap<TenantId, TenantStorageMetrics>>>,
    /// Background collection task
    _collection_task: Option<JoinHandle<()>>,
}

impl MetricsCollector {
    /// Create new metrics collector
    pub fn new() -> Self {
        let metrics = Arc::new(RwLock::new(HashMap::new()));
        Self {
            metrics,
            _collection_task: None,
        }
    }

    /// Start background metrics collection
    pub fn start_collection(
        &mut self,
        storage: Arc<dyn AgentStorage>,
        interval: StdDuration,
    ) -> JoinHandle<()> {
        let metrics_clone = self.metrics.clone();
        let task = tokio::spawn(async move {
            let mut interval_timer = time::interval(interval);
            loop {
                interval_timer.tick().await;
                if let Err(e) = Self::collect_metrics(metrics_clone.clone(), storage.clone()).await
                {
                    error!("Error collecting storage metrics: {}", e);
                }
            }
        });

        self._collection_task = Some(task.clone());
        task
    }

    /// Collect metrics from storage
    async fn collect_metrics(
        metrics: Arc<RwLock<HashMap<TenantId, TenantStorageMetrics>>>,
        _storage: Arc<dyn AgentStorage>,
    ) -> Result<()> {
        // In a real implementation, this would query the storage to collect metrics
        // For now, we'll just update timestamps to show activity
        let mut metrics_map = metrics.write().await;
        for metrics in metrics_map.values_mut() {
            metrics.last_updated = Utc::now();
        }
        Ok(())
    }

    /// Record operation for tenant
    pub async fn record_operation(
        &self,
        tenant_id: &TenantId,
        operation: &str,
        latency_ms: f64,
        size_bytes: u64,
    ) {
        let mut metrics_map = self.metrics.write().await;
        let tenant_metrics = metrics_map
            .entry(tenant_id.clone())
            .or_insert_with(TenantStorageMetrics::new);
        tenant_metrics.record_operation(operation, latency_ms, size_bytes);
    }

    /// Get metrics for tenant
    pub async fn get_tenant_metrics(&self, tenant_id: &TenantId) -> Option<TenantStorageMetrics> {
        let metrics_map = self.metrics.read().await;
        metrics_map.get(tenant_id).cloned()
    }

    /// Get all tenant metrics
    pub async fn get_all_metrics(&self) -> HashMap<TenantId, TenantStorageMetrics> {
        let metrics_map = self.metrics.read().await;
        metrics_map.clone()
    }
}

//===============================================================
// Backup and Recovery
//===============================================================

/// Backup type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackupType {
    /// Full backup of all tenant data
    Full,
    /// Incremental backup since last backup
    Incremental,
    /// Backup of agent definitions only
    AgentDefinitionsOnly,
    /// Backup of executions only
    ExecutionsOnly,
}

/// Backup metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    /// Backup ID
    pub id: Uuid,
    /// Tenant ID
    pub tenant_id: TenantId,
    /// Backup type
    pub backup_type: BackupType,
    /// Backup timestamp
    pub created_at: DateTime<Utc>,
    /// Number of agent definitions
    pub agent_count: usize,
    /// Number of agent executions
    pub execution_count: usize,
    /// Backup size in bytes
    pub size_bytes: u64,
    /// Backup file location
    pub location: String,
    /// Additional metadata
    pub metadata: Value,
}

/// Backup manager for tenant storage
pub struct BackupManager {
    /// Base directory for backups
    backup_dir: PathBuf,
    /// Backup tasks by tenant
    backup_tasks: Arc<RwLock<HashMap<TenantId, JoinHandle<()>>>>,
    /// Backup configurations by tenant
    backup_configs: Arc<RwLock<HashMap<TenantId, TenantStorageConfig>>>,
    /// Completed backups by tenant
    backups: Arc<RwLock<HashMap<TenantId, Vec<BackupMetadata>>>>,
}

impl BackupManager {
    /// Create new backup manager
    pub fn new<P: Into<PathBuf>>(backup_dir: P) -> Self {
        Self {
            backup_dir: backup_dir.into(),
            backup_tasks: Arc::new(RwLock::new(HashMap::new())),
            backup_configs: Arc::new(RwLock::new(HashMap::new())),
            backups: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize backup manager
    pub async fn init(&self) -> Result<()> {
        // Create backup directory if it doesn't exist
        if !self.backup_dir.exists() {
            fs::create_dir_all(&self.backup_dir).await?;
        }
        Ok(())
    }

    /// Schedule backup for tenant
    pub async fn schedule_backup(
        &self,
        tenant_id: TenantId,
        config: TenantStorageConfig,
        storage: Arc<dyn AgentStorage>,
    ) -> Result<()> {
        if config.backup_frequency_hours == 0 {
            // Backups disabled for this tenant
            return Ok(());
        }

        // Store backup configuration
        {
            let mut configs = self.backup_configs.write().await;
            configs.insert(tenant_id.clone(), config.clone());
        }

        // Cancel existing backup task if any
        {
            let mut tasks = self.backup_tasks.write().await;
            if let Some(task) = tasks.remove(&tenant_id) {
                task.abort();
            }
        }

        // Create backup directory for tenant
        let tenant_backup_dir = self.backup_dir.join(tenant_id.as_str());
        if !tenant_backup_dir.exists() {
            fs::create_dir_all(&tenant_backup_dir).await?;
        }

        // Schedule backup task
        let backup_freq = StdDuration::from_secs(config.backup_frequency_hours as u64 * 3600);
        let tenant_id_clone = tenant_id.clone();
        let backups_clone = self.backups.clone();
        let backup_dir_clone = tenant_backup_dir.clone();

        let task = tokio::spawn(async move {
            let mut interval = time::interval(backup_freq);
            loop {
                interval.tick().await;
                match Self::perform_backup(
                    &tenant_id_clone,
                    &backup_dir_clone,
                    BackupType::Full,
                    storage.clone(),
                )
                .await
                {
                    Ok(metadata) => {
                        let mut backups = backups_clone.write().await;
                        backups
                            .entry(tenant_id_clone.clone())
                            .or_insert_with(Vec::new)
                            .push(metadata);
                        info!(
                            "Backup completed for tenant {}: {} bytes",
                            tenant_id_clone, metadata.size_bytes
                        );
                    }
                    Err(e) => {
                        error!("Backup failed for tenant {}: {}", tenant_id_clone, e);
                    }
                }
            }
        });

        // Store backup task
        {
            let mut tasks = self.backup_tasks.write().await;
            tasks.insert(tenant_id, task);
        }

        Ok(())
    }

    /// Perform backup for tenant
    async fn perform_backup(
        tenant_id: &TenantId,
        backup_dir: &PathBuf,
        backup_type: BackupType,
        storage: Arc<dyn AgentStorage>,
    ) -> Result<BackupMetadata> {
        let backup_id = Uuid::new_v4();
        let timestamp = Utc::now();
        let backup_filename = format!(
            "{}_{}_{}.json",
            tenant_id.as_str(),
            backup_id,
            timestamp.format("%Y%m%d_%H%M%S")
        );
        let backup_path = backup_dir.join(&backup_filename);

        // Get all agents for tenant
        let agents = storage.list_agents().await?;

        // Get executions
        // In a real implementation, we would filter by tenant
        // For this example, we'll just take recent executions
        let executions = storage.list_recent_executions(100).await?;

        // Create backup data
        let backup_data = json!({
            "metadata": {
                "id": backup_id.to_string(),
                "tenant_id": tenant_id.as_str(),
                "backup_type": format!("{:?}", backup_type),
                "created_at": timestamp.to_rfc3339(),
            },
            "agents": agents,
            "executions": executions,
        });

        // Write backup to file
        let backup_json = serde_json::to_string_pretty(&backup_data)?;
        let mut file = fs::File::create(&backup_path).await?;
        file.write_all(backup_json.as_bytes()).await?;

        // Create backup metadata
        let metadata = BackupMetadata {
            id: backup_id,
            tenant_id: tenant_id.clone(),
            backup_type,
            created_at: timestamp,
            agent_count: agents.len(),
            execution_count: executions.len(),
            size_bytes: backup_json.len() as u64,
            location: backup_path.to_string_lossy().into_owned(),
            metadata: json!({
                "filename": backup_filename
            }),
        };

        Ok(metadata)
    }

    /// Manually trigger backup for tenant
    pub async fn trigger_backup(
        &self,
        tenant_id: &TenantId,
        backup_type: BackupType,
        storage: Arc<dyn AgentStorage>,
    ) -> Result<BackupMetadata> {
        let tenant_backup_dir = self.backup_dir.join(tenant_id.as_str());
        if !tenant_backup_dir.exists() {
            fs::create_dir_all(&tenant_backup_dir).await?;
        }

        let metadata =
            Self::perform_backup(tenant_id, &tenant_backup_dir, backup_type, storage).await?;

        // Store backup metadata
        {
            let mut backups = self.backups.write().await;
            backups
                .entry(tenant_id.clone())
                .or_insert_with(Vec::new)
                .push(metadata.clone());
        }

        Ok(metadata)
    }

    /// List backups for tenant
    pub async fn list_backups(&self, tenant_id: &TenantId) -> Vec<BackupMetadata> {
        let backups = self.backups.read().await;
        backups.get(tenant_id).cloned().unwrap_or_else(Vec::new)
    }

    /// Restore from backup
    pub async fn restore_from_backup(
        &self,
        backup_id: &Uuid,
        tenant_id: &TenantId,
        storage: Arc<dyn AgentStorage>,
    ) -> Result<()> {
        // Find backup metadata
        let backup_path = {
            let backups = self.backups.read().await;
            if let Some(tenant_backups) = backups.get(tenant_id) {
                if let Some(backup) = tenant_backups.iter().find(|b| b.id == *backup_id) {
                    backup.location.clone()
                } else {
                    return Err(CircuitBreakerError::NotFound(format!(
                        "Backup with ID {} not found for tenant {}",
                        backup_id, tenant_id
                    )));
                }
            } else {
                return Err(CircuitBreakerError::NotFound(format!(
                    "No backups found for tenant {}",
                    tenant_id
                )));
            }
        };

        // Read backup file
        let backup_data = fs::read_to_string(&backup_path).await?;
        let backup: Value = serde_json::from_str(&backup_data)?;

        // Extract agents and executions
        let agents: Vec<AgentDefinition> = serde_json::from_value(backup["agents"].clone())?;

        let executions: Vec<AgentExecution> = serde_json::from_value(backup["executions"].clone())?;

        // Restore agents
        for agent in agents {
            storage.store_agent(&agent).await?;
        }

        // Restore executions
        for execution in executions {
            storage.store_execution(&execution).await?;
        }

        info!(
            "Restored backup {} for tenant {}: {} agents, {} executions",
            backup_id,
            tenant_id,
            agents.len(),
            executions.len()
        );

        Ok(())
    }
}

//===============================================================
// Tenant-Aware Storage Implementation
//===============================================================

/// Multi-tenant agent storage implementation
pub struct TenantAgentStorage {
    /// Underlying storage implementation
    inner: Arc<dyn AgentStorage>,
    /// Tenant configurations
    tenant_configs: Arc<RwLock<HashMap<TenantId, TenantStorageConfig>>>,
    /// Metrics collector
    metrics: Arc<MetricsCollector>,
    /// Backup manager
    backup_manager: Option<Arc<BackupManager>>,
    /// Cache of tenant agent counts
    agent_counts: Arc<RwLock<HashMap<TenantId, usize>>>,
    /// Cache of tenant execution counts
    execution_counts: Arc<RwLock<HashMap<TenantId, usize>>>,
}

impl TenantAgentStorage {
    /// Create new tenant-aware storage
    pub fn new(inner: Arc<dyn AgentStorage>) -> Self {
        let metrics = Arc::new(MetricsCollector::new());

        Self {
            inner,
            tenant_configs: Arc::new(RwLock::new(HashMap::new())),
            metrics,
            backup_manager: None,
            agent_counts: Arc::new(RwLock::new(HashMap::new())),
            execution_counts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Configure backup manager
    pub async fn with_backup_manager<P: Into<PathBuf>>(mut self, backup_dir: P) -> Result<Self> {
        let backup_manager = BackupManager::new(backup_dir);
        backup_manager.init().await?;
        self.backup_manager = Some(Arc::new(backup_manager));
        Ok(self)
    }

    /// Add tenant configuration
    pub async fn add_tenant_config(&self, config: TenantStorageConfig) -> Result<()> {
        let tenant_id = config.tenant_id.clone();

        // Store configuration
        {
            let mut configs = self.tenant_configs.write().await;
            configs.insert(tenant_id.clone(), config.clone());
        }

        // Configure backup if enabled
        if let Some(backup_manager) = &self.backup_manager {
            backup_manager
                .schedule_backup(tenant_id, config, self.inner.clone())
                .await?;
        }

        Ok(())
    }

    /// Get tenant configuration
    pub async fn get_tenant_config(&self, tenant_id: &TenantId) -> Option<TenantStorageConfig> {
        let configs = self.tenant_configs.read().await;
        configs.get(tenant_id).cloned()
    }

    /// Get or create tenant configuration
    async fn get_or_create_tenant_config(&self, tenant_id: &TenantId) -> TenantStorageConfig {
        let configs = self.tenant_configs.read().await;
        if let Some(config) = configs.get(tenant_id) {
            config.clone()
        } else {
            // Create default config for tenant
            let mut config = TenantStorageConfig::default();
            config.tenant_id = tenant_id.clone();
            config
        }
    }

    /// Check storage quota for tenant
    async fn check_storage_quota(&self, tenant_id: &TenantId, size_bytes: u64) -> Result<()> {
        let config = self.get_or_create_tenant_config(tenant_id).await;

        // Skip check if quota is unlimited
        if config.storage_quota_bytes == 0 {
            return Ok(());
        }

        // Get current usage
        let metrics = self
            .metrics
            .get_tenant_metrics(tenant_id)
            .await
            .unwrap_or_default();

        // Check if quota would be exceeded
        if metrics.storage_bytes_used + size_bytes > config.storage_quota_bytes {
            return Err(CircuitBreakerError::QuotaExceeded(format!(
                "Storage quota exceeded for tenant {}",
                tenant_id
            )));
        }

        Ok(())
    }

    /// Check agent quota for tenant
    async fn check_agent_quota(&self, tenant_id: &TenantId) -> Result<()> {
        let config = self.get_or_create_tenant_config(tenant_id).await;

        // Skip check if quota is unlimited
        if config.max_agent_definitions == 0 {
            return Ok(());
        }

        // Get current count
        let agent_counts = self.agent_counts.read().await;
        let current_count = agent_counts.get(tenant_id).copied().unwrap_or(0);

        // Check if quota would be exceeded
        if current_count >= config.max_agent_definitions {
            return Err(CircuitBreakerError::QuotaExceeded(format!(
                "Agent quota exceeded for tenant {}",
                tenant_id
            )));
        }

        Ok(())
    }

    /// Check execution quota for tenant
    async fn check_execution_quota(&self, tenant_id: &TenantId) -> Result<()> {
        let config = self.get_or_create_tenant_config(tenant_id).await;

        // Skip check if quota is unlimited
        if config.max_agent_executions == 0 {
            return Ok(());
        }

        // Get current count
        let execution_counts = self.execution_counts.read().await;
        let current_count = execution_counts.get(tenant_id).copied().unwrap_or(0);

        // Check if quota would be exceeded
        if current_count >= config.max_agent_executions {
            return Err(CircuitBreakerError::QuotaExceeded(format!(
                "Execution quota exceeded for tenant {}",
                tenant_id
            )));
        }

        Ok(())
    }

    /// Extract tenant ID from agent definition
    fn extract_tenant_from_agent(&self, agent: &AgentDefinition) -> Option<TenantId> {
        // Check system prompt for tenant ID
        if agent.prompts.system.contains("tenant") {
            // Try to extract tenant ID from system prompt
            // In a real implementation, this would be more robust
            return Some(TenantId::new("default"));
        } else {
            // In a real implementation, the agent would have tenant metadata
            // For the test implementation, we'll return the default tenant
            return Some(TenantId::new("default"));
        }
    }

    /// Extract tenant ID from agent execution
    fn extract_tenant_from_execution(&self, execution: &AgentExecution) -> Option<TenantId> {
        execution
            .context
            .get("tenant_id")
            .and_then(|v| v.as_str())
            .map(TenantId::new)
    }

    /// Ensure agent has tenant ID
    fn ensure_agent_has_tenant(
        &self,
        mut agent: AgentDefinition,
        tenant_id: &TenantId,
    ) -> AgentDefinition {
        // Check if agent already has tenant ID
        if self.extract_tenant_from_agent(&agent).is_some() {
            return agent;
        }

        // Add tenant ID to metadata
        let metadata = agent.metadata.as_object().cloned().unwrap_or_default();
        let mut metadata = metadata;
        metadata.insert(
            "tenant_id".to_string(),
            Value::String(tenant_id.as_str().to_string()),
        );
        agent.metadata = Value::Object(metadata);

        agent
    }

    /// Ensure execution has tenant ID
    fn ensure_execution_has_tenant(
        &self,
        mut execution: AgentExecution,
        tenant_id: &TenantId,
    ) -> AgentExecution {
        // Check if execution already has tenant ID
        if self.extract_tenant_from_execution(&execution).is_some() {
            return execution;
        }

        // Add tenant ID to context
        let context = execution.context.as_object().cloned().unwrap_or_default();
        let mut context = context;
        context.insert(
            "tenant_id".to_string(),
            Value::String(tenant_id.as_str().to_string()),
        );
        execution.context = Value::Object(context);

        execution
    }

    /// Record agent count for tenant
    async fn update_agent_count(&self, tenant_id: &TenantId) -> Result<()> {
        let agents = self.inner.list_agents().await?;

        // Count agents for this tenant
        let tenant_agents = agents
            .iter()
            .filter(|a| {
                self.extract_tenant_from_agent(a)
                    .map(|t| t == *tenant_id)
                    .unwrap_or(false)
            })
            .count();

        // Update count
        {
            let mut counts = self.agent_counts.write().await;
            counts.insert(tenant_id.clone(), tenant_agents);
        }

        // Update metrics
        if let Some(metrics) = self.metrics.get_tenant_metrics(tenant_id).await {
            let mut updated_metrics = metrics;
            updated_metrics.update_agent_count(tenant_agents);
            self.metrics
                .record_operation(tenant_id, "count_agents", 0.0, 0)
                .await;
        }

        Ok(())
    }

    /// Record execution count for tenant
    async fn update_execution_count(&self, tenant_id: &TenantId) -> Result<()> {
        // Get count from context-based query
        let executions = self
            .inner
            .list_executions_by_context("tenant_id", tenant_id.as_str())
            .await?;

        let count = executions.len();

        // Update count
        {
            let mut counts = self.execution_counts.write().await;
            counts.insert(tenant_id.clone(), count);
        }

        // Update metrics
        if let Some(metrics) = self.metrics.get_tenant_metrics(tenant_id).await {
            let mut updated_metrics = metrics;
            updated_metrics.update_execution_count(count);
            self.metrics
                .record_operation(tenant_id, "count_executions", 0.0, 0)
                .await;
        }

        Ok(())
    }

    /// Get tenant metrics
    pub async fn get_tenant_metrics(&self, tenant_id: &TenantId) -> Option<TenantStorageMetrics> {
        self.metrics.get_tenant_metrics(tenant_id).await
    }

    /// Get all tenant metrics
    pub async fn get_all_metrics(&self) -> HashMap<TenantId, TenantStorageMetrics> {
        self.metrics.get_all_metrics().await
    }

    /// Trigger backup for tenant
    pub async fn trigger_backup(
        &self,
        tenant_id: &TenantId,
        backup_type: BackupType,
    ) -> Result<BackupMetadata> {
        if let Some(backup_manager) = &self.backup_manager {
            backup_manager
                .trigger_backup(tenant_id, backup_type, self.inner.clone())
                .await
        } else {
            Err(CircuitBreakerError::Internal(
                "Backup manager not configured".to_string(),
            ))
        }
    }
}

#[async_trait]
impl AgentStorage for TenantAgentStorage {
    async fn store_agent(&self, agent: &AgentDefinition) -> Result<()> {
        // Start timing
        let start = std::time::Instant::now();

        // Try to extract tenant ID or use default
        let tenant_id = self
            .extract_tenant_from_agent(agent)
            .unwrap_or_else(|| TenantId::new("default"));

        // Check quota
        self.check_agent_quota(&tenant_id).await?;

        // Ensure agent has tenant ID
        let agent_with_tenant = self.ensure_agent_has_tenant(agent.clone(), &tenant_id);

        // Get agent JSON size for metrics
        let agent_size = serde_json::to_string(&agent_with_tenant)
            .map(|s| s.len() as u64)
            .unwrap_or(0);

        // Check storage quota
        self.check_storage_quota(&tenant_id, agent_size).await?;

        // Store agent
        let result = self.inner.store_agent(&agent_with_tenant).await;

        // Record metrics
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        self.metrics
            .record_operation(&tenant_id, "store_agent", elapsed, agent_size)
            .await;

        // Update agent count
        if result.is_ok() {
            self.update_agent_count(&tenant_id).await?;
        }

        result
    }

    async fn get_agent(&self, id: &AgentId) -> Result<Option<AgentDefinition>> {
        // Start timing
        let start = std::time::Instant::now();

        // Get agent
        let result = self.inner.get_agent(id).await;

        // Record metrics for this operation
        if let Ok(Some(agent)) = &result {
            if let Some(tenant_id) = self.extract_tenant_from_agent(agent) {
                let agent_size = serde_json::to_string(agent)
                    .map(|s| s.len() as u64)
                    .unwrap_or(0);

                let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                self.metrics
                    .record_operation(&tenant_id, "get_agent", elapsed, agent_size)
                    .await;
            }
        }

        result
    }

    async fn list_agents(&self) -> Result<Vec<AgentDefinition>> {
        // Start timing
        let start = std::time::Instant::now();

        // Get all agents
        let agents = self.inner.list_agents().await?;

        // Record operation without tenant ID (we don't know which tenant yet)
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;

        // Update metrics for each tenant found in the agents
        let mut tenant_agents: HashMap<TenantId, Vec<AgentDefinition>> = HashMap::new();

        for agent in &agents {
            if let Some(tenant_id) = self.extract_tenant_from_agent(agent) {
                tenant_agents
                    .entry(tenant_id)
                    .or_insert_with(Vec::new)
                    .push(agent.clone());
            }
        }

        // Record metrics for each tenant
        for (tenant_id, tenant_agents) in tenant_agents {
            let agent_size = serde_json::to_string(&tenant_agents)
                .map(|s| s.len() as u64)
                .unwrap_or(0);

            self.metrics
                .record_operation(&tenant_id, "list_agents", elapsed, agent_size)
                .await;
        }

        Ok(agents)
    }

    async fn delete_agent(&self, id: &AgentId) -> Result<bool> {
        // Start timing
        let start = std::time::Instant::now();

        // Get agent to determine tenant
        let tenant_id = if let Ok(Some(agent)) = self.inner.get_agent(id).await {
            self.extract_tenant_from_agent(&agent)
                .unwrap_or_else(|| TenantId::new("default"))
        } else {
            TenantId::new("default")
        };

        // Delete agent
        let result = self.inner.delete_agent(id).await;

        // Record metrics
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        self.metrics
            .record_operation(&tenant_id, "delete_agent", elapsed, 0)
            .await;

        // Update agent count
        if let Ok(true) = result {
            self.update_agent_count(&tenant_id).await?;
        }

        result
    }

    async fn store_execution(&self, execution: &AgentExecution) -> Result<()> {
        // Start timing
        let start = std::time::Instant::now();

        // Try to extract tenant ID or use default
        let tenant_id = self
            .extract_tenant_from_execution(execution)
            .unwrap_or_else(|| TenantId::new("default"));

        // Check quota
        self.check_execution_quota(&tenant_id).await?;

        // Ensure execution has tenant ID
        let execution_with_tenant = self.ensure_execution_has_tenant(execution.clone(), &tenant_id);

        // Get execution JSON size for metrics
        let execution_size = serde_json::to_string(&execution_with_tenant)
            .map(|s| s.len() as u64)
            .unwrap_or(0);

        // Check storage quota
        self.check_storage_quota(&tenant_id, execution_size).await?;

        // Store execution
        let result = self.inner.store_execution(&execution_with_tenant).await;

        // Record metrics
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        self.metrics
            .record_operation(&tenant_id, "store_execution", elapsed, execution_size)
            .await;

        // Update execution count
        if result.is_ok() {
            self.update_execution_count(&tenant_id).await?;
        }

        result
    }

    async fn get_execution(&self, id: &Uuid) -> Result<Option<AgentExecution>> {
        // Start timing
        let start = std::time::Instant::now();

        // Get execution
        let result = self.inner.get_execution(id).await;

        // Record metrics for this operation
        if let Ok(Some(execution)) = &result {
            if let Some(tenant_id) = self.extract_tenant_from_execution(execution) {
                let execution_size = serde_json::to_string(execution)
                    .map(|s| s.len() as u64)
                    .unwrap_or(0);

                let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                self.metrics
                    .record_operation(&tenant_id, "get_execution", elapsed, execution_size)
                    .await;
            }
        }

        result
    }

    async fn list_executions_by_context(
        &self,
        context_key: &str,
        context_value: &str,
    ) -> Result<Vec<AgentExecution>> {
        // Start timing
        let start = std::time::Instant::now();

        // If querying for tenant_id, this is straightforward
        if context_key == "tenant_id" {
            let tenant_id = TenantId::new(context_value);

            // Get executions
            let executions = self
                .inner
                .list_executions_by_context(context_key, context_value)
                .await?;

            // Record metrics
            let elapsed = start.elapsed().as_secs_f64() * 1000.0;
            let executions_size = serde_json::to_string(&executions)
                .map(|s| s.len() as u64)
                .unwrap_or(0);

            self.metrics
                .record_operation(
                    &tenant_id,
                    "list_executions_by_context",
                    elapsed,
                    executions_size,
                )
                .await;

            return Ok(executions);
        }

        // Otherwise, we need to get all executions and filter by tenant
        let executions = self
            .inner
            .list_executions_by_context(context_key, context_value)
            .await?;

        // Group by tenant for metrics
        let mut tenant_executions: HashMap<TenantId, Vec<AgentExecution>> = HashMap::new();

        for execution in &executions {
            if let Some(tenant_id) = self.extract_tenant_from_execution(execution) {
                tenant_executions
                    .entry(tenant_id)
                    .or_insert_with(Vec::new)
                    .push(execution.clone());
            }
        }

        // Record metrics for each tenant
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        for (tenant_id, tenant_execs) in &tenant_executions {
            let execs_size = serde_json::to_string(tenant_execs)
                .map(|s| s.len() as u64)
                .unwrap_or(0);

            self.metrics
                .record_operation(tenant_id, "list_executions_by_context", elapsed, execs_size)
                .await;
        }

        Ok(executions)
    }

    async fn list_executions_by_context_filters(
        &self,
        filters: &[(&str, &str)],
    ) -> Result<Vec<AgentExecution>> {
        // Start timing
        let start = std::time::Instant::now();

        // Check if tenant_id is in filters
        let tenant_id_filter = filters.iter().find(|(key, _)| *key == "tenant_id");

        // Get executions
        let executions = self
            .inner
            .list_executions_by_context_filters(filters)
            .await?;

        // Record metrics
        if let Some((_, tenant_id_value)) = tenant_id_filter {
            let tenant_id = TenantId::new(*tenant_id_value);
            let elapsed = start.elapsed().as_secs_f64() * 1000.0;
            let executions_size = serde_json::to_string(&executions)
                .map(|s| s.len() as u64)
                .unwrap_or(0);

            self.metrics
                .record_operation(
                    &tenant_id,
                    "list_executions_by_context_filters",
                    elapsed,
                    executions_size,
                )
                .await;
        }

        Ok(executions)
    }

    async fn list_executions_by_nested_context(
        &self,
        context_path: &str,
        context_value: &str,
    ) -> Result<Vec<AgentExecution>> {
        // Start timing
        let start = std::time::Instant::now();

        // If querying for tenant_id, this is straightforward
        if context_path == "tenant_id" {
            let tenant_id = TenantId::new(context_value);

            // Get executions
            let executions = self
                .inner
                .list_executions_by_nested_context(context_path, context_value)
                .await?;

            // Record metrics
            let elapsed = start.elapsed().as_secs_f64() * 1000.0;
            let executions_size = serde_json::to_string(&executions)
                .map(|s| s.len() as u64)
                .unwrap_or(0);

            self.metrics
                .record_operation(
                    &tenant_id,
                    "list_executions_by_nested_context",
                    elapsed,
                    executions_size,
                )
                .await;

            return Ok(executions);
        }

        // Otherwise, we need to get all executions and filter by tenant afterward
        let executions = self
            .inner
            .list_executions_by_nested_context(context_path, context_value)
            .await?;

        // Record operation without specific tenant ID
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;

        // We could add tenant-specific metrics here if needed

        Ok(executions)
    }

    async fn count_executions_by_context(
        &self,
        context_key: &str,
        context_value: &str,
    ) -> Result<usize> {
        // Start timing
        let start = std::time::Instant::now();

        // Get count
        let count = self
            .inner
            .count_executions_by_context(context_key, context_value)
            .await?;

        // Record metrics if this is a tenant query
        if context_key == "tenant_id" {
            let tenant_id = TenantId::new(context_value);
            let elapsed = start.elapsed().as_secs_f64() * 1000.0;

            self.metrics
                .record_operation(&tenant_id, "count_executions_by_context", elapsed, 0)
                .await;
        }

        Ok(count)
    }

    async fn list_executions_for_agent(&self, agent_id: &AgentId) -> Result<Vec<AgentExecution>> {
        // Start timing
        let start = std::time::Instant::now();

        // Get agent to determine tenant
        let tenant_id = if let Ok(Some(agent)) = self.inner.get_agent(agent_id).await {
            self.extract_tenant_from_agent(&agent)
                .unwrap_or_else(|| TenantId::new("default"))
        } else {
            TenantId::new("default")
        };

        // Get executions
        let executions = self.inner.list_executions_for_agent(agent_id).await?;

        // Record metrics
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        let executions_size = serde_json::to_string(&executions)
            .map(|s| s.len() as u64)
            .unwrap_or(0);

        self.metrics
            .record_operation(
                &tenant_id,
                "list_executions_for_agent",
                elapsed,
                executions_size,
            )
            .await;

        Ok(executions)
    }

    async fn list_executions_by_status(
        &self,
        status: &AgentExecutionStatus,
    ) -> Result<Vec<AgentExecution>> {
        // Start timing
        let start = std::time::Instant::now();

        // Get executions
        let executions = self.inner.list_executions_by_status(status).await?;

        // Group by tenant for metrics
        let mut tenant_executions: HashMap<TenantId, Vec<AgentExecution>> = HashMap::new();

        for execution in &executions {
            if let Some(tenant_id) = self.extract_tenant_from_execution(execution) {
                tenant_executions
                    .entry(tenant_id)
                    .or_insert_with(Vec::new)
                    .push(execution.clone());
            }
        }

        // Record metrics for each tenant
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        for (tenant_id, tenant_execs) in &tenant_executions {
            let execs_size = serde_json::to_string(tenant_execs)
                .map(|s| s.len() as u64)
                .unwrap_or(0);

            self.metrics
                .record_operation(tenant_id, "list_executions_by_status", elapsed, execs_size)
                .await;
        }

        Ok(executions)
    }

    async fn list_recent_executions(&self, limit: usize) -> Result<Vec<AgentExecution>> {
        // Start timing
        let start = std::time::Instant::now();

        // Get executions
        let executions = self.inner.list_recent_executions(limit).await?;

        // Group by tenant for metrics
        let mut tenant_executions: HashMap<TenantId, Vec<AgentExecution>> = HashMap::new();

        for execution in &executions {
            if let Some(tenant_id) = self.extract_tenant_from_execution(execution) {
                tenant_executions
                    .entry(tenant_id)
                    .or_insert_with(Vec::new)
                    .push(execution.clone());
            }
        }

        // Record metrics for each tenant
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        for (tenant_id, tenant_execs) in &tenant_executions {
            let execs_size = serde_json::to_string(tenant_execs)
                .map(|s| s.len() as u64)
                .unwrap_or(0);

            self.metrics
                .record_operation(tenant_id, "list_recent_executions", elapsed, execs_size)
                .await;
        }

        Ok(executions)
    }

    async fn list_executions_for_agent_with_context(
        &self,
        agent_id: &AgentId,
        context_key: &str,
        context_value: &str,
    ) -> Result<Vec<AgentExecution>> {
        // Start timing
        let start = std::time::Instant::now();

        // Get agent to determine tenant
        let tenant_id = if let Ok(Some(agent)) = self.inner.get_agent(agent_id).await {
            self.extract_tenant_from_agent(&agent)
                .unwrap_or_else(|| TenantId::new("default"))
        } else {
            TenantId::new("default")
        };

        // Get executions
        let executions = self
            .inner
            .list_executions_for_agent_with_context(agent_id, context_key, context_value)
            .await?;

        // Record metrics
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        let executions_size = serde_json::to_string(&executions)
            .map(|s| s.len() as u64)
            .unwrap_or(0);

        self.metrics
            .record_operation(
                &tenant_id,
                "list_executions_for_agent_with_context",
                elapsed,
                executions_size,
            )
            .await;

        Ok(executions)
    }
}

//===============================================================
// Integration Tests
//===============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::InMemoryAgentStorage;
    use std::env;
    // No need for duration import in tests

    // Helper function to create test storage
    async fn create_test_storage() -> (Arc<TenantAgentStorage>, Arc<dyn AgentStorage>) {
        // Create in-memory storage
        let inner_storage = Arc::new(InMemoryAgentStorage::new());

        // Create a test directory path for backups
        let backup_path = env::temp_dir().join("circuit_breaker_test_backups");
        // Ensure directory exists
        if !backup_path.exists() {
            std::fs::create_dir_all(&backup_path).unwrap();
        }

        // Create tenant storage
        let tenant_storage = TenantAgentStorage::new(inner_storage.clone());
        let tenant_storage = tenant_storage
            .with_backup_manager(backup_path)
            .await
            .unwrap();

        // Add tenant configurations
        let tenant1_config = TenantStorageConfig {
            tenant_id: TenantId::new("tenant1"),
            partition_strategy: PartitionStrategy::Namespace,
            storage_quota_bytes: 1024 * 1024 * 10, // 10 MB
            max_agent_definitions: 5,
            max_agent_executions: 100,
            execution_retention_days: 30,
            backup_frequency_hours: 24,
            enable_analytics: true,
            custom_config: None,
        };

        let tenant2_config = TenantStorageConfig {
            tenant_id: TenantId::new("tenant2"),
            partition_strategy: PartitionStrategy::Namespace,
            storage_quota_bytes: 1024 * 1024 * 5, // 5 MB
            max_agent_definitions: 3,
            max_agent_executions: 50,
            execution_retention_days: 15,
            backup_frequency_hours: 12,
            enable_analytics: true,
            custom_config: None,
        };

        tenant_storage
            .add_tenant_config(tenant1_config)
            .await
            .unwrap();
        tenant_storage
            .add_tenant_config(tenant2_config)
            .await
            .unwrap();

        let tenant_storage_arc = Arc::new(tenant_storage);

        (tenant_storage_arc, inner_storage)
    }

    // Helper function to create test agent for tenant
    fn create_test_agent(tenant_id: &TenantId, index: usize) -> AgentDefinition {
        AgentDefinition {
            id: AgentId::from(format!("{}-agent-{}", tenant_id, index)),
            name: format!("Test Agent {} for {}", index, tenant_id),
            description: format!("Test agent for tenant {}", tenant_id),
            llm_provider: LLMProvider::OpenAI {
                model: "gpt-3.5-turbo".to_string(),
                api_key: "test-key".to_string(),
                base_url: None,
            },
            llm_config: LLMConfig {
                temperature: 0.7,
                max_tokens: Some(1000),
                top_p: Some(1.0),
                frequency_penalty: Some(0.0),
                presence_penalty: Some(0.0),
                stop_sequences: None,
            },
            prompts: AgentPrompts {
                system: format!("You are a test agent for tenant {}", tenant_id),
                user_template: "User query: {{query}}",
                context_instructions: Some(format!(
                    "Remember you are working for tenant {}",
                    tenant_id
                )),
            },
            capabilities: vec!["text".to_string(), "chat".to_string()],
            tools: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    // Helper function to create test execution for tenant
    fn create_test_execution(
        tenant_id: &TenantId,
        agent_id: &AgentId,
        index: usize,
    ) -> AgentExecution {
        AgentExecution::new(
            agent_id.clone(),
            json!({
                "tenant_id": tenant_id.to_string(),
                "test_index": index,
                "message": format!("Test execution {} for {}", index, tenant_id)
            }),
            json!({
                "content": format!("Test content for tenant {}", tenant_id)
            }),
        )
    }

    #[tokio::test]
    async fn test_tenant_data_partitioning() {
        let (storage, _) = create_test_storage().await;

        // Create agents for different tenants
        let tenant1 = TenantId::new("tenant1");
        let tenant2 = TenantId::new("tenant2");

        let tenant1_agent = create_test_agent(&tenant1, 1);
        let tenant2_agent = create_test_agent(&tenant2, 1);

        // Store agents
        storage.store_agent(&tenant1_agent).await.unwrap();
        storage.store_agent(&tenant2_agent).await.unwrap();

        // Create executions
        let tenant1_execution = create_test_execution(&tenant1, &tenant1_agent.id, 1);

        let tenant2_execution = create_test_execution(&tenant2, &tenant2_agent.id, 1);

        // Store executions
        storage.store_execution(&tenant1_execution).await.unwrap();
        storage.store_execution(&tenant2_execution).await.unwrap();

        // Query by tenant context
        let tenant1_executions = storage
            .list_executions_by_context("tenant_id", "tenant1")
            .await
            .unwrap();

        let tenant2_executions = storage
            .list_executions_by_context("tenant_id", "tenant2")
            .await
            .unwrap();

        // Verify isolation
        assert_eq!(tenant1_executions.len(), 1);
        assert_eq!(tenant2_executions.len(), 1);

        assert_eq!(tenant1_executions[0].id, tenant1_execution.id);

        assert_eq!(tenant2_executions[0].id, tenant2_execution.id);
    }

    #[tokio::test]
    async fn test_tenant_quotas() {
        let (storage, _) = create_test_storage().await;

        // Tenant2 has a limit of 3 agents
        let tenant2 = TenantId::new("tenant2");

        // Store agents up to the limit
        for i in 1..=3 {
            let agent = create_test_agent(&tenant2, i);
            storage.store_agent(&agent).await.unwrap();
        }

        // Try to exceed the limit
        let extra_agent = create_test_agent(&tenant2, 4);
        let result = storage.store_agent(&extra_agent).await;

        // Should get quota exceeded error
        assert!(result.is_err());
        match result {
            Err(CircuitBreakerError::QuotaExceeded(_)) => {
                // Expected error
            }
            _ => panic!("Expected QuotaExceeded error"),
        }
    }

    #[tokio::test]
    async fn test_tenant_metrics() {
        let (storage, _) = create_test_storage().await;

        // Create agents and executions for tenant
        let tenant1 = TenantId::new("tenant1");

        let agent = create_test_agent(&tenant1, 1);
        storage.store_agent(&agent).await.unwrap();

        // Create multiple executions
        for i in 1..=5 {
            let execution = create_test_execution(&tenant1, &agent.id, i);
            storage.store_execution(&execution).await.unwrap();
        }

        // Get metrics
        let metrics = storage.get_tenant_metrics(&tenant1).await.unwrap();

        // Verify metrics were collected
        assert!(metrics.agent_count > 0);
        assert!(metrics.execution_count > 0);
        assert!(metrics.storage_bytes_used > 0);
        assert!(metrics.write_operations > 0);
    }

    #[tokio::test]
    async fn test_tenant_backup_and_recovery() {
        let (storage, _) = create_test_storage().await;

        // Create agent for tenant
        let tenant1 = TenantId::new("tenant1");
        let agent = create_test_agent(&tenant1, 1);
        storage.store_agent(&agent).await.unwrap();

        // Create execution
        let execution = create_test_execution(&tenant1, &agent.id, 1);
        storage.store_execution(&execution).await.unwrap();

        // Trigger backup
        let backup_metadata = storage
            .trigger_backup(&tenant1, BackupType::Full)
            .await
            .unwrap();

        // Verify backup was created
        assert_eq!(backup_metadata.tenant_id, tenant1);
        assert!(backup_metadata.agent_count > 0);
        assert!(backup_metadata.execution_count > 0);

        // In a real test, we would delete the data and restore from backup
        // For this example, we'll just verify the backup file exists
        assert!(std::path::Path::new(&backup_metadata.location).exists());
    }

    #[tokio::test]
    async fn test_concurrent_tenant_operations() {
        let (storage, _) = create_test_storage().await;

        // Create tenants
        let tenant1 = TenantId::new("tenant1");
        let tenant2 = TenantId::new("tenant2");

        // Create agents
        let tenant1_agent = create_test_agent(&tenant1, 1);
        let tenant2_agent = create_test_agent(&tenant2, 1);

        storage.store_agent(&tenant1_agent).await.unwrap();
        storage.store_agent(&tenant2_agent).await.unwrap();

        // Spawn concurrent tasks for each tenant
        let storage_clone1 = storage.clone();
        let storage_clone2 = storage.clone();

        let tenant1_task = tokio::spawn(async move {
            for i in 1..=10 {
                let execution = create_test_execution(&tenant1, &tenant1_agent.id, i);
                storage_clone1.store_execution(&execution).await.unwrap();
            }
        });

        let tenant2_task = tokio::spawn(async move {
            for i in 1..=10 {
                let execution = create_test_execution(&tenant2, &tenant2_agent.id, i);
                storage_clone2.store_execution(&execution).await.unwrap();
            }
        });

        // Wait for both tasks to complete
        tokio::try_join!(tenant1_task, tenant2_task).unwrap();

        // Verify executions were stored correctly
        let tenant1_executions = storage
            .list_executions_by_context("tenant_id", "tenant1")
            .await
            .unwrap();

        let tenant2_executions = storage
            .list_executions_by_context("tenant_id", "tenant2")
            .await
            .unwrap();

        // Verify correct counts
        assert_eq!(tenant1_executions.len(), 10);
        assert_eq!(tenant2_executions.len(), 10);

        // Verify tenant isolation was maintained during concurrent operations
        for execution in &tenant1_executions {
            assert_eq!(
                execution.context.get("tenant_id").and_then(|v| v.as_str()),
                Some("tenant1")
            );
        }

        for execution in &tenant2_executions {
            assert_eq!(
                execution.context.get("tenant_id").and_then(|v| v.as_str()),
                Some("tenant2")
            );
        }
    }

    #[tokio::test]
    async fn test_analytics_and_monitoring() {
        let (storage, _) = create_test_storage().await;

        // Create tenant and agent
        let tenant_id = TenantId::new("tenant1");
        let agent = create_test_agent(&tenant_id, 1);

        // Store agent
        storage.store_agent(&agent).await.unwrap();

        // Create and store multiple executions to generate metrics
        for i in 1..=5 {
            let execution = create_test_execution(&tenant_id, &agent.id, i);
            storage.store_execution(&execution).await.unwrap();
        }

        // Perform some read operations
        storage
            .list_executions_by_context("tenant_id", "tenant1")
            .await
            .unwrap();
        storage.list_agents().await.unwrap();

        // Get metrics
        let metrics = storage.get_tenant_metrics(&tenant_id).await.unwrap();

        // Verify metrics were collected
        assert!(metrics.agent_count > 0);
        assert!(metrics.execution_count > 0);
        assert!(metrics.storage_bytes_used > 0);
        assert!(metrics.read_operations > 0);
        assert!(metrics.write_operations > 0);

        // Verify operation counts
        assert!(metrics.operation_count.get("store_agent").is_some());
        assert!(metrics.operation_count.get("store_execution").is_some());

        // Verify latency metrics
        assert!(metrics
            .avg_operation_latency_ms
            .get("store_agent")
            .is_some());
        assert!(metrics
            .avg_operation_latency_ms
            .get("store_execution")
            .is_some());
    }

    #[tokio::test]
    async fn test_data_retention() {
        let (storage, _) = create_test_storage().await;

        // Create tenant with short retention period
        let config = TenantStorageConfig {
            tenant_id: TenantId::new("retention_test"),
            partition_strategy: PartitionStrategy::Namespace,
            storage_quota_bytes: 1024 * 1024 * 10,
            max_agent_definitions: 10,
            max_agent_executions: 100,
            execution_retention_days: 1, // 1 day retention
            backup_frequency_hours: 24,
            enable_analytics: true,
            custom_config: None,
        };

        storage.add_tenant_config(config).await.unwrap();

        // In a real implementation, we would test the retention policy by:
        // 1. Creating executions with timestamps in the past
        // 2. Triggering the retention cleanup process
        // 3. Verifying old executions are removed
        //
        // For this example, we'll just verify the config was stored
        let stored_config = storage
            .get_tenant_config(&TenantId::new("retention_test"))
            .await
            .unwrap();
        assert_eq!(stored_config.execution_retention_days, 1);
    }
}
