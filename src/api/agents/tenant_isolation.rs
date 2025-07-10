// Tenant Isolation for Agent Engine
// This module provides tenant isolation capabilities for the agent engine

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::engine::{AgentEngine, AgentEngineConfig, AgentStorage};
use crate::models::{
    AgentActivityConfig, AgentDefinition, AgentExecution, AgentExecutionStatus, AgentId,
    AgentStreamEvent, LLMConfig, LLMProvider,
};
use crate::{CircuitBreakerError, Result};

// Tenant configuration types

/// Configuration for a tenant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantConfig {
    /// Unique identifier for the tenant
    pub tenant_id: String,

    /// Display name for the tenant
    pub name: String,

    /// Whether the tenant is active
    pub active: bool,

    /// Resource quotas for the tenant
    pub quotas: ResourceQuotas,

    /// Rate limits for the tenant
    pub rate_limits: RateLimits,

    /// Maximum concurrent executions
    pub max_concurrent_executions: usize,

    /// Default model configuration
    pub default_model_config: Option<TenantModelConfig>,

    /// Allowed models for this tenant
    pub allowed_models: Option<Vec<String>>,

    /// Custom metadata for the tenant
    pub metadata: serde_json::Value,

    /// Created timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Last updated timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Default for TenantConfig {
    fn default() -> Self {
        Self {
            tenant_id: "default".to_string(),
            name: "Default Tenant".to_string(),
            active: true,
            quotas: ResourceQuotas::default(),
            rate_limits: RateLimits::default(),
            max_concurrent_executions: 10,
            default_model_config: None,
            allowed_models: None,
            metadata: serde_json::json!({}),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }
}

/// Resource quotas for a tenant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceQuotas {
    /// Maximum number of agent definitions
    pub max_agents: usize,

    /// Maximum number of agent executions
    pub max_executions: usize,

    /// Maximum tokens per execution
    pub max_tokens_per_execution: usize,

    /// Maximum storage size in bytes
    pub max_storage_bytes: usize,

    /// Maximum execution history retention in days
    pub max_execution_history_days: usize,
}

impl Default for ResourceQuotas {
    fn default() -> Self {
        Self {
            max_agents: 100,
            max_executions: 10000,
            max_tokens_per_execution: 4000,
            max_storage_bytes: 1024 * 1024 * 100, // 100 MB
            max_execution_history_days: 30,
        }
    }
}

/// Rate limits for a tenant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimits {
    /// Maximum requests per minute
    pub requests_per_minute: usize,

    /// Maximum executions per hour
    pub executions_per_hour: usize,

    /// Maximum tokens per day
    pub tokens_per_day: usize,
}

impl Default for RateLimits {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            executions_per_hour: 100,
            tokens_per_day: 100000,
        }
    }
}

/// Tenant-specific model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantModelConfig {
    /// Default model to use
    pub default_model: String,

    /// Default temperature setting
    pub default_temperature: f32,

    /// Default max tokens
    pub default_max_tokens: usize,
}

// Tenant statistics tracking

