#!/usr/bin/env npx tsx
// NATS Integration Demo - TypeScript GraphQL Client
// This demonstrates using GraphQL API with NATS storage backend
// Assumes Circuit Breaker server is running with NATS storage
// Run with: npx tsx examples/typescript/nats_demo.ts

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

interface NATSResourceGQL {
  id: string;
  workflowId: string;
  state: string;
  data: any;
  metadata: any;
  createdAt: string;
  updatedAt: string;
  history: HistoryEventGQL[];
  natsSequence?: string;
  natsTimestamp?: string;
  natsSubject?: string;
  activityHistory: ActivityRecordGQL[];
}

interface ActivityRecordGQL {
  fromState: string;
  toState: string;
  activityId: string;
  timestamp: string;
  triggeredBy?: string;
  natsSequence?: string;
  metadata?: any;
}

interface HistoryEventGQL {
  timestamp: string;
  activity: string;
  fromState: string;
  toState: string;
  data?: any;
}

interface CreateWorkflowInstanceInput {
  workflowId: string;
  initialData?: any;
  metadata?: any;
  triggeredBy?: string;
}

interface ExecuteActivityWithNATSInput {
  resourceId: string;
  activityId: string;
  newState: string;
  triggeredBy?: string;
  data?: any;
}

class CircuitBreakerNATSClient {
  constructor(private baseUrl: string = "http://localhost:4000") {}

  // Helper function to pause for demonstrations
  private async pauseForDemo(message: string): Promise<void> {
    console.log(`\n⏸️  ${message}`);
    console.log("   Press Enter to continue...");

    // Wait for user input
    await new Promise((resolve) => {
      process.stdin.once("data", () => resolve(void 0));
    });
  }

  private async graphqlRequest<T>(
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
      throw new Error(`HTTP error! status: ${response.status}`);
    }

    return response.json() as Promise<GraphQLResponse<T>>;
  }

  private handleErrors<T>(response: GraphQLResponse<T>): T {
    if (response.errors) {
      throw new Error(`GraphQL errors: ${JSON.stringify(response.errors)}`);
    }
    if (!response.data) {
      throw new Error("No data in GraphQL response");
    }
    return response.data;
  }

  async createWorkflow(input: {
    name: string;
    description?: string;
    states: string[];
    initialState: string;
    activities: Array<{
      id: string;
      fromStates: string[];
      toState: string;
      conditions?: string[];
      description?: string;
    }>;
  }): Promise<WorkflowGQL> {
    const query = `
      mutation CreateWorkflow($input: WorkflowDefinitionInput!) {
        createWorkflow(input: $input) {
          id
          name
          states
          initialState
          activities {
            id
            fromStates
            toState
            conditions
            description
          }
          createdAt
          updatedAt
        }
      }
    `;

    const response = await this.graphqlRequest<{ createWorkflow: WorkflowGQL }>(
      query,
      { input },
    );
    return this.handleErrors(response).createWorkflow;
  }

  async createWorkflowInstance(
    input: CreateWorkflowInstanceInput,
  ): Promise<NATSResourceGQL> {
    const query = `
      mutation CreateWorkflowInstance($input: CreateWorkflowInstanceInput!) {
        createWorkflowInstance(input: $input) {
          id
          workflowId
          state
          data
          metadata
          createdAt
          updatedAt
          natsSequence
          natsTimestamp
          natsSubject
          activityHistory {
            fromState
            toState
            activityId
            timestamp
            triggeredBy
            natsSequence
            metadata
          }
        }
      }
    `;

    const response = await this.graphqlRequest<{
      createWorkflowInstance: NATSResourceGQL;
    }>(query, { input });
    return this.handleErrors(response).createWorkflowInstance;
  }

  async executeActivityWithNats(
    input: ExecuteActivityWithNATSInput,
  ): Promise<NATSResourceGQL> {
    const query = `
      mutation ExecuteActivityWithNATS($input: ExecuteActivityWithNATSInput!) {
        executeActivityWithNats(input: $input) {
          id
          state
          data
          natsSequence
          natsTimestamp
          activityHistory {
            fromState
            toState
            activityId
            timestamp
            triggeredBy
            natsSequence
          }
        }
      }
    `;

    const response = await this.graphqlRequest<{
      executeActivityWithNats: NATSResourceGQL;
    }>(query, { input });
    return this.handleErrors(response).executeActivityWithNats;
  }

  async getResourcesInState(
    workflowId: string,
    stateId: string,
  ): Promise<NATSResourceGQL[]> {
    const query = `
      query ResourcesInState($workflowId: String!, $stateId: String!) {
        resourcesInState(workflowId: $workflowId, stateId: $stateId) {
          id
          state
          data
          natsSequence
          natsSubject
          activityHistory {
            fromState
            toState
            timestamp
            triggeredBy
          }
        }
      }
    `;

    const response = await this.graphqlRequest<{
      resourcesInState: NATSResourceGQL[];
    }>(query, {
      workflowId,
      stateId,
    });
    return this.handleErrors(response).resourcesInState;
  }

  async getNATSResource(id: string): Promise<NATSResourceGQL | null> {
    const query = `
      query GetNATSResource($id: String!) {
        natsResource(id: $id) {
          id
          workflowId
          state
          data
          natsSequence
          natsTimestamp
          natsSubject
          activityHistory {
            fromState
            toState
            activityId
            timestamp
            triggeredBy
            natsSequence
            metadata
          }
        }
      }
    `;

    const response = await this.graphqlRequest<{
      natsResource: NATSResourceGQL | null;
    }>(query, { id });
    return this.handleErrors(response).natsResource;
  }
}

