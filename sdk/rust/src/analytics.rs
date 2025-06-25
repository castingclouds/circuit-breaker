//! Analytics and Budget Management Client
//!
//! This module provides functionality for managing budgets, tracking costs, and monitoring
//! analytics for the Circuit Breaker workflow automation server.
//!
//! # Examples
//!
//! ```rust
//! use circuit_breaker_sdk::{Client, Result};
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let client = Client::builder()
//!         .base_url("http://localhost:4000")?
//!         .build()?;
//!
//!     // Get budget status
//!     let budget = client.analytics()
//!         .budget_status()
//!         .user_id("user123")
//!         .get()
//!         .await?;
//!
//!     println!("Budget used: {:.2}%", budget.percentage_used);
//!
//!     // Set a budget limit
//!     let new_budget = client.analytics()
//!         .set_budget()
//!         .user_id("user123")
//!         .limit(100.0)
//!         .period("monthly")
//!         .warning_threshold(0.8)
//!         .execute()
//!         .await?;
//!
//!     // Get cost analytics
//!     let analytics = client.analytics()
//!         .cost_analytics()
//!         .user_id("user123")
//!         .date_range("2024-01-01", "2024-01-31")
//!         .get()
//!         .await?;
//!
//!     println!("Total cost: ${:.2}", analytics.total_cost);
//!
//!     Ok(())
//! }
//! ```

use crate::client::Client;
use crate::types::*;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Analytics client for budget and cost management operations
pub struct AnalyticsClient {
    client: Client,
}

impl AnalyticsClient {
    /// Create a new analytics client
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Get budget status for a user or project
    pub fn budget_status(&self) -> BudgetStatusBuilder {
        BudgetStatusBuilder::new(self.client.clone())
    }

    /// Get cost analytics for a time period
    pub fn cost_analytics(&self) -> CostAnalyticsBuilder {
        CostAnalyticsBuilder::new(self.client.clone())
    }

    /// Set budget limits
    pub fn set_budget(&self) -> SetBudgetBuilder {
        SetBudgetBuilder::new(self.client.clone())
    }

    /// Subscribe to real-time cost updates
    pub async fn subscribe_cost_updates(&self, user_id: Option<&str>) -> Result<CostUpdateStream> {
        // This would need WebSocket/SSE implementation
        // For now, return an error indicating subscriptions aren't implemented
        Err(crate::Error::Configuration {
            message: "Real-time subscriptions not yet implemented".to_string(),
        })
    }
}

/// Budget status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetStatus {
    /// Unique budget identifier
    pub budget_id: String,
    /// Budget limit amount
    pub limit: f64,
    /// Amount already used
    pub used: f64,
    /// Percentage of budget used (0.0 to 100.0)
    pub percentage_used: f64,
    /// Whether budget is exhausted
    pub is_exhausted: bool,
    /// Whether budget is in warning state
    pub is_warning: bool,
    /// Remaining budget amount
    pub remaining: f64,
    /// Status message
    pub message: String,
}

/// Cost analytics data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostAnalytics {
    /// Total cost for the period
    pub total_cost: f64,
    /// Total tokens used
    pub total_tokens: i32,
    /// Average cost per token
    pub average_cost_per_token: f64,
    /// Cost breakdown by provider
    pub provider_breakdown: HashMap<String, f64>,
    /// Cost breakdown by model
    pub model_breakdown: HashMap<String, f64>,
    /// Daily costs over the period
    pub daily_costs: HashMap<String, f64>,
    /// Start of the analytics period
    pub period_start: String,
    /// End of the analytics period
    pub period_end: String,
}

/// Budget input for setting limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetInput {
    /// User ID for user-specific budget
    pub user_id: Option<String>,
    /// Project ID for project-specific budget
    pub project_id: Option<String>,
    /// Budget limit amount
    pub limit: f64,
    /// Budget period (daily, weekly, monthly)
    pub period: String,
    /// Warning threshold (0.0 to 1.0)
    pub warning_threshold: f64,
}

/// Cost analytics input parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostAnalyticsInput {
    /// User ID to filter analytics
    pub user_id: Option<String>,
    /// Project ID to filter analytics
    pub project_id: Option<String>,
    /// Start date for analytics (ISO 8601)
    pub start_date: String,
    /// End date for analytics (ISO 8601)
    pub end_date: String,
}

/// Builder for budget status queries
pub struct BudgetStatusBuilder {
    client: Client,
    user_id: Option<String>,
    project_id: Option<String>,
}

impl BudgetStatusBuilder {
    fn new(client: Client) -> Self {
        Self {
            client,
            user_id: None,
            project_id: None,
        }
    }

    /// Set user ID for user-specific budget
    pub fn user_id<S: Into<String>>(mut self, user_id: S) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Set project ID for project-specific budget
    pub fn project_id<S: Into<String>>(mut self, project_id: S) -> Self {
        self.project_id = Some(project_id.into());
        self
    }