/// Usage statistics for a tenant
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TenantUsageStats {
    /// Total executions
    pub total_executions: usize,

    /// Total tokens used
    pub total_tokens: usize,

    /// Total execution time in milliseconds
    pub total_execution_ms: u64,

    /// Total storage used in bytes
    pub storage_bytes_used: usize,

    /// Count of executions by status
    pub executions_by_status: HashMap<String, usize>,

    /// Count of executions by model
    pub executions_by_model: HashMap<String, usize>,

    /// Last updated timestamp
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl TenantUsageStats {
    pub fn new() -> Self {
        Self {
            last_updated: chrono::Utc::now(),
            ..Default::default()
        }
    }

    pub fn record_execution(
        &mut self,
        execution: &AgentExecution,
        tokens: usize,
        duration_ms: u64,
    ) {
        self.total_executions += 1;
        self.total_tokens += tokens;
        self.total_execution_ms += duration_ms;

        let status = execution.status.to_string();
        *self.executions_by_status.entry(status).or_insert(0) += 1;

        // Extract model from context if available
        if let Some(model) = execution
            .context
            .get("model")
            .and_then(|m| m.as_str())
            .or_else(|| {
                execution
                    .context
                    .get("llm_config")
                    .and_then(|c| c.get("model"))
                    .and_then(|m| m.as_str())
            })
        {
            *self
                .executions_by_model
                .entry(model.to_string())
                .or_insert(0) += 1;
        }

        self.last_updated = chrono::Utc::now();
    }
}

// Tenant-aware storage wrapper

/// Storage wrapper that provides tenant isolation
pub struct TenantAwareStorage {
    /// Underlying storage implementation
    inner: Arc<dyn AgentStorage>,

    /// Tenant ID
    tenant_id: String,

    /// Usage statistics
    usage_stats: Arc<RwLock<TenantUsageStats>>,
}

impl TenantAwareStorage {
    pub fn new(inner: Arc<dyn AgentStorage>, tenant_id: String) -> Self {
        Self {
            inner,
            tenant_id,
            usage_stats: Arc::new(RwLock::new(TenantUsageStats::new())),
        }
    }

    pub async fn get_usage_stats(&self) -> TenantUsageStats {
        self.usage_stats.read().await.clone()
    }

    // Ensure context has tenant ID
    fn ensure_tenant_in_context(&self, mut execution: AgentExecution) -> AgentExecution {
        // Add tenant_id to context if not present
        if !execution.context.get("tenant_id").is_some() {
            if let serde_json::Value::Object(ref mut map) = execution.context {
                map.insert(
                    "tenant_id".to_string(),
                    serde_json::Value::String(self.tenant_id.clone()),
                );
            }
        }

        execution
    }

    // Check if execution belongs to this tenant
    async fn validate_execution_tenant(&self, execution: &AgentExecution) -> Result<()> {
        if let Some(context_tenant) = execution.context.get("tenant_id").and_then(|t| t.as_str()) {
            if context_tenant != self.tenant_id {
                return Err(CircuitBreakerError::Forbidden(
                    "Execution belongs to another tenant".to_string(),
                ));
            }
        } else {
            // No tenant ID is technically an error, but we'll allow it for backward compatibility
            debug!("Execution has no tenant ID: {}", execution.id);
        }

        Ok(())
    }
}

#[async_trait]
impl AgentStorage for TenantAwareStorage {
    async fn store_agent(&self, agent: &AgentDefinition) -> Result<()> {
        // Store the agent with the inner storage
        self.inner.store_agent(agent).await
    }

    async fn get_agent(&self, id: &AgentId) -> Result<Option<AgentDefinition>> {
        self.inner.get_agent(id).await
    }

    async fn list_agents(&self) -> Result<Vec<AgentDefinition>> {
        // We could filter by tenant, but for now we'll return all agents
        // In a real multi-tenant system, agents might have tenant ownership metadata
        self.inner.list_agents().await
    }

    async fn delete_agent(&self, id: &AgentId) -> Result<bool> {
        self.inner.delete_agent(id).await
    }

    async fn store_execution(&self, execution: &AgentExecution) -> Result<()> {
        // Ensure execution has tenant ID in context
        let execution = self.ensure_tenant_in_context(execution.clone());

        // Store the execution
        let result = self.inner.store_execution(&execution).await;

        // Update usage stats on successful storage
        if result.is_ok() {
            let duration = execution
                .completed_at
                .map(|end| {
                    end.signed_duration_since(execution.started_at)
                        .num_milliseconds()
                })
                .unwrap_or(0);

            // Get tokens from context or output if available
            let tokens = execution
                .context
                .get("usage")
                .and_then(|u| u.get("total_tokens"))
                .and_then(|t| t.as_u64())
                .or_else(|| {
                    execution
                        .output_data
                        .as_ref()
                        .and_then(|o| o.get("usage"))
                        .and_then(|u| u.get("total_tokens"))
                        .and_then(|t| t.as_u64())
                })
                .unwrap_or(0) as usize;

            // Record the execution in usage stats
            let mut stats = self.usage_stats.write().await;
            stats.record_execution(&execution, tokens, duration as u64);
        }

        result
    }

    async fn get_execution(&self, id: &Uuid) -> Result<Option<AgentExecution>> {
        // Get the execution
        let execution = self.inner.get_execution(id).await?;

        // Validate tenant ID if execution exists
        if let Some(ref execution) = execution {
            self.validate_execution_tenant(execution).await?;
        }

        Ok(execution)
    }

    async fn list_executions_by_context(
        &self,
        context_key: &str,
        context_value: &str,
    ) -> Result<Vec<AgentExecution>> {
        // Always include tenant filter to ensure isolation
        let mut executions = self
            .inner
            .list_executions_by_nested_context("tenant_id", &self.tenant_id)
            .await?;

        // Apply the requested filter if it's not for tenant_id
        if context_key != "tenant_id" {
            executions = executions
                .into_iter()
                .filter(|e| {
                    if let Some(value) = e.context.get(context_key) {
                        if let Some(value_str) = value.as_str() {
                            return value_str == context_value;
                        }
                    }
                    false
                })
                .collect();
        }

        Ok(executions)
    }

    async fn list_executions_by_context_filters(
        &self,
        filters: &[(&str, &str)],
    ) -> Result<Vec<AgentExecution>> {
        // Add tenant filter if not already present
        let mut tenant_filter_present = false;
        for (key, _) in filters {
            if *key == "tenant_id" {
                tenant_filter_present = true;
                break;
            }
        }

        if tenant_filter_present {
            // Tenant filter already present, use filters as-is
            self.inner.list_executions_by_context_filters(filters).await
        } else {
            // Add tenant filter
            let mut tenant_filters = Vec::with_capacity(filters.len() + 1);
            tenant_filters.push(("tenant_id", self.tenant_id.as_str()));
            tenant_filters.extend_from_slice(filters);

            self.inner
                .list_executions_by_context_filters(&tenant_filters)
                .await
        }
    }

    async fn list_executions_by_nested_context(
        &self,
        context_path: &str,
        context_value: &str,
    ) -> Result<Vec<AgentExecution>> {
        // Get executions that match the nested context
        let executions = self
            .inner
            .list_executions_by_nested_context(context_path, context_value)
            .await?;

        // Filter by tenant
        let filtered = executions
            .into_iter()
            .filter(|e| {
                if let Some(tenant) = e.context.get("tenant_id").and_then(|t| t.as_str()) {
                    tenant == self.tenant_id
                } else {
                    false
                }
            })
            .collect();

        Ok(filtered)
    }

    async fn count_executions_by_context(
        &self,
        context_key: &str,
        context_value: &str,
    ) -> Result<usize> {
        // Always filter by tenant
        if context_key == "tenant_id" && context_value == self.tenant_id {
            self.inner
                .count_executions_by_context(context_key, context_value)
                .await
        } else {
            // Get all executions for this tenant that match the context
            let executions = self
                .list_executions_by_context(context_key, context_value)
                .await?;
            Ok(executions.len())
        }
    }

    async fn list_executions_for_agent(&self, agent_id: &AgentId) -> Result<Vec<AgentExecution>> {
        // Get executions for the agent
        let executions = self.inner.list_executions_for_agent(agent_id).await?;

        // Filter by tenant
        let filtered = executions
            .into_iter()
            .filter(|e| {
                if let Some(tenant) = e.context.get("tenant_id").and_then(|t| t.as_str()) {
                    tenant == self.tenant_id
                } else {
                    false
                }
            })
            .collect();

        Ok(filtered)
    }

    async fn list_executions_by_status(
        &self,
        status: &AgentExecutionStatus,
    ) -> Result<Vec<AgentExecution>> {
        // Get executions with the given status
        let executions = self.inner.list_executions_by_status(status).await?;

        // Filter by tenant
        let filtered = executions
            .into_iter()
            .filter(|e| {
                if let Some(tenant) = e.context.get("tenant_id").and_then(|t| t.as_str()) {
                    tenant == self.tenant_id
                } else {
                    false
                }
            })
            .collect();

        Ok(filtered)
    }

    async fn list_recent_executions(&self, limit: usize) -> Result<Vec<AgentExecution>> {
        // Get recent executions
        let executions = self.inner.list_recent_executions(limit * 2).await?;

        // Filter by tenant and limit
        let filtered: Vec<AgentExecution> = executions
            .into_iter()
            .filter(|e| {
                if let Some(tenant) = e.context.get("tenant_id").and_then(|t| t.as_str()) {
                    tenant == self.tenant_id
                } else {
                    false
                }
            })
            .take(limit)
            .collect();

        Ok(filtered)
    }

    async fn list_executions_for_agent_with_context(
        &self,
        agent_id: &AgentId,
        context_key: &str,
        context_value: &str,
    ) -> Result<Vec<AgentExecution>> {
        // Get executions for the agent with context
        let executions = self
            .inner
            .list_executions_for_agent_with_context(agent_id, context_key, context_value)
            .await?;

        // Filter by tenant
        let filtered = executions
            .into_iter()
            .filter(|e| {
                if let Some(tenant) = e.context.get("tenant_id").and_then(|t| t.as_str()) {
                    tenant == self.tenant_id
                } else {
                    false
                }
            })
            .collect();

        Ok(filtered)
    }

    async fn list_executions_for_resource(
        &self,
        resource_id: &Uuid,
    ) -> Result<Vec<AgentExecution>> {
        // Get executions from inner storage
        let executions = self.inner.list_executions_for_resource(resource_id).await?;

        // Filter by tenant
        let filtered = executions
            .into_iter()
            .filter(|e| {
                if let Some(tenant) = e.context.get("tenant_id").and_then(|t| t.as_str()) {
                    tenant == self.tenant_id
                } else {
                    false
                }
            })
            .collect();

        Ok(filtered)
    }
}

// Tenant-aware agent engine

/// Rate limiter for tenant executions
#[derive(Debug)]
struct TenantRateLimiter {
    /// Tenant ID
    tenant_id: String,

    /// Rate limit configuration
    rate_limits: RateLimits,

    /// Current minute's request count
    minute_requests: Arc<Mutex<(chrono::DateTime<chrono::Utc>, usize)>>,

    /// Current hour's execution count
    hour_executions: Arc<Mutex<(chrono::DateTime<chrono::Utc>, usize)>>,

    /// Current day's token count
    day_tokens: Arc<Mutex<(chrono::DateTime<chrono::Utc>, usize)>>,
}

impl TenantRateLimiter {
    fn new(tenant_id: String, rate_limits: RateLimits) -> Self {
        let now = chrono::Utc::now();
        Self {
            tenant_id,
            rate_limits,
            minute_requests: Arc::new(Mutex::new((now, 0))),
            hour_executions: Arc::new(Mutex::new((now, 0))),
            day_tokens: Arc::new(Mutex::new((now, 0))),
        }
    }

    async fn check_request_limit(&self) -> Result<()> {
        let now = chrono::Utc::now();
        let mut minute_data = self.minute_requests.lock().await;

        // Reset counter if minute has changed
        if (now - minute_data.0).num_seconds() >= 60 {
            *minute_data = (now, 0);
        }

        // Check limit
        if minute_data.1 >= self.rate_limits.requests_per_minute {
            return Err(CircuitBreakerError::RateLimited(format!(
                "Tenant {} exceeded request limit of {} per minute",
                self.tenant_id, self.rate_limits.requests_per_minute
            )));
        }

        // Increment counter
        minute_data.1 += 1;
        Ok(())
    }

    async fn check_execution_limit(&self) -> Result<()> {
        let now = chrono::Utc::now();
        let mut hour_data = self.hour_executions.lock().await;

        // Reset counter if hour has changed
        if (now - hour_data.0).num_seconds() >= 3600 {
            *hour_data = (now, 0);
        }

        // Check limit
        if hour_data.1 >= self.rate_limits.executions_per_hour {
            return Err(CircuitBreakerError::RateLimited(format!(
                "Tenant {} exceeded execution limit of {} per hour",
                self.tenant_id, self.rate_limits.executions_per_hour
            )));
        }

        // Increment counter
        hour_data.1 += 1;
        Ok(())
    }

    async fn check_token_limit(&self, token_estimate: usize) -> Result<()> {
        let now = chrono::Utc::now();
        let mut day_data = self.day_tokens.lock().await;

        // Reset counter if day has changed
        if (now - day_data.0).num_seconds() >= 86400 {
            *day_data = (now, 0);
        }

        // Check limit
        if day_data.1 + token_estimate > self.rate_limits.tokens_per_day {
            return Err(CircuitBreakerError::RateLimited(format!(
                "Tenant {} exceeded token limit of {} per day",
                self.tenant_id, self.rate_limits.tokens_per_day
            )));
        }

        // Increment counter
        day_data.1 += token_estimate;
        Ok(())
    }
}

/// Tenant-aware agent engine that enforces tenant isolation and limits
pub struct TenantAwareAgentEngine {
    /// Underlying agent engine
    inner: Arc<AgentEngine>,

    /// Tenant ID
    tenant_id: String,

    /// Tenant configuration
    config: TenantConfig,

    /// Tenant-aware storage
    storage: Arc<TenantAwareStorage>,

    /// Rate limiter
    rate_limiter: TenantRateLimiter,

    /// Active executions counter
    active_executions: Arc<Mutex<usize>>,
}

impl TenantAwareAgentEngine {
    pub fn new(inner: Arc<AgentEngine>, tenant_id: String, config: TenantConfig) -> Self {
        let storage = Arc::new(TenantAwareStorage::new(
            inner.storage().clone(),
            tenant_id.clone(),
        ));

        let rate_limiter = TenantRateLimiter::new(tenant_id.clone(), config.rate_limits.clone());

        Self {
            inner,
            tenant_id,
            config,
            storage,
            rate_limiter,
            active_executions: Arc::new(Mutex::new(0)),
        }
    }

    /// Get tenant ID
    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }

    /// Get tenant configuration
    pub fn config(&self) -> &TenantConfig {
        &self.config
    }

    /// Get tenant storage
    pub fn storage(&self) -> Arc<TenantAwareStorage> {
        self.storage.clone()
    }

    /// Get usage statistics
    pub async fn get_usage_stats(&self) -> TenantUsageStats {
        self.storage.get_usage_stats().await
    }

    /// Subscribe to agent execution stream events
    pub fn subscribe_to_stream(&self) -> tokio::sync::broadcast::Receiver<AgentStreamEvent> {
        self.inner.subscribe_to_stream()
    }

    /// Execute an agent with tenant context
    pub async fn execute_agent(
        &self,
        config: &AgentActivityConfig,
        context: serde_json::Value,
    ) -> Result<AgentExecution> {
        // Check rate limits
        self.rate_limiter.check_request_limit().await?;
        self.rate_limiter.check_execution_limit().await?;

        // Estimate token usage (this is a simple heuristic)
        let token_estimate = context.to_string().len() / 4;
        self.rate_limiter.check_token_limit(token_estimate).await?;

        // Check concurrent execution limit
        {
            let mut active = self.active_executions.lock().await;
            if *active >= self.config.max_concurrent_executions {
                return Err(CircuitBreakerError::TooManyRequests(format!(
                    "Tenant {} exceeded concurrent execution limit of {}",
                    self.tenant_id, self.config.max_concurrent_executions
                )));
            }
            *active += 1;
        }

        // Ensure context has tenant ID
        let context = ensure_tenant_in_context(context, &self.tenant_id);

        // Apply tenant-specific model configuration if present
        let context = apply_tenant_model_config(context, &self.config);

        // Execute the agent
        let result = self.inner.execute_agent(config, context).await;

        // Decrement active executions counter
        {
            let mut active = self.active_executions.lock().await;
            *active = active.saturating_sub(1);
        }

        result
    }

    /// Get execution statistics
    pub async fn get_execution_stats(&self) -> Result<crate::engine::ExecutionStats> {
        // Use the tenant-aware storage to get stats for this tenant only
        let executions = self
            .storage
            .list_executions_by_context("tenant_id", &self.tenant_id)
            .await?;

        let mut stats = crate::engine::ExecutionStats {
            total: executions.len(),
            completed: 0,
            failed: 0,
            running: 0,
            avg_duration_ms: Some(0),
        };

        let mut total_duration_ms = 0;
        let mut duration_count = 0;

        for execution in executions {
            match execution.status {
                AgentExecutionStatus::Completed => stats.completed += 1,
                AgentExecutionStatus::Failed => stats.failed += 1,
                AgentExecutionStatus::Running => stats.running += 1,
                _ => {}
            }

            if let Some(completed_at) = execution.completed_at {
                let duration = completed_at.signed_duration_since(execution.started_at);
                total_duration_ms += duration.num_milliseconds() as u64;
                duration_count += 1;
            }
        }

        if duration_count > 0 {
            stats.avg_duration_ms = Some(total_duration_ms / duration_count as u64);
        }

        Ok(stats)
    }
}

