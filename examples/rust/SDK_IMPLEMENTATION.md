# Circuit Breaker Rust SDK Implementation Plan

This document outlines the comprehensive SDK implementation for Circuit Breaker in Rust, providing developers with an ergonomic, type-safe interface for building workflows using the GraphQL endpoint, OpenAI router, and Functions system.

## SDK Architecture Overview

```
circuit-breaker-sdk/
├── src/
│   ├── lib.rs                  # Main library entry point
│   ├── client.rs               # Main SDK client
│   ├── error.rs                # Error types and handling
│   ├── types/
│   │   ├── mod.rs              # Core type definitions
│   │   ├── workflow.rs         # Workflow types
│   │   ├── resource.rs         # Resource types
│   │   ├── function.rs         # Function system types
│   │   └── llm.rs              # LLM types
│   ├── workflow/
│   │   ├── mod.rs              # Workflow module
│   │   ├── manager.rs          # Workflow management
│   │   ├── builder.rs          # Workflow builder pattern
│   │   └── executor.rs         # Workflow execution
│   ├── resource/
│   │   ├── mod.rs              # Resource module
│   │   ├── manager.rs          # Resource management
│   │   └── tracker.rs          # Resource state tracking
│   ├── function/
│   │   ├── mod.rs              # Function module
│   │   ├── manager.rs          # Function system integration
│   │   ├── executor.rs         # Function execution
│   │   └── docker.rs           # Docker container management
│   ├── llm/
│   │   ├── mod.rs              # LLM module
│   │   ├── router.rs           # OpenAI-compatible router
│   │   ├── providers.rs        # LLM provider implementations
│   │   └── streaming.rs        # Streaming support
│   ├── agent/
│   │   ├── mod.rs              # Agent module
│   │   ├── builder.rs          # AI agent construction
│   │   ├── state_machine.rs    # State machine agents
│   │   └── conversation.rs     # Conversational agents
│   ├── rules/
│   │   ├── mod.rs              # Rules module
│   │   ├── engine.rs           # Rules engine for state transitions
│   │   ├── builder.rs          # Rule builder and composition
│   │   ├── evaluator.rs        # Rule evaluation logic
│   │   └── registry.rs         # Global rule registry
│   ├── graphql/
│   │   ├── mod.rs              # GraphQL module
│   │   ├── client.rs           # GraphQL client
│   │   └── queries.rs          # GraphQL queries and mutations
│   └── utils/
│       ├── mod.rs              # Utilities module
│       ├── validation.rs       # Input validation
│       └── logger.rs           # Logging utilities
├── examples/
│   ├── basic_workflow.rs
│   ├── ai_agent.rs
│   ├── function_chains.rs
│   └── llm_integration.rs
├── tests/
│   ├── integration/
│   ├── unit/
│   └── common/
├── docs/
│   ├── api-reference.md
│   ├── getting-started.md
│   └── examples.md
├── Cargo.toml
├── README.md
└── CHANGELOG.md
```

## Core Implementation Plan

### 1. Main SDK Client (`src/client.rs`)

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct CircuitBreakerSDK {
    config: Arc<SDKConfig>,
    graphql_client: Arc<GraphQLClient>,
    llm_router: Arc<LLMRouter>,
    function_manager: Arc<FunctionManager>,
    workflow_manager: Arc<WorkflowManager>,
    resource_manager: Arc<ResourceManager>,
    rules_engine: Arc<RulesEngine>,
}

impl CircuitBreakerSDK {
    pub async fn new(config: SDKConfig) -> Result<Self, SDKError> {
        let graphql_client = Arc::new(GraphQLClient::new(&config.graphql_endpoint)?);
        let llm_router = Arc::new(LLMRouter::new(config.llm_config.clone()).await?);
        let function_manager = Arc::new(FunctionManager::new(config.function_config.clone()).await?);
        let workflow_manager = Arc::new(WorkflowManager::new(graphql_client.clone()));
        let resource_manager = Arc::new(ResourceManager::new(graphql_client.clone()));
        let rules_engine = Arc::new(RulesEngine::new(config.rules_config.clone())?);

        Ok(Self {
            config: Arc::new(config),
            graphql_client,
            llm_router,
            function_manager,
            workflow_manager,
            resource_manager,
            rules_engine,
        })
    }

    /// Access workflow management functionality
    pub fn workflows(&self) -> &WorkflowManager {
        &self.workflow_manager
    }

    /// Access resource management functionality
    pub fn resources(&self) -> &ResourceManager {
        &self.resource_manager
    }

    /// Access function system functionality
    pub fn functions(&self) -> &FunctionManager {
        &self.function_manager
    }

    /// Access LLM router functionality
    pub fn llm(&self) -> &LLMRouter {
        &self.llm_router
    }

    /// Create a new agent builder
    pub fn agent_builder(&self, name: impl Into<String>) -> AgentBuilder {
        AgentBuilder::new(name.into(), self.clone())
    }

    /// Access rules engine functionality
    pub fn rules(&self) -> &RulesEngine {
        &self.rules_engine
    }
}
```

### 2. Workflow Builder Pattern (`src/workflow/builder.rs`)

```rust
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct WorkflowBuilder {
    workflow: WorkflowDefinition,
}

impl WorkflowBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            workflow: WorkflowDefinition {
                name: name.into(),
                states: Vec::new(),
                activities: Vec::new(),
                initial_state: None,
                metadata: HashMap::new(),
            },
        }
    }

    pub fn add_state(mut self, state: impl Into<String>) -> Self {
        self.workflow.states.push(state.into());
        self
    }

    pub fn add_transition(
        mut self,
        from: impl Into<String>,
        to: impl Into<String>,
        activity: impl Into<String>,
    ) -> Self {
        self.workflow.activities.push(ActivityDefinition {
            id: activity.into(),
            name: None,
            from_states: vec![from.into()],
            to_state: to.into(),
            conditions: Vec::new(),
            rules: None,
            functions: None,
        });
        self
    }

    pub fn add_rule(mut self, activity: impl Into<String>, rule: Rule) -> Self {
        if let Some(activity_def) = self.workflow.activities.iter_mut()
            .find(|a| a.id == activity.into()) {
            activity_def.rules.get_or_insert_with(Vec::new).push(rule);
        }
        self
    }

    pub fn add_rules(mut self, activity: impl Into<String>, rules: Vec<Rule>, require_all: Option<bool>) -> Self {
        let activity_id = activity.into();
        if let Some(activity_def) = self.workflow.activities.iter_mut()
            .find(|a| a.id == activity_id) {
            activity_def.rules.get_or_insert_with(Vec::new).extend(rules);
            activity_def.requires_all_rules = require_all;
        }
        self
    }

    pub fn add_simple_rule(mut self, activity: impl Into<String>, field: impl Into<String>, operator: RuleOperator, value: serde_json::Value) -> Self {
        let rule = Rule::simple(format!("{}_rule", field.into()))
            .field_condition(field.into(), operator, value)
            .build();
        self.add_rule(activity, rule)
    }

    pub fn set_initial_state(mut self, state: impl Into<String>) -> Self {
        self.workflow.initial_state = Some(state.into());
        self
    }

    // Advanced workflow patterns
    pub fn branch(self, condition: impl Into<String>) -> BranchBuilder {
        BranchBuilder::new(self, condition.into())
    }

    pub fn parallel(self) -> ParallelBuilder {
        ParallelBuilder::new(self)
    }

    pub fn loop_while(self, condition: impl Into<String>) -> LoopBuilder {
        LoopBuilder::new(self, condition.into())
    }

    pub fn build(self) -> Result<WorkflowDefinition, WorkflowBuilderError> {
        self.validate()?;
        Ok(self.workflow)
    }

    pub async fn validate_rules(&self, rules_engine: &RulesEngine) -> Result<RuleValidationResult, WorkflowBuilderError> {
        rules_engine.validate_workflow(&self.workflow).await
    }

    pub fn with_rules_engine(self, rules_engine: Arc<RulesEngine>) -> Self {
        // Associate rules engine for validation during build
        self
    }

    fn validate(&self) -> Result<(), WorkflowBuilderError> {
        if self.workflow.states.is_empty() {
            return Err(WorkflowBuilderError::NoStates);
        }
        if self.workflow.initial_state.is_none() {
            return Err(WorkflowBuilderError::NoInitialState);
        }
        // Additional validation logic...
        Ok(())
    }
}