    /// Execute the budget status query
    pub async fn get(self) -> Result<BudgetStatus> {
        let query = r#"
            query BudgetStatus($userId: String, $projectId: String) {
                budgetStatus(userId: $userId, projectId: $projectId) {
                    budgetId
                    limit
                    used
                    percentageUsed
                    isExhausted
                    isWarning
                    remaining
                    message
                }
            }
        "#;

        #[derive(Serialize)]
        struct Variables {
            #[serde(rename = "userId")]
            user_id: Option<String>,
            #[serde(rename = "projectId")]
            project_id: Option<String>,
        }

        let variables = Variables {
            user_id: self.user_id,
            project_id: self.project_id,
        };

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "budgetStatus")]
            budget_status: BudgetStatusGQL,
        }

        #[derive(Deserialize)]
        struct BudgetStatusGQL {
            #[serde(rename = "budgetId")]
            budget_id: String,
            limit: f64,
            used: f64,
            #[serde(rename = "percentageUsed")]
            percentage_used: f64,
            #[serde(rename = "isExhausted")]
            is_exhausted: bool,
            #[serde(rename = "isWarning")]
            is_warning: bool,
            remaining: f64,
            message: String,
        }

        let response: Response = self.client.graphql_query(query, Some(variables)).await?;

        Ok(BudgetStatus {
            budget_id: response.budget_status.budget_id,
            limit: response.budget_status.limit,
            used: response.budget_status.used,
            percentage_used: response.budget_status.percentage_used,
            is_exhausted: response.budget_status.is_exhausted,
            is_warning: response.budget_status.is_warning,
            remaining: response.budget_status.remaining,
            message: response.budget_status.message,
        })
    }
}

/// Builder for cost analytics queries
pub struct CostAnalyticsBuilder {
    client: Client,
    user_id: Option<String>,
    project_id: Option<String>,
    start_date: Option<String>,
    end_date: Option<String>,
}

impl CostAnalyticsBuilder {
    fn new(client: Client) -> Self {
        Self {
            client,
            user_id: None,
            project_id: None,
            start_date: None,
            end_date: None,
        }
    }

    /// Set user ID to filter analytics
    pub fn user_id<S: Into<String>>(mut self, user_id: S) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Set project ID to filter analytics
    pub fn project_id<S: Into<String>>(mut self, project_id: S) -> Self {
        self.project_id = Some(project_id.into());
        self
    }

    /// Set date range for analytics
    pub fn date_range<S: Into<String>>(mut self, start_date: S, end_date: S) -> Self {
        self.start_date = Some(start_date.into());
        self.end_date = Some(end_date.into());
        self
    }

    /// Set start date for analytics
    pub fn start_date<S: Into<String>>(mut self, start_date: S) -> Self {
        self.start_date = Some(start_date.into());
        self
    }

    /// Set end date for analytics
    pub fn end_date<S: Into<String>>(mut self, end_date: S) -> Self {
        self.end_date = Some(end_date.into());
        self
    }

    /// Execute the cost analytics query
    pub async fn get(self) -> Result<CostAnalytics> {
        let query = r#"
            query CostAnalytics($input: CostAnalyticsInput!) {
                costAnalytics(input: $input) {
                    totalCost
                    totalTokens
                    averageCostPerToken
                    providerBreakdown
                    modelBreakdown
                    dailyCosts
                    periodStart
                    periodEnd
                }
            }
        "#;

        let input = CostAnalyticsInput {
            user_id: self.user_id,
            project_id: self.project_id,
            start_date: self.start_date.ok_or_else(|| crate::Error::Validation {
                message: "start_date is required".to_string(),
            })?,
            end_date: self.end_date.ok_or_else(|| crate::Error::Validation {
                message: "end_date is required".to_string(),
            })?,
        };

        #[derive(Serialize)]
        struct Variables {
            input: CostAnalyticsInput,
        }

        let variables = Variables { input };

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "costAnalytics")]
            cost_analytics: CostAnalyticsGQL,
        }

        #[derive(Deserialize)]
        struct CostAnalyticsGQL {
            #[serde(rename = "totalCost")]
            total_cost: f64,
            #[serde(rename = "totalTokens")]
            total_tokens: i32,
            #[serde(rename = "averageCostPerToken")]
            average_cost_per_token: f64,
            #[serde(rename = "providerBreakdown")]
            provider_breakdown: serde_json::Value,
            #[serde(rename = "modelBreakdown")]
            model_breakdown: serde_json::Value,
            #[serde(rename = "dailyCosts")]
            daily_costs: serde_json::Value,
            #[serde(rename = "periodStart")]
            period_start: String,
            #[serde(rename = "periodEnd")]
            period_end: String,
        }

        let response: Response = self.client.graphql_query(query, Some(variables)).await?;

        // Convert JSON values to HashMaps
        let provider_breakdown =
            if let Ok(map) = serde_json::from_value(response.cost_analytics.provider_breakdown) {
                map
            } else {
                HashMap::new()
            };

        let model_breakdown =
            if let Ok(map) = serde_json::from_value(response.cost_analytics.model_breakdown) {
                map
            } else {
                HashMap::new()
            };

        let daily_costs =
            if let Ok(map) = serde_json::from_value(response.cost_analytics.daily_costs) {
                map
            } else {
                HashMap::new()
            };

        Ok(CostAnalytics {
            total_cost: response.cost_analytics.total_cost,
            total_tokens: response.cost_analytics.total_tokens,
            average_cost_per_token: response.cost_analytics.average_cost_per_token,
            provider_breakdown,
            model_breakdown,
            daily_costs,
            period_start: response.cost_analytics.period_start,
            period_end: response.cost_analytics.period_end,
        })
    }
}

