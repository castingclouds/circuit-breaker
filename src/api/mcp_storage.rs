//! MCP Instance Storage
//!
//! This module provides persistent storage for MCP server instances using NATS KV.

use anyhow::{anyhow, Result};
use async_nats::jetstream::kv::Store;
use async_trait::async_trait;
use futures::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use super::mcp_types::{MCPApp, MCPInstallation, MCPServerInstance, RemoteOAuthConfig};
use super::oauth::StoredOAuthToken;

/// Storage trait for MCP instances
#[async_trait]
pub trait MCPStorage: Send + Sync {
    // Server instances
    async fn store_server_instance(&self, instance: &MCPServerInstance) -> Result<()>;
    async fn get_server_instance(&self, instance_id: &str) -> Result<Option<MCPServerInstance>>;
    async fn list_server_instances(&self) -> Result<Vec<MCPServerInstance>>;
    async fn delete_server_instance(&self, instance_id: &str) -> Result<bool>;

    // OAuth configurations for remote instances
    async fn store_oauth_config(&self, instance_id: &str, config: &RemoteOAuthConfig)
        -> Result<()>;
    async fn get_oauth_config(&self, instance_id: &str) -> Result<Option<RemoteOAuthConfig>>;
    async fn delete_oauth_config(&self, instance_id: &str) -> Result<bool>;

    // Apps
    async fn store_app(&self, app: &MCPApp) -> Result<()>;
    async fn get_app(&self, app_id: &str) -> Result<Option<MCPApp>>;
    async fn list_apps(&self) -> Result<Vec<MCPApp>>;
    async fn delete_app(&self, app_id: &str) -> Result<bool>;

    // Installations
    async fn store_installation(&self, installation: &MCPInstallation) -> Result<()>;
    async fn get_installation(&self, installation_id: &str) -> Result<Option<MCPInstallation>>;
    async fn list_installations(&self) -> Result<Vec<MCPInstallation>>;
    async fn delete_installation(&self, installation_id: &str) -> Result<bool>;

    // OAuth tokens for persistent storage
    async fn store_oauth_token(&self, token_key: &str, token: &StoredOAuthToken) -> Result<()>;
    async fn get_oauth_token(&self, token_key: &str) -> Result<Option<StoredOAuthToken>>;
    async fn list_oauth_tokens(&self) -> Result<Vec<(String, StoredOAuthToken)>>;
    async fn delete_oauth_token(&self, token_key: &str) -> Result<bool>;
}

/// In-memory implementation of MCPStorage for development/testing
#[derive(Debug)]
pub struct InMemoryMCPStorage {
    instances: RwLock<HashMap<String, MCPServerInstance>>,
    oauth_configs: RwLock<HashMap<String, RemoteOAuthConfig>>,
    apps: RwLock<HashMap<String, MCPApp>>,
    installations: RwLock<HashMap<String, MCPInstallation>>,
    oauth_tokens: RwLock<HashMap<String, StoredOAuthToken>>,
}

impl Default for InMemoryMCPStorage {
    fn default() -> Self {
        Self {
            instances: RwLock::new(HashMap::new()),
            oauth_configs: RwLock::new(HashMap::new()),
            apps: RwLock::new(HashMap::new()),
            installations: RwLock::new(HashMap::new()),
            oauth_tokens: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl MCPStorage for InMemoryMCPStorage {
    async fn store_server_instance(&self, instance: &MCPServerInstance) -> Result<()> {
        let mut instances = self.instances.write().await;
        instances.insert(instance.instance_id.clone(), instance.clone());
        debug!("Stored MCP instance in memory: {}", instance.instance_id);
        Ok(())
    }

    async fn get_server_instance(&self, instance_id: &str) -> Result<Option<MCPServerInstance>> {
        let instances = self.instances.read().await;
        Ok(instances.get(instance_id).cloned())
    }

    async fn list_server_instances(&self) -> Result<Vec<MCPServerInstance>> {
        let instances = self.instances.read().await;
        Ok(instances.values().cloned().collect())
    }

    async fn delete_server_instance(&self, instance_id: &str) -> Result<bool> {
        let mut instances = self.instances.write().await;
        Ok(instances.remove(instance_id).is_some())
    }

    async fn store_oauth_config(
        &self,
        instance_id: &str,
        config: &RemoteOAuthConfig,
    ) -> Result<()> {
        let mut configs = self.oauth_configs.write().await;
        configs.insert(instance_id.to_string(), config.clone());
        debug!(
            "Stored OAuth config in memory for instance: {}",
            instance_id
        );
        Ok(())
    }

    async fn get_oauth_config(&self, instance_id: &str) -> Result<Option<RemoteOAuthConfig>> {
        let configs = self.oauth_configs.read().await;
        Ok(configs.get(instance_id).cloned())
    }

    async fn delete_oauth_config(&self, instance_id: &str) -> Result<bool> {
        let mut configs = self.oauth_configs.write().await;
        Ok(configs.remove(instance_id).is_some())
    }

    async fn store_app(&self, app: &MCPApp) -> Result<()> {
        let mut apps = self.apps.write().await;
        apps.insert(app.app_id.clone(), app.clone());
        debug!("Stored MCP app in memory: {}", app.app_id);
        Ok(())
    }

    async fn get_app(&self, app_id: &str) -> Result<Option<MCPApp>> {
        let apps = self.apps.read().await;
        Ok(apps.get(app_id).cloned())
    }

    async fn list_apps(&self) -> Result<Vec<MCPApp>> {
        let apps = self.apps.read().await;
        Ok(apps.values().cloned().collect())
    }

    async fn delete_app(&self, app_id: &str) -> Result<bool> {
        let mut apps = self.apps.write().await;
        Ok(apps.remove(app_id).is_some())
    }

    async fn store_installation(&self, installation: &MCPInstallation) -> Result<()> {
        let mut installations = self.installations.write().await;
        installations.insert(installation.installation_id.clone(), installation.clone());
        debug!(
            "Stored MCP installation in memory: {}",
            installation.installation_id
        );
        Ok(())
    }

    async fn get_installation(&self, installation_id: &str) -> Result<Option<MCPInstallation>> {
        let installations = self.installations.read().await;
        Ok(installations.get(installation_id).cloned())
    }

    async fn list_installations(&self) -> Result<Vec<MCPInstallation>> {
        let installations = self.installations.read().await;
        Ok(installations.values().cloned().collect())
    }

    async fn delete_installation(&self, installation_id: &str) -> Result<bool> {
        let mut installations = self.installations.write().await;
        Ok(installations.remove(installation_id).is_some())
    }

    async fn store_oauth_token(&self, token_key: &str, token: &StoredOAuthToken) -> Result<()> {
        let mut tokens = self.oauth_tokens.write().await;
        tokens.insert(token_key.to_string(), token.clone());
        debug!("Stored OAuth token for key: {}", token_key);
        Ok(())
    }

    async fn get_oauth_token(&self, token_key: &str) -> Result<Option<StoredOAuthToken>> {
        let tokens = self.oauth_tokens.read().await;
        let token = tokens.get(token_key).cloned();
        debug!(
            "Retrieved OAuth token for key: {} - found: {}",
            token_key,
            token.is_some()
        );
        Ok(token)
    }

    async fn list_oauth_tokens(&self) -> Result<Vec<(String, StoredOAuthToken)>> {
        let tokens = self.oauth_tokens.read().await;
        let token_list = tokens.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        debug!("Listed {} OAuth tokens", tokens.len());
        Ok(token_list)
    }

    async fn delete_oauth_token(&self, token_key: &str) -> Result<bool> {
        let mut tokens = self.oauth_tokens.write().await;
        let removed = tokens.remove(token_key).is_some();
        debug!(
            "Deleted OAuth token for key: {} - success: {}",
            token_key, removed
        );
        Ok(removed)
    }
}

/// NATS KV-based implementation of MCPStorage
#[derive(Debug, Clone)]
pub struct NATSMCPStorage {
    client: async_nats::Client,
    jetstream: async_nats::jetstream::Context,
    instances_store: Arc<RwLock<Option<Store>>>,
    oauth_configs_store: Arc<RwLock<Option<Store>>>,
    apps_store: Arc<RwLock<Option<Store>>>,
    installations_store: Arc<RwLock<Option<Store>>>,
    oauth_tokens_store: Arc<RwLock<Option<Store>>>,
}

impl NATSMCPStorage {
    /// Create a new NATS MCP storage instance
    pub async fn new(nats_url: &str) -> Result<Self> {
        let client = async_nats::connect(nats_url)
            .await
            .map_err(|e| anyhow!("Failed to connect to NATS: {}", e))?;

        let jetstream = async_nats::jetstream::new(client.clone());

        let storage = Self {
            client,
            jetstream,
            instances_store: Arc::new(RwLock::new(None)),
            oauth_configs_store: Arc::new(RwLock::new(None)),
            apps_store: Arc::new(RwLock::new(None)),
            installations_store: Arc::new(RwLock::new(None)),
            oauth_tokens_store: Arc::new(RwLock::new(None)),
        };

        // Initialize KV stores
        storage.ensure_kv_stores().await?;

        info!("NATS MCP storage initialized successfully");
        Ok(storage)
    }

    /// Ensure all required KV stores exist
    async fn ensure_kv_stores(&self) -> Result<()> {
        // MCP Server Instances
        let instances_store = self
            .jetstream
            .create_key_value(async_nats::jetstream::kv::Config {
                bucket: "mcp_instances".to_string(),
                description: "MCP Server Instances".to_string(),
                ..Default::default()
            })
            .await
            .map_err(|e| anyhow!("Failed to create mcp_instances KV store: {}", e))?;

        *self.instances_store.write().await = Some(instances_store);

        // OAuth Configurations
        let oauth_configs_store = self
            .jetstream
            .create_key_value(async_nats::jetstream::kv::Config {
                bucket: "mcp_oauth_configs".to_string(),
                description: "MCP OAuth Configurations".to_string(),
                ..Default::default()
            })
            .await
            .map_err(|e| anyhow!("Failed to create mcp_oauth_configs KV store: {}", e))?;

        *self.oauth_configs_store.write().await = Some(oauth_configs_store);

        // MCP Apps
        let apps_store = self
            .jetstream
            .create_key_value(async_nats::jetstream::kv::Config {
                bucket: "mcp_apps".to_string(),
                description: "MCP Applications".to_string(),
                ..Default::default()
            })
            .await
            .map_err(|e| anyhow!("Failed to create mcp_apps KV store: {}", e))?;

        *self.apps_store.write().await = Some(apps_store);

        // MCP Installations
        let installations_store = self
            .jetstream
            .create_key_value(async_nats::jetstream::kv::Config {
                bucket: "mcp_installations".to_string(),
                description: "MCP Installations".to_string(),
                ..Default::default()
            })
            .await
            .map_err(|e| anyhow!("Failed to create mcp_installations KV store: {}", e))?;

        *self.installations_store.write().await = Some(installations_store);

        // OAuth Tokens
        let oauth_tokens_store = self
            .jetstream
            .create_key_value(async_nats::jetstream::kv::Config {
                bucket: "mcp_oauth_tokens".to_string(),
                description: "MCP OAuth Tokens".to_string(),
                ..Default::default()
            })
            .await
            .map_err(|e| anyhow!("Failed to create mcp_oauth_tokens KV store: {}", e))?;

        *self.oauth_tokens_store.write().await = Some(oauth_tokens_store);

        info!("All NATS KV stores for MCP storage initialized");
        Ok(())
    }

    /// Get the instances KV store
    async fn get_instances_store(&self) -> Result<Store> {
        let store_lock = self.instances_store.read().await;
        store_lock
            .as_ref()
            .ok_or_else(|| anyhow!("MCP instances KV store not initialized"))
            .cloned()
    }

    /// Get the OAuth configs KV store
    async fn get_oauth_configs_store(&self) -> Result<Store> {
        let store_lock = self.oauth_configs_store.read().await;
        store_lock
            .as_ref()
            .ok_or_else(|| anyhow!("MCP OAuth configs KV store not initialized"))
            .cloned()
    }

    /// Get the apps KV store
    async fn get_apps_store(&self) -> Result<Store> {
        let store_lock = self.apps_store.read().await;
        store_lock
            .as_ref()
            .ok_or_else(|| anyhow!("MCP apps KV store not initialized"))
            .cloned()
    }

    /// Get the installations KV store
    async fn get_installations_store(&self) -> Result<Store> {
        let store_lock = self.installations_store.read().await;
        store_lock
            .as_ref()
            .ok_or_else(|| anyhow!("MCP installations KV store not initialized"))
            .cloned()
    }

    /// Get the OAuth tokens KV store
    async fn get_oauth_tokens_store(&self) -> Result<Store> {
        let store_lock = self.oauth_tokens_store.read().await;
        store_lock
            .as_ref()
            .ok_or_else(|| anyhow!("MCP OAuth tokens KV store not initialized"))
            .cloned()
    }
}

#[async_trait]
impl MCPStorage for NATSMCPStorage {
    async fn store_server_instance(&self, instance: &MCPServerInstance) -> Result<()> {
        let store = self.get_instances_store().await?;
        let data = serde_json::to_vec(instance)
            .map_err(|e| anyhow!("Failed to serialize MCP instance: {}", e))?;

        store
            .put(&instance.instance_id, data.into())
            .await
            .map_err(|e| anyhow!("Failed to store MCP instance in NATS KV: {}", e))?;

        info!("Stored MCP instance in NATS KV: {}", instance.instance_id);
        Ok(())
    }

    async fn get_server_instance(&self, instance_id: &str) -> Result<Option<MCPServerInstance>> {
        let store = self.get_instances_store().await?;

        match store.get(instance_id).await {
            Ok(Some(entry)) => {
                let instance: MCPServerInstance = serde_json::from_slice(entry.as_ref())
                    .map_err(|e| anyhow!("Failed to deserialize MCP instance: {}", e))?;
                debug!("Retrieved MCP instance from NATS KV: {}", instance_id);
                Ok(Some(instance))
            }
            Ok(None) => {
                debug!("MCP instance not found in NATS KV: {}", instance_id);
                Ok(None)
            }
            Err(e) => {
                error!("Failed to get MCP instance from NATS KV: {}", e);
                Err(anyhow!("Failed to get MCP instance from NATS KV: {}", e))
            }
        }
    }

    async fn list_server_instances(&self) -> Result<Vec<MCPServerInstance>> {
        let store = self.get_instances_store().await?;
        let mut instances = Vec::new();

        let mut keys = store
            .keys()
            .await
            .map_err(|e| anyhow!("Failed to list MCP instance keys from NATS KV: {}", e))?;

        while let Some(key) = keys.next().await {
            let key = key.map_err(|e| anyhow!("Failed to get MCP instance key: {}", e))?;
            if let Some(entry) = store
                .get(&key)
                .await
                .map_err(|e| anyhow!("Failed to get MCP instance from NATS KV: {}", e))?
            {
                match serde_json::from_slice::<MCPServerInstance>(entry.as_ref()) {
                    Ok(instance) => instances.push(instance),
                    Err(e) => warn!("Failed to deserialize MCP instance {}: {}", key, e),
                }
            }
        }

        debug!("Listed {} MCP instances from NATS KV", instances.len());
        Ok(instances)
    }

    async fn delete_server_instance(&self, instance_id: &str) -> Result<bool> {
        let store = self.get_instances_store().await?;

        match store.delete(instance_id).await {
            Ok(_) => {
                info!("Deleted MCP instance from NATS KV: {}", instance_id);
                Ok(true)
            }
            Err(e) => {
                error!("Failed to delete MCP instance from NATS KV: {}", e);
                Ok(false)
            }
        }
    }

    async fn store_oauth_config(
        &self,
        instance_id: &str,
        config: &RemoteOAuthConfig,
    ) -> Result<()> {
        let store = self.get_oauth_configs_store().await?;
        let data = serde_json::to_vec(config)
            .map_err(|e| anyhow!("Failed to serialize OAuth config: {}", e))?;

        store
            .put(instance_id, data.into())
            .await
            .map_err(|e| anyhow!("Failed to store OAuth config in NATS KV: {}", e))?;

        info!(
            "Stored OAuth config in NATS KV for instance: {}",
            instance_id
        );
        Ok(())
    }

    async fn get_oauth_config(&self, instance_id: &str) -> Result<Option<RemoteOAuthConfig>> {
        let store = self.get_oauth_configs_store().await?;

        match store.get(instance_id).await {
            Ok(Some(entry)) => {
                let config: RemoteOAuthConfig = serde_json::from_slice(entry.as_ref())
                    .map_err(|e| anyhow!("Failed to deserialize OAuth config: {}", e))?;
                debug!(
                    "Retrieved OAuth config from NATS KV for instance: {}",
                    instance_id
                );
                Ok(Some(config))
            }
            Ok(None) => {
                debug!(
                    "OAuth config not found in NATS KV for instance: {}",
                    instance_id
                );
                Ok(None)
            }
            Err(e) => {
                error!("Failed to get OAuth config from NATS KV: {}", e);
                Err(anyhow!("Failed to get OAuth config from NATS KV: {}", e))
            }
        }
    }

    async fn delete_oauth_config(&self, instance_id: &str) -> Result<bool> {
        let store = self.get_oauth_configs_store().await?;

        match store.delete(instance_id).await {
            Ok(_) => {
                info!(
                    "Deleted OAuth config from NATS KV for instance: {}",
                    instance_id
                );
                Ok(true)
            }
            Err(e) => {
                error!("Failed to delete OAuth config from NATS KV: {}", e);
                Ok(false)
            }
        }
    }

    async fn store_app(&self, app: &MCPApp) -> Result<()> {
        let store = self.get_apps_store().await?;
        let data =
            serde_json::to_vec(app).map_err(|e| anyhow!("Failed to serialize MCP app: {}", e))?;

        store
            .put(&app.app_id, data.into())
            .await
            .map_err(|e| anyhow!("Failed to store MCP app in NATS KV: {}", e))?;

        info!("Stored MCP app in NATS KV: {}", app.app_id);
        Ok(())
    }

    async fn get_app(&self, app_id: &str) -> Result<Option<MCPApp>> {
        let store = self.get_apps_store().await?;

        match store.get(app_id).await {
            Ok(Some(entry)) => {
                let app: MCPApp = serde_json::from_slice(entry.as_ref())
                    .map_err(|e| anyhow!("Failed to deserialize MCP app: {}", e))?;
                debug!("Retrieved MCP app from NATS KV: {}", app_id);
                Ok(Some(app))
            }
            Ok(None) => {
                debug!("MCP app not found in NATS KV: {}", app_id);
                Ok(None)
            }
            Err(e) => {
                error!("Failed to get MCP app from NATS KV: {}", e);
                Err(anyhow!("Failed to get MCP app from NATS KV: {}", e))
            }
        }
    }

    async fn list_apps(&self) -> Result<Vec<MCPApp>> {
        let store = self.get_apps_store().await?;
        let mut apps = Vec::new();

        let mut keys = store
            .keys()
            .await
            .map_err(|e| anyhow!("Failed to list MCP app keys from NATS KV: {}", e))?;

        while let Some(key) = keys.next().await {
            let key = key.map_err(|e| anyhow!("Failed to get MCP app key: {}", e))?;
            if let Some(entry) = store
                .get(&key)
                .await
                .map_err(|e| anyhow!("Failed to get MCP app from NATS KV: {}", e))?
            {
                match serde_json::from_slice::<MCPApp>(entry.as_ref()) {
                    Ok(app) => apps.push(app),
                    Err(e) => warn!("Failed to deserialize MCP app {}: {}", key, e),
                }
            }
        }

        debug!("Listed {} MCP apps from NATS KV", apps.len());
        Ok(apps)
    }

    async fn delete_app(&self, app_id: &str) -> Result<bool> {
        let store = self.get_apps_store().await?;

        match store.delete(app_id).await {
            Ok(_) => {
                info!("Deleted MCP app from NATS KV: {}", app_id);
                Ok(true)
            }
            Err(e) => {
                error!("Failed to delete MCP app from NATS KV: {}", e);
                Ok(false)
            }
        }
    }

    async fn store_installation(&self, installation: &MCPInstallation) -> Result<()> {
        let store = self.get_installations_store().await?;
        let data = serde_json::to_vec(installation)
            .map_err(|e| anyhow!("Failed to serialize MCP installation: {}", e))?;

        store
            .put(&installation.installation_id, data.into())
            .await
            .map_err(|e| anyhow!("Failed to store MCP installation in NATS KV: {}", e))?;

        info!(
            "Stored MCP installation in NATS KV: {}",
            installation.installation_id
        );
        Ok(())
    }

    async fn get_installation(&self, installation_id: &str) -> Result<Option<MCPInstallation>> {
        let store = self.get_installations_store().await?;

        match store.get(installation_id).await {
            Ok(Some(entry)) => {
                let installation: MCPInstallation = serde_json::from_slice(entry.as_ref())
                    .map_err(|e| anyhow!("Failed to deserialize MCP installation: {}", e))?;
                debug!(
                    "Retrieved MCP installation from NATS KV: {}",
                    installation_id
                );
                Ok(Some(installation))
            }
            Ok(None) => {
                debug!("MCP installation not found in NATS KV: {}", installation_id);
                Ok(None)
            }
            Err(e) => {
                error!("Failed to get MCP installation from NATS KV: {}", e);
                Err(anyhow!(
                    "Failed to get MCP installation from NATS KV: {}",
                    e
                ))
            }
        }
    }

    async fn list_installations(&self) -> Result<Vec<MCPInstallation>> {
        let store = self.get_installations_store().await?;
        let mut installations = Vec::new();

        let mut keys = store
            .keys()
            .await
            .map_err(|e| anyhow!("Failed to list MCP installation keys from NATS KV: {}", e))?;

        while let Some(key) = keys.next().await {
            let key = key.map_err(|e| anyhow!("Failed to get MCP installation key: {}", e))?;
            if let Some(entry) = store
                .get(&key)
                .await
                .map_err(|e| anyhow!("Failed to get MCP installation from NATS KV: {}", e))?
            {
                match serde_json::from_slice::<MCPInstallation>(entry.as_ref()) {
                    Ok(installation) => installations.push(installation),
                    Err(e) => warn!("Failed to deserialize MCP installation {}: {}", key, e),
                }
            }
        }

        debug!(
            "Listed {} MCP installations from NATS KV",
            installations.len()
        );
        Ok(installations)
    }

    async fn delete_installation(&self, installation_id: &str) -> Result<bool> {
        let store = self.get_installations_store().await?;

        match store.delete(installation_id).await {
            Ok(_) => {
                info!("Deleted MCP installation from NATS KV: {}", installation_id);
                Ok(true)
            }
            Err(e) => {
                error!("Failed to delete MCP installation from NATS KV: {}", e);
                Ok(false)
            }
        }
    }

    async fn store_oauth_token(&self, token_key: &str, token: &StoredOAuthToken) -> Result<()> {
        let store = self.get_oauth_tokens_store().await?;
        let data = serde_json::to_vec(token)
            .map_err(|e| anyhow!("Failed to serialize OAuth token: {}", e))?;

        store
            .put(token_key, data.into())
            .await
            .map_err(|e| anyhow!("Failed to store OAuth token in NATS KV: {}", e))?;

        info!("Stored OAuth token in NATS KV: {}", token_key);
        Ok(())
    }

    async fn get_oauth_token(&self, token_key: &str) -> Result<Option<StoredOAuthToken>> {
        let store = self.get_oauth_tokens_store().await?;

        match store.get(token_key).await {
            Ok(Some(entry)) => {
                let token: StoredOAuthToken = serde_json::from_slice(entry.as_ref())
                    .map_err(|e| anyhow!("Failed to deserialize OAuth token: {}", e))?;
                debug!("Retrieved OAuth token from NATS KV: {}", token_key);
                Ok(Some(token))
            }
            Ok(None) => {
                debug!("OAuth token not found in NATS KV: {}", token_key);
                Ok(None)
            }
            Err(e) => {
                error!("Failed to get OAuth token from NATS KV: {}", e);
                Err(anyhow!("Failed to get OAuth token from NATS KV: {}", e))
            }
        }
    }

    async fn list_oauth_tokens(&self) -> Result<Vec<(String, StoredOAuthToken)>> {
        let store = self.get_oauth_tokens_store().await?;
        let mut tokens = Vec::new();

        let mut keys = store
            .keys()
            .await
            .map_err(|e| anyhow!("Failed to list OAuth token keys from NATS KV: {}", e))?;

        while let Some(key) = keys.next().await {
            match key {
                Ok(key_str) => {
                    if let Ok(Some(token)) = self.get_oauth_token(&key_str).await {
                        tokens.push((key_str, token));
                    }
                }
                Err(e) => {
                    warn!("Failed to process OAuth token key: {}", e);
                }
            }
        }

        debug!("Listed {} OAuth tokens from NATS KV", tokens.len());
        Ok(tokens)
    }

    async fn delete_oauth_token(&self, token_key: &str) -> Result<bool> {
        let store = self.get_oauth_tokens_store().await?;

        match store.delete(token_key).await {
            Ok(_) => {
                info!("Deleted OAuth token from NATS KV: {}", token_key);
                Ok(true)
            }
            Err(e) => {
                error!("Failed to delete OAuth token from NATS KV: {}", e);
                Ok(false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::mcp_types::{MCPApplicationType, MCPCapabilities, MCPServerStatus};

    fn create_test_instance() -> MCPServerInstance {
        MCPServerInstance {
            instance_id: "test-instance-123".to_string(),
            app_id: "test-app".to_string(),
            installation_id: "test-installation".to_string(),
            name: "Test Instance".to_string(),
            description: "A test MCP instance".to_string(),
            capabilities: MCPCapabilities {
                tools: true,
                prompts: true,
                resources: true,
                logging: true,
            },
            project_contexts: vec!["project1".to_string(), "project2".to_string()],
            app_type: MCPApplicationType::Local,
            created_at: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
            status: MCPServerStatus::Active,
        }
    }

    #[tokio::test]
    async fn test_in_memory_storage() {
        let storage = InMemoryMCPStorage::default();
        let instance = create_test_instance();

        // Store instance
        storage.store_server_instance(&instance).await.unwrap();

        // Get instance
        let retrieved = storage
            .get_server_instance(&instance.instance_id)
            .await
            .unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().instance_id, instance.instance_id);

        // List instances
        let instances = storage.list_server_instances().await.unwrap();
        assert_eq!(instances.len(), 1);

        // Delete instance
        let deleted = storage
            .delete_server_instance(&instance.instance_id)
            .await
            .unwrap();
        assert!(deleted);

        // Verify deletion
        let retrieved = storage
            .get_server_instance(&instance.instance_id)
            .await
            .unwrap();
        assert!(retrieved.is_none());
    }
}