pub struct BranchBuilder {
    parent: WorkflowBuilder,
    condition: String,
    branches: Vec<(String, WorkflowBuilder)>,
}

impl BranchBuilder {
    pub fn when(mut self, condition: impl Into<String>, builder: WorkflowBuilder) -> Self {
        self.branches.push((condition.into(), builder));
        self
    }

    pub fn otherwise(self, builder: WorkflowBuilder) -> WorkflowBuilder {
        // Merge branches into parent workflow
        // Implementation details...
        self.parent
    }
}
```

### 3. Function System Integration (`src/function/manager.rs`)

```rust
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::process::Command;

pub struct FunctionManager {
    docker_client: DockerClient,
    function_registry: Arc<RwLock<HashMap<FunctionId, FunctionDefinition>>>,
    execution_engine: ExecutionEngine,
}

impl FunctionManager {
    pub async fn new(config: FunctionConfig) -> Result<Self, FunctionError> {
        Ok(Self {
            docker_client: DockerClient::new(config.docker_config).await?,
            function_registry: Arc::new(RwLock::new(HashMap::new())),
            execution_engine: ExecutionEngine::new(config.execution_config),
        })
    }

    pub async fn create_function(
        &self,
        definition: FunctionDefinition,
    ) -> Result<FunctionId, FunctionError> {
        // Validate function definition
        self.validate_function(&definition)?;

        // Register function
        let function_id = definition.id.clone();
        self.function_registry.write().await.insert(function_id.clone(), definition);

        Ok(function_id)
    }

    pub async fn execute_function(
        &self,
        id: &FunctionId,
        input: serde_json::Value,
    ) -> Result<FunctionResult, FunctionError> {
        let function = self.get_function(id).await?;
        self.execution_engine.execute(&function, input).await
    }

    pub async fn chain_functions(
        &self,
        chain: &[FunctionChain],
    ) -> Result<ChainResult, FunctionError> {
        let mut results = Vec::new();
        let mut current_input = serde_json::Value::Null;

        for chain_link in chain {
            let result = self.execute_function(&chain_link.function_id, current_input).await?;
            current_input = result.output.clone();
            results.push(result);
        }

        Ok(ChainResult { results })
    }

    // Docker integration
    pub async fn deploy_container(
        &self,
        config: ContainerConfig,
    ) -> Result<ContainerId, FunctionError> {
        self.docker_client.deploy_container(config).await
    }

    pub async fn execute_container(
        &self,
        id: &ContainerId,
        input: serde_json::Value,
    ) -> Result<ContainerResult, FunctionError> {
        self.docker_client.execute_container(id, input).await
    }

    // Event-driven execution
    pub fn on_workflow_event<F>(&self, event_type: WorkflowEventType, handler: F)
    where
        F: Fn(WorkflowEvent) -> Result<(), FunctionError> + Send + Sync + 'static,
    {
        // Register event handler
        // Implementation details...
    }

    pub fn on_resource_state<F>(&self, state: impl Into<String>, handler: F)
    where
        F: Fn(ResourceStateEvent) -> Result<(), FunctionError> + Send + Sync + 'static,
    {
        // Register state handler
        // Implementation details...
    }

    async fn get_function(&self, id: &FunctionId) -> Result<FunctionDefinition, FunctionError> {
        self.function_registry
            .read()
            .await
            .get(id)
            .cloned()
            .ok_or(FunctionError::FunctionNotFound(id.clone()))
    }

    fn validate_function(&self, definition: &FunctionDefinition) -> Result<(), FunctionError> {
        // Validation logic
        Ok(())
    }
}

#[async_trait]
pub trait ExecutionEngine {
    async fn execute(
        &self,
        function: &FunctionDefinition,
        input: serde_json::Value,
    ) -> Result<FunctionResult, FunctionError>;
}

pub struct DockerExecutionEngine {
    docker_client: DockerClient,
}