async function runNATSWorkflowDemo(): Promise<void> {
  console.log("📋 Creating workflow with NATS storage backend...");
  console.log(
    "This demo will walk you through each step of NATS integration.\n",
  );

  const client = new CircuitBreakerNATSClient();

  // Enable raw mode for better input handling
  if (process.stdin.isTTY) {
    process.stdin.setRawMode(false);
  }

  try {
    // Step 1: Create a workflow definition
    const workflow = await client.createWorkflow({
      name: "NATS Document Review Process (TypeScript)",
      description: "A document review workflow using NATS streaming backend",
      states: ["draft", "review", "approved", "published", "rejected"],
      initialState: "draft",
      activities: [
        {
          id: "submit_for_review",
          fromStates: ["draft"],
          toState: "review",
          conditions: [],
          description: "Submit document for review",
        },
        {
          id: "approve",
          fromStates: ["review"],
          toState: "approved",
          conditions: [],
          description: "Approve the document",
        },
        {
          id: "reject",
          fromStates: ["review"],
          toState: "rejected",
          conditions: [],
          description: "Reject the document",
        },
        {
          id: "publish",
          fromStates: ["approved"],
          toState: "published",
          conditions: [],
          description: "Publish the document",
        },
      ],
    });

    console.log(`✅ Created workflow: "${workflow.name}" (ID: ${workflow.id})`);
    console.log("🔍 What just happened:");
    console.log(
      "   • Workflow definition was sent via GraphQL to the Circuit Breaker server",
    );
    console.log(
      "   • Server stored the workflow in NATS JetStream with subject: cb.workflows.{id}.definition",
    );
    console.log(
      '   • NATS stream "CIRCUIT_BREAKER_GLOBAL" now contains this workflow definition',
    );

    await (client as any).pauseForDemo(
      "STEP 1 COMPLETE: Workflow created and stored in NATS",
    );

    // Brief delay to ensure workflow is fully persisted in NATS
    console.log("⏳ Waiting for NATS persistence...");
    await new Promise((resolve) => setTimeout(resolve, 500));

    // Step 2: Create workflow instances using NATS-enhanced mutations
    console.log("\n📄 Creating workflow instances with NATS tracking...");
    console.log("🔍 About to demonstrate:");
    console.log("   • NATS-enhanced GraphQL mutation: createWorkflowInstance");
    console.log(
      "   • Each resource will be stored as a message in NATS with metadata",
    );
    console.log("   • Real-time event publishing to NATS subjects");

    await (client as any).pauseForDemo(
      "Ready to create workflow instances with NATS tracking",
    );

    const instances = [
      {
        title: "TypeScript Technical Specification",
        department: "engineering",
      },
      { title: "TypeScript Marketing Proposal", department: "marketing" },
      { title: "TypeScript Legal Contract", department: "legal" },
    ];

    const resourceIds: string[] = [];

    for (const { title, department } of instances) {
      try {
        const resource = await client.createWorkflowInstance({
          workflowId: workflow.id,
          initialData: {
            title,
            content: `This is the TypeScript content for ${title}`,
            priority: "medium",
          },
          metadata: {
            department,
            created_by: "typescript_demo_user",
            urgency: "normal",
          },
          triggeredBy: "typescript_nats_demo",
        });

        resourceIds.push(resource.id);

        console.log(`📝 Created instance: ${title} (Resource: ${resource.id})`);
        console.log(`   📍 State: ${resource.state}`);
        console.log(`   🔗 NATS Subject: ${resource.natsSubject || "N/A"}`);

        if (resource.natsSequence) {
          console.log(`   📊 NATS Sequence: ${resource.natsSequence}`);
        }

        console.log(`   🔍 Debug: Resource ID added to list: ${resource.id}`);
        console.log("   ✨ This resource is now persisted in NATS JetStream!");
      } catch (error) {
        console.error(`❌ Failed to create instance for ${title}:`, error);
      }
    }

    // Add verification step
    console.log("\n🔍 Verifying all resources were created successfully...");
    console.log(`📊 Created ${resourceIds.length} resources with IDs:`);
    resourceIds.forEach((id, index) => {
      console.log(`   ${index + 1}. ${id}`);
    });

    await (client as any).pauseForDemo(
      "STEP 2 COMPLETE: All workflow instances created and stored in NATS",
    );

    // Step 3: Query resources in specific states using NATS-optimized queries
    console.log("\n🔍 Querying resources in 'draft' state using NATS...");
    console.log("🔍 About to demonstrate:");
    console.log("   • NATS-optimized GraphQL query: resourcesInState");
    console.log("   • Efficient filtering using NATS subject patterns");
    console.log("   • Retrieving resources from specific workflow states");

    await (client as any).pauseForDemo(
      "Ready to query resources using NATS-optimized operations",
    );

    try {
      const resourcesInDraft = await client.getResourcesInState(
        workflow.id,
        "draft",
      );
      console.log(
        `📊 Found ${resourcesInDraft.length} resources in 'draft' state`,
      );

      for (const resource of resourcesInDraft) {
        const title = resource.data?.title || "Unknown";
        console.log(`   🎫 Resource ${resource.id}: ${title}`);
      }

      console.log("\n✨ These results came directly from NATS JetStream!");
      console.log(
        "   • Query used NATS subject filtering: cb.workflows.{id}.states.draft.resources",
      );
      console.log(
        "   • Much faster than scanning all resources in traditional databases",
      );
    } catch (error) {
      console.error("❌ Failed to query resources in state:", error);
    }

    await (client as any).pauseForDemo(
      "STEP 3 COMPLETE: Successfully queried tokens using NATS",
    );

    // Step 4: Perform transitions with NATS event tracking
    console.log("\n⚡ Performing transitions with NATS event tracking...");
    console.log("🔍 About to demonstrate:");
    console.log("   • NATS-enhanced activity: executeActivityWithNats");
    console.log("   • Real-time event publishing to transition event streams");
    console.log(
      "   • Automatic NATS metadata tracking (sequences, timestamps)",
    );
    console.log('   • Moving the FIRST token from "draft" to "review" place');
    console.log(
      "   • (Note: Only transitioning one token to keep demo focused)",
    );

    await (client as any).pauseForDemo(
      "Ready to perform a NATS-tracked token transition on the first token",
    );

    if (resourceIds.length > 0) {
      const firstTokenId = resourceIds[0];
      console.log(
        `🔍 Debug: Attempting to transition the first token ID: ${firstTokenId}`,
      );

      // Add a small delay to ensure token is fully persisted
      await new Promise((resolve) => setTimeout(resolve, 1000));

      try {
        // First, let's verify the token exists by querying it
        console.log(
          "🔍 Verifying resource exists before activity execution...",
        );
        const existingResource = await client.getNATSResource(firstTokenId);

        let transitionedResource: NATSResourceGQL;
        let actualTokenId = firstTokenId;

        if (!existingResource) {
          console.log(
            "❌ Resource not found in NATS storage. Available resources:",
          );
          const allDraftResources = await client.getResourcesInState(
            workflow.id,
            "draft",
          );
          allDraftResources.forEach((resource) => {
            console.log(`   🎫 Available resource: ${resource.id}`);
          });

          if (allDraftResources.length > 0) {
            console.log(
              "🔄 Using first available resource from state query instead...",
            );
            actualTokenId = allDraftResources[0].id;
            resourceIds[0] = actualTokenId; // Update our list

            transitionedResource = await client.executeActivityWithNats({
              resourceId: actualTokenId,
              activityId: "submit_for_review",
              newState: "review",
              triggeredBy: "typescript_nats_demo_transition",
              data: {
                reviewed_by: "typescript_demo_reviewer",
                review_notes: "Ready for review from TypeScript",
              },
            });
          } else {
            throw new Error("No resources available for activity execution");
          }
        } else {
          console.log(
            "✅ Resource found, proceeding with activity execution...",
          );
          transitionedResource = await client.executeActivityWithNats({
            resourceId: firstTokenId,
            activityId: "submit_for_review",
            newState: "review",
            triggeredBy: "typescript_nats_demo_transition",
            data: {
              reviewed_by: "typescript_demo_reviewer",
              review_notes: "Ready for review from TypeScript",
            },
          });
        }

        console.log(
          `✅ Executed activity on resource ${actualTokenId} to state: ${transitionedResource.state}`,
        );

        const history = transitionedResource.activityHistory;
        if (history && history.length > 0) {
          const lastActivity = history[history.length - 1];
          console.log(
            `📝 Last activity: ${lastActivity.fromState} → ${lastActivity.toState}`,
          );
        } else {
          console.log("   📈 No activity history found");
        }

        console.log("\n✨ Transition completed with full NATS event tracking!");
        console.log(
          "   • Transition event published to: cb.workflows.{id}.events.transitions",
        );
        console.log(
          "   • Token moved to new NATS subject: cb.workflows.{id}.places.review.tokens",
        );
        console.log("   • All changes are now persistent in NATS JetStream");
        console.log(
          '   • NOTE: The other tokens remain in "draft" state (only first token was transitioned)',
        );
      } catch (error) {
        console.error("❌ Failed to perform transition:", error);
        console.log(
          "💡 This might be due to timing issues with NATS persistence or token ID mismatch",
        );
      }
    }

    await (client as any).pauseForDemo(
      "STEP 4 COMPLETE: Token transition with NATS event tracking",
    );

    // Step 5: Demonstrate NATS-enhanced token retrieval
    console.log("\n🔎 Retrieving token with NATS metadata...");
    console.log("🔍 About to demonstrate:");
    console.log("   • Enhanced token retrieval with full NATS metadata");
    console.log("   • Complete transition history with NATS sequences");
    console.log("   • Real-time timestamps from NATS JetStream");

    await (client as any).pauseForDemo(
      "Ready to retrieve token with complete NATS metadata",
    );

    if (resourceIds.length > 0) {
      const tokenId = resourceIds[0];
      try {
        const natsResource = await client.getNATSResource(tokenId);
        if (natsResource) {
          console.log("🎫 NATS Resource Details:");
          console.log(`   📋 ID: ${natsResource.id}`);
          console.log(`   📍 Current State: ${natsResource.state}`);
          console.log(
            `   🔗 NATS Subject: ${natsResource.natsSubject || "N/A"}`,
          );

          if (natsResource.natsSequence) {
            console.log(`   📊 NATS Sequence: ${natsResource.natsSequence}`);
          }

          if (natsResource.natsTimestamp) {
            console.log(`   🕒 NATS Timestamp: ${natsResource.natsTimestamp}`);
          }

          const history = natsResource.activityHistory || [];
          console.log(`   📈 Activity History (${history.length} events):`);

          history.forEach((activity, i) => {
            console.log(
              `      ${i + 1}. ${activity.fromState} → ${activity.toState} (${activity.activityId})`,
            );
            if (activity.timestamp) {
              console.log(
                `         🕒 At: ${new Date(activity.timestamp).toLocaleString()}`,
              );
            }
          });

          console.log("\n✨ Complete audit trail stored in NATS!");
          console.log("   • Every transition is immutably recorded");
          console.log("   • NATS sequence numbers provide ordering guarantees");
          console.log(
            "   • Distributed teams can see real-time workflow progress",
          );
        } else {
          console.log("❌ Token not found");
        }
      } catch (error) {
        console.error("❌ Failed to get NATS token:", error);
      }
    }

    await (client as any).pauseForDemo(
      "STEP 5 COMPLETE: Retrieved token with full NATS metadata",
    );

    console.log("\n🎉 TypeScript NATS Integration Demo Features Demonstrated:");
    console.log("   ✅ NATS JetStream storage backend (server-side)");
    console.log("   ✅ Automatic stream creation per workflow");
    console.log("   ✅ Enhanced token tracking with NATS metadata");
    console.log("   ✅ Event-driven transition recording");
    console.log("   ✅ Efficient place-based token queries");
    console.log("   ✅ Real-time transition history with NATS sequences");
    console.log("   ✅ GraphQL API integration with NATS storage");
    console.log("   ✅ TypeScript client library for NATS workflows");

    console.log("\n🚀 NATS Benefits Demonstrated:");
    console.log(
      "   🔄 Distributed: Multiple services can connect to the same NATS cluster",
    );
    console.log("   💾 Persistent: All data survives server restarts");
    console.log("   ⚡ Fast: Subject-based filtering is extremely efficient");
    console.log(
      "   🔒 Reliable: Built-in acknowledgments and replay capability",
    );
    console.log("   📈 Scalable: Handles millions of messages per second");

    await (client as any).pauseForDemo(
      "DEMO COMPLETE: All NATS integration features demonstrated",
    );
  } catch (error) {
    console.error("❌ Demo failed:", error);
    throw error;
  }
}

