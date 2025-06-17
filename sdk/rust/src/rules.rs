//! Rules module for the Circuit Breaker SDK
//!
//! This module provides client interfaces for creating and managing business rules.

use crate::{types::*, Client, Result};
use serde::{Deserialize, Serialize};

/// Client for rule operations
#[derive(Debug, Clone)]
pub struct RuleClient {
    client: Client,
}

impl RuleClient {
    /// Create a new rule client
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Create a new rule
    pub fn create(&self) -> RuleBuilder {
        RuleBuilder::new(self.client.clone())
    }

    /// Get a rule by ID
    pub async fn get(&self, id: RuleId) -> Result<Rule> {
        let query = r#"
            query GetRule($id: ID!) {
                rule(id: $id) {
                    id
                    name
                    description
                    version
                    createdAt
                    updatedAt
                    tags
                }
            }
        "#;

        #[derive(Serialize)]
        struct Variables {
            id: RuleId,
        }

        #[derive(Deserialize)]
        struct Response {
            rule: RuleData,
        }

        let response: Response = self.client.graphql(query, Variables { id }).await?;

        Ok(Rule {
            client: self.client.clone(),
            data: response.rule,
        })
    }

    /// List rules
    pub async fn list(&self) -> Result<Vec<Rule>> {
        let query = r#"
            query ListRules {
                rules {
                    id
                    name
                    description
                    version
                    createdAt
                    updatedAt
                    tags
                }
            }
        "#;

        #[derive(Deserialize)]
        struct Response {
            rules: Vec<RuleData>,
        }

        let response: Response = self.client.graphql(query, ()).await?;

        Ok(response
            .rules
            .into_iter()
            .map(|data| Rule {
                client: self.client.clone(),
                data,
            })
            .collect())
    }

    /// Evaluate a rule against data
    pub async fn evaluate(
        &self,
        rule_id: String,
        data: serde_json::Value,
    ) -> Result<RuleEvaluationResult> {
        let mutation = r#"
            mutation EvaluateRule($input: RuleEvaluationInput!) {
                evaluateRule(input: $input) {
                    ruleId
                    passed
                    reason
                    details
                }
            }
        "#;

        #[derive(Serialize)]
        struct Variables {
            input: RuleEvaluationInput,
        }

        #[derive(Serialize)]
        struct RuleEvaluationInput {
            #[serde(rename = "ruleId")]
            rule_id: String,
            data: serde_json::Value,
            metadata: Option<serde_json::Value>,
        }

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "evaluateRule")]
            evaluate_rule: RuleEvaluationResult,
        }

        let response: Response = self
            .client
            .graphql(
                mutation,
                Variables {
                    input: RuleEvaluationInput {
                        rule_id,
                        data,
                        metadata: None,
                    },
                },
            )
            .await?;

        Ok(response.evaluate_rule)
    }
}

/// Rule evaluation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleEvaluationResult {
    #[serde(rename = "ruleId")]
    pub rule_id: String,
    pub passed: bool,
    pub reason: String,
    pub details: Option<serde_json::Value>,
}

/// Builder for creating rules
pub struct RuleBuilder {
    client: Client,
    name: Option<String>,
    description: Option<String>,
    condition_type: Option<String>,
    field: Option<String>,
    value: Option<serde_json::Value>,
    substring: Option<String>,
    tags: Option<Vec<String>>,
}

impl RuleBuilder {
    fn new(client: Client) -> Self {
        Self {
            client,
            name: None,
            description: None,
            condition_type: None,
            field: None,
            value: None,
            substring: None,
            tags: None,
        }
    }

    /// Set the rule name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the rule description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the condition type
    pub fn set_condition_type(mut self, condition_type: impl Into<String>) -> Self {
        self.condition_type = Some(condition_type.into());
        self
    }

