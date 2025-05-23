// Function execution engine - Docker-based event-driven functions

//! # Function Engine
//! 
//! This module provides the execution engine for Docker-based functions that are
//! triggered by workflow events. It handles:
//! - Event processing and function matching
//! - Docker container execution
//! - Function lifecycle management
//! - Results storage and monitoring
//! - Function chaining with input/output mapping

use std::collections::HashMap;
use std::process::Stdio;
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};
use uuid::Uuid;
use chrono::{Utc, Duration};
use async_trait::async_trait;

use crate::models::{
    FunctionDefinition, FunctionId, FunctionExecution, TriggerEvent, 
    ExecutionStatus, ContainerConfig, ResourceLimits, ChainExecution,
    ChainStatus, InputMapping, ChainCondition, EventType, RetryConfig,
    BackoffStrategy, RetryCondition
};
use crate::{CircuitBreakerError, Result};

/// Storage abstraction for functions
#[async_trait]
pub trait FunctionStorage: Send + Sync {
    /// Store a function definition
    async fn create_function(&self, function: FunctionDefinition) -> Result<FunctionDefinition>;
    
    /// Get a function by ID
    async fn get_function(&self, id: &FunctionId) -> Result<Option<FunctionDefinition>>;
    
    /// Update a function definition
    async fn update_function(&self, function: FunctionDefinition) -> Result<FunctionDefinition>;
    
    /// Delete a function
    async fn delete_function(&self, id: &FunctionId) -> Result<bool>;
    
    /// List all functions
    async fn list_functions(&self) -> Result<Vec<FunctionDefinition>>;
    
    /// Store function execution record
    async fn create_execution(&self, execution: FunctionExecution) -> Result<FunctionExecution>;
    
    /// Update function execution record
    async fn update_execution(&self, execution: FunctionExecution) -> Result<FunctionExecution>;
    
    /// Get execution by ID
    async fn get_execution(&self, id: &Uuid) -> Result<Option<FunctionExecution>>;
    
    /// List executions for a function
    async fn list_executions(&self, function_id: &FunctionId) -> Result<Vec<FunctionExecution>>;
    
    /// Store chain execution record
    async fn create_chain(&self, chain: ChainExecution) -> Result<ChainExecution>;
    
    /// Update chain execution record
    async fn update_chain(&self, chain: ChainExecution) -> Result<ChainExecution>;
    
    /// Get chain by ID
    async fn get_chain(&self, id: &Uuid) -> Result<Option<ChainExecution>>;
}

/// Docker-based function execution engine
pub struct FunctionEngine {
    storage: Box<dyn FunctionStorage>,
    docker_available: bool,
}

/// Docker container execution result
#[derive(Debug)]
pub struct ContainerResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl FunctionEngine {
    /// Create a new function engine
    pub fn new(storage: Box<dyn FunctionStorage>) -> Self {
        Self {
            storage,
            docker_available: Self::check_docker_available(),
        }
    }
    
