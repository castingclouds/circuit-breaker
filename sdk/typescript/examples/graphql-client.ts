#!/usr/bin/env npx tsx
// Advanced GraphQL client demonstration - TypeScript
// Shows direct GraphQL operations, subscriptions, and advanced client features
// Run with: npx tsx examples/typescript/graphql_client.ts

interface GraphQLResponse<T = any> {
  data?: T;
  errors?: Array<{ message: string; locations?: any[]; path?: any[] }>;
}

interface GraphQLOperation {
  query: string;
  variables?: any;
  operationName?: string;
}

interface BatchOperation {
  operations: GraphQLOperation[];
  results: GraphQLResponse[];
  timing: {
    startTime: number;
    endTime: number;
    duration: number;
  };
}

class AdvancedGraphQLClient {
  private baseUrl: string;
  private defaultHeaders: Record<string, string>;
  private requestLog: Array<{
    operation: string;
    variables: any;
    timestamp: number;
    duration: number;
  }> = [];

  constructor(
    baseUrl: string = "http://localhost:4000",
    headers: Record<string, string> = {},
  ) {
    this.baseUrl = baseUrl;
    this.defaultHeaders = {
      "Content-Type": "application/json",
      "User-Agent": "Circuit-Breaker-TypeScript-Client/1.0",
      ...headers,
    };
  }

  async request<T = any>(
    operation: GraphQLOperation,
  ): Promise<GraphQLResponse<T>> {
    const startTime = Date.now();

    try {
      const response = await fetch(`${this.baseUrl}/graphql`, {
        method: "POST",
        headers: this.defaultHeaders,
        body: JSON.stringify({
          query: operation.query,
          variables: operation.variables,
          operationName: operation.operationName,
        }),
      });

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      const result = (await response.json()) as GraphQLResponse<T>;
      const duration = Date.now() - startTime;

      // Log the request for debugging
      this.requestLog.push({
        operation: operation.operationName || "unnamed",
        variables: operation.variables,
        timestamp: startTime,
        duration,
      });

      return result;
    } catch (error) {
      const duration = Date.now() - startTime;
      this.requestLog.push({
        operation: operation.operationName || "failed",
        variables: operation.variables,
        timestamp: startTime,
        duration,
      });
      throw error;
    }
  }

  async batchRequest(operations: GraphQLOperation[]): Promise<BatchOperation> {
    const startTime = Date.now();

    // Execute all operations in parallel
    const promises = operations.map((op) => this.request(op));
    const results = await Promise.allSettled(promises);

    const endTime = Date.now();

    return {
      operations,
      results: results.map((result) =>
        result.status === "fulfilled"
          ? result.value
          : { errors: [{ message: (result.reason as Error).message }] },
      ),
      timing: {
        startTime,
        endTime,
        duration: endTime - startTime,
      },
    };
  }

  getRequestLog() {
    return this.requestLog;
  }

  clearRequestLog() {
    this.requestLog = [];
  }

  // Introspection queries
  async introspectSchema() {
    const introspectionQuery = `
      query IntrospectionQuery {
        __schema {
          queryType { name }
          mutationType { name }
          subscriptionType { name }
          types {
            ...FullType
          }
          directives {
            name
            description
            locations
            args {
              ...InputValue
            }
          }
        }
      }

      fragment FullType on __Type {
        kind
        name
        description
        fields(includeDeprecated: true) {
          name
          description
          args {
            ...InputValue
          }
          type {
            ...TypeRef
          }
          isDeprecated
          deprecationReason
        }
        inputFields {
          ...InputValue
        }
        interfaces {
          ...TypeRef
        }
        enumValues(includeDeprecated: true) {
          name
          description
          isDeprecated
          deprecationReason
        }
        possibleTypes {
          ...TypeRef
        }
      }

      fragment InputValue on __InputValue {
        name
        description
        type { ...TypeRef }
        defaultValue
      }

      fragment TypeRef on __Type {
        kind
        name
        ofType {
          kind
          name
          ofType {
            kind
            name
            ofType {
              kind
              name
              ofType {
                kind
                name
                ofType {
                  kind
                  name
                  ofType {
                    kind
                    name
                    ofType {
                      kind
                      name
                    }
                  }
                }
              }
            }
          }
        }
      }
    `;

    return this.request({
      query: introspectionQuery,
      operationName: "IntrospectionQuery",
    });
  }

