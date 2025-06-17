#!/usr/bin/env npx tsx
// Basic workflow demonstration - TypeScript GraphQL Client
// Shows core workflow operations using GraphQL API
// Run with: npx tsx examples/typescript/basic_workflow.ts

interface GraphQLResponse<T = any> {
  data?: T;
  errors?: Array<{ message: string; locations?: any[]; path?: any[] }>;
}

interface WorkflowGQL {
  id: string;
  name: string;
  states: string[];
  activities: ActivityGQL[];
  initialState: string;
  createdAt: string;
  updatedAt: string;
}

interface ActivityGQL {
  id: string;
  name?: string;
  fromStates: string[];
  toState: string;
  conditions: string[];
  description?: string;
}

interface ResourceGQL {
  id: string;
  workflowId: string;
  state: string;
  data: any;
  metadata: any;
  createdAt: string;
  updatedAt: string;
  history: HistoryEventGQL[];
}

interface HistoryEventGQL {
  timestamp: string;
  activity: string;
  fromState: string;
  toState: string;
  data?: any;
}

class CircuitBreakerClient {
  constructor(private baseUrl: string = "http://localhost:4000") {}

  async graphql<T = any>(
    query: string,
    variables?: any,
  ): Promise<GraphQLResponse<T>> {
    const response = await fetch(`${this.baseUrl}/graphql`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ query, variables }),
    });

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }

    return (await response.json()) as GraphQLResponse<T>;
  }

  async createWorkflow(input: any) {
    const mutation = `
      mutation CreateWorkflow($input: WorkflowDefinitionInput!) {
        createWorkflow(input: $input) {
          id
          name
          states
          activities {
            id
            fromStates
            toState
            conditions
          }
          initialState
          createdAt
          updatedAt
        }
      }
    `;

    return this.graphql<{ createWorkflow: WorkflowGQL }>(mutation, { input });
  }

  async createResource(input: any) {
    const mutation = `
      mutation CreateResource($input: ResourceCreateInput!) {
        createResource(input: $input) {
          id
          workflowId
          state
          data
          metadata
          createdAt
          updatedAt
          history {
            timestamp
            activity
            fromState
            toState
            data
          }
        }
      }
    `;

    return this.graphql<{ createResource: ResourceGQL }>(mutation, { input });
  }

  async executeActivity(input: any) {
    tryconst mutation = `
      mutation ExecuteActivity($input: ActivityExecuteInput!) {
        executeActivity(input: $input) {
          id
          workflowId
          state
          data
          metadata
          createdAt
          updatedAt
          history {
            timestamp
            activity
            fromState
            toState
            data
          }
        }
      }
    `;

    return this.graphql<{ executeActivity: ResourceGQL }>(mutation, { input });
  }

  async getResource(id: string) {
    const query = `
      query GetResource($id: String!) {
        resource(id: $id) {
          id
          workflowId
          state
          data
          metadata
          createdAt
          updatedAt
          history {
            timestamp
            activity
            fromState
            toState
            data
          }
        }
      }
    `;

    return this.graphql<{ resource: ResourceGQL | null }>(query, { id });
  }

  async listWorkflows() {
    const query = `
      query ListWorkflows {
        workflows {
          id
          name
          states
          activities {
            id
            fromStates
            toState
            conditions
          }
          initialState
          createdAt
        }
      }
    `;

    return this.graphql<{ workflows: WorkflowGQL[] }>(query);
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

async function main() {
  console.log("🚀 Circuit Breaker Basic Workflow Demo - TypeScript Client");
  console.log("==========================================================");
  console.log();

  const client = new CircuitBreakerClient();

  try {
    // Create a simple application development workflow
    logInfo("Creating Application Development Workflow...");

    const workflowInput = {
      name: "Application Development Process",
      states: [
        "planning",
        "development",
        "testing",
        "staging",
        "production",
        "maintenance",
      ],
      activities: [
        {
          id: "start_development",
          fromStates: ["planning"],
          toState: "development",
          conditions: [],
        },
        {
          id: "submit_for_testing",
          fromStates: ["development"],
          toState: "testing",
          conditions: [],
        },
        {
          id: "back_to_development",
          fromStates: ["testing"],
          toState: "development",
          conditions: [],
        },
        {
          id: "promote_to_staging",
          fromStates: ["testing"],
          toState: "staging",
          conditions: [],
        },
        {
          id: "deploy_to_production",
          fromStates: ["staging"],
          toState: "production",
          conditions: [],
        },
        {
          id: "enter_maintenance",
          fromStates: ["production"],
          toState: "maintenance",
          conditions: [],
        },
        {
          id: "back_to_planning",
          fromStates: ["maintenance"],
          toState: "planning",
          conditions: [],
        },
      ],
      initialState: "planning",
    };

    const workflowResult = await client.createWorkflow(workflowInput);

    if (workflowResult.errors) {
      logError(
        `Failed to create workflow: ${workflowResult.errors.map((e) => e.message).join(", ")}`,
      );
      return;
    }

    const workflow = workflowResult.data!.createWorkflow;
    logSuccess(`Created workflow: ${workflow.name} (${workflow.id})`);
    logInfo(`States: ${workflow.states.join(" → ")}`);
    logInfo(`Activities: ${workflow.activities.length} defined`);
    console.log();

    // Create a feature development token
    logInfo("Creating Feature Development Token...");

    const resourceInput = {
      workflowId: workflow.id,
      initialState: "planning",
      data: {
        featureName: "User Authentication",
        assignedDeveloper: "Alice Smith",
        priority: "high",
        estimatedHours: 40,
        requirements: [
          "Login/logout functionality",
          "Password reset capability",
          "Session management",
          "Security audit",
        ],
      },
      metadata: {
        createdBy: "project-manager",
        project: "main-application",
        sprint: "sprint-2024-01",
      },
    };

    const resourceResult = await client.createResource(resourceInput);

    if (resourceResult.errors) {
      logError(
        `Failed to create resource: ${resourceResult.errors.map((e) => e.message).join(", ")}`,
      );
      return;
    }

    const resource = resourceResult.data!.createResource;
    logSuccess(`Created resource: ${resource.id}`);
    logInfo(`Feature: ${resource.data.featureName}`);
    logInfo(`Developer: ${resource.data.assignedDeveloper}`);
    logInfo(`Current state: ${resource.state}`);
    console.log();

    // Simulate development lifecycle
    const activities = [
      { id: "start_development", description: "Start Development Phase" },
      { id: "submit_for_testing", description: "Submit for Testing" },
      { id: "promote_to_staging", description: "Promote to Staging" },
      { id: "deploy_to_production", description: "Deploy to Production" },
    ];

    let currentResource = resource;

    for (const activity of activities) {
      logInfo(`Executing activity: ${activity.description}`);

      const activityInput = {
        resourceId: currentResource.id,
        activityId: activity.id,
        data: {
          timestamp: new Date().toISOString(),
          performedBy: "automated-system",
          notes: `Executed ${activity.description}`,
        },
      };

      const activityResult = await client.executeActivity(activityInput);

      if (activityResult.errors) {
        logError(
          `Failed to execute activity: ${activityResult.errors.map((e) => e.message).join(", ")}`,
        );
        continue;
      }

      currentResource = activityResult.data!.executeActivity;
      logSuccess(`Activity completed: ${currentResource.state}`);

      // Add a small delay to make the demo more realistic
      await new Promise((resolve) => setTimeout(resolve, 100));
    }

    console.log();
    logInfo("Development Lifecycle Complete!");
    logInfo(`Final state: ${currentResource.state}`);

    // Show history
    console.log();
    logInfo("Complete Development History:");
    currentResource.history.forEach((event, index) => {
      const timestamp = new Date(event.timestamp).toLocaleTimeString();
      console.log(
        `  ${index + 1}. ${event.fromState} → ${event.toState} via ${event.activity} (${timestamp})`,
      );
    });

    console.log();
    logInfo("Workflow demonstrates:");
    console.log("  • Complex state transitions with cycles");
    console.log("  • Rich resource data for application features");
    console.log("  • Complete audit trail of state changes");
    console.log("  • GraphQL API integration from TypeScript");
    console.log("  • Production-ready development workflow");
  } catch (error) {
    logError(`Demo failed: ${error}`);
    process.exit(1);
  }
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch(console.error);
}

export {
  CircuitBreakerClient,
  type WorkflowGQL,
  type ResourceGQL,
  type HistoryEventGQL,
};