async function main(): Promise<void> {
  console.log("🚀 Circuit Breaker NATS Integration Demo (TypeScript Client)");
  console.log("=============================================================");
  console.log(
    "This interactive demo will walk you through NATS integration step-by-step.",
  );
  console.log("");
  console.log("📋 Prerequisites:");
  console.log(
    "   1. NATS server running: docker run -p 4222:4222 -p 8222:8222 nats:alpine --jetstream --http_port 8222",
  );
  console.log(
    "   2. Circuit Breaker server with NATS: export STORAGE_BACKEND=nats && cargo run --bin server",
  );
  console.log("   3. Server should be running on localhost:4000");
  console.log("");
  console.log("🎯 What you'll see:");
  console.log("   • Live workflow creation and storage in NATS JetStream");
  console.log("   • Real-time token operations with NATS metadata tracking");
  console.log("   • Efficient place-based queries using NATS subject patterns");
  console.log("   • Event-driven transitions with complete audit trails");
  console.log(
    "   • Polyglot architecture: TypeScript client → GraphQL → NATS-powered Rust backend",
  );
  console.log("");
  console.log("⏸️  Ready to begin the demo?");
  console.log("   Press Enter to start...");

  // Wait for user input to start
  await new Promise((resolve) => {
    process.stdin.once("data", () => resolve(void 0));
  });

  try {
    await runNATSWorkflowDemo();
    console.log(
      "\n✅ TypeScript NATS integration demo completed successfully!",
    );
    console.log("");
    console.log("🎓 What you learned:");
    console.log(
      "   • How NATS JetStream provides distributed workflow storage",
    );
    console.log("   • Real-time event publishing and consumption patterns");
    console.log("   • Efficient querying using NATS subject hierarchies");
    console.log("   • Complete audit trails with immutable event sequences");
    console.log("   • Polyglot workflow architecture benefits");
    console.log("");
    console.log("🔗 Next Steps:");
    console.log("   • Explore the NATS admin interface: http://localhost:8222");
    console.log("   • Try the Rust demo: cargo run --example nats_demo");
    console.log("   • Check the documentation: docs/NATS_IMPLEMENTATION.md");
    console.log("");
    console.log("⏸️  Demo session ending...");
    console.log("   Press Enter to exit.");

    // Final pause
    await new Promise((resolve) => {
      process.stdin.once("data", () => resolve(void 0));
    });
  } catch (error) {
    console.error("\n❌ Demo failed:", error);
    console.log(
      "💡 Make sure the Circuit Breaker server is running on localhost:4000 with NATS storage",
    );
    process.exit(1);
  }
}

// Run the demo
if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch(console.error);
}