#[async_trait]
impl ExecutionEngine for DockerExecutionEngine {
    async fn execute(
        &self,
        function: &FunctionDefinition,
        input: serde_json::Value,
    ) -> Result<FunctionResult, FunctionError> {
        let container_id = self.docker_client
            .create_container(&function.container)
            .await?;

        let result = self.docker_client
            .run_container(&container_id, input)
            .await?;

        self.docker_client
            .cleanup_container(&container_id)
            .await?;

        Ok(FunctionResult {
            function_id: function.id.clone(),
            output: result.output,
            logs: result.logs,
            execution_time: result.execution_time,
            status: ExecutionStatus::Success,
        })
    }
}
```

### 4. Rules Engine (`src/rules/engine.rs`)

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use async_trait::async_trait;

pub struct RulesEngine {
    rule_registry: Arc<RwLock<HashMap<String, Rule>>>,
    evaluation_cache: Arc<RwLock<HashMap<String, CachedEvaluation>>>,
    config: RulesConfig,
}

impl RulesEngine {
    pub fn new(config: Option<RulesConfig>) -> Result<Self, RulesError> {
        let config = config.unwrap_or_default();
        let mut engine = Self {
            rule_registry: Arc::new(RwLock::new(HashMap::new())),
            evaluation_cache: Arc::new(RwLock::new(HashMap::new())),
            config,
        };
        
        engine.initialize_common_rules()?;
        Ok(engine)
    }

    // Rule management
    pub async fn register_rule(&self, name: impl Into<String>, rule: Rule) {
        self.rule_registry.write().await.insert(name.into(), rule);
    }

    pub async fn get_rule(&self, name: &str) -> Option<Rule> {
        self.rule_registry.read().await.get(name).cloned()
    }

    pub async fn remove_rule(&self, name: &str) -> bool {
        self.rule_registry.write().await.remove(name).is_some()
    }

    // Rule evaluation
    pub async fn can_transition(
        &self,
        resource: &Resource,
        activity: &ActivityDefinition,
    ) -> Result<bool, RulesError> {
        let context = RuleContext {
            resource: resource.clone(),
            activity: activity.clone(),
            metadata: HashMap::new(),
        };

        // Evaluate structured rules
        if let Some(rules) = &activity.rules {
            let evaluation = self.evaluate_rules(rules, &context).await?;
            if !evaluation.passed {
                return Ok(false);
            }
        }

        // Evaluate legacy string conditions
        for condition in &activity.conditions {
            if !self.evaluate_condition(condition, &context).await? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    pub async fn evaluate_rules(
        &self,
        rules: &[Rule],
        context: &RuleContext,
    ) -> Result<RuleEvaluationResult, RulesError> {
        let start_time = std::time::Instant::now();
        let mut results = Vec::new();
        let mut all_passed = true;

        for rule in rules {
            let result = self.evaluate_single_rule(rule, context).await?;
            all_passed &= result.passed;
            results.push(result);
        }

        Ok(RuleEvaluationResult {
            passed: all_passed,
            results,
            errors: Vec::new(),
            evaluation_time: start_time.elapsed(),
        })
    }

    pub async fn get_available_transitions(
        &self,
        resource: &Resource,
        workflow: &WorkflowDefinition,
    ) -> Result<Vec<ActivityDefinition>, RulesError> {
        let mut available = Vec::new();

        for activity in &workflow.activities {
            if activity.from_states.contains(&resource.state) {
                if self.can_transition(resource, activity).await? {
                    available.push(activity.clone());
                }
            }
        }

        Ok(available)
    }

    // Rule building helpers
    pub fn create_rule(&self, name: impl Into<String>) -> RuleBuilder {
        RuleBuilder::new(name.into())
    }

    pub fn and(&self, rules: Vec<Rule>) -> CompositeRule {
        CompositeRule {
            name: "composite_and".to_string(),
            rule_type: RuleType::Composite,
            operator: LogicalOperator::And,
            rules,
            description: None,
            category: None,
            metadata: HashMap::new(),
        }
    }

    pub fn or(&self, rules: Vec<Rule>) -> CompositeRule {
        CompositeRule {
            name: "composite_or".to_string(),
            rule_type: RuleType::Composite,
            operator: LogicalOperator::Or,
            rules,
            description: None,
            category: None,
            metadata: HashMap::new(),
        }
    }

    pub fn not(&self, rule: Rule) -> CompositeRule {
        CompositeRule {
            name: "composite_not".to_string(),
            rule_type: RuleType::Composite,
            operator: LogicalOperator::Not,
            rules: vec![rule],
            description: None,
            category: None,
            metadata: HashMap::new(),
        }
    }

    pub async fn validate_workflow(&self, workflow: &WorkflowDefinition) -> Result<RuleValidationResult, RulesError> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        for activity in &workflow.activities {
            if let Some(rules) = &activity.rules {
                for rule in rules {
                    if let Err(e) = self.validate_rule(rule).await {
                        errors.push(format!("Activity '{}': {}", activity.id, e));
                    }
                }
            }
        }

        Ok(RuleValidationResult {
            valid: errors.is_empty(),
            errors,
            warnings,
        })
    }

    // Private methods
    async fn evaluate_single_rule(&self, rule: &Rule, context: &RuleContext) -> Result<RuleResult, RulesError> {
        match &rule.rule_type {
            RuleType::Simple => self.evaluate_simple_rule(rule, context).await,
            RuleType::Composite => self.evaluate_composite_rule(rule, context).await,
            RuleType::Custom => self.evaluate_custom_rule(rule, context).await,
            RuleType::JavaScript => self.evaluate_javascript_rule(rule, context).await,
        }
    }

    async fn evaluate_simple_rule(&self, rule: &Rule, context: &RuleContext) -> Result<RuleResult, RulesError> {
        // Simple rule evaluation logic
        let passed = if let Some(condition) = &rule.condition {
            self.evaluate_condition(condition, context).await?
        } else {
            false
        };

        Ok(RuleResult {
            rule: rule.clone(),
            passed,
            error: None,
            context: None,
        })
    }

    async fn evaluate_composite_rule(&self, rule: &Rule, context: &RuleContext) -> Result<RuleResult, RulesError> {
        // Composite rule evaluation logic
        Ok(RuleResult {
            rule: rule.clone(),
            passed: true,
            error: None,
            context: None,
        })
    }

    async fn evaluate_custom_rule(&self, rule: &Rule, context: &RuleContext) -> Result<RuleResult, RulesError> {
        // Custom rule evaluation logic
        Ok(RuleResult {
            rule: rule.clone(),
            passed: true,
            error: None,
            context: None,
        })
    }

    async fn evaluate_javascript_rule(&self, rule: &Rule, context: &RuleContext) -> Result<RuleResult, RulesError> {
        // JavaScript rule evaluation logic
        Ok(RuleResult {
            rule: rule.clone(),
            passed: true,
            error: None,
            context: None,
        })
    }

    async fn evaluate_condition(&self, condition: &str, context: &RuleContext) -> Result<bool, RulesError> {
        // Legacy condition evaluation logic
        Ok(true)
    }

    async fn validate_rule(&self, rule: &Rule) -> Result<(), RulesError> {
        // Rule validation logic
        Ok(())
    }

    fn initialize_common_rules(&mut self) -> Result<(), RulesError> {
        // Initialize common predefined rules
        Ok(())
    }
}

pub struct RuleBuilder {
    rule: Rule,
}

impl RuleBuilder {
    pub fn new(name: String) -> Self {
        Self {
            rule: Rule {
                name,
                rule_type: RuleType::Simple,
                condition: None,
                evaluator: None,
                description: None,
                category: None,
                metadata: HashMap::new(),
            },
        }
    }

    // Simple conditions
    pub fn field_equals(mut self, field: impl Into<String>, value: serde_json::Value) -> Self {
        self.rule.condition = Some(format!("data.{} == {}", field.into(), value));
        self
    }

    pub fn field_greater_than(mut self, field: impl Into<String>, value: f64) -> Self {
        self.rule.condition = Some(format!("data.{} > {}", field.into(), value));
        self
    }

    pub fn field_contains(mut self, field: impl Into<String>, value: impl Into<String>) -> Self {
        self.rule.condition = Some(format!("data.{}.contains('{}')", field.into(), value.into()));
        self
    }

    pub fn field_exists(mut self, field: impl Into<String>) -> Self {
        self.rule.condition = Some(format!("data.{} != null", field.into()));
        self
    }

    pub fn field_condition(mut self, field: impl Into<String>, operator: RuleOperator, value: serde_json::Value) -> Self {
        let op_str = match operator {
            RuleOperator::Equals => "==",
            RuleOperator::NotEquals => "!=",
            RuleOperator::GreaterThan => ">",
            RuleOperator::LessThan => "<",
            RuleOperator::GreaterThanOrEqual => ">=",
            RuleOperator::LessThanOrEqual => "<=",
            RuleOperator::Contains => "contains",
            RuleOperator::StartsWith => "startsWith",
            RuleOperator::EndsWith => "endsWith",
        };
        self.rule.condition = Some(format!("data.{} {} {}", field.into(), op_str, value));
        self
    }

    // Complex conditions
    pub fn custom<F>(mut self, evaluator: F) -> Self 
    where
        F: Fn(&RuleContext) -> Result<bool, RulesError> + Send + Sync + 'static,
    {
        self.rule.rule_type = RuleType::Custom;
        self.rule.evaluator = Some(Box::new(evaluator));
        self
    }

    pub fn javascript(mut self, expression: impl Into<String>) -> Self {
        self.rule.rule_type = RuleType::JavaScript;
        self.rule.condition = Some(expression.into());
        self
    }

    // Metadata
    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.rule.description = Some(text.into());
        self
    }

    pub fn category(mut self, category: impl Into<String>) -> Self {
        self.rule.category = Some(category.into());
        self
    }

    pub fn build(self) -> Rule {
        self.rule
    }
}
```

### 5. LLM Router (`src/llm/router.rs`)