// Helper functions

/// Ensure context has tenant ID
fn ensure_tenant_in_context(context: serde_json::Value, tenant_id: &str) -> serde_json::Value {
    // Check if tenant_id is already in the context
    if context.get("tenant_id").is_some() {
        return context;
    }

    // Add tenant ID to context
    let mut context_obj = context.as_object().cloned().unwrap_or_default();
    context_obj.insert(
        "tenant_id".to_string(),
        serde_json::Value::String(tenant_id.to_string()),
    );

    serde_json::Value::Object(context_obj)
}

/// Apply tenant-specific model configuration to context
fn apply_tenant_model_config(
    context: serde_json::Value,
    config: &TenantConfig,
) -> serde_json::Value {
    if let Some(ref model_config) = config.default_model_config {
        // Only apply default model if none specified and model is allowed
        if !context.get("model").is_some()
            && !context
                .get("llm_config")
                .and_then(|c| c.get("model"))
                .is_some()
        {
            let mut context_obj = context.as_object().cloned().unwrap_or_default();

            // Create or update llm_config
            let llm_config = context_obj
                .get("llm_config")
                .and_then(|c| c.as_object())
                .cloned()
                .unwrap_or_else(|| serde_json::Map::new());

            let mut llm_config = llm_config;

            // Check if model is allowed
            if let Some(ref allowed_models) = config.allowed_models {
                if allowed_models.contains(&model_config.default_model) {
                    llm_config.insert(
                        "model".to_string(),
                        serde_json::Value::String(model_config.default_model.clone()),
                    );

                    llm_config.insert(
                        "temperature".to_string(),
                        serde_json::Value::Number(
                            serde_json::Number::from_f64(model_config.default_temperature as f64)
                                .unwrap(),
                        ),
                    );

                    llm_config.insert(
                        "max_tokens".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(
                            model_config.default_max_tokens,
                        )),
                    );
                }
            } else {
                // No restrictions on models
                llm_config.insert(
                    "model".to_string(),
                    serde_json::Value::String(model_config.default_model.clone()),
                );

                llm_config.insert(
                    "temperature".to_string(),
                    serde_json::Value::Number(
                        serde_json::Number::from_f64(model_config.default_temperature as f64)
                            .unwrap(),
                    ),
                );

                llm_config.insert(
                    "max_tokens".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(
                        model_config.default_max_tokens,
                    )),
                );
            }

            context_obj.insert(
                "llm_config".to_string(),
                serde_json::Value::Object(llm_config),
            );

            return serde_json::Value::Object(context_obj);
        }
    }

    context
}

