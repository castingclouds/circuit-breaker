//! Functions module for the Circuit Breaker SDK
//!
//! This module provides client interfaces for creating and managing serverless functions.

use crate::{schema::QueryBuilder, types::*, Client, Result};
use serde::{Deserialize, Serialize};

/// Client for function operations
#[derive(Debug, Clone)]
pub struct FunctionClient {
    client: Client,
}

impl FunctionClient {
    /// Create a new function client
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Create a new function
    pub fn create(&self) -> FunctionBuilder {
        FunctionBuilder::new(self.client.clone())
    }

    /// Get a function by ID
    pub async fn get(&self, id: FunctionId) -> Result<Function> {
        let query = QueryBuilder::query_with_params(
            "GetFunction",
            "function(id: $id)",
            &[
                "id",
                "name",
                "description",
                "runtime",
                "entrypoint",
                "createdAt",
                "updatedAt",
            ],
            &[("id", "ID!")],
        );

        #[derive(Serialize)]
        struct Variables {
            id: FunctionId,
        }

        #[derive(Deserialize)]
        struct Response {
            function: FunctionData,
        }

        let response: Response = self.client.graphql(&query, Variables { id }).await?;

        Ok(Function {
            client: self.client.clone(),
            data: response.function,
        })
    }

    /// List functions
    pub async fn list(&self) -> Result<Vec<Function>> {
        let query = QueryBuilder::query(
            "ListFunctions",
            "functions",
            &[
                "id",
                "name",
                "description",
                "runtime",
                "entrypoint",
                "createdAt",
                "updatedAt",
            ],
        );

        #[derive(Deserialize)]
        struct Response {
            functions: Vec<FunctionData>,
        }

        let response: Response = self.client.graphql(&query, ()).await?;

        Ok(response
            .functions
            .into_iter()
            .map(|data| Function {
                client: self.client.clone(),
                data,
            })
            .collect())
    }
}

/// Builder for creating functions
pub struct FunctionBuilder {
    client: Client,
    name: Option<String>,
    description: Option<String>,
    runtime: Option<FunctionRuntime>,
    code: Option<FunctionCode>,
    entrypoint: Option<String>,
}

impl FunctionBuilder {
    fn new(client: Client) -> Self {
        Self {
            client,
            name: None,
            description: None,
            runtime: None,
            code: None,
            entrypoint: None,
        }
    }

    /// Set the function name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the function description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the function runtime
    pub fn runtime(mut self, runtime: FunctionRuntime) -> Self {
        self.runtime = Some(runtime);
        self
    }

    /// Set the function code
    pub fn code(mut self, code: FunctionCode) -> Self {
        self.code = Some(code);
        self
    }

    /// Set the function entrypoint
    pub fn entrypoint(mut self, entrypoint: impl Into<String>) -> Self {
        self.entrypoint = Some(entrypoint.into());
        self
    }

    /// Build and create the function
    pub async fn build(self) -> Result<Function> {
        let name = self.name.ok_or_else(|| crate::Error::Validation {
            message: "Function name is required".to_string(),
        })?;

        let runtime = self.runtime.ok_or_else(|| crate::Error::Validation {
            message: "Function runtime is required".to_string(),
        })?;

        let code = self.code.ok_or_else(|| crate::Error::Validation {
            message: "Function code is required".to_string(),
        })?;

        let entrypoint = self.entrypoint.ok_or_else(|| crate::Error::Validation {
            message: "Function entrypoint is required".to_string(),
        })?;

        let mutation = QueryBuilder::mutation_with_params(
            "CreateFunction",
            "createFunction(input: $input)",
            &[
                "id",
                "name",
                "description",
                "runtime",
                "entrypoint",
                "createdAt",
                "updatedAt",
            ],
            &[("input", "CreateFunctionInput!")],
        );

        #[derive(Serialize)]
        struct Variables {
            input: CreateFunctionInput,
        }

        #[derive(Serialize)]
        struct CreateFunctionInput {
            name: String,
            description: Option<String>,
            runtime: FunctionRuntime,
            code: FunctionCode,
            entrypoint: String,
        }

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "createFunction")]
            create_function: FunctionData,
        }

        let response: Response = self
            .client
            .graphql(
                &mutation,
                Variables {
                    input: CreateFunctionInput {
                        name,
                        description: self.description,
                        runtime,
                        code,
                        entrypoint,
                    },
                },
            )
            .await?;

        Ok(Function {
            client: self.client,
            data: response.create_function,
        })
    }
}