```rust
use std::collections::HashMap;
use async_trait::async_trait;
use futures::Stream;

pub struct LLMRouter {
    providers: HashMap<String, Box<dyn LLMProvider>>,
    default_provider: Option<String>,
    load_balancer: LoadBalancer,
    failover_handler: FailoverHandler,
}

impl LLMRouter {
    pub async fn new(config: LLMConfig) -> Result<Self, LLMError> {
        let mut providers = HashMap::new();
        
        for provider_config in config.providers {
            let provider = create_provider(&provider_config).await?;
            providers.insert(provider_config.name.clone(), provider);
        }

        Ok(Self {
            providers,
            default_provider: config.default_provider,
            load_balancer: LoadBalancer::new(config.load_balancing),
            failover_handler: FailoverHandler::new(config.failover),
        })
    }

    // OpenAI-compatible interface
    pub async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, LLMError> {
        let provider = self.select_provider(&request.model)?;
        
        match provider.chat_completion(request).await {
            Ok(response) => Ok(response),
            Err(e) => self.handle_failure(e, &request).await,
        }
    }

    pub async fn chat_completion_stream(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<impl Stream<Item = Result<ChatCompletionChunk, LLMError>>, LLMError> {
        let provider = self.select_provider(&request.model)?;
        provider.chat_completion_stream(request).await
    }

    // Provider management
    pub async fn add_provider(
        &mut self,
        name: impl Into<String>,
        provider: Box<dyn LLMProvider>,
    ) {
        self.providers.insert(name.into(), provider);
    }

    pub fn set_default_provider(&mut self, name: impl Into<String>) {
        self.default_provider = Some(name.into());
    }

    // Load balancing and failover
    pub async fn route_request(
        &self,
        request: LLMRequest,
    ) -> Result<LLMResponse, LLMError> {
        let provider_name = self.load_balancer.select_provider(&request)?;
        let provider = self.providers.get(&provider_name)
            .ok_or(LLMError::ProviderNotFound(provider_name))?;

        provider.handle_request(request).await
    }

    pub async fn get_provider_health(&self) -> Vec<ProviderHealthStatus> {
        let mut health_statuses = Vec::new();
        
        for (name, provider) in &self.providers {
            let status = provider.health_check().await;
            health_statuses.push(ProviderHealthStatus {
                name: name.clone(),
                status,
                last_check: chrono::Utc::now(),
            });
        }
        
        health_statuses
    }

    fn select_provider(&self, model: &str) -> Result<&dyn LLMProvider, LLMError> {
        // Provider selection logic based on model
        if let Some(provider_name) = self.model_to_provider(model) {
            self.providers.get(&provider_name)
                .map(|p| p.as_ref())
                .ok_or(LLMError::ProviderNotFound(provider_name))
        } else if let Some(default) = &self.default_provider {
            self.providers.get(default)
                .map(|p| p.as_ref())
                .ok_or(LLMError::DefaultProviderNotFound)
        } else {
            Err(LLMError::NoProviderAvailable)
        }
    }

    fn model_to_provider(&self, model: &str) -> Option<String> {
        // Model to provider mapping logic
        match model {
            m if m.starts_with("gpt-") => Some("openai".to_string()),
            m if m.starts_with("claude-") => Some("anthropic".to_string()),
            m if m.starts_with("llama") => Some("ollama".to_string()),
            _ => None,
        }
    }

    async fn handle_failure(
        &self,
        error: LLMError,
        request: &ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, LLMError> {
        self.failover_handler.handle_failure(error, request, &self.providers).await
    }
}

#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, LLMError>;

    async fn chat_completion_stream(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<impl Stream<Item = Result<ChatCompletionChunk, LLMError>>, LLMError>;

    async fn handle_request(&self, request: LLMRequest) -> Result<LLMResponse, LLMError>;

    async fn health_check(&self) -> HealthStatus;

    fn supported_models(&self) -> Vec<String>;
}
```

### 5. AI Agent Builder (`src/agent/builder.rs`)

```rust
use std::sync::Arc;

pub struct AgentBuilder {
    name: String,
    sdk: CircuitBreakerSDK,
    agent_type: AgentType,
    config: AgentConfig,
}

impl AgentBuilder {
    pub fn new(name: String, sdk: CircuitBreakerSDK) -> Self {
        Self {
            name,
            sdk,
            agent_type: AgentType::Conversational,
            config: AgentConfig::default(),
        }
    }

    // Agent type builders
    pub fn state_machine(mut self) -> StateMachineAgentBuilder {
        self.agent_type = AgentType::StateMachine;
        StateMachineAgentBuilder::new(self)
    }

    pub fn conversational(mut self) -> ConversationalAgentBuilder {
        self.agent_type = AgentType::Conversational;
        ConversationalAgentBuilder::new(self)
    }

    pub fn workflow_agent(mut self, workflow_id: impl Into<String>) -> WorkflowAgentBuilder {
        self.agent_type = AgentType::WorkflowIntegrated;
        WorkflowAgentBuilder::new(self, workflow_id.into())
    }

    pub async fn build(self) -> Result<Agent, AgentError> {
        match self.agent_type {
            AgentType::StateMachine => {
                StateMachineAgent::new(self.name, self.config, self.sdk).await
            }
            AgentType::Conversational => {
                ConversationalAgent::new(self.name, self.config, self.sdk).await
            }
            AgentType::WorkflowIntegrated => {
                WorkflowAgent::new(self.name, self.config, self.sdk).await
            }
        }
    }
}

pub struct StateMachineAgentBuilder {
    builder: AgentBuilder,
    states: Vec<AgentState>,
    transitions: Vec<StateTransition>,
}

impl StateMachineAgentBuilder {
    pub fn new(builder: AgentBuilder) -> Self {
        Self {
            builder,
            states: Vec::new(),
            transitions: Vec::new(),
        }
    }

    pub fn add_state(mut self, name: impl Into<String>, prompt: impl Into<String>) -> Self {
        self.states.push(AgentState {
            name: name.into(),
            prompt: prompt.into(),
            actions: Vec::new(),
        });
        self
    }

    pub fn add_transition(
        mut self,
        from: impl Into<String>,
        to: impl Into<String>,
        condition: impl Into<String>,
    ) -> Self {
        self.transitions.push(StateTransition {
            from_state: from.into(),
            to_state: to.into(),
            condition: condition.into(),
        });
        self
    }

    pub fn set_llm_provider(mut self, provider: impl Into<String>) -> Self {
        self.builder.config.llm_provider = Some(provider.into());
        self
    }

    pub async fn build(mut self) -> Result<StateMachineAgent, AgentError> {
        self.builder.config.states = self.states;
        self.builder.config.transitions = self.transitions;
        
        match self.builder.build().await? {
            Agent::StateMachine(agent) => Ok(agent),
            _ => Err(AgentError::InvalidAgentType),
        }
    }
}

pub struct ConversationalAgentBuilder {
    builder: AgentBuilder,
}

impl ConversationalAgentBuilder {
    pub fn new(builder: AgentBuilder) -> Self {
        Self { builder }
    }

    pub fn set_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.builder.config.system_prompt = Some(prompt.into());
        self
    }

    pub fn set_llm_provider(mut self, provider: impl Into<String>) -> Self {
        self.builder.config.llm_provider = Some(provider.into());
        self
    }

    pub fn add_workflow_integration(mut self, workflow_id: impl Into<String>) -> Self {
        self.builder.config.workflow_integrations.push(workflow_id.into());
        self
    }

    pub fn enable_memory(mut self, enabled: bool) -> Self {
        self.builder.config.memory_enabled = enabled;
        self
    }

    pub async fn build(mut self) -> Result<ConversationalAgent, AgentError> {
        match self.builder.build().await? {
            Agent::Conversational(agent) => Ok(agent),
            _ => Err(AgentError::InvalidAgentType),
        }
    }
}
```