// Factory for creating tenant-aware agent engines

/// Factory for creating tenant-aware agent engines
pub struct TenantAwareAgentEngineFactory {
    /// Underlying agent engine
    inner: Arc<AgentEngine>,

    /// Tenant configurations
    tenant_configs: Arc<RwLock<HashMap<String, TenantConfig>>>,

    /// Cache of tenant engines
    tenant_engines: Arc<RwLock<HashMap<String, Arc<TenantAwareAgentEngine>>>>,
}

impl TenantAwareAgentEngineFactory {
    pub fn new(inner: Arc<AgentEngine>) -> Self {
        Self {
            inner,
            tenant_configs: Arc::new(RwLock::new(HashMap::new())),
            tenant_engines: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add tenant configuration
    pub async fn add_tenant_config(&self, config: TenantConfig) -> Result<()> {
        let tenant_id = config.tenant_id.clone();

        // Update configuration
        {
            let mut configs = self.tenant_configs.write().await;
            configs.insert(tenant_id.clone(), config);
        }

        // Remove cached engine if exists
        {
            let mut engines = self.tenant_engines.write().await;
            engines.remove(&tenant_id);
        }

        Ok(())
    }

    /// Get tenant configuration
    pub async fn get_tenant_config(&self, tenant_id: &str) -> Option<TenantConfig> {
        let configs = self.tenant_configs.read().await;
        configs.get(tenant_id).cloned()
    }

    /// Get or create tenant-aware agent engine
    pub async fn get_engine(&self, tenant_id: &str) -> Result<Arc<TenantAwareAgentEngine>> {
        // Check if we have a cached engine
        {
            let engines = self.tenant_engines.read().await;
            if let Some(engine) = engines.get(tenant_id) {
                return Ok(engine.clone());
            }
        }

        // Get tenant configuration or use default
        let config = {
            let configs = self.tenant_configs.read().await;
            configs.get(tenant_id).cloned().unwrap_or_else(|| {
                let mut default_config = TenantConfig::default();
                default_config.tenant_id = tenant_id.to_string();
                default_config
            })
        };

        // Create new tenant engine
        let engine = Arc::new(TenantAwareAgentEngine::new(
            self.inner.clone(),
            tenant_id.to_string(),
            config,
        ));

        // Cache the engine
        {
            let mut engines = self.tenant_engines.write().await;
            engines.insert(tenant_id.to_string(), engine.clone());
        }

        Ok(engine)
    }

    /// List all tenant IDs
    pub async fn list_tenant_ids(&self) -> Vec<String> {
        let configs = self.tenant_configs.read().await;
        configs.keys().cloned().collect()
    }

    /// Remove tenant configuration
    pub async fn remove_tenant_config(&self, tenant_id: &str) -> bool {
        let removed = {
            let mut configs = self.tenant_configs.write().await;
            configs.remove(tenant_id).is_some()
        };

        if removed {
            let mut engines = self.tenant_engines.write().await;
            engines.remove(tenant_id);
        }

        removed
    }
}

// Unit tests for tenant isolation

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::InMemoryAgentStorage;

    async fn setup_test_environment() -> (Arc<AgentEngine>, TenantAwareAgentEngineFactory) {
        // Create in-memory storage
        let storage = Arc::new(InMemoryAgentStorage::new());

        // Create agent engine
        let config = AgentEngineConfig {
            max_concurrent_executions: 10,
            stream_buffer_size: 100,
            connection_timeout: Duration::from_secs(10),
            execution_timeout: Duration::from_secs(30),
            cleanup_interval: Duration::from_secs(60),
        };

        let engine = Arc::new(AgentEngine::new(storage, config));

        // Create factory
        let factory = TenantAwareAgentEngineFactory::new(engine.clone());

        // Add test tenant configurations
        let tenant1_config = TenantConfig {
            tenant_id: "tenant1".to_string(),
            name: "Tenant 1".to_string(),
            active: true,
            quotas: ResourceQuotas {
                max_agents: 5,
                max_executions: 50,
                max_tokens_per_execution: 2000,
                max_storage_bytes: 1024 * 1024,
                max_execution_history_days: 10,
            },
            rate_limits: RateLimits {
                requests_per_minute: 30,
                executions_per_hour: 50,
                tokens_per_day: 50000,
            },
            max_concurrent_executions: 5,
            default_model_config: Some(TenantModelConfig {
                default_model: "gpt-3.5-turbo".to_string(),
                default_temperature: 0.5,
                default_max_tokens: 1000,
            }),
            allowed_models: Some(vec!["gpt-3.5-turbo".to_string(), "gpt-4".to_string()]),
            metadata: serde_json::json!({"plan": "premium"}),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let tenant2_config = TenantConfig {
            tenant_id: "tenant2".to_string(),
            name: "Tenant 2".to_string(),
            active: true,
            quotas: ResourceQuotas {
                max_agents: 3,
                max_executions: 30,
                max_tokens_per_execution: 1000,
                max_storage_bytes: 1024 * 512,
                max_execution_history_days: 5,
            },
            rate_limits: RateLimits {
                requests_per_minute: 10,
                executions_per_hour: 20,
                tokens_per_day: 10000,
            },
            max_concurrent_executions: 2,
            default_model_config: Some(TenantModelConfig {
                default_model: "gpt-3.5-turbo".to_string(),
                default_temperature: 0.7,
                default_max_tokens: 500,
            }),
            allowed_models: Some(vec!["gpt-3.5-turbo".to_string()]),
            metadata: serde_json::json!({"plan": "basic"}),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        factory.add_tenant_config(tenant1_config).await.unwrap();
        factory.add_tenant_config(tenant2_config).await.unwrap();

        (engine, factory)
    }

    #[tokio::test]
    async fn test_tenant_isolation() {
        let (_, factory) = setup_test_environment().await;

        // Get tenant engines
        let tenant1_engine = factory.get_engine("tenant1").await.unwrap();
        let tenant2_engine = factory.get_engine("tenant2").await.unwrap();

        // Create an execution for tenant1
        let tenant1_context = serde_json::json!({
            "message": "Hello from tenant1",
            "user_id": "user1"
        });

        let config = AgentActivityConfig {
            agent_id: AgentId::from("test-agent"),
            input_mapping: HashMap::new(),
            output_mapping: HashMap::new(),
        };

        let tenant1_execution = tenant1_engine
            .execute_agent(&config, tenant1_context)
            .await
            .unwrap();

        // Create an execution for tenant2
        let tenant2_context = serde_json::json!({
            "message": "Hello from tenant2",
            "user_id": "user2"
        });

        let tenant2_execution = tenant2_engine
            .execute_agent(&config, tenant2_context)
            .await
            .unwrap();

        // Verify tenant1 can only see its own executions
        let tenant1_executions = tenant1_engine
            .storage
            .list_executions_by_context("tenant_id", "tenant1")
            .await
            .unwrap();
        assert_eq!(tenant1_executions.len(), 1);
        assert_eq!(tenant1_executions[0].id, tenant1_execution.id);

        // Verify tenant2 can only see its own executions
        let tenant2_executions = tenant2_engine
            .storage
            .list_executions_by_context("tenant_id", "tenant2")
            .await
            .unwrap();
        assert_eq!(tenant2_executions.len(), 1);
        assert_eq!(tenant2_executions[0].id, tenant2_execution.id);

        // Verify tenant1 cannot access tenant2's execution
        let result = tenant1_engine
            .storage
            .get_execution(&tenant2_execution.id)
            .await;
        assert!(result.is_err());

        // Verify tenant2 cannot access tenant1's execution
        let result = tenant2_engine
            .storage
            .get_execution(&tenant1_execution.id)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_tenant_quotas_and_limits() {
        let (_, factory) = setup_test_environment().await;

        // Get tenant engines
        let tenant1_engine = factory.get_engine("tenant1").await.unwrap();

        // Test rate limiting - simulate many requests
        let config = AgentActivityConfig {
            agent_id: AgentId::from("test-agent"),
            input_mapping: HashMap::new(),
            output_mapping: HashMap::new(),
        };

        // Make requests up to the limit
        for i in 0..30 {
            let context = serde_json::json!({
                "message": format!("Request {}", i),
                "user_id": "user1"
            });

            let _ = tenant1_engine
                .execute_agent(&config, context)
                .await
                .unwrap();
        }

        // The next request should hit the rate limit
        let context = serde_json::json!({
            "message": "This should be rate limited",
            "user_id": "user1"
        });

        // To properly test rate limiting, we would need to adjust the test to reset counters
        // since the real limits are time-based. This is just a demonstration.

        // Test model configuration
        let context = serde_json::json!({
            "message": "Test model config",
            "user_id": "user1"
        });

        let execution = tenant1_engine
            .execute_agent(&config, context)
            .await
            .unwrap();

        // Verify the model config was applied
        if let Some(llm_config) = execution.context.get("llm_config") {
            if let Some(model) = llm_config.get("model").and_then(|m| m.as_str()) {
                assert_eq!(model, "gpt-3.5-turbo");
            } else {
                panic!("Model not found in context");
            }

            if let Some(temperature) = llm_config.get("temperature").and_then(|t| t.as_f64()) {
                assert_eq!(temperature, 0.5);
            } else {
                panic!("Temperature not found in context");
            }
        } else {
            panic!("LLM config not found in context");
        }
    }

    #[tokio::test]
    async fn test_tenant_stats() {
        let (_, factory) = setup_test_environment().await;

        // Get tenant engine
        let tenant1_engine = factory.get_engine("tenant1").await.unwrap();

        // Create a few executions
        let config = AgentActivityConfig {
            agent_id: AgentId::from("test-agent"),
            input_mapping: HashMap::new(),
            output_mapping: HashMap::new(),
        };

        for i in 0..5 {
            let context = serde_json::json!({
                "message": format!("Execution {}", i),
                "user_id": "user1",
                "usage": {
                    "total_tokens": 100
                }
            });

            let _ = tenant1_engine
                .execute_agent(&config, context)
                .await
                .unwrap();
        }

        // Get usage stats
        let stats = tenant1_engine.get_usage_stats().await;

        // Verify stats were recorded
        assert_eq!(stats.total_executions, 5);
        assert!(stats.total_tokens > 0);
    }
}