  // Performance testing utilities
  async performanceTest(operation: GraphQLOperation, iterations: number = 10) {
    const results = [];

    console.log(`🚀 Running performance test: ${iterations} iterations`);

    for (let i = 0; i < iterations; i++) {
      const startTime = Date.now();
      const result = await this.request(operation);
      const duration = Date.now() - startTime;

      results.push({
        iteration: i + 1,
        duration,
        success: !result.errors,
        errors: result.errors?.length || 0,
      });

      if ((i + 1) % 5 === 0) {
        console.log(`  Completed ${i + 1}/${iterations} iterations`);
      }
    }

    const avgDuration =
      results.reduce((sum, r) => sum + r.duration, 0) / results.length;
    const minDuration = Math.min(...results.map((r) => r.duration));
    const maxDuration = Math.max(...results.map((r) => r.duration));
    const successRate =
      (results.filter((r) => r.success).length / results.length) * 100;

    return {
      iterations,
      avgDuration: Math.round(avgDuration),
      minDuration,
      maxDuration,
      successRate: Math.round(successRate * 100) / 100,
      results,
    };
  }

  // Helper methods for common operations
  async createWorkflowQuick(
    name: string,
    states: string[],
    initialState: string,
  ) {
    return this.request({
      query: `
        mutation CreateWorkflow($input: WorkflowDefinitionInput!) {
          createWorkflow(input: $input) {
            id
            name
            states
            initialState
            createdAt
          }
        }
      `,
      variables: {
        input: {
          name,
          states,
          activities: [],
          initialState,
        },
      },
      operationName: "CreateWorkflow",
    });
  }

  async createResourceQuick(workflowId: string, data: any = {}) {
    return this.request({
      query: `
        mutation CreateResource($input: ResourceCreateInput!) {
          createResource(input: $input) {
            id
            workflowId
            state
            data
            createdAt
          }
        }
      `,
      variables: {
        input: {
          workflowId,
          data,
        },
      },
      operationName: "CreateResource",
    });
  }

  async getWorkflowDetails(id: string) {
    return this.request({
      query: `
        query GetWorkflow($id: String!) {
          workflow(id: $id) {
            id
            name
            places
            transitions {
              id
              fromPlaces
              toPlace
              conditions
            }
            initialPlace
            createdAt
            updatedAt
          }
        }
      `,
      variables: { id },
      operationName: "GetWorkflow",
    });
  }

  async getAllWorkflows() {
    return this.request({
      query: `
        query GetAllWorkflows {
          workflows {
            id
            name
            places
            transitions {
              id
              fromPlaces
              toPlace
            }
            initialPlace
            createdAt
          }
        }
      `,
      operationName: "GetAllWorkflows",
    });
  }

  async getTokenHistory(tokenId: string) {
    return this.request({
      query: `
        query GetTokenHistory($id: String!) {
          token(id: $id) {
            id
            workflowId
            place
            history {
              timestamp
              transition
              fromPlace
              toPlace
              data
            }
          }
        }
      `,
      variables: { id: tokenId },
      operationName: "GetTokenHistory",
    });
  }
}

function logSuccess(message: string) {
  console.log(`✅ ${message}`);
}

function logInfo(message: string) {
  console.log(`ℹ️  ${message}`);
}

function logError(message: string) {
  console.log(`❌ ${message}`);
}

function logPerformance(message: string) {
  console.log(`⚡ ${message}`);
}