## Type Definitions (`src/types/mod.rs`)

### Core Types

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SDKConfig {
    pub graphql_endpoint: String,
    pub llm_config: Option<LLMConfig>,
    pub function_config: Option<FunctionConfig>,
    pub rules_config: Option<RulesConfig>,
    pub logging_config: Option<LoggingConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub name: String,
    pub states: Vec<String>,
    pub activities: Vec<ActivityDefinition>,
    pub initial_state: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityDefinition {
    pub id: String,
    pub name: Option<String>,
    pub from_states: Vec<String>,
    pub to_state: String,
    pub conditions: Vec<String>, // Legacy string-based conditions
    pub rules: Option<Vec<Rule>>, // New structured rules
    pub functions: Option<Vec<FunctionTrigger>>,
    pub requires_all_rules: Option<bool>, // AND vs OR logic for multiple rules
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub id: String,
    pub workflow_id: String,
    pub state: String,
    pub data: serde_json::Value,
    pub metadata: HashMap<String, serde_json::Value>,
    pub history: Vec<HistoryEvent>,
}

// Type aliases for clarity
pub type WorkflowId = String;
pub type ResourceId = String;
pub type FunctionId = String;
pub type ContainerId = String;
pub type AgentId = String;
```

### Function System Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub id: FunctionId,
    pub name: String,
    pub container: ContainerConfig,
    pub triggers: Vec<EventTrigger>,
    pub chains: Vec<FunctionChain>,
    pub input_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerConfig {
    pub image: String,
    pub command: Option<Vec<String>>,
    pub environment: HashMap<String, String>,
    pub mounts: Vec<ContainerMount>,
    pub resources: Option<ResourceLimits>,
    pub working_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventTrigger {
    pub trigger_type: EventTriggerType,
    pub condition: String,
    pub input_mapping: Option<InputMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventTriggerType {
    WorkflowEvent,
    ResourceState,
    FunctionCompletion,
    ScheduledTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionChain {
    pub target_function: FunctionId,
    pub condition: ChainCondition,
    pub input_mapping: InputMapping,
    pub delay: Option<chrono::Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChainCondition {
    Always,
    OnSuccess,
    OnFailure,
    Custom(String),
}
```

### Rules Engine Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulesConfig {
    pub enable_cache: Option<bool>,
    pub cache_size: Option<usize>,
    pub custom_rules: Option<HashMap<String, Rule>>,
    pub evaluation_timeout: Option<std::time::Duration>,
}

