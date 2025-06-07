//! Cost Optimization and Budget Management for LLM Router
//! 
//! This module provides comprehensive cost tracking, budget management,
//! and intelligent routing decisions based on cost optimization rules.

use super::*;
use std::collections::BTreeMap;
use std::cmp::Ordering;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::Datelike;

/// Cost optimization engine
pub struct CostOptimizer {
    budget_manager: Arc<BudgetManager>,
    cost_analyzer: Arc<CostAnalyzer>,
    optimization_rules: Arc<RwLock<Vec<OptimizationRule>>>,
    cost_history: Arc<RwLock<BTreeMap<DateTime<Utc>, Vec<CostInfo>>>>,
}

impl CostOptimizer {
    pub fn new(
        budget_manager: Arc<BudgetManager>,
        cost_analyzer: Arc<CostAnalyzer>,
    ) -> Self {
        let mut rules = Vec::new();
        
        // Default optimization rules
        rules.push(OptimizationRule {
            name: "Cost Threshold Switch".to_string(),
            condition: RuleCondition::CostPerTokenAbove(0.001),
            action: RuleAction::SwitchToProvider(LLMProviderType::Anthropic),
            priority: 1,
            enabled: true,
        });
        
        rules.push(OptimizationRule {
            name: "Daily Budget Warning".to_string(),
            condition: RuleCondition::DailyBudgetUsageAbove(0.8),
            action: RuleAction::SwitchToProvider(LLMProviderType::OpenAI),
            priority: 2,
            enabled: true,
        });
        
        rules.push(OptimizationRule {
            name: "High Volume Discount".to_string(),
            condition: RuleCondition::DailyTokensAbove(100000),
            action: RuleAction::PreferProvider(LLMProviderType::Together),
            priority: 3,
            enabled: true,
        });

        Self {
            budget_manager,
            cost_analyzer,
            optimization_rules: Arc::new(RwLock::new(rules)),
            cost_history: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }

    /// Analyze costs and suggest optimal provider
    pub async fn suggest_provider(
        &self,
        request: &LLMRequest,
        available_providers: &[LLMProviderType],
        user_id: &str,
        project_id: Option<&str>,
    ) -> Result<ProviderRecommendation, CostError> {
        let context = CostContext {
            user_id: user_id.to_string(),
            project_id: project_id.map(|s| s.to_string()),
            request_size: self.estimate_request_tokens(request),
            expected_output_tokens: request.max_tokens.unwrap_or(1000),
            current_time: Utc::now(),
        };

        // Check budget constraints
        let budget_status = self.budget_manager.check_budget(&context).await?;
        if budget_status.is_exhausted {
            return Err(CostError::BudgetExhausted(budget_status.message));
        }

        // Get cost estimates for each provider
        let mut provider_costs = Vec::new();
        for provider in available_providers {
            if let Ok(estimate) = self.cost_analyzer.estimate_cost(
                provider,
                &request.model,
                context.request_size,
                context.expected_output_tokens,
            ).await {
                provider_costs.push(ProviderCostEstimate {
                    provider: provider.clone(),
                    estimated_cost: estimate.total_cost,
                    cost_per_token: estimate.cost_per_token,
                    confidence: estimate.confidence,
                    latency_penalty: self.calculate_latency_penalty(provider).await,
                });
            }
        }

        if provider_costs.is_empty() {
            return Err(CostError::NoValidProviders);
        }

        // Apply optimization rules
        let rules = self.optimization_rules.read().await;
        let mut recommendations = self.apply_optimization_rules(&rules, &provider_costs, &context).await?;

        // Sort by total score (cost + penalties)
        recommendations.sort_by(|a, b| a.total_score.partial_cmp(&b.total_score).unwrap_or(Ordering::Equal));

        // Select best recommendation
        let best_recommendation = recommendations.into_iter().next()
            .ok_or(CostError::NoValidProviders)?;

        // Record the decision for future optimization
        self.record_routing_decision(&context, &best_recommendation).await;

        Ok(best_recommendation)
    }

    /// Apply optimization rules to provider costs
    async fn apply_optimization_rules(
        &self,
        rules: &[OptimizationRule],
        provider_costs: &[ProviderCostEstimate],
        context: &CostContext,
    ) -> Result<Vec<ProviderRecommendation>, CostError> {
        let mut recommendations = Vec::new();

        for cost_estimate in provider_costs {
            let mut recommendation = ProviderRecommendation {
                provider: cost_estimate.provider.clone(),
                estimated_cost: cost_estimate.estimated_cost,
                confidence: cost_estimate.confidence,
                reasons: Vec::new(),
                total_score: cost_estimate.estimated_cost,
                optimization_applied: Vec::new(),
            };

            // Apply each rule
            for rule in rules {
                if !rule.enabled {
                    continue;
                }

                if self.evaluate_rule_condition(&rule.condition, cost_estimate, context).await? {
                    match &rule.action {
                        RuleAction::SwitchToProvider(target_provider) => {
                            if cost_estimate.provider == *target_provider {
                                recommendation.total_score *= 0.8; // 20% bonus
                                recommendation.reasons.push(format!("Switched to {} due to rule: {}", target_provider, rule.name));
                                recommendation.optimization_applied.push(rule.name.clone());
                            }
                        }
                        RuleAction::PreferProvider(preferred_provider) => {
                            if cost_estimate.provider == *preferred_provider {
                                recommendation.total_score *= 0.9; // 10% bonus
                                recommendation.reasons.push(format!("Preferred {} due to rule: {}", preferred_provider, rule.name));
                                recommendation.optimization_applied.push(rule.name.clone());
                            }
                        }
                        RuleAction::ApplyCostMultiplier(multiplier) => {
                            recommendation.total_score *= multiplier;
                            recommendation.reasons.push(format!("Applied cost multiplier {} due to rule: {}", multiplier, rule.name));
                            recommendation.optimization_applied.push(rule.name.clone());
                        }
                        RuleAction::BlockProvider => {
                            recommendation.total_score = f64::MAX; // Effectively block
                            recommendation.reasons.push(format!("Provider blocked due to rule: {}", rule.name));
                            recommendation.optimization_applied.push(rule.name.clone());
                        }
                    }
                }
            }

            // Apply latency penalty
            recommendation.total_score += cost_estimate.latency_penalty;

            recommendations.push(recommendation);
        }

        Ok(recommendations)
    }

    /// Evaluate a rule condition
    async fn evaluate_rule_condition(
        &self,
        condition: &RuleCondition,
        cost_estimate: &ProviderCostEstimate,
        context: &CostContext,
    ) -> Result<bool, CostError> {
        match condition {
            RuleCondition::CostPerTokenAbove(threshold) => {
                Ok(cost_estimate.cost_per_token > *threshold)
            }
            RuleCondition::TotalCostAbove(threshold) => {
                Ok(cost_estimate.estimated_cost > *threshold)
            }
            RuleCondition::DailyBudgetUsageAbove(percentage) => {
                let usage = self.budget_manager.get_daily_usage(&context.user_id, context.project_id.as_deref()).await?;
                Ok(usage.percentage_used > *percentage)
            }
            RuleCondition::MonthlyBudgetUsageAbove(percentage) => {
                let usage = self.budget_manager.get_monthly_usage(&context.user_id, context.project_id.as_deref()).await?;
                Ok(usage.percentage_used > *percentage)
            }
            RuleCondition::DailyTokensAbove(threshold) => {
                let today = context.current_time.date_naive();
                let day_start = today.and_hms_opt(0, 0, 0).unwrap().and_local_timezone(Utc).unwrap();
                let tokens_used = self.get_tokens_used_since(&context.user_id, day_start).await?;
                Ok(tokens_used > *threshold)
            }
            RuleCondition::ProviderErrorRateAbove(provider, threshold) => {
                let error_rate = self.get_provider_error_rate(provider).await?;
                Ok(error_rate > *threshold)
            }
            RuleCondition::ProviderLatencyAbove(provider, threshold_ms) => {
                let avg_latency = self.get_provider_average_latency(provider).await?;
                Ok(avg_latency > *threshold_ms)
            }
        }
    }

    /// Estimate tokens in a request
    fn estimate_request_tokens(&self, request: &LLMRequest) -> u32 {
        // Simple token estimation: ~4 characters per token
        let total_chars: usize = request.messages.iter()
            .map(|msg| msg.content.len())
            .sum();
        (total_chars / 4) as u32
    }

    /// Calculate latency penalty for provider selection
    async fn calculate_latency_penalty(&self, provider: &LLMProviderType) -> f64 {
        // Simple latency penalty based on historical performance
        match provider {
            LLMProviderType::OpenAI => 0.001,
            LLMProviderType::Anthropic => 0.002,
            LLMProviderType::Google => 0.0015,
            LLMProviderType::Groq => 0.0005, // Fastest
            _ => 0.003,
        }
    }

    /// Record routing decision for future optimization
    async fn record_routing_decision(&self, context: &CostContext, recommendation: &ProviderRecommendation) {
        let decision = RoutingDecision {
            timestamp: context.current_time,
            user_id: context.user_id.clone(),
            project_id: context.project_id.clone(),
            provider: recommendation.provider.clone(),
            estimated_cost: recommendation.estimated_cost,
            reasons: recommendation.reasons.clone(),
            optimization_applied: recommendation.optimization_applied.clone(),
        };

        // Store for analysis (simplified storage)
        tracing::debug!("Routing decision recorded: {:?}", decision);
    }

    /// Get tokens used since a specific time
    async fn get_tokens_used_since(&self, user_id: &str, since: DateTime<Utc>) -> Result<u32, CostError> {
        let history = self.cost_history.read().await;
        let mut total_tokens = 0;

        for (_timestamp, costs) in history.range(since..) {
            for cost in costs {
                if cost.user_id.as_ref() == Some(&user_id.to_string()) {
                    total_tokens += cost.input_tokens + cost.output_tokens;
                }
            }
        }

        Ok(total_tokens)
    }

    /// Get provider error rate
    async fn get_provider_error_rate(&self, provider: &LLMProviderType) -> Result<f64, CostError> {
        // Simplified error rate calculation
        Ok(match provider {
            LLMProviderType::OpenAI => 0.01,
            LLMProviderType::Anthropic => 0.005,
            LLMProviderType::Google => 0.02,
            _ => 0.03,
        })
    }

    /// Get provider average latency
    async fn get_provider_average_latency(&self, provider: &LLMProviderType) -> Result<u64, CostError> {
        // Simplified latency calculation
        Ok(match provider {
            LLMProviderType::Groq => 200,
            LLMProviderType::OpenAI => 800,
            LLMProviderType::Anthropic => 1200,
            LLMProviderType::Google => 1000,
            _ => 1500,
        })
    }

    /// Add or update optimization rule
    pub async fn add_optimization_rule(&self, rule: OptimizationRule) {
        let mut rules = self.optimization_rules.write().await;
        if let Some(existing) = rules.iter_mut().find(|r| r.name == rule.name) {
            *existing = rule;
        } else {
            rules.push(rule);
        }
        
        // Sort by priority
        rules.sort_by(|a, b| a.priority.cmp(&b.priority));
    }

    /// Remove optimization rule
    pub async fn remove_optimization_rule(&self, rule_name: &str) {
        let mut rules = self.optimization_rules.write().await;
        rules.retain(|r| r.name != rule_name);
    }

    /// Get all optimization rules
    pub async fn get_optimization_rules(&self) -> Vec<OptimizationRule> {
        let rules = self.optimization_rules.read().await;
        rules.clone()
    }

    /// Record actual cost for learning and optimization
    pub async fn record_actual_cost(&self, cost_info: CostInfo) {
        let mut history = self.cost_history.write().await;
        let day_key = cost_info.timestamp.date_naive().and_hms_opt(0, 0, 0)
            .unwrap().and_local_timezone(Utc).unwrap();
        
        history.entry(day_key).or_insert_with(Vec::new).push(cost_info);
        
        // Keep only last 30 days
        let cutoff = Utc::now() - chrono::Duration::days(30);
        history.retain(|k, _| *k >= cutoff);
    }

    /// Get cost analytics for a time period
    pub async fn get_cost_analytics(
        &self,
        user_id: Option<&str>,
        project_id: Option<&str>,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> CostAnalytics {
        let history = self.cost_history.read().await;
        let mut total_cost = 0.0;
        let mut total_tokens = 0;
        let mut provider_costs = HashMap::new();
        let mut model_costs = HashMap::new();
        let mut daily_costs = BTreeMap::new();

        for (day, costs) in history.range(start..=end) {
            let mut day_cost = 0.0;
            
            for cost in costs {
                let matches_filter = user_id.map_or(true, |uid| cost.user_id.as_ref() == Some(&uid.to_string())) &&
                                   project_id.map_or(true, |pid| cost.project_id.as_ref() == Some(&pid.to_string()));
                
                if matches_filter {
                    total_cost += cost.cost_usd;
                    total_tokens += cost.input_tokens + cost.output_tokens;
                    day_cost += cost.cost_usd;
                    
                    *provider_costs.entry(cost.provider.clone()).or_insert(0.0) += cost.cost_usd;
                    *model_costs.entry(cost.model.clone()).or_insert(0.0) += cost.cost_usd;
                }
            }
            
            daily_costs.insert(*day, day_cost);
        }

        CostAnalytics {
            total_cost,
            total_tokens,
            average_cost_per_token: if total_tokens > 0 { total_cost / total_tokens as f64 } else { 0.0 },
            provider_breakdown: provider_costs,
            model_breakdown: model_costs,
            daily_costs,
            period_start: start,
            period_end: end,
        }
    }
}

/// Budget manager for tracking and enforcing spending limits
pub struct BudgetManager {
    budgets: Arc<RwLock<HashMap<String, Budget>>>,
    usage_tracker: Arc<InMemoryUsageTracker>,
}

impl BudgetManager {
    pub fn new(usage_tracker: Arc<InMemoryUsageTracker>) -> Self {
        Self {
            budgets: Arc::new(RwLock::new(HashMap::new())),
            usage_tracker,
        }
    }

    /// Set budget for user or project
    pub async fn set_budget(&self, budget: Budget) {
        let mut budgets = self.budgets.write().await;
        budgets.insert(budget.id.clone(), budget);
    }

    /// Check budget status
    pub async fn check_budget(&self, context: &CostContext) -> Result<BudgetStatus, CostError> {
        let budget_id = if let Some(project_id) = &context.project_id {
            format!("project:{}", project_id)
        } else {
            format!("user:{}", context.user_id)
        };

        let budgets = self.budgets.read().await;
        if let Some(budget) = budgets.get(&budget_id) {
            let current_usage = match budget.period {
                BudgetPeriod::Daily => {
                    self.usage_tracker.get_daily_usage(&context.user_id, context.project_id.as_deref()).await?
                }
                BudgetPeriod::Monthly => {
                    self.usage_tracker.get_monthly_usage(&context.user_id, context.project_id.as_deref()).await?
                }
                BudgetPeriod::Yearly => {
                    self.usage_tracker.get_yearly_usage(&context.user_id, context.project_id.as_deref()).await?
                }
            };

            let percentage_used = if budget.limit > 0.0 {
                current_usage.total_cost / budget.limit
            } else {
                0.0
            };

            let is_exhausted = current_usage.total_cost >= budget.limit;
            let is_warning = percentage_used >= budget.warning_threshold;

            Ok(BudgetStatus {
                budget_id: budget.id.clone(),
                limit: budget.limit,
                used: current_usage.total_cost,
                percentage_used,
                is_exhausted,
                is_warning,
                remaining: budget.limit - current_usage.total_cost,
                message: if is_exhausted {
                    format!("Budget exhausted: ${:.2} of ${:.2} used", current_usage.total_cost, budget.limit)
                } else if is_warning {
                    format!("Budget warning: {:.1}% of budget used", percentage_used * 100.0)
                } else {
                    format!("Budget healthy: ${:.2} of ${:.2} used", current_usage.total_cost, budget.limit)
                },
            })
        } else {
            // No budget set - unlimited
            Ok(BudgetStatus {
                budget_id: budget_id,
                limit: f64::MAX,
                used: 0.0,
                percentage_used: 0.0,
                is_exhausted: false,
                is_warning: false,
                remaining: f64::MAX,
                message: "No budget limit set".to_string(),
            })
        }
    }

    /// Get daily usage
    pub async fn get_daily_usage(&self, user_id: &str, project_id: Option<&str>) -> Result<UsageInfo, CostError> {
        self.usage_tracker.get_daily_usage(user_id, project_id).await
    }

    /// Get monthly usage
    pub async fn get_monthly_usage(&self, user_id: &str, project_id: Option<&str>) -> Result<UsageInfo, CostError> {
        self.usage_tracker.get_monthly_usage(user_id, project_id).await
    }

    /// Get all budgets
    pub async fn get_all_budgets(&self) -> HashMap<String, Budget> {
        let budgets = self.budgets.read().await;
        budgets.clone()
    }
}

/// Cost analyzer for estimating and tracking costs
pub struct CostAnalyzer {
    provider_pricing: HashMap<LLMProviderType, HashMap<String, ModelPricing>>,
}

impl CostAnalyzer {
    pub fn new() -> Self {
        let mut analyzer = Self {
            provider_pricing: HashMap::new(),
        };
        
        analyzer.load_default_pricing();
        analyzer
    }

    /// Load default pricing information
    fn load_default_pricing(&mut self) {
        // OpenAI pricing
        let mut openai_pricing = HashMap::new();
        openai_pricing.insert("gpt-4".to_string(), ModelPricing {
            input_cost_per_token: 0.00003,
            output_cost_per_token: 0.00006,
            context_window: 8192,
        });
        openai_pricing.insert("gpt-3.5-turbo".to_string(), ModelPricing {
            input_cost_per_token: 0.000001,
            output_cost_per_token: 0.000002,
            context_window: 4096,
        });
        self.provider_pricing.insert(LLMProviderType::OpenAI, openai_pricing);

        // Anthropic pricing
        let mut anthropic_pricing = HashMap::new();
        anthropic_pricing.insert("claude-3-opus".to_string(), ModelPricing {
            input_cost_per_token: 0.000015,
            output_cost_per_token: 0.000075,
            context_window: 200000,
        });
        anthropic_pricing.insert("claude-3-sonnet".to_string(), ModelPricing {
            input_cost_per_token: 0.000003,
            output_cost_per_token: 0.000015,
            context_window: 200000,
        });
        anthropic_pricing.insert("claude-4".to_string(), ModelPricing {
            input_cost_per_token: 0.000003,
            output_cost_per_token: 0.000015,
            context_window: 200000,
        });
        self.provider_pricing.insert(LLMProviderType::Anthropic, anthropic_pricing);
    }

    /// Estimate cost for a request
    pub async fn estimate_cost(
        &self,
        provider: &LLMProviderType,
        model: &str,
        input_tokens: u32,
        output_tokens: u32,
    ) -> Result<CostEstimate, CostError> {
        if let Some(provider_models) = self.provider_pricing.get(provider) {
            if let Some(pricing) = provider_models.get(model) {
                let input_cost = input_tokens as f64 * pricing.input_cost_per_token;
                let output_cost = output_tokens as f64 * pricing.output_cost_per_token;
                let total_cost = input_cost + output_cost;

                Ok(CostEstimate {
                    input_cost,
                    output_cost,
                    total_cost,
                    cost_per_token: total_cost / (input_tokens + output_tokens) as f64,
                    confidence: 0.9, // High confidence for known models
                })
            } else {
                // Unknown model - estimate based on similar models
                let avg_pricing = self.get_average_pricing_for_provider(provider);
                let input_cost = input_tokens as f64 * avg_pricing.input_cost_per_token;
                let output_cost = output_tokens as f64 * avg_pricing.output_cost_per_token;
                let total_cost = input_cost + output_cost;

                Ok(CostEstimate {
                    input_cost,
                    output_cost,
                    total_cost,
                    cost_per_token: total_cost / (input_tokens + output_tokens) as f64,
                    confidence: 0.5, // Lower confidence for estimates
                })
            }
        } else {
            Err(CostError::UnknownProvider(provider.to_string()))
        }
    }

    /// Get average pricing for a provider
    fn get_average_pricing_for_provider(&self, provider: &LLMProviderType) -> ModelPricing {
        if let Some(provider_models) = self.provider_pricing.get(provider) {
            let count = provider_models.len() as f64;
            let total_input = provider_models.values().map(|p| p.input_cost_per_token).sum::<f64>();
            let total_output = provider_models.values().map(|p| p.output_cost_per_token).sum::<f64>();
            let avg_context = provider_models.values().map(|p| p.context_window).sum::<u32>() / provider_models.len() as u32;

            ModelPricing {
                input_cost_per_token: total_input / count,
                output_cost_per_token: total_output / count,
                context_window: avg_context,
            }
        } else {
            // Default fallback pricing
            ModelPricing {
                input_cost_per_token: 0.00001,
                output_cost_per_token: 0.00002,
                context_window: 4096,
            }
        }
    }

    /// Update pricing for a model
    pub fn update_model_pricing(&mut self, provider: LLMProviderType, model: String, pricing: ModelPricing) {
        self.provider_pricing
            .entry(provider)
            .or_insert_with(HashMap::new)
            .insert(model, pricing);
    }
}

/// Data structures

#[derive(Debug, Clone)]
pub struct CostContext {
    pub user_id: String,
    pub project_id: Option<String>,
    pub request_size: u32,
    pub expected_output_tokens: u32,
    pub current_time: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ProviderCostEstimate {
    pub provider: LLMProviderType,
    pub estimated_cost: f64,
    pub cost_per_token: f64,
    pub confidence: f64,
    pub latency_penalty: f64,
}

#[derive(Debug, Clone)]
pub struct ProviderRecommendation {
    pub provider: LLMProviderType,
    pub estimated_cost: f64,
    pub confidence: f64,
    pub reasons: Vec<String>,
    pub total_score: f64,
    pub optimization_applied: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct OptimizationRule {
    pub name: String,
    pub condition: RuleCondition,
    pub action: RuleAction,
    pub priority: u32,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub enum RuleCondition {
    CostPerTokenAbove(f64),
    TotalCostAbove(f64),
    DailyBudgetUsageAbove(f64),
    MonthlyBudgetUsageAbove(f64),
    DailyTokensAbove(u32),
    ProviderErrorRateAbove(LLMProviderType, f64),
    ProviderLatencyAbove(LLMProviderType, u64),
}

#[derive(Debug, Clone)]
pub enum RuleAction {
    SwitchToProvider(LLMProviderType),
    PreferProvider(LLMProviderType),
    ApplyCostMultiplier(f64),
    BlockProvider,
}

#[derive(Debug, Clone)]
pub struct Budget {
    pub id: String,
    pub user_id: Option<String>,
    pub project_id: Option<String>,
    pub limit: f64,
    pub period: BudgetPeriod,
    pub warning_threshold: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub enum BudgetPeriod {
    Daily,
    Monthly,
    Yearly,
}

#[derive(Debug, Clone)]
pub struct BudgetStatus {
    pub budget_id: String,
    pub limit: f64,
    pub used: f64,
    pub percentage_used: f64,
    pub is_exhausted: bool,
    pub is_warning: bool,
    pub remaining: f64,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct CostEstimate {
    pub input_cost: f64,
    pub output_cost: f64,
    pub total_cost: f64,
    pub cost_per_token: f64,
    pub confidence: f64,
}

#[derive(Debug, Clone)]
pub struct ModelPricing {
    pub input_cost_per_token: f64,
    pub output_cost_per_token: f64,
    pub context_window: u32,
}

#[derive(Debug, Clone)]
pub struct UsageInfo {
    pub total_cost: f64,
    pub total_tokens: u32,
    pub request_count: u32,
    pub percentage_used: f64,
}

#[derive(Debug, Clone)]
pub struct CostAnalytics {
    pub total_cost: f64,
    pub total_tokens: u32,
    pub average_cost_per_token: f64,
    pub provider_breakdown: HashMap<LLMProviderType, f64>,
    pub model_breakdown: HashMap<String, f64>,
    pub daily_costs: BTreeMap<DateTime<Utc>, f64>,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct RoutingDecision {
    pub timestamp: DateTime<Utc>,
    pub user_id: String,
    pub project_id: Option<String>,
    pub provider: LLMProviderType,
    pub estimated_cost: f64,
    pub reasons: Vec<String>,
    pub optimization_applied: Vec<String>,
}

/// Usage tracker trait
#[async_trait::async_trait]
pub trait UsageTracker: Send + Sync {
    async fn get_daily_usage(&self, user_id: &str, project_id: Option<&str>) -> Result<UsageInfo, CostError>;
    async fn get_monthly_usage(&self, user_id: &str, project_id: Option<&str>) -> Result<UsageInfo, CostError>;
    async fn get_yearly_usage(&self, user_id: &str, project_id: Option<&str>) -> Result<UsageInfo, CostError>;
    async fn record_usage(&self, cost_info: &CostInfo) -> Result<(), CostError>;
}

/// Cost optimization errors
#[derive(Debug, thiserror::Error)]
pub enum CostError {
    #[error("Budget exhausted: {0}")]
    BudgetExhausted(String),
    
    #[error("No valid providers available")]
    NoValidProviders,
    
    #[error("Unknown provider: {0}")]
    UnknownProvider(String),
    
    #[error("Usage tracking error: {0}")]
    UsageTracking(String),
    
    #[error("Budget management error: {0}")]
    BudgetManagement(String),
    
    #[error("Internal cost optimization error: {0}")]
    Internal(String),
}

/// In-memory usage tracker for development
pub struct InMemoryUsageTracker {
    usage_data: Arc<RwLock<HashMap<String, Vec<CostInfo>>>>,
}

impl InMemoryUsageTracker {
    pub fn new() -> Self {
        Self {
            usage_data: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl UsageTracker for InMemoryUsageTracker {
    async fn get_daily_usage(&self, user_id: &str, project_id: Option<&str>) -> Result<UsageInfo, CostError> {
        let usage_data = self.usage_data.read().await;
        let key = if let Some(pid) = project_id {
            format!("project:{}", pid)
        } else {
            format!("user:{}", user_id)
        };
        
        if let Some(costs) = usage_data.get(&key) {
            let today = Utc::now().date_naive();
            let day_start = today.and_hms_opt(0, 0, 0).unwrap().and_local_timezone(Utc).unwrap();
            
            let daily_costs: Vec<_> = costs.iter()
                .filter(|cost| cost.timestamp >= day_start)
                .collect();
            
            let total_cost = daily_costs.iter().map(|c| c.cost_usd).sum();
            let total_tokens = daily_costs.iter().map(|c| c.input_tokens + c.output_tokens).sum();
            let request_count = daily_costs.len() as u32;
            
            Ok(UsageInfo {
                total_cost,
                total_tokens,
                request_count,
                percentage_used: 0.0,
            })
        } else {
            Ok(UsageInfo {
                total_cost: 0.0,
                total_tokens: 0,
                request_count: 0,
                percentage_used: 0.0,
            })
        }
    }

    async fn get_monthly_usage(&self, user_id: &str, project_id: Option<&str>) -> Result<UsageInfo, CostError> {
        let usage_data = self.usage_data.read().await;
        let key = if let Some(pid) = project_id {
            format!("project:{}", pid)
        } else {
            format!("user:{}", user_id)
        };
        
        if let Some(costs) = usage_data.get(&key) {
            let now = Utc::now();
            let month_start = now.date_naive()
                .with_day(1).unwrap()
                .and_hms_opt(0, 0, 0).unwrap()
                .and_local_timezone(Utc).unwrap();
            
            let monthly_costs: Vec<_> = costs.iter()
                .filter(|cost| cost.timestamp >= month_start)
                .collect();
            
            let total_cost = monthly_costs.iter().map(|c| c.cost_usd).sum();
            let total_tokens = monthly_costs.iter().map(|c| c.input_tokens + c.output_tokens).sum();
            let request_count = monthly_costs.len() as u32;
            
            Ok(UsageInfo {
                total_cost,
                total_tokens,
                request_count,
                percentage_used: 0.0,
            })
        } else {
            Ok(UsageInfo {
                total_cost: 0.0,
                total_tokens: 0,
                request_count: 0,
                percentage_used: 0.0,
            })
        }
    }

    async fn get_yearly_usage(&self, user_id: &str, project_id: Option<&str>) -> Result<UsageInfo, CostError> {
        let usage_data = self.usage_data.read().await;
        let key = if let Some(pid) = project_id {
            format!("project:{}", pid)
        } else {
            format!("user:{}", user_id)
        };
        
        if let Some(costs) = usage_data.get(&key) {
            let now = Utc::now();
            let year_start = now.date_naive()
                .with_ordinal(1).unwrap()
                .and_hms_opt(0, 0, 0).unwrap()
                .and_local_timezone(Utc).unwrap();
            
            let yearly_costs: Vec<_> = costs.iter()
                .filter(|cost| cost.timestamp >= year_start)
                .collect();
            
            let total_cost = yearly_costs.iter().map(|c| c.cost_usd).sum();
            let total_tokens = yearly_costs.iter().map(|c| c.input_tokens + c.output_tokens).sum();
            let request_count = yearly_costs.len() as u32;
            
            Ok(UsageInfo {
                total_cost,
                total_tokens,
                request_count,
                percentage_used: 0.0,
            })
        } else {
            Ok(UsageInfo {
                total_cost: 0.0,
                total_tokens: 0,
                request_count: 0,
                percentage_used: 0.0,
            })
        }
    }

    async fn record_usage(&self, cost_info: &CostInfo) -> Result<(), CostError> {
        let mut usage_data = self.usage_data.write().await;
        
        let key = if let Some(project_id) = &cost_info.project_id {
            format!("project:{}", project_id)
        } else if let Some(user_id) = &cost_info.user_id {
            format!("user:{}", user_id)
        } else {
            return Err(CostError::UsageTracking("No user or project ID provided".to_string()));
        };
        
        usage_data.entry(key).or_insert_with(Vec::new).push(cost_info.clone());
        
        Ok(())
    }
}