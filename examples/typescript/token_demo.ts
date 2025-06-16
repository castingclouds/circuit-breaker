#!/usr/bin/env npx tsx
// Resource operations demonstration - TypeScript GraphQL Client
// Shows detailed resource lifecycle operations using GraphQL API
// Run with: npx tsx examples/typescript/token_demo.ts

import { CircuitBreakerClient, type ResourceGQL } from "./basic_workflow.js";

function logSuccess(message: string) {
  console.log(`✅ ${message}`);
}

function logInfo(message: string) {
  console.log(`ℹ️  ${message}`);
}

function logError(message: string) {
  console.log(`❌ ${message}`);
}

function logWarning(message: string) {
  console.log(`⚠️  ${message}`);
}

async function main() {
  console.log(
    "🚀 Circuit Breaker Resource Operations Demo - TypeScript Client",
  );
  console.log(
    "===============================================================",
  );
  console.log();

  const client = new CircuitBreakerClient();

  try {
    // Create AI content creation workflow
    logInfo("Creating AI Content Creation Workflow...");

    const workflowInput = {
      name: "AI-Powered Content Creation",
      states: [
        "ideation",
        "drafting",
        "review",
        "revision",
        "approval",
        "published",
        "archived",
      ],
      activities: [
        {
          id: "start_drafting",
          fromStates: ["ideation"],
          toState: "drafting",
          conditions: [],
        },
        {
          id: "submit_for_review",
          fromStates: ["drafting"],
          toState: "review",
          conditions: [],
        },
        {
          id: "request_revision",
          fromStates: ["review"],
          toState: "revision",
          conditions: [],
        },
        {
          id: "back_to_drafting",
          fromStates: ["revision"],
          toState: "drafting",
          conditions: [],
        },
        {
          id: "approve_content",
          fromStates: ["review"],
          toState: "approval",
          conditions: [],
        },
        {
          id: "publish_content",
          fromStates: ["approval"],
          toState: "published",
          conditions: [],
        },
        {
          id: "archive_content",
          fromStates: ["published"],
          toState: "archived",
          conditions: [],
        },
        {
          id: "back_to_ideation",
          fromStates: ["archived"],
          toState: "ideation",
          conditions: [],
        },
      ],
      initialState: "ideation",
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
    console.log();

    // Create multiple content resources
    const contentTopics = [
      {
        title: "Introduction to Rust Programming",
        type: "tutorial",
        targetAudience: "beginners",
        estimatedReadTime: 15,
      },
      {
        title: "Advanced TypeScript Patterns",
        type: "guide",
        targetAudience: "intermediate",
        estimatedReadTime: 25,
      },
      {
        title: "Building Scalable APIs with GraphQL",
        type: "article",
        targetAudience: "advanced",
        estimatedReadTime: 20,
      },
    ];

    const resources: ResourceGQL[] = [];

    for (const [index, topic] of contentTopics.entries()) {
      logInfo(`Creating content resource ${index + 1}/3: ${topic.title}`);

      const resourceInput = {
        workflowId: workflow.id,
        initialState: "ideation",
        data: {
          title: topic.title,
          type: topic.type,
          targetAudience: topic.targetAudience,
          estimatedReadTime: topic.estimatedReadTime,
          keywords: topic.title.toLowerCase().split(" ").slice(0, 3),
          status: "planning",
          wordCount: 0,
          authorId: "ai-assistant",
          createdAt: new Date().toISOString(),
        },
        metadata: {
          priority: index === 1 ? "high" : "medium",
          contentType: topic.type,
          seoOptimized: false,
          socialMediaReady: false,
          version: "1.0",
        },
      };

      const resourceResult = await client.createResource(resourceInput);

      if (resourceResult.errors) {
        logError(
          `Failed to create resource: ${resourceResult.errors.map((e) => e.message).join(", ")}`,
        );
        continue;
      }

      const resource = resourceResult.data!.createResource;
      resources.push(resource);
      logSuccess(`Created resource: ${resource.data.title} (${resource.id})`);
    }

    console.log();
    logInfo(`Created ${resources.length} content resources`);

    // Demonstrate different resource lifecycles
    for (const [index, resource] of resources.entries()) {
      console.log();
      logInfo(`Processing content ${index + 1}: ${resource.data.title}`);

      let currentResource = resource;

      // Start drafting
      logInfo("Starting drafting phase...");
      const draftResult = await client.executeActivity({
        resourceId: currentResource.id,
        activityId: "start_drafting",
        data: {
          action: "start_drafting",
          timestamp: new Date().toISOString(),
          notes: "AI content generation initiated",
        },
      });

      if (draftResult.errors) {
        logError(
          `Failed to start drafting: ${draftResult.errors.map((e) => e.message).join(", ")}`,
        );
        continue;
      }

      currentResource = draftResult.data!.executeActivity;

      // Update resource data to simulate content creation
      currentResource.data.wordCount = Math.floor(Math.random() * 1000) + 500;
      currentResource.data.status = "draft_complete";

      // Submit for review
      logInfo("Submitting for review...");
      const reviewResult = await client.executeActivity({
        resourceId: currentResource.id,
        activityId: "submit_for_review",
        data: {
          action: "submit_for_review",
          timestamp: new Date().toISOString(),
          wordCount: currentResource.data.wordCount,
          readabilityScore: Math.floor(Math.random() * 40) + 60,
        },
      });

      if (reviewResult.errors) {
        logError(
          `Failed to submit for review: ${reviewResult.errors.map((e) => e.message).join(", ")}`,
        );
        continue;
      }

      currentResource = reviewResult.data!.executeActivity;

      // Simulate different review outcomes
      const reviewOutcome = index === 1 ? "revision" : "approval";

      if (reviewOutcome === "revision") {
        logWarning("Content needs revision...");

        const revisionResult = await client.executeActivity({
          resourceId: currentResource.id,
          activityId: "request_revision",
          data: {
            action: "request_revision",
            timestamp: new Date().toISOString(),
            feedback: "Needs more technical examples and clearer explanations",
          },
        });

        if (revisionResult.errors) {
          logError(
            `Failed to request revision: ${revisionResult.errors.map((e) => e.message).join(", ")}`,
          );
          continue;
        }

        currentResource = revisionResult.data!.executeActivity;

        // Back to drafting
        logInfo("Returning to drafting for revisions...");
        const backToDraftResult = await client.executeActivity({
          resourceId: currentResource.id,
          activityId: "back_to_drafting",
          data: {
            action: "back_to_drafting",
            timestamp: new Date().toISOString(),
            revisionNotes: "Incorporating reviewer feedback",
          },
        });

        if (backToDraftResult.errors) {
          logError(
            `Failed to return to drafting: ${backToDraftResult.errors.map((e) => e.message).join(", ")}`,
          );
          continue;
        }

        currentResource = backToDraftResult.data!.executeActivity;

        // Resubmit
        logInfo("Resubmitting revised content...");
        const resubmitResult = await client.executeActivity({
          resourceId: currentResource.id,
          activityId: "submit_for_review",
          data: {
            action: "resubmit_for_review",
            timestamp: new Date().toISOString(),
            revisionComplete: true,
            wordCount: currentResource.data.wordCount + 200,
          },
        });

        if (resubmitResult.errors) {
          logError(
            `Failed to resubmit: ${resubmitResult.errors.map((e) => e.message).join(", ")}`,
          );
          continue;
        }

        currentResource = resubmitResult.data!.executeActivity;
      }

      // Approve content
      logInfo("Approving content...");
      const approveResult = await client.executeActivity({
        resourceId: currentResource.id,
        activityId: "approve_content",
        data: {
          action: "approve_content",
          timestamp: new Date().toISOString(),
          approvedBy: "content-manager",
          qualityScore: Math.floor(Math.random() * 20) + 80,
        },
      });

      if (approveResult.errors) {
        logError(
          `Failed to approve content: ${approveResult.errors.map((e) => e.message).join(", ")}`,
        );
        continue;
      }

      currentResource = approveResult.data!.executeActivity;

      // Publish content
      logInfo("Publishing content...");
      const publishResult = await client.executeActivity({
        resourceId: currentResource.id,
        activityId: "publish_content",
        data: {
          action: "publish_content",
          timestamp: new Date().toISOString(),
          publishUrl: `https://blog.example.com/${currentResource.data.title.toLowerCase().replace(/\s+/g, "-")}`,
          seoOptimized: true,
        },
      });

      if (publishResult.errors) {
        logError(
          `Failed to publish content: ${publishResult.errors.map((e) => e.message).join(", ")}`,
        );
        continue;
      }

      currentResource = publishResult.data!.executeActivity;
      logSuccess(`Content published: ${currentResource.data.title}`);

      // Show resource history
      logInfo("Resource lifecycle history:");
      currentResource.history.forEach((event, histIndex) => {
        const timestamp = new Date(event.timestamp).toLocaleTimeString();
        console.log(
          `  ${histIndex + 1}. ${event.fromState} → ${event.toState} via ${event.activity} (${timestamp})`,
        );
      });
    }

    console.log();
    logInfo("Resource Demo Summary:");
    console.log("  • Created multiple content resources with different data");
    console.log("  • Demonstrated complex workflow with revision cycles");
    console.log("  • Showed resource state transitions and data updates");
    console.log("  • Complete audit trail for each resource lifecycle");
    console.log("  • TypeScript integration with GraphQL API");
  } catch (error) {
    logError(`Demo failed: ${error}`);
    process.exit(1);
  }
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch(console.error);
}

export { main as resourceDemo };