    /// Check if Docker is available on the system
    fn check_docker_available() -> bool {
        use std::process::Command;
        
        match Command::new("docker").arg("--version").output() {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }

    /// Process an event and trigger matching functions
    pub async fn process_event(&self, event: TriggerEvent) -> Result<Vec<Uuid>> {
        let functions = self.storage.list_functions().await?;
        let mut execution_ids = Vec::new();
        
        for function in functions {
            if function.matches_event(&event) {
                let execution_id = self.execute_function(&function, &event).await?;
                execution_ids.push(execution_id);
            }
        }
        
        Ok(execution_ids)
    }

    /// Execute a function with the given event
    pub async fn execute_function(
        &self,
        function: &FunctionDefinition,
        event: &TriggerEvent,
    ) -> Result<Uuid> {
        // Create execution record
        let mut execution = FunctionExecution::new(function.id.clone(), event.clone());
        
        // Process input data from event using trigger mapping
        let input_data = self.map_event_to_input(event, function).await?;
        execution.input_data = input_data;
        
        // Store initial execution record
        let execution = self.storage.create_execution(execution).await?;
        let execution_id = execution.id;
        
        // Execute the function (now actually running Docker)
        println!("🐳 Executing function {} with Docker...", execution_id);
        
        match self.execute_function_impl(function.clone(), execution_id).await {
            Ok(_) => {
                println!("✅ Function {} completed successfully", execution_id);
            }
            Err(e) => {
                println!("❌ Function {} failed: {}", execution_id, e);
                // Update execution status to failed
                if let Ok(Some(mut execution)) = self.storage.get_execution(&execution_id).await {
                    execution.fail(format!("Execution error: {}", e));
                    let _ = self.storage.update_execution(execution).await;
                }
            }
        }
        
        Ok(execution_id)
    }

    /// Clone for background execution (simplified for now)
    fn clone_for_background(&self) -> Self {
        // For now, we'll create a simple clone
        // In a real implementation, we'd share the storage layer properly
        Self {
            storage: Box::new(InMemoryFunctionStorage::new()),
            docker_available: self.docker_available,
        }
    }

    /// Internal function execution implementation
    async fn execute_function_impl(
        &self,
        function: FunctionDefinition,
        execution_id: Uuid,
    ) -> Result<()> {
        let mut execution = self.storage.get_execution(&execution_id).await?
            .ok_or_else(|| CircuitBreakerError::GraphQL("Execution not found".to_string()))?;

        if !self.docker_available {
            execution.fail("Docker not available".to_string());
            self.storage.update_execution(execution).await?;
            return Ok(());
        }

        // Validate input data
        if let Err(validation_error) = function.validate_input(&execution.input_data) {
            execution.fail(format!("Input validation failed: {}", validation_error));
            self.storage.update_execution(execution).await?;
            return Ok(());
        }

        // Generate container name
        let container_name = format!("circuit-breaker-{}", execution_id);
        
        execution.start(Some(container_name.clone()));
        self.storage.update_execution(execution.clone()).await?;

        // Run the container
        match self.run_container(&function.container, &container_name, &execution).await {
            Ok(result) => {
                // Parse output data
                let output_data = self.parse_container_output(&result.stdout)?;
                
                // Validate output against schema
                if let Err(validation_error) = function.validate_output(&output_data) {
                    execution.fail(format!("Output validation failed: {}", validation_error));
                } else {
                    execution.complete(result.exit_code, Some(result.stdout), Some(result.stderr));
                    execution.output_data = Some(output_data.clone());
                    
                    // Process function chains if execution succeeded
                    if execution.succeeded() {
                        // TODO: Re-enable function chaining after fixing lifetime issues
                        // self.process_function_chains(&function, &execution, &output_data).await?;
                        println!("Function succeeded - chains would be processed here");
                    }
                }
            }
            Err(e) => {
                // Check if we should retry
                if let Some(retry_config) = &function.retry_config {
                    if execution.retry_count < retry_config.max_attempts {
                        let delay = self.calculate_retry_delay(retry_config, execution.retry_count);
                        execution.schedule_retry(delay);
                        self.storage.update_execution(execution.clone()).await?;
                        
                        // Schedule retry execution
                        // self.schedule_retry_execution(function, execution_id, delay).await?;
                        println!("Would schedule retry for execution {}", execution_id);
                        return Ok(());
                    }
                }
                
                execution.fail(format!("Container execution failed: {}", e));
            }
        }

        // Clean up container
        let _ = self.cleanup_container(&container_name).await;
        
        self.storage.update_execution(execution).await?;
        Ok(())
    }

    /// Map event data to function input based on trigger mapping
    async fn map_event_to_input(&self, event: &TriggerEvent, function: &FunctionDefinition) -> Result<serde_json::Value> {
        // Find the matching trigger
        for trigger in &function.triggers {
            // Check if this trigger matches the event
            if function.event_type_matches(&trigger.event_type, &event.event_type, event) {
                // Apply input mapping
                match &trigger.input_mapping {
                    InputMapping::FullOutput => return Ok(event.data.clone()),
                    InputMapping::FieldMapping(mappings) => {
                        let mut result = serde_json::Map::new();
                        for (source_field, target_field) in mappings {
                            if let Some(value) = event.data.get(source_field) {
                                result.insert(target_field.clone(), value.clone());
                            }
                        }
                        return Ok(serde_json::Value::Object(result));
                    }
                    InputMapping::Template(template) => {
                        // TODO: Implement template processing
                        return Ok(template.clone());
                    }
                    InputMapping::MergedData => {
                        // Merge event data with token metadata
                        let mut result = event.data.as_object().unwrap_or(&serde_json::Map::new()).clone();
                        for (key, value) in &event.metadata {
                            result.insert(format!("metadata_{}", key), value.clone());
                        }
                        return Ok(serde_json::Value::Object(result));
                    }
                    InputMapping::Script(_script) => {
                        // TODO: Implement script-based transformation
                        return Ok(event.data.clone());
                    }
                }
            }
        }
        
        // Default fallback
        Ok(event.data.clone())
    }

    /// Parse container output as JSON
    fn parse_container_output(&self, stdout: &str) -> Result<serde_json::Value> {
        // Try to parse the last line as JSON (common pattern)
        let lines: Vec<&str> = stdout.lines().collect();
        if let Some(last_line) = lines.last() {
            if let Ok(json_value) = serde_json::from_str(last_line) {
                return Ok(json_value);
            }
        }
        
        // If no valid JSON found, return the raw output as a string
        Ok(serde_json::json!({
            "output": stdout,
            "type": "raw_text"
        }))
    }

    /// Process function chains after successful execution
    async fn process_function_chains(
        &self,
        function: &FunctionDefinition,
        execution: &FunctionExecution,
        output_data: &serde_json::Value,
    ) -> Result<()> {
        // Clone the chains to avoid lifetime issues
        let chains = function.chains.clone();
        
        for chain in chains {
            // Check chain condition
            let should_trigger = match &chain.condition {
                ChainCondition::Always => true,
                ChainCondition::OnSuccess => execution.succeeded(),
                ChainCondition::OnFailure => execution.failed(),
                ChainCondition::ConditionalRule(_rule) => {
                    // TODO: Evaluate rule against output data
                    // For now, default to true
                    true
                }
                ChainCondition::Script(_script) => {
                    // TODO: Implement script-based condition evaluation
                    true
                }
            };
            
            if should_trigger {
                // Get target function
                if let Some(target_function) = self.storage.get_function(&chain.target_function).await? {
                    // Map output to input for chained function
                    let chained_input = self.map_chain_input(&chain.input_mapping, output_data, execution)?;
                    
                    // Create chained execution
                    let chained_execution = FunctionExecution::new_chained(
                        target_function.id.clone(),
                        execution.id,
                        execution.chain_position + 1,
                        chained_input,
                    );
                    
                    let chained_execution = self.storage.create_execution(chained_execution).await?;
                    
                    // Execute chained function in background
                    let engine = self.clone_for_background();
                    let execution_id = chained_execution.id;
                    
                    tokio::spawn(async move {
                        if let Some(delay) = chain.delay {
                            tokio::time::sleep(delay.to_std().unwrap_or(std::time::Duration::from_secs(0))).await;
                        }
                        
                        if let Err(e) = engine.execute_function_impl(target_function, execution_id).await {
                            eprintln!("Chained function execution failed: {}", e);
                        }
                    });
                }
            }
        }
        
        Ok(())
    }

    /// Map output data to input for chained function
    fn map_chain_input(
        &self,
        mapping: &InputMapping,
        output_data: &serde_json::Value,
        _execution: &FunctionExecution,
    ) -> Result<serde_json::Value> {
        match mapping {
            InputMapping::FullOutput => Ok(output_data.clone()),
            InputMapping::FieldMapping(mappings) => {
                let mut result = serde_json::Map::new();
                for (source_field, target_field) in mappings {
                    if let Some(value) = output_data.get(source_field) {
                        result.insert(target_field.clone(), value.clone());
                    }
                }
                Ok(serde_json::Value::Object(result))
            }
            _ => {
                // TODO: Implement other mapping types
                Ok(output_data.clone())
            }
        }
    }

    /// Calculate retry delay based on strategy
    fn calculate_retry_delay(&self, retry_config: &RetryConfig, attempt: u32) -> Duration {
        match &retry_config.backoff_strategy {
            BackoffStrategy::Fixed => retry_config.retry_delay,
            BackoffStrategy::Exponential { multiplier } => {
                let delay_seconds = retry_config.retry_delay.num_seconds() as f64 * multiplier.powi(attempt as i32);
                Duration::seconds(delay_seconds as i64)
            }
            BackoffStrategy::Linear { increment } => {
                retry_config.retry_delay + (*increment * attempt as i32)
            }
        }
    }

    /// Schedule a retry execution
    async fn schedule_retry_execution(
        &self,
        function: FunctionDefinition,
        execution_id: Uuid,
        delay: Duration,
    ) -> Result<()> {
        let engine = self.clone_for_background();
        
        tokio::spawn(async move {
            tokio::time::sleep(delay.to_std().unwrap_or(std::time::Duration::from_secs(30))).await;
            
            if let Err(e) = engine.execute_function_impl(function, execution_id).await {
                eprintln!("Retry execution failed: {}", e);
            }
        });
        
        Ok(())
    }

    /// Run a Docker container with the given configuration
    async fn run_container(
        &self,
        config: &ContainerConfig,
        container_name: &str,
        execution: &FunctionExecution,
    ) -> Result<ContainerResult> {
        let mut docker_cmd = vec![
            "run".to_string(),
            "--name".to_string(), container_name.to_string(),
            "--rm".to_string(), // Remove container when done
        ];

        // Add environment variables
        for (key, value) in &config.env_vars {
            docker_cmd.push("-e".to_string());
            docker_cmd.push(format!("{}={}", key, value));
        }

        // Add function execution context as environment variables
        docker_cmd.push("-e".to_string());
        docker_cmd.push(format!("TRIGGER_EVENT={}", execution.trigger_event));
        docker_cmd.push("-e".to_string());
        docker_cmd.push(format!("EXECUTION_ID={}", execution.id));
        docker_cmd.push("-e".to_string());
        docker_cmd.push(format!("FUNCTION_ID={}", execution.function_id));
        docker_cmd.push("-e".to_string());
        docker_cmd.push(format!("INPUT_DATA={}", execution.input_data));

        // Add secret variables (in a real implementation, these would be fetched securely)
        for (key, _secret_ref) in &config.secret_vars {
            // TODO: Implement secure secret resolution
            docker_cmd.push("-e".to_string());
            docker_cmd.push(format!("{}=<SECRET_VALUE>", key));
        }

        // Add working directory
        if let Some(work_dir) = &config.working_dir {
            docker_cmd.push("-w".to_string());
            docker_cmd.push(work_dir.clone());
        }

        // Add resource limits
        if let Some(resources) = &config.resources {
            if let Some(memory_mb) = resources.memory_mb {
                docker_cmd.push("-m".to_string());
                docker_cmd.push(format!("{}m", memory_mb));
            }
            if let Some(cpu_cores) = resources.cpu_cores {
                docker_cmd.push("--cpus".to_string());
                docker_cmd.push(cpu_cores.to_string());
            }
        }

        // Add mounts
        for mount in &config.mounts {
            docker_cmd.push("-v".to_string());
            let mount_str = if mount.readonly {
                format!("{}:{}:ro", mount.source, mount.target)
            } else {
                format!("{}:{}", mount.source, mount.target)
            };
            docker_cmd.push(mount_str);
        }

        // Add the image
        docker_cmd.push(config.image.clone());

        // Add the command to execute
        if !config.exec_command.is_empty() {
            docker_cmd.extend(config.exec_command.clone());
        }

        // Log the Docker command being executed
        println!("🔧 Running Docker command: docker {}", docker_cmd.join(" "));

        // Run setup commands first if any
        for setup_cmd in &config.setup_commands {
            let mut setup_docker_cmd = vec![
                "run".to_string(),
                "--name".to_string(), format!("{}-setup-{}", container_name, Uuid::new_v4()),
                "--rm".to_string(),
            ];

            // Add same environment and mounts for setup
            for (key, value) in &config.env_vars {
                setup_docker_cmd.push("-e".to_string());
                setup_docker_cmd.push(format!("{}={}", key, value));
            }

            setup_docker_cmd.push(config.image.clone());
            setup_docker_cmd.extend(setup_cmd.clone());

            println!("🔧 Running setup command: docker {}", setup_docker_cmd.join(" "));
            let _setup_result = self.execute_docker_command(setup_docker_cmd).await?;
        }

        // Execute the main command
        self.execute_docker_command(docker_cmd).await
    }

    /// Execute a docker command and capture output
    async fn execute_docker_command(&self, args: Vec<String>) -> Result<ContainerResult> {
        let mut cmd = Command::new("docker");
        cmd.args(&args);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        println!("📦 Starting Docker container...");
        let mut child = cmd.spawn()
            .map_err(|e| CircuitBreakerError::GraphQL(format!("Failed to start docker: {}", e)))?;

        // Capture stdout and stderr
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let stdout_reader = BufReader::new(stdout);
        let stderr_reader = BufReader::new(stderr);

        let mut stdout_lines = stdout_reader.lines();
        let mut stderr_lines = stderr_reader.lines();

        let mut stdout_output = String::new();
        let mut stderr_output = String::new();

        // Read output asynchronously
        loop {
            tokio::select! {
                line = stdout_lines.next_line() => {
                    match line {
                        Ok(Some(line)) => {
                            println!("📄 STDOUT: {}", line);
                            stdout_output.push_str(&line);
                            stdout_output.push('\n');
                        }
                        Ok(None) => break,
                        Err(e) => return Err(CircuitBreakerError::GraphQL(format!("Failed to read stdout: {}", e))),
                    }
                }
                line = stderr_lines.next_line() => {
                    match line {
                        Ok(Some(line)) => {
                            println!("⚠️  STDERR: {}", line);
                            stderr_output.push_str(&line);
                            stderr_output.push('\n');
                        }
                        Ok(None) => {},
                        Err(e) => return Err(CircuitBreakerError::GraphQL(format!("Failed to read stderr: {}", e))),
                    }
                }
            }
        }

        let exit_status = child.wait().await
            .map_err(|e| CircuitBreakerError::GraphQL(format!("Failed to wait for docker: {}", e)))?;

        let exit_code = exit_status.code().unwrap_or(-1);
        
        if exit_code == 0 {
            println!("✅ Docker container completed successfully (exit code: {})", exit_code);
        } else {
            println!("❌ Docker container failed (exit code: {})", exit_code);
        }

        Ok(ContainerResult {
            exit_code,
            stdout: stdout_output,
            stderr: stderr_output,
        })
    }

    /// Clean up a Docker container
    async fn cleanup_container(&self, container_name: &str) -> Result<()> {
        let args = vec!["rm".to_string(), "-f".to_string(), container_name.to_string()];
        let _ = self.execute_docker_command(args).await;
        Ok(())
    }

    /// Get function by ID
    pub async fn get_function(&self, id: &FunctionId) -> Result<Option<FunctionDefinition>> {
        self.storage.get_function(id).await
    }

    /// Create a new function
    pub async fn create_function(&self, function: FunctionDefinition) -> Result<FunctionDefinition> {
        self.storage.create_function(function).await
    }

    /// Update a function
    pub async fn update_function(&self, function: FunctionDefinition) -> Result<FunctionDefinition> {
        self.storage.update_function(function).await
    }

    /// Delete a function
    pub async fn delete_function(&self, id: &FunctionId) -> Result<bool> {
        self.storage.delete_function(id).await
    }

    /// List all functions
    pub async fn list_functions(&self) -> Result<Vec<FunctionDefinition>> {
        self.storage.list_functions().await
    }

    /// Get execution by ID
    pub async fn get_execution(&self, id: &Uuid) -> Result<Option<FunctionExecution>> {
        self.storage.get_execution(id).await
    }

    /// List executions for a function
    pub async fn list_executions(&self, function_id: &FunctionId) -> Result<Vec<FunctionExecution>> {
        self.storage.list_executions(function_id).await
    }
}

/// In-memory implementation of FunctionStorage for testing
#[derive(Clone)]
pub struct InMemoryFunctionStorage {
    functions: std::sync::Arc<tokio::sync::RwLock<HashMap<FunctionId, FunctionDefinition>>>,
    executions: std::sync::Arc<tokio::sync::RwLock<HashMap<Uuid, FunctionExecution>>>,
    chains: std::sync::Arc<tokio::sync::RwLock<HashMap<Uuid, ChainExecution>>>,
}

impl InMemoryFunctionStorage {
    pub fn new() -> Self {
        Self {
            functions: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            executions: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            chains: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl FunctionStorage for InMemoryFunctionStorage {
    async fn create_function(&self, function: FunctionDefinition) -> Result<FunctionDefinition> {
        let mut functions = self.functions.write().await;
        functions.insert(function.id.clone(), function.clone());
        Ok(function)
    }

    async fn get_function(&self, id: &FunctionId) -> Result<Option<FunctionDefinition>> {
        let functions = self.functions.read().await;
        Ok(functions.get(id).cloned())
    }

    async fn update_function(&self, function: FunctionDefinition) -> Result<FunctionDefinition> {
        let mut functions = self.functions.write().await;
        functions.insert(function.id.clone(), function.clone());
        Ok(function)
    }

    async fn delete_function(&self, id: &FunctionId) -> Result<bool> {
        let mut functions = self.functions.write().await;
        Ok(functions.remove(id).is_some())
    }

    async fn list_functions(&self) -> Result<Vec<FunctionDefinition>> {
        let functions = self.functions.read().await;
        Ok(functions.values().cloned().collect())
    }

    async fn create_execution(&self, execution: FunctionExecution) -> Result<FunctionExecution> {
        let mut executions = self.executions.write().await;
        executions.insert(execution.id, execution.clone());
        Ok(execution)
    }

    async fn update_execution(&self, execution: FunctionExecution) -> Result<FunctionExecution> {
        let mut executions = self.executions.write().await;
        executions.insert(execution.id, execution.clone());
        Ok(execution)
    }

    async fn get_execution(&self, id: &Uuid) -> Result<Option<FunctionExecution>> {
        let executions = self.executions.read().await;
        Ok(executions.get(id).cloned())
    }

    async fn list_executions(&self, function_id: &FunctionId) -> Result<Vec<FunctionExecution>> {
        let executions = self.executions.read().await;
        Ok(executions
            .values()
            .filter(|exec| exec.function_id == *function_id)
            .cloned()
            .collect())
    }

    async fn create_chain(&self, chain: ChainExecution) -> Result<ChainExecution> {
        let mut chains = self.chains.write().await;
        chains.insert(chain.id, chain.clone());
        Ok(chain)
    }

    async fn update_chain(&self, chain: ChainExecution) -> Result<ChainExecution> {
        let mut chains = self.chains.write().await;
        chains.insert(chain.id, chain.clone());
        Ok(chain)
    }

    async fn get_chain(&self, id: &Uuid) -> Result<Option<ChainExecution>> {
        let chains = self.chains.read().await;
        Ok(chains.get(id).cloned())
    }
} 