impl Default for RulesConfig {
    fn default() -> Self {
        Self {
            enable_cache: Some(true),
            cache_size: Some(1000),
            custom_rules: None,
            evaluation_timeout: Some(std::time::Duration::from_secs(30)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub name: String,
    pub rule_type: RuleType,
    pub condition: Option<String>,
    #[serde(skip)]
    pub evaluator: Option<Box<dyn Fn(&RuleContext) -> Result<bool, RulesError> + Send + Sync>>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleType {
    Simple,
    Composite,
    Custom,
    JavaScript,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeRule {
    pub name: String,
    pub rule_type: RuleType,
    pub operator: LogicalOperator,
    pub rules: Vec<Rule>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogicalOperator {
    And,
    Or,
    Not,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleOperator {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    Contains,
    StartsWith,
    EndsWith,
}

#[derive(Debug, Clone)]
pub struct RuleContext {
    pub resource: Resource,
    pub activity: ActivityDefinition,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleEvaluationResult {
    pub passed: bool,
    pub results: Vec<RuleResult>,
    pub errors: Vec<String>,
    pub evaluation_time: std::time::Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleResult {
    pub rule: Rule,
    pub passed: bool,
    pub error: Option<String>,
    pub context: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CachedEvaluation {
    pub result: bool,
    pub timestamp: std::time::Instant,
    pub context_hash: u64,
}
```

### LLM Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    pub providers: Vec<LLMProviderConfig>,
    pub default_provider: Option<String>,
    pub load_balancing: Option<LoadBalancingConfig>,
    pub failover: Option<FailoverConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub stream: Option<bool>,
    pub tools: Option<Vec<Tool>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    pub name: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatRole {
    #[serde(rename = "system")]
    System,
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
    #[serde(rename = "tool")]
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
}
```

## Implementation Phases

### Phase 1: Core SDK Foundation (Week 1-2)
- [ ] Basic GraphQL client with reqwest
- [ ] Core error handling with thiserror
- [ ] Workflow CRUD operations
- [ ] Resource management
- [ ] Unit tests for core functionality

### Phase 2: Workflow Builder & Execution (Week 3-4)
- [ ] Fluent workflow builder API
- [ ] Rules engine integration for state transitions
- [ ] Advanced workflow patterns (branching, parallel, loops)
- [ ] Resource tracking and state management
- [ ] Workflow execution engine with rule evaluation
- [ ] Integration tests

### Phase 3: Function System Integration (Week 5-6)
- [ ] Function definition and management
- [ ] Docker container integration with bollard
- [ ] Event-driven function execution
- [ ] Function chaining and composition
- [ ] Container lifecycle management
- [ ] Rule-based function triggering

### Phase 4: LLM Router & AI Integration (Week 7-8)
- [ ] OpenAI-compatible API client
- [ ] Multiple LLM provider support (OpenAI, Anthropic, Ollama)
- [ ] Streaming response handling with async-stream
- [ ] Load balancing and failover
- [ ] Usage tracking and billing

### Phase 5: AI Agent Framework (Week 9-10)
- [ ] State machine agent builder
- [ ] Conversational agent framework
- [ ] Workflow-integrated agents
- [ ] Agent memory and context management
- [ ] Multi-agent coordination

### Phase 6: Advanced Features (Week 11-12)
- [ ] Real-time subscriptions via GraphQL/WebSocket
- [ ] Batch operations and bulk processing
- [ ] Workflow analytics and monitoring
- [ ] Plugin system for extensibility
- [ ] Performance optimization and benchmarking

## Usage Examples

### Basic Workflow Creation

```rust
use circuit_breaker_sdk::CircuitBreakerSDK;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sdk = CircuitBreakerSDK::new(SDKConfig {
        graphql_endpoint: "http://localhost:4000/graphql".to_string(),
        ..Default::default()
    }).await?;

    // Using builder pattern
    let workflow = sdk.workflows()
        .builder("Order Processing")
        .add_state("pending")
        .add_state("processing")
        .add_state("completed")
        .add_transition("pending", "processing", "start_processing")
        .add_transition("processing", "completed", "complete_order")
        .set_initial_state("pending")
        .build()?;

    let workflow_id = sdk.workflows().create(workflow).await?;
    println!("Created workflow: {}", workflow_id);

    Ok(())
}
```

### Rules Engine Integration

```rust
use circuit_breaker_sdk::{CircuitBreakerSDK, Rule, RuleOperator};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sdk = CircuitBreakerSDK::new(config).await?;

    // Register custom business rules
    sdk.rules().register_rule("payment_verified", Rule {
        name: "payment_verified".to_string(),
        rule_type: RuleType::Simple,
        condition: Some("data.payment_status == 'verified'".to_string()),
        evaluator: None,
        description: Some("Checks if payment has been verified".to_string()),
        category: Some("payment".to_string()),
        metadata: HashMap::new(),
    }).await;

    sdk.rules().register_rule("inventory_available", Rule {
        name: "inventory_available".to_string(),
        rule_type: RuleType::Custom,
        condition: None,
        evaluator: Some(Box::new(|context| {
            let items = context.resource.data.get("items")
                .and_then(|v| v.as_array())
                .unwrap_or(&vec![]);
            Ok(items.iter().all(|item| {
                item.get("quantity")
                    .and_then(|q| q.as_u64())
                    .unwrap_or(0) > 0
            }))
        })),
        description: Some("Validates inventory availability".to_string()),
        category: Some("inventory".to_string()),
        metadata: HashMap::new(),
    }).await;

    // Build workflow with rules
    let workflow = sdk.workflows()
        .builder("Order Processing")
        .add_state("pending")
        .add_state("processing")
        .add_state("completed")
        .add_transition("pending", "processing", "start_processing")
        .add_rule("start_processing", sdk.rules().create_rule("can_process")
            .field_equals("status", serde_json::Value::String("valid".to_string()))
            .field_exists("customer_id")
            .build())
        .add_rules("start_processing", vec![
            sdk.rules().get_rule("payment_verified").await.unwrap(),
            sdk.rules().get_rule("inventory_available").await.unwrap(),
        ], Some(true)) // Require all rules to pass
        .add_transition("processing", "completed", "complete_order")
        .set_initial_state("pending")
        .build()?;

    let workflow_id = sdk.workflows().create(workflow).await?;

    // Create resource
    let resource = sdk.resources().create(ResourceCreateInput {
        workflow_id: workflow_id.clone(),
        initial_state: "pending".to_string(),
        data: serde_json::json!({
            "customer_id": "cust_123",
            "status": "valid",
            "payment_status": "verified",
            "items": [
                {"id": "item1", "quantity": 5},
                {"id": "item2", "quantity": 3}
            ]
        }),
        metadata: HashMap::new(),
    }).await?;

    // Check if transition is possible
    let activity = ActivityDefinition {
        id: "start_processing".to_string(),
        name: None,
        from_states: vec!["pending".to_string()],
        to_state: "processing".to_string(),
        conditions: Vec::new(),
        rules: Some(vec![
            sdk.rules().get_rule("payment_verified").await.unwrap(),
            sdk.rules().get_rule("inventory_available").await.unwrap(),
        ]),
        functions: None,
        requires_all_rules: Some(true),
    };

    let can_transition = sdk.rules().can_transition(&resource, &activity).await?;
    println!("Can transition: {}", can_transition);

    let available_transitions = sdk.rules().get_available_transitions(&resource, &workflow).await?;
    println!("Available transitions: {}", available_transitions.len());

    // Evaluate specific rules
    let evaluation = sdk.rules().evaluate_rules(&activity.rules.unwrap(), &RuleContext {
        resource: resource.clone(),
        activity: activity.clone(),
        metadata: HashMap::new(),
    }).await?;

    println!("Rules evaluation passed: {}", evaluation.passed);
    for result in evaluation.results {
        println!("  Rule '{}': {}", result.rule.name, result.passed);
    }

    Ok(())
}
```

### Function Integration

```rust
use circuit_breaker_sdk::types::{FunctionDefinition, ContainerConfig, EventTrigger};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sdk = CircuitBreakerSDK::new(config).await?;

    // Create a data processing function
    let function = FunctionDefinition {
        id: "order-processor".to_string(),
        name: "Order Data Processor".to_string(),
        container: ContainerConfig {
            image: "node:18-alpine".to_string(),
            command: Some(vec!["node".to_string(), "process-order.js".to_string()]),
            environment: HashMap::new(),
            mounts: Vec::new(),
            resources: None,
            working_dir: Some("/app".to_string()),
        },
        triggers: vec![EventTrigger {
            trigger_type: EventTriggerType::ResourceState,
            condition: "state == 'processing'".to_string(),
            input_mapping: Some(InputMapping::FullData),
        }],
        chains: Vec::new(),
        input_schema: Some(serde_json::json!({
            "type": "object",
            "properties": {
                "orderId": {"type": "string"},
                "items": {"type": "array"}
            },
            "required": ["orderId", "items"]
        })),
        output_schema: None,
        metadata: HashMap::new(),
    };

    let function_id = sdk.functions().create_function(function).await?;

    // Chain with notification function
    sdk.functions().chain_functions(&[
        FunctionChain {
            target_function: function_id,
            condition: ChainCondition::OnSuccess,
            input_mapping: InputMapping::FullData,
            delay: Some(chrono::Duration::seconds(5)),
        },
        FunctionChain {
            target_function: "notification-sender".to_string(),
            condition: ChainCondition::Always,
            input_mapping: InputMapping::FieldMapping(HashMap::from([
                ("result".to_string(), "data".to_string()),
            ])),
            delay: None,
        },
    ]).await?;

    Ok(())
}
```

### AI Agent Creation

```rust
use circuit_breaker_sdk::{CircuitBreakerSDK, AgentBuilder};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sdk = CircuitBreakerSDK::new(config).await?;

    // Create a customer service agent
    let agent = sdk.agent_builder("Customer Service Bot")
        .conversational()
        .set_system_prompt("You are a helpful customer service representative")
        .set_llm_provider("openai-gpt4")
        .add_workflow_integration("customer-support-workflow")
        .enable_memory(true)
        .build()
        .await?;

    // Deploy agent
    let agent_id = agent.deploy().await?;

    // Use agent in conversation
    let response = agent.chat(ChatRequest {
        message: "I need help with my order".to_string(),
        context: Some(serde_json::json!({
            "customerId": "cust_123"
        })),
    }).await?;

    println!("Agent response: {}", response.message);

    Ok(())
}
```

### LLM Router Usage

```rust
use circuit_breaker_sdk::llm::{LLMRouter, ChatCompletionRequest, ChatMessage, ChatRole};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sdk = CircuitBreakerSDK::new(config).await?;

    // Configure multiple providers
    sdk.llm().add_provider("openai", Box::new(OpenAIProvider::new(
        std::env::var("OPENAI_API_KEY")?,
        "https://api.openai.com/v1".to_string(),
    ))).await;

    sdk.llm().add_provider("claude", Box::new(AnthropicProvider::new(
        std::env::var("ANTHROPIC_API_KEY")?,
        "https://api.anthropic.com".to_string(),
    ))).await;

    // Use with automatic failover
    let request = ChatCompletionRequest {
        model: "gpt-4".to_string(),
        messages: vec![ChatMessage {
            role: ChatRole::User,
            content: "Explain quantum computing".to_string(),
            name: None,
            tool_calls: None,
        }],
        temperature: Some(0.7),
        max_tokens: Some(500),
        stream: Some(false),
        tools: None,
    };

    let completion = sdk.llm().chat_completion(request).await?;
    println!("Response: {}", completion.choices[0].message.content);

    // Streaming example
    let stream_request = ChatCompletionRequest {
        model: "gpt-4".to_string(),
        messages: vec![ChatMessage {
            role: ChatRole::User,
            content: "Write a story about AI".to_string(),
            name: None,
            tool_calls: None,
        }],
        temperature: Some(0.8),
        max_tokens: Some(1000),
        stream: Some(true),
        tools: None,
    };

    let mut stream = sdk.llm().chat_completion_stream(stream_request).await?;
    
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(chunk) => {
                if let Some(content) = chunk.choices.first()
                    .and_then(|choice| choice.delta.content.as_ref()) {
                    print!("{}", content);
                }
            }
            Err(e) => eprintln!("Stream error: {}", e),
        }
    }

    Ok(())
}
```

## Error Handling Strategy

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SDKError {
    #[error("GraphQL error: {0}")]
    GraphQL(#[from] GraphQLError),
    
    #[error("Workflow error: {0}")]
    Workflow(#[from] WorkflowError),
    
    #[error("Resource error: {0}")]
    Resource(#[from] ResourceError),
    
    #[error("Function error: {0}")]
    Function(#[from] FunctionError),
    
    #[error("LLM error: {0}")]
    LLM(#[from] LLMError),
    
    #[error("Agent error: {0}")]
    Agent(#[from] AgentError),
    
    #[error("Rules error: {0}")]
    Rules(#[from] RulesError),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

// Specific error types for each module
#[derive(Error, Debug)]
pub enum WorkflowError {
    #[error("Workflow not found: {id}")]
    NotFound { id: String },
    
    #[error("Invalid workflow definition: {reason}")]
    InvalidDefinition { reason: String },
    
    #[error("State transition not allowed: {from} -> {to}")]
    InvalidTransition { from: String, to: String },
}

#[derive(Error, Debug)]
pub enum FunctionError {
    #[error("Function not found: {0}")]
    FunctionNotFound(FunctionId),
    
    #[error("Container execution failed: {0}")]
    ContainerExecutionFailed(String),
    
    #[error("Docker error: {0}")]
    Docker(String),
    
    #[error("Invalid function definition: {0}")]
    InvalidDefinition(String),
}

#[derive(Error, Debug)]
pub enum LLMError {
    #[error("Provider not found: {0}")]
    ProviderNotFound(String),
    
    #[error("No provider available for model: {0}")]
    NoProviderAvailable,
    
    #[error("Default provider not found")]
    DefaultProviderNotFound,
    
    #[error("Rate limit exceeded for provider: {0}")]
    RateLimitExceeded(String),
    
    #[error("API error: {0}")]
    APIError(String),
}

#[derive(Error, Debug)]
pub enum RulesError {
    #[error("Rule not found: {0}")]
    RuleNotFound(String),
    
    #[error("Rule evaluation failed: {0}")]
    EvaluationFailed(String),
    
    #[error("Invalid rule definition: {0}")]
    InvalidDefinition(String),
    
    #[error("Rule evaluation timeout")]
    EvaluationTimeout,
    
    #[error("Condition parsing error: {0}")]
    ConditionParsingError(String),
}

// Result type alias for convenience
pub type SDKResult<T> = Result<T, SDKError>;
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[tokio::test]
    async fn test_workflow_builder() {
        let workflow = WorkflowBuilder::new("Test Workflow")
            .add_state("start")
            .add_state("end")
            .add_transition("start", "end", "complete")
            .set_initial_state("start")
            .build()
            .unwrap();

        assert_eq!(workflow.name, "Test Workflow");
        assert_eq!(workflow.states.len(), 2);
        assert_eq!(workflow.activities.len(), 1);
    }

    #[tokio::test]
    async fn test_function_execution() {
        // Mock function execution test
        let function_def = FunctionDefinition {
            id: "test-function".to_string(),
            name: "Test Function".to_string(),
            container: ContainerConfig {
                image: "echo".to_string(),
                command: Some(vec!["echo".to_string(), "hello".to_string()]),
                environment: HashMap::new(),
                mounts: Vec::new(),
                resources: None,
                working_dir: None,
            },
            triggers: Vec::new(),
            chains: Vec::new(),
            input_schema: None,
            output_schema: None,
            metadata: HashMap::new(),
        };

        // Test function validation
        assert!(validate_function(&function_def).is_ok());
    }

    #[tokio::test]
    async fn test_llm_provider_selection() {
        let router = LLMRouter::new(LLMConfig {
            providers: vec![],
            default_provider: Some("openai".to_string()),
            load_balancing: None,
            failover: None,
        }).await.unwrap();

        // Test model to provider mapping
        assert_eq!(router.model_to_provider("gpt-4"), Some("openai".to_string()));
        assert_eq!(router.model_to_provider("claude-3"), Some("anthropic".to_string()));
    }

    #[tokio::test]
    async fn test_rules_engine() {
        let rules_engine = RulesEngine::new(None).unwrap();
        
        // Test rule registration
        let rule = Rule {
            name: "test_rule".to_string(),
            rule_type: RuleType::Simple,
            condition: Some("data.status == 'valid'".to_string()),
            evaluator: None,
            description: Some("Test rule".to_string()),
            category: None,
            metadata: HashMap::new(),
        };
        
        rules_engine.register_rule("test_rule", rule).await;
        let retrieved_rule = rules_engine.get_rule("test_rule").await;
        assert!(retrieved_rule.is_some());
    }

    #[tokio::test]
    async fn test_rule_evaluation() {
        let rules_engine = RulesEngine::new(None).unwrap();
        
        let resource = Resource {
            id: "test_resource".to_string(),
            workflow_id: "test_workflow".to_string(),
            state: "pending".to_string(),
            data: serde_json::json!({"status": "valid", "amount": 100}),
            metadata: HashMap::new(),
            history: Vec::new(),
        };
        
        let activity = ActivityDefinition {
            id: "test_activity".to_string(),
            name: None,
            from_states: vec!["pending".to_string()],
            to_state: "processing".to_string(),
            conditions: Vec::new(),
            rules: Some(vec![
                Rule {
                    name: "status_check".to_string(),
                    rule_type: RuleType::Simple,
                    condition: Some("data.status == 'valid'".to_string()),
                    evaluator: None,
                    description: None,
                    category: None,
                    metadata: HashMap::new(),
                }
            ]),
            functions: None,
            requires_all_rules: Some(true),
        };
        
        let can_transition = rules_engine.can_transition(&resource, &activity).await.unwrap();
        assert!(can_transition);
    }
}
```

### Integration Tests

```rust
// tests/integration/workflow_tests.rs
use circuit_breaker_sdk::*;

#[tokio::test]
async fn test_complete_workflow_lifecycle() {
    let sdk = CircuitBreakerSDK::new(test_config()).await.unwrap();

    // Create workflow
    let workflow = sdk.workflows()
        .builder("Integration Test Workflow")
        .add_state("pending")
        .add_state("completed")
        .add_transition("pending", "completed", "complete")
        .set_initial_state("pending")
        .build()
        .unwrap();

    let workflow_id = sdk.workflows().create(workflow).await.unwrap();

    // Create resource
    let resource_id = sdk.resources().create(ResourceCreateInput {
        workflow_id: workflow_id.clone(),
        initial_state: "pending".to_string(),
        data: serde_json::json!({"test": "data"}),
        metadata: HashMap::new(),
    }).await.unwrap();

    // Execute activity
    let result = sdk.resources().execute_activity(ActivityExecuteInput {
        resource_id: resource_id.clone(),
        activity_id: "complete".to_string(),
        data: Some(serde_json::json!({"completed": true})),
    }).await.unwrap();

    assert_eq!(result.state, "completed");
}
```

### End-to-End Tests

```rust
// tests/e2e/full_system_tests.rs
use circuit_breaker_sdk::*;

#[tokio::test]
async fn test_full_ai_workflow_system() {
    let sdk = CircuitBreakerSDK::new(production_config()).await.unwrap();

    // Create AI-enhanced workflow
    let workflow = sdk.workflows()
        .builder("AI Customer Support")
        .add_state("received")
        .add_state("analyzing")
        .add_state("responding")
        .add_state("resolved")
        .add_transition("received", "analyzing", "start_analysis")
        .add_transition("analyzing", "responding", "generate_response")
        .add_transition("responding", "resolved", "send_response")
        .set_initial_state("received")
        .build()
        .unwrap();

    let workflow_id = sdk.workflows().create(workflow).await.unwrap();

    // Create AI function for analysis
    let analysis_function = FunctionDefinition {
        id: "sentiment-analysis".to_string(),
        name: "Customer Sentiment Analysis".to_string(),
        container: ContainerConfig {
            image: "python:3.11-slim".to_string(),
            command: Some(vec![
                "python".to_string(),
                "-c".to_string(),
                "import json; print(json.dumps({'sentiment': 'positive', 'confidence': 0.85}))".to_string()
            ]),
            environment: HashMap::new(),
            mounts: Vec::new(),
            resources: None,
            working_dir: None,
        },
        triggers: vec![EventTrigger {
            trigger_type: EventTriggerType::ResourceState,
            condition: "state == 'analyzing'".to_string(),
            input_mapping: Some(InputMapping::FullData),
        }],
        chains: Vec::new(),
        input_schema: None,
        output_schema: None,
        metadata: HashMap::new(),
    };

    sdk.functions().create_function(analysis_function).await.unwrap();

    // Create conversational agent
    let agent = sdk.agent_builder("Support Agent")
        .conversational()
        .set_system_prompt("You are a helpful customer support agent")
        .set_llm_provider("openai-gpt4")
        .enable_memory(true)
        .build()
        .await
        .unwrap();

    // Test complete flow
    let resource_id = sdk.resources().create(ResourceCreateInput {
        workflow_id,
        initial_state: "received".to_string(),
        data: serde_json::json!({
            "customer_message": "I'm having trouble with my order",
            "customer_id": "cust_123"
        }),
        metadata: HashMap::new(),
    }).await.unwrap();

    // Execute the workflow through all states
    sdk.resources().execute_activity(ActivityExecuteInput {
        resource_id: resource_id.clone(),
        activity_id: "start_analysis".to_string(),
        data: None,
    }).await.unwrap();

    // Verify functions were triggered and agent responded
    // This would involve polling for completion and verifying results
    
    let final_resource = sdk.resources().get(&resource_id).await.unwrap();
    assert_eq!(final_resource.state, "resolved");
}
```

## Documentation Plan

### API Reference
- Comprehensive rustdoc documentation for all public APIs
- Type definitions and trait bounds
- Method signatures with examples
- Error conditions and handling

### Getting Started Guide
- Installation via Cargo
- Basic concepts and terminology
- Step-by-step tutorials
- Common patterns and best practices

### Advanced Topics
- Custom provider implementation
- Plugin development with trait objects
- Performance tuning and optimization
- Scaling considerations and async patterns

## Distribution Strategy

### Cargo Package Structure

```toml
[package]
name = "circuit-breaker-sdk"
version = "1.0.0"
edition = "2021"
authors = ["Circuit Breaker Team"]
description = "Rust SDK for Circuit Breaker workflow engine"
license = "MIT OR Apache-2.0"
readme = "README.md"
homepage = "https://github.com/circuit-breaker/sdk"
repository = "https://github.com/circuit-breaker/sdk"
documentation = "https://docs.rs/circuit-breaker-sdk"
keywords = ["workflow", "automation", "ai", "llm", "graphql"]
categories = ["web-programming", "asynchronous", "api-bindings"]

[dependencies]
# Core async runtime
tokio = { version = "1.0", features = ["full"] }
async-trait = "0.1"
futures = "0.3"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
uuid = { version = "1.0", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }

# HTTP client
reqwest = { version = "0.11", features = ["json", "stream"] }

# GraphQL
graphql-client = "0.13"

# Docker integration
bollard = "0.15"

# Error handling
thiserror = "1.0"
anyhow = "1.0"

# Logging
tracing = "0.1"

# Streaming
async-stream = "0.3"
pin-project-lite = "0.2"

# WebSocket
tokio-tungstenite = "0.20"

[dev-dependencies]
tokio-test = "0.4"
wiremock = "0.5"
tempfile = "3.0"

[features]
default = ["docker", "llm", "agents"]
docker = ["bollard"]
llm = []
agents = ["llm"]
full = ["docker", "llm", "agents"]

[[example]]
name = "basic_workflow"
path = "examples/basic_workflow.rs"

[[example]]
name = "ai_agent"
path = "examples/ai_agent.rs"
required-features = ["agents"]

[[example]]
name = "function_chains"
path = "examples/function_chains.rs"
required-features = ["docker"]

[[example]]
name = "llm_integration"
path = "examples/llm_integration.rs"
required-features = ["llm"]
```

### Build Configuration
- Cargo features for optional functionality
- Cross-compilation support for multiple platforms
- WASM compatibility for browser usage
- Optimized release builds with LTO

## Success Metrics

### Developer Experience
- Time to first successful workflow: < 3 minutes
- API learning curve: Intuitive for Rust developers
- Documentation completeness: 100% API coverage with examples
- Community adoption: crates.io downloads, GitHub stars

### Technical Performance
- Compile time optimization
- Runtime performance benchmarks
- Memory usage profiling
- Async performance analysis

### Reliability
- Error handling coverage with Result types
- Graceful degradation patterns
- Provider failover success rate
- Integration test coverage > 95%

### Type Safety
- Compile-time guarantees for workflow definitions
- Strong typing for all API interactions
- Zero-cost abstractions where possible
- Minimal runtime errors through design

## Advanced Features

### Macro Support for Workflow Definition

```rust
use circuit_breaker_sdk::workflow;

// Declarative workflow definition with macros
workflow! {
    name: "Order Processing",
    initial_state: "pending",
    
    states: [
        "pending",
        "processing", 
        "completed",
        "cancelled"
    ],
    
    transitions: [
        "pending" -> "processing" via "start_processing",
        "processing" -> "completed" via "complete_order",
        "processing" -> "cancelled" via "cancel_order",
        "pending" -> "cancelled" via "cancel_order"
    ],
    
    rules: [
        "start_processing" requires |data| data["payment_verified"].as_bool() == Some(true),
        "complete_order" requires |data| data["items_shipped"].as_bool() == Some(true)
    ]
}
```

### Plugin System

```rust
use async_trait::async_trait;

#[async_trait]
pub trait SDKPlugin: Send + Sync {
    fn name(&self) -> &str;
    async fn initialize(&mut self, sdk: &CircuitBreakerSDK) -> Result<(), Box<dyn std::error::Error>>;
    async fn on_workflow_created(&self, workflow: &WorkflowDefinition) -> Result<(), Box<dyn std::error::Error>>;
    async fn on_resource_state_changed(&self, resource: &Resource, old_state: &str) -> Result<(), Box<dyn std::error::Error>>;
}

impl CircuitBreakerSDK {
    pub async fn register_plugin(&mut self, plugin: Box<dyn SDKPlugin>) -> Result<(), SDKError> {
        // Plugin registration logic
        Ok(())
    }
}
```

This comprehensive implementation plan provides a robust foundation for building a production-ready Rust SDK that leverages Rust's strengths in type safety, performance, and ergonomic API design while making Circuit Breaker accessible to Rust developers.