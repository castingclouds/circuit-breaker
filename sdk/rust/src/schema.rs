//! GraphQL schema loader that references actual schema files as source of truth
//!
//! This module provides access to the GraphQL schema files without duplicating
//! the schema definitions, ensuring the actual .graphql files remain the single
//! source of truth.

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// GraphQL schema files embedded at compile time from the actual schema directory
pub struct Schema {
    files: HashMap<&'static str, &'static str>,
}

impl Schema {
    /// Create a new schema instance with all embedded schema files
    pub fn new() -> Self {
        let mut files = HashMap::new();

        // Embed all schema files at compile time using include_str!
        // This ensures we're always using the actual schema files as source of truth
        files.insert("agents", include_str!("../../../schema/agents.graphql"));
        files.insert(
            "analytics",
            include_str!("../../../schema/analytics.graphql"),
        );
        files.insert("llm", include_str!("../../../schema/llm.graphql"));
        files.insert("mcp", include_str!("../../../schema/mcp.graphql"));
        files.insert("rules", include_str!("../../../schema/rules.graphql"));
        files.insert("types", include_str!("../../../schema/types.graphql"));
        files.insert("workflow", include_str!("../../../schema/workflow.graphql"));
        files.insert(
            "subscriptions",
            include_str!("../../../schema/subscriptions.graphql"),
        );
        files.insert("nats", include_str!("../../../schema/nats.graphql"));

        Self { files }
    }

    /// Get a specific schema file content
    pub fn get(&self, name: &str) -> Option<&'static str> {
        self.files.get(name).copied()
    }

    /// Get all schema files
    pub fn all(&self) -> &HashMap<&'static str, &'static str> {
        &self.files
    }

    /// Get the complete schema by combining all files
    pub fn get_complete_schema(&self) -> String {
        let mut complete_schema = String::new();

        // Add base types first if available
        if let Some(types_schema) = self.get("types") {
            complete_schema.push_str(types_schema);
            complete_schema.push('\n');
        }

        // Add all other schemas
        for (name, content) in &self.files {
            if *name != "types" {
                complete_schema.push_str(content);
                complete_schema.push('\n');
            }
        }

        complete_schema
    }
}

/// Static schema instance for global access
static SCHEMA: Lazy<Schema> = Lazy::new(Schema::new);

/// Get the global schema instance
pub fn schema() -> &'static Schema {
    &SCHEMA
}

/// Helper to build GraphQL operations based on the schema
pub struct QueryBuilder;

impl QueryBuilder {
    /// Build a query with the given name, root field, and selected fields
    pub fn query(name: &str, root_field: &str, fields: &[&str]) -> String {
        self::build_operation("query", name, root_field, fields, &[])
    }

    /// Build a query with parameters
    pub fn query_with_params(
        name: &str,
        root_field: &str,
        fields: &[&str],
        params: &[(&str, &str)],
    ) -> String {
        self::build_operation("query", name, root_field, fields, params)
    }

    /// Build a mutation with the given name, root field, and selected fields
    pub fn mutation(name: &str, root_field: &str, fields: &[&str]) -> String {
        self::build_operation("mutation", name, root_field, fields, &[])
    }

    /// Build a mutation with parameters
    pub fn mutation_with_params(
        name: &str,
        root_field: &str,
        fields: &[&str],
        params: &[(&str, &str)],
    ) -> String {
        self::build_operation("mutation", name, root_field, fields, params)
    }
}

/// Helper function to build GraphQL operations
fn build_operation(
    operation_type: &str,
    name: &str,
    root_field: &str,
    fields: &[&str],
    params: &[(&str, &str)],
) -> String {
    let mut operation = format!("{} {}", operation_type, name);

    // Add parameters if provided
    if !params.is_empty() {
        let param_defs: Vec<String> = params
            .iter()
            .map(|(name, type_)| format!("${}: {}", name, type_))
            .collect();
        operation.push_str(&format!("({})", param_defs.join(", ")));
    }

    operation.push_str(" {\n");
    operation.push_str(&format!("  {}", root_field));

    // Add field selections if provided
    if !fields.is_empty() {
        operation.push_str(" {\n");
        for field in fields {
            operation.push_str(&format!("    {}\n", field));
        }
        operation.push_str("  }");
    }

    operation.push_str("\n}");
    operation
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_loading() {
        let schema = Schema::new();

        // Test that all schema files are loaded
        assert!(schema.get("agents").is_some());
        assert!(schema.get("analytics").is_some());
        assert!(schema.get("llm").is_some());
        assert!(schema.get("mcp").is_some());
        assert!(schema.get("rules").is_some());
        assert!(schema.get("types").is_some());
        assert!(schema.get("workflow").is_some());
        assert!(schema.get("subscriptions").is_some());
        assert!(schema.get("nats").is_some());

        // Test that non-existent schema returns None
        assert!(schema.get("nonexistent").is_none());
    }

    #[test]
    fn test_schema_content() {
        let schema = Schema::new();

        // Test that schema files contain expected content
        let agents_schema = schema.get("agents").unwrap();
        assert!(agents_schema.contains("extend type Query"));
        assert!(agents_schema.contains("agent(id: String!)"));

        let llm_schema = schema.get("llm").unwrap();
        assert!(llm_schema.contains("llmChatCompletion"));
        assert!(llm_schema.contains("llmProviders"));
    }

    #[test]
    fn test_global_schema_access() {
        let schema = schema();
        assert!(schema.get("agents").is_some());
    }

    #[test]
    fn test_complete_schema() {
        let schema = Schema::new();
        let complete = schema.get_complete_schema();

        // Should contain content from multiple schema files
        assert!(complete.contains("AgentDefinitionGQL"));
        assert!(complete.contains("LlmProviderGQL"));
        assert!(complete.contains("BudgetStatusGQL"));
    }

    #[test]
    fn test_query_builder() {
        let query = QueryBuilder::query("TestQuery", "testField", &["id", "name"]);
        assert!(query.contains("query TestQuery"));
        assert!(query.contains("testField"));
        assert!(query.contains("id"));
        assert!(query.contains("name"));
    }

    #[test]
    fn test_query_builder_with_params() {
        let query = QueryBuilder::query_with_params(
            "TestQuery",
            "testField(id: $id)",
            &["id", "name"],
            &[("id", "ID!")],
        );
        assert!(query.contains("query TestQuery($id: ID!)"));
        assert!(query.contains("testField(id: $id)"));
    }

    #[test]
    fn test_mutation_builder() {
        let mutation = QueryBuilder::mutation("TestMutation", "testMutation", &["success"]);
        assert!(mutation.contains("mutation TestMutation"));
        assert!(mutation.contains("testMutation"));
        assert!(mutation.contains("success"));
    }
}