    /// Set the field for the condition
    pub fn set_field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }

    /// Set the value for the condition
    pub fn set_value(mut self, value: serde_json::Value) -> Self {
        self.value = Some(value);
        self
    }

    /// Set the substring for the condition
    pub fn set_substring(mut self, substring: impl Into<String>) -> Self {
        self.substring = Some(substring.into());
        self
    }

    /// Set tags for the rule
    pub fn set_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags);
        self
    }

    /// Add a tag to the rule
    pub fn add_tag(mut self, tag: impl Into<String>) -> Self {
        if let Some(ref mut tags) = self.tags {
            tags.push(tag.into());
        } else {
            self.tags = Some(vec![tag.into()]);
        }
        self
    }

    /// Add a field greater than condition
    pub fn field_greater_than(
        mut self,
        field: impl Into<String>,
        value: serde_json::Value,
    ) -> Self {
        self.condition_type = Some("FieldGreaterThan".to_string());
        self.field = Some(field.into());
        self.value = Some(value);
        self
    }

    /// Set field less than condition
    pub fn field_less_than(mut self, field: impl Into<String>, value: serde_json::Value) -> Self {
        self.condition_type = Some("FieldLessThan".to_string());
        self.field = Some(field.into());
        self.value = Some(value);
        self
    }

    /// Set field equals condition
    pub fn field_equals(mut self, field: impl Into<String>, value: serde_json::Value) -> Self {
        self.condition_type = Some("FieldEquals".to_string());
        self.field = Some(field.into());
        self.value = Some(value);
        self
    }

    /// Build and create the rule
    pub async fn build(self) -> Result<Rule> {
        let name = self.name.ok_or_else(|| crate::Error::Validation {
            message: "Rule name is required".to_string(),
        })?;

        let condition_type = self
            .condition_type
            .unwrap_or_else(|| "FieldGreaterThan".to_string());

        let mutation = r#"
            mutation CreateRule($input: RuleInput!) {
                createRule(input: $input) {
                    id
                    name
                    description
                    version
                    createdAt
                    updatedAt
                    tags
                }
            }
        "#;

        #[derive(Serialize)]
        struct Variables {
            input: RuleInput,
        }

        #[derive(Serialize)]
        struct RuleInput {
            name: String,
            description: String,
            condition: RuleConditionInput,
            tags: Option<Vec<String>>,
        }

        #[derive(Serialize)]
        struct RuleConditionInput {
            #[serde(rename = "conditionType")]
            condition_type: String,
            field: Option<String>,
            value: Option<serde_json::Value>,
            substring: Option<String>,
            rules: Option<Vec<RuleConditionInput>>,
            rule: Option<Box<RuleConditionInput>>,
            script: Option<String>,
        }

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "createRule")]
            create_rule: RuleData,
        }

        let response: Response = self
            .client
            .graphql(
                mutation,
                Variables {
                    input: RuleInput {
                        name,
                        description: self.description.unwrap_or_else(|| "".to_string()),
                        condition: RuleConditionInput {
                            condition_type,
                            field: self.field,
                            value: self.value,
                            substring: self.substring,
                            rules: None,
                            rule: None,
                            script: None,
                        },
                        tags: self.tags,
                    },
                },
            )
            .await?;

        Ok(Rule {
            client: self.client,
            data: response.create_rule,
        })
    }
}

/// A rule instance
#[derive(Debug, Clone)]
pub struct Rule {
    client: Client,
    data: RuleData,
}

impl Rule {
    /// Get the rule ID
    pub fn id(&self) -> String {
        self.data.id.clone()
    }

    /// Get the rule name
    pub fn name(&self) -> &str {
        &self.data.name
    }

    /// Get the rule description
    pub fn description(&self) -> &str {
        &self.data.description
    }

    /// Get the rule version
    pub fn version(&self) -> i32 {
        self.data.version
    }

    /// Evaluate the rule with given data
    pub async fn evaluate(&self, data: serde_json::Value) -> Result<bool> {
        let mutation = r#"
            mutation EvaluateRule($ruleId: ID!, $data: JSON!) {
                evaluateRule(ruleId: $ruleId, data: $data) {
                    result
                    message
                }
            }
        "#;

        #[derive(Serialize)]
        struct Variables {
            #[serde(rename = "ruleId")]
            rule_id: String,
            data: serde_json::Value,
        }

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "evaluateRule")]
            evaluate_rule: EvaluationResult,
        }

        #[derive(Deserialize)]
        struct EvaluationResult {
            result: bool,
            message: Option<String>,
        }

        let response: Response = self
            .client
            .graphql(
                mutation,
                Variables {
                    rule_id: self.data.id.clone(),
                    data,
                },
            )
            .await?;

        Ok(response.evaluate_rule.result)
    }

    /// Delete the rule
    pub async fn delete(self) -> Result<()> {
        let mutation = r#"
            mutation DeleteRule($id: ID!) {
                deleteRule(id: $id) {
                    success
                }
            }
        "#;

        #[derive(Serialize)]
        struct Variables {
            id: String,
        }

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "deleteRule")]
            delete_rule: DeleteResult,
        }

        #[derive(Deserialize)]
        struct DeleteResult {
            success: bool,
        }

        let _response: Response = self
            .client
            .graphql(
                mutation,
                Variables {
                    id: self.data.id.clone(),
                },
            )
            .await?;

        Ok(())
    }
}

// Internal data structures
#[derive(Debug, Clone, Deserialize)]
struct RuleData {
    id: String,
    name: String,
    description: String,
    version: i32,
    tags: Option<Vec<String>>,
    #[serde(rename = "createdAt")]
    created_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "updatedAt")]
    updated_at: chrono::DateTime<chrono::Utc>,
}

/// Rule evaluator for client-side evaluation
pub struct RuleEvaluator;

impl RuleEvaluator {
    /// Evaluate a rule condition against data
    pub fn evaluate(
        condition: &serde_json::Value,
        data: &serde_json::Value,
    ) -> RuleEvaluationResult {
        let condition_type = condition
            .get("conditionType")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");

        match condition_type {
            "FieldGreaterThan" => Self::evaluate_field_greater_than(condition, data),
            "FieldLessThan" => Self::evaluate_field_less_than(condition, data),
            "FieldEquals" => Self::evaluate_field_equals(condition, data),
            _ => RuleEvaluationResult {
                rule_id: uuid::Uuid::nil().to_string(),
                passed: false,
                reason: format!("Unknown condition type: {}", condition_type),
                details: None,
            },
        }
    }

    fn evaluate_field_greater_than(
        condition: &serde_json::Value,
        data: &serde_json::Value,
    ) -> RuleEvaluationResult {
        let field = condition
            .get("field")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let threshold = condition.get("value").unwrap_or(&serde_json::Value::Null);

        let field_value = data.get(field).unwrap_or(&serde_json::Value::Null);

        let passed = match (field_value, threshold) {
            (serde_json::Value::Number(a), serde_json::Value::Number(b)) => {
                a.as_f64().unwrap_or(0.0) > b.as_f64().unwrap_or(0.0)
            }
            _ => false,
        };

        RuleEvaluationResult {
            rule_id: uuid::Uuid::nil().to_string(),
            passed,
            reason: if passed {
                format!(
                    "Field '{}' value {:?} is greater than {:?}",
                    field, field_value, threshold
                )
            } else {
                format!(
                    "Field '{}' value {:?} is not greater than {:?}",
                    field, field_value, threshold
                )
            },
            details: Some(serde_json::json!({
                "field": field,
                "value": field_value,
                "threshold": threshold,
                "condition": "greater_than"
            })),
        }
    }