/// Builder for setting budget limits
pub struct SetBudgetBuilder {
    client: Client,
    user_id: Option<String>,
    project_id: Option<String>,
    limit: Option<f64>,
    period: Option<String>,
    warning_threshold: Option<f64>,
}

impl SetBudgetBuilder {
    fn new(client: Client) -> Self {
        Self {
            client,
            user_id: None,
            project_id: None,
            limit: None,
            period: None,
            warning_threshold: None,
        }
    }

    /// Set user ID for user-specific budget
    pub fn user_id<S: Into<String>>(mut self, user_id: S) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Set project ID for project-specific budget
    pub fn project_id<S: Into<String>>(mut self, project_id: S) -> Self {
        self.project_id = Some(project_id.into());
        self
    }

    /// Set budget limit amount
    pub fn limit(mut self, limit: f64) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set budget period (daily, weekly, monthly)
    pub fn period<S: Into<String>>(mut self, period: S) -> Self {
        self.period = Some(period.into());
        self
    }

    /// Set warning threshold (0.0 to 1.0)
    pub fn warning_threshold(mut self, threshold: f64) -> Self {
        self.warning_threshold = Some(threshold);
        self
    }

    /// Execute the set budget mutation
    pub async fn execute(self) -> Result<BudgetStatus> {
        let query = r#"
            mutation SetBudget($input: BudgetInput!) {
                setBudget(input: $input) {
                    budgetId
                    limit
                    used
                    percentageUsed
                    isExhausted
                    isWarning
                    remaining
                    message
                }
            }
        "#;

        let input = BudgetInput {
            user_id: self.user_id,
            project_id: self.project_id,
            limit: self.limit.ok_or_else(|| crate::Error::Validation {
                message: "limit is required".to_string(),
            })?,
            period: self.period.ok_or_else(|| crate::Error::Validation {
                message: "period is required".to_string(),
            })?,
            warning_threshold: self
                .warning_threshold
                .ok_or_else(|| crate::Error::Validation {
                    message: "warning_threshold is required".to_string(),
                })?,
        };

        #[derive(Serialize)]
        struct Variables {
            input: BudgetInput,
        }

        let variables = Variables { input };

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "setBudget")]
            set_budget: BudgetStatusGQL,
        }

        #[derive(Deserialize)]
        struct BudgetStatusGQL {
            #[serde(rename = "budgetId")]
            budget_id: String,
            limit: f64,
            used: f64,
            #[serde(rename = "percentageUsed")]
            percentage_used: f64,
            #[serde(rename = "isExhausted")]
            is_exhausted: bool,
            #[serde(rename = "isWarning")]
            is_warning: bool,
            remaining: f64,
            message: String,
        }

        let response: Response = self.client.graphql_query(query, Some(variables)).await?;

        Ok(BudgetStatus {
            budget_id: response.set_budget.budget_id,
            limit: response.set_budget.limit,
            used: response.set_budget.used,
            percentage_used: response.set_budget.percentage_used,
            is_exhausted: response.set_budget.is_exhausted,
            is_warning: response.set_budget.is_warning,
            remaining: response.set_budget.remaining,
            message: response.set_budget.message,
        })
    }
}

/// Stream of cost updates (placeholder for future subscription implementation)
pub struct CostUpdateStream {
    // This would contain WebSocket/SSE stream implementation
}

/// Convenience function to create a cost analytics query
pub fn cost_analytics(client: &Client, start_date: &str, end_date: &str) -> CostAnalyticsBuilder {
    client
        .analytics()
        .cost_analytics()
        .date_range(start_date, end_date)
}

/// Convenience function to create a budget status query
pub fn budget_status(client: &Client) -> BudgetStatusBuilder {
    client.analytics().budget_status()
}

/// Convenience function to create a set budget operation
pub fn set_budget(client: &Client, limit: f64, period: &str) -> SetBudgetBuilder {
    client
        .analytics()
        .set_budget()
        .limit(limit)
        .period(period)
        .warning_threshold(0.8) // Default warning threshold
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_input_serialization() {
        let input = BudgetInput {
            user_id: Some("user123".to_string()),
            project_id: None,
            limit: 100.0,
            period: "monthly".to_string(),
            warning_threshold: 0.8,
        };

        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("user123"));
        assert!(json.contains("100.0"));
        assert!(json.contains("monthly"));
    }

    #[test]
    fn test_cost_analytics_input_validation() {
        let input = CostAnalyticsInput {
            user_id: Some("user123".to_string()),
            project_id: None,
            start_date: "2024-01-01".to_string(),
            end_date: "2024-01-31".to_string(),
        };

        assert_eq!(input.start_date, "2024-01-01");
        assert_eq!(input.end_date, "2024-01-31");
    }
}