/// A function instance
#[derive(Debug, Clone)]
pub struct Function {
    client: Client,
    data: FunctionData,
}

impl Function {
    /// Get the function ID
    pub fn id(&self) -> FunctionId {
        self.data.id
    }

    /// Get the function name
    pub fn name(&self) -> &str {
        &self.data.name
    }

    /// Get the function description
    pub fn description(&self) -> Option<&str> {
        self.data.description.as_deref()
    }

    /// Get the function runtime
    pub fn runtime(&self) -> &FunctionRuntime {
        &self.data.runtime
    }

    /// Execute the function
    pub async fn execute(&self, input: serde_json::Value) -> Result<FunctionExecution> {
        let mutation = QueryBuilder::mutation_with_params(
            "ExecuteFunction",
            "executeFunction(functionId: $functionId, input: $input)",
            &[
                "id",
                "functionId",
                "status",
                "input",
                "output",
                "startedAt",
                "completedAt",
                "errorMessage",
            ],
            &[("functionId", "ID!"), ("input", "JSON!")],
        );

        #[derive(Serialize)]
        struct Variables {
            #[serde(rename = "functionId")]
            function_id: FunctionId,
            input: serde_json::Value,
        }

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "executeFunction")]
            execute_function: FunctionExecutionData,
        }

        let response: Response = self
            .client
            .graphql(
                &mutation,
                Variables {
                    function_id: self.data.id.clone(),
                    input,
                },
            )
            .await?;

        Ok(FunctionExecution {
            client: self.client.clone(),
            data: response.execute_function,
        })
    }

    /// Delete the function
    pub async fn delete(self) -> Result<()> {
        let mutation = QueryBuilder::mutation_with_params(
            "DeleteFunction",
            "deleteFunction(id: $id)",
            &["success"],
            &[("id", "ID!")],
        );

        #[derive(Serialize)]
        struct Variables {
            id: FunctionId,
        }

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "deleteFunction")]
            delete_function: DeleteResult,
        }

        #[derive(Deserialize)]
        struct DeleteResult {
            success: bool,
        }

        let _response: Response = self
            .client
            .graphql(&mutation, Variables { id: self.data.id })
            .await?;

        Ok(())
    }
}

/// A function execution instance
#[derive(Debug, Clone)]
pub struct FunctionExecution {
    client: Client,
    data: FunctionExecutionData,
}

impl FunctionExecution {
    /// Get the execution ID
    pub fn id(&self) -> &str {
        &self.data.id
    }

    /// Get the function ID
    pub fn function_id(&self) -> FunctionId {
        self.data.function_id
    }

    /// Get the execution status
    pub fn status(&self) -> &ExecutionStatus {
        &self.data.status
    }

    /// Get the execution output
    pub fn output(&self) -> Option<&serde_json::Value> {
        self.data.output.as_ref()
    }

    /// Check if the execution is complete
    pub fn is_complete(&self) -> bool {
        matches!(
            self.data.status,
            ExecutionStatus::Completed | ExecutionStatus::Failed | ExecutionStatus::Cancelled
        )
    }
}

// Internal data structures
#[derive(Debug, Clone, Deserialize)]
struct FunctionData {
    id: FunctionId,
    name: String,
    description: Option<String>,
    runtime: FunctionRuntime,
    entrypoint: String,
    #[serde(rename = "createdAt")]
    created_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "updatedAt")]
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Deserialize)]
struct FunctionExecutionData {
    id: String,
    #[serde(rename = "functionId")]
    function_id: FunctionId,
    status: ExecutionStatus,
    input: serde_json::Value,
    output: Option<serde_json::Value>,
    #[serde(rename = "startedAt")]
    started_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "completedAt")]
    completed_at: Option<chrono::DateTime<chrono::Utc>>,
}