    fn evaluate_field_less_than(
        condition: &serde_json::Value,
        data: &serde_json::Value,
    ) -> RuleEvaluationResult {
        let field = condition
            .get("field")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let threshold = condition.get("value").unwrap_or(&serde_json::Value::Null);

        let field_value = data.get(field).unwrap_or(&serde_json::Value::Null);

        let passed = match (field_value, threshold) {
            (serde_json::Value::Number(a), serde_json::Value::Number(b)) => {
                a.as_f64().unwrap_or(0.0) < b.as_f64().unwrap_or(0.0)
            }
            _ => false,
        };

        RuleEvaluationResult {
            rule_id: uuid::Uuid::nil().to_string(),
            passed,
            reason: if passed {
                format!(
                    "Field '{}' value {:?} is less than {:?}",
                    field, field_value, threshold
                )
            } else {
                format!(
                    "Field '{}' value {:?} is not less than {:?}",
                    field, field_value, threshold
                )
            },
            details: Some(serde_json::json!({
                "field": field,
                "value": field_value,
                "threshold": threshold,
                "condition": "less_than"
            })),
        }
    }

    fn evaluate_field_equals(
        condition: &serde_json::Value,
        data: &serde_json::Value,
    ) -> RuleEvaluationResult {
        let field = condition
            .get("field")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let expected = condition.get("value").unwrap_or(&serde_json::Value::Null);

        let field_value = data.get(field).unwrap_or(&serde_json::Value::Null);

        let passed = field_value == expected;

        RuleEvaluationResult {
            rule_id: uuid::Uuid::nil().to_string(),
            passed,
            reason: if passed {
                format!(
                    "Field '{}' value {:?} equals {:?}",
                    field, field_value, expected
                )
            } else {
                format!(
                    "Field '{}' value {:?} does not equal {:?}",
                    field, field_value, expected
                )
            },
            details: Some(serde_json::json!({
                "field": field,
                "value": field_value,
                "expected": expected,
                "condition": "equals"
            })),
        }
    }
}

/// Standalone rule builder for creating rule conditions
pub struct RuleBuilderStandalone {
    name: String,
    description: String,
    condition: Option<serde_json::Value>,
    tags: Vec<String>,
}

impl RuleBuilderStandalone {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            condition: None,
            tags: Vec::new(),
        }
    }

    pub fn field_greater_than(
        name: impl Into<String>,
        description: impl Into<String>,
        field: impl Into<String>,
        value: serde_json::Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            condition: Some(serde_json::json!({
                "conditionType": "FieldGreaterThan",
                "field": field.into(),
                "value": value
            })),
            tags: Vec::new(),
        }
    }

    pub fn field_less_than(
        name: impl Into<String>,
        description: impl Into<String>,
        field: impl Into<String>,
        value: serde_json::Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            condition: Some(serde_json::json!({
                "conditionType": "FieldLessThan",
                "field": field.into(),
                "value": value
            })),
            tags: Vec::new(),
        }
    }

    pub fn field_equals(
        name: impl Into<String>,
        description: impl Into<String>,
        field: impl Into<String>,
        value: serde_json::Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            condition: Some(serde_json::json!({
                "conditionType": "FieldEquals",
                "field": field.into(),
                "value": value
            })),
            tags: Vec::new(),
        }
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn build(self) -> RuleDefinition {
        RuleDefinition {
            name: self.name,
            description: self.description,
            condition: self.condition,
            tags: self.tags,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuleDefinition {
    pub name: String,
    pub description: String,
    pub condition: Option<serde_json::Value>,
    pub tags: Vec<String>,
}

/// Evaluate a rule condition against data (convenience function)
pub fn evaluate_rule(rule: &RuleDefinition, data: &serde_json::Value) -> RuleEvaluationResult {
    if let Some(condition) = &rule.condition {
        RuleEvaluator::evaluate(condition, data)
    } else {
        RuleEvaluationResult {
            rule_id: uuid::Uuid::nil().to_string(),
            passed: false,
            reason: "No condition defined".to_string(),
            details: None,
        }
    }
}