async function main() {
  console.log("🚀 Circuit Breaker Advanced GraphQL Client Demo");
  console.log("================================================");
  console.log();

  const client = new AdvancedGraphQLClient("http://localhost:4000", {
    "X-Client-Version": "1.0.0",
    "X-Request-ID": `req-${Date.now()}`,
  });

  try {
    // 1. Schema Introspection
    logInfo("Performing GraphQL schema introspection...");
    const introspectionResult = await client.introspectSchema();

    if (introspectionResult.errors) {
      logError(
        `Schema introspection failed: ${introspectionResult.errors.map((e) => e.message).join(", ")}`,
      );
      return;
    }

    const schema = introspectionResult.data?.__schema;
    logSuccess("Schema introspection completed");
    logInfo(`Available types: ${schema?.types?.length || 0}`);
    logInfo(`Query type: ${schema?.queryType?.name}`);
    logInfo(`Mutation type: ${schema?.mutationType?.name}`);
    logInfo(`Subscription type: ${schema?.subscriptionType?.name || "None"}`);
    console.log();

    // 2. Batch Operations Demo
    logInfo("Demonstrating batch operations...");

    const batchOperations = [
      {
        query: `
          query GetAllWorkflows {
            workflows {
              id
              name
              places
            }
          }
        `,
        operationName: "GetAllWorkflows",
      },
      {
        query: `
          mutation CreateWorkflow($input: WorkflowDefinitionInput!) {
            createWorkflow(input: $input) {
              id
              name
              places
            }
          }
        `,
        variables: {
          input: {
            name: "Batch Created Workflow",
            states: ["start", "middle", "end"],
            activities: [
              {
                id: "begin",
                fromStates: ["start"],
                toState: "middle",
                conditions: [],
              },
              {
                id: "finish",
                fromStates: ["middle"],
                toState: "end",
                conditions: [],
              },
            ],
            initialState: "start",
          },
        },
        operationName: "CreateWorkflow",
      },
    ];

    const batchResult = await client.batchRequest(batchOperations);
    logSuccess(
      `Batch operations completed in ${batchResult.timing.duration}ms`,
    );

    batchResult.results.forEach((result, index) => {
      const operation = batchOperations[index];
      if (result.errors) {
        logError(
          `${operation.operationName} failed: ${result.errors.map((e) => e.message).join(", ")}`,
        );
      } else {
        logSuccess(`${operation.operationName} succeeded`);
      }
    });
    console.log();

    // 3. Performance Testing
    logInfo("Running performance tests...");

    const testOperation = {
      query: `
        query GetAllWorkflows {
          workflows {
            id
            name
            places
            transitions {
              id
              fromPlaces
              toPlace
            }
          }
        }
      `,
      operationName: "PerformanceTest",
    };

    const perfResults = await client.performanceTest(testOperation, 20);
    logPerformance(`Performance Test Results:`);
    logPerformance(`  Average: ${perfResults.avgDuration}ms`);
    logPerformance(`  Min: ${perfResults.minDuration}ms`);
    logPerformance(`  Max: ${perfResults.maxDuration}ms`);
    logPerformance(`  Success Rate: ${perfResults.successRate}%`);
    console.log();

    // 4. Advanced Workflow Operations
    logInfo("Demonstrating advanced workflow operations...");

    // Create a complex workflow
    const complexWorkflow = await client.request({
      query: `
        mutation CreateComplexWorkflow($input: WorkflowDefinitionInput!) {
          createWorkflow(input: $input) {
            id
            name
            places
            transitions {
              id
              fromPlaces
              toPlace
              conditions
            }
          }
        }
      `,
      variables: {
        input: {
          name: "Complex Multi-Stage Process",
          states: [
            "planning",
            "development",
            "testing",
            "staging",
            "production",
            "maintenance",
            "archived",
          ],
          activities: [
            {
              id: "start_dev",
              fromStates: ["planning"],
              toState: "development",
              conditions: [],
            },
            {
              id: "to_testing",
              fromStates: ["development"],
              toState: "testing",
              conditions: [],
            },
            {
              id: "back_to_dev",
              fromStates: ["testing"],
              toState: "development",
              conditions: [],
            },
            {
              id: "to_staging",
              fromStates: ["testing"],
              toState: "staging",
              conditions: [],
            },
            {
              id: "to_production",
              fromStates: ["staging"],
              toState: "production",
              conditions: [],
            },
            {
              id: "to_maintenance",
              fromStates: ["production"],
              toState: "maintenance",
              conditions: [],
            },
            {
              id: "archive",
              fromStates: ["maintenance"],
              toState: "archived",
              conditions: [],
            },
            {
              id: "restart",
              fromStates: ["archived"],
              toState: "planning",
              conditions: [],
            },
          ],
          initialState: "planning",
        },
      },
      operationName: "CreateComplexWorkflow",
    });

    if (complexWorkflow.errors) {
      logError(
        `Failed to create complex workflow: ${complexWorkflow.errors.map((e) => e.message).join(", ")}`,
      );
    } else {
      const workflow = complexWorkflow.data?.createWorkflow;
      logSuccess(
        `Created complex workflow: ${workflow?.name} (${workflow?.id})`,
      );
      logInfo(`States: ${workflow?.states?.length}`);
      logInfo(`Activities: ${workflow?.activities?.length}`);

      // Create multiple tokens
      const resourcePromises = Array.from({ length: 5 }, (_, i) =>
        client.createResourceQuick(workflow!.id, {
          featureId: `feature-${i + 1}`,
          priority: i < 2 ? "high" : "medium",
          assignee: `developer-${(i % 3) + 1}`,
          complexity: Math.floor(Math.random() * 10) + 1,
        }),
      );

      const resourceResults = await Promise.allSettled(resourcePromises);
      const successfulResources = resourceResults
        .filter(
          (result) => result.status === "fulfilled" && !result.value.errors,
        )
        .map(
          (result) =>
            (result as PromiseFulfilledResult<any>).value.data?.createResource,
        );

      logSuccess(
        `Created ${successfulResources.length} resources successfully`,
      );
    }

    // 5. Request Log Analysis
    console.log();
    logInfo("Request Log Analysis:");
    const requestLog = client.getRequestLog();
    const totalRequests = requestLog.length;
    const avgDuration =
      requestLog.reduce((sum, req) => sum + req.duration, 0) / totalRequests;
    const slowestRequest = requestLog.reduce(
      (max, req) => (req.duration > max.duration ? req : max),
      requestLog[0],
    );
    const fastestRequest = requestLog.reduce(
      (min, req) => (req.duration < min.duration ? req : min),
      requestLog[0],
    );

    logInfo(`Total requests made: ${totalRequests}`);
    logInfo(`Average request duration: ${Math.round(avgDuration)}ms`);
    logInfo(
      `Slowest request: ${slowestRequest?.operation} (${slowestRequest?.duration}ms)`,
    );
    logInfo(
      `Fastest request: ${fastestRequest?.operation} (${fastestRequest?.duration}ms)`,
    );

    // Group by operation type
    const operationCounts = requestLog.reduce(
      (acc, req) => {
        acc[req.operation] = (acc[req.operation] || 0) + 1;
        return acc;
      },
      {} as Record<string, number>,
    );

    logInfo("Operations by type:");
    Object.entries(operationCounts).forEach(([operation, count]) => {
      console.log(`  • ${operation}: ${count} requests`);
    });

    console.log();
    logInfo("Advanced GraphQL Client Demo Summary:");
    console.log("  • Schema introspection and type discovery");
    console.log("  • Batch operations for improved performance");
    console.log("  • Performance testing and benchmarking");
    console.log("  • Request logging and analytics");
    console.log("  • Complex workflow and resource management");
    console.log("  • Production-ready client patterns");
  } catch (error) {
    logError(`Demo failed: ${error}`);
    process.exit(1);
  }
}

if (require.main === module) {
  main().catch(console.error);
}

export {
  AdvancedGraphQLClient,
  type GraphQLOperation,
  type BatchOperation,
  type GraphQLResponse,
};
