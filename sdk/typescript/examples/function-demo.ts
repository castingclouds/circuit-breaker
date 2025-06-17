#!/usr/bin/env tsx
/**
 * Function System Demo - Circuit Breaker TypeScript SDK
 *
 * This example demonstrates the function system capabilities:
 * - Creating and managing Docker-based functions
 * - Event-driven triggers and function chains
 * - Function execution with real containers
 * - Resource management and monitoring
 * - Function templates and builders
 *
 * Run with: npx tsx examples/function-demo.ts
 */

/// <reference types="node" />

import {
  CircuitBreakerSDK,
  createFunctionBuilder,
  createFunctionTemplate,
  FunctionManager,
  FunctionBuilder,
  CommonTemplates,
  FunctionDefinition,
  ContainerConfig,
  EventTrigger,
  FunctionChain,
  ResourceLimits,
  ContainerMount,
  InputMapping,
  ChainCondition,
  FunctionCreateInput,
  FunctionExecuteInput,
  CircuitBreakerError,
  FunctionError,
  formatError,
  generateRequestId,
} from "../src/index.js";

// ============================================================================
// Configuration
// ============================================================================

const config = {
  graphqlEndpoint:
    process.env.CIRCUIT_BREAKER_ENDPOINT || "http://localhost:4000/graphql",
  timeout: 60000, // Longer timeout for function operations
  debug: process.env.NODE_ENV === "development",
  logging: {
    level: "info" as const,
    structured: false,
  },
  headers: {
    "User-Agent": "CircuitBreaker-SDK-FunctionDemo/0.1.0",
  },
};

// ============================================================================
// Helper Functions
// ============================================================================

function logSuccess(message: string, data?: any): void {
  console.log(`✅ ${message}`);
  if (data && config.debug) {
    console.log(JSON.stringify(data, null, 2));
  }
}

function logInfo(message: string, data?: any): void {
  console.log(`ℹ️  ${message}`);
  if (data && config.debug) {
    console.log(JSON.stringify(data, null, 2));
  }
}

function logError(message: string, error?: any): void {
  console.error(`❌ ${message}`);
  if (error) {
    if (error instanceof CircuitBreakerError) {
      console.error(`   Error: ${formatError(error)}`);
      if (error.context && config.debug) {
        console.error(`   Context: ${JSON.stringify(error.context, null, 2)}`);
      }
    } else {
      console.error(`   ${error.message || error}`);
      if (config.debug && error.stack) {
        console.error(`   Stack: ${error.stack}`);
      }
    }
  }
}

function logWarning(message: string, data?: any): void {
  console.warn(`⚠️  ${message}`);
  if (data && config.debug) {
    console.warn(JSON.stringify(data, null, 2));
  }
}

// ============================================================================
// Function Definitions
// ============================================================================

function createDataProcessorFunction(): FunctionDefinition {
  return {
    id: "data-processor-" + generateRequestId().slice(0, 8),
    name: "Data Processor",
    description: "Processes incoming data and transforms it",
    enabled: true,
    container: {
      image: "node:18-alpine",
      command: [
        "node",
        "-e",
        `
      const data = JSON.parse(process.env.INPUT_DATA || '{}');
      const processed = {
        ...data,
        processed: true,
        timestamp: new Date().toISOString(),
        processingId: Math.random().toString(36).substr(2, 9)
      };
      console.log(JSON.stringify(processed));
    `,
      ],
      workingDir: "/app",
      environment: {},
      mounts: [],
      resources: {
        memory: 128 * 1024 * 1024,
        cpu: 0.5,
      },
    },
    triggers: [
      {
        type: "resource_state",
        condition: 'data.type === "raw"',
        description: "Trigger when raw data token is created",
        inputMapping: "full_data",
        enabled: true,
      },
    ],
    chains: [],
    inputSchema: {
      schema: {
        type: "object",
        properties: {
          data: { type: "object" },
          type: { type: "string" },
        },
        required: ["data"],
      },
    },
    outputSchema: {
      schema: {
        type: "object",
        properties: {
          processed: { type: "boolean" },
          timestamp: { type: "string" },
          processingId: { type: "string" },
        },
      },
    },
    tags: ["data-processing", "transformation"],
  };
}

function createValidatorFunction(): FunctionDefinition {
  return {
    id: "validator-" + generateRequestId().slice(0, 8),
    name: "Data Validator",
    description: "Validates processed data against business rules",
    enabled: true,
    container: {
      image: "node:18-alpine",
      command: [
        "node",
        "-e",
        `
      const data = JSON.parse(process.env.INPUT_DATA || '{}');
      const isValid = data.processed && data.timestamp && data.processingId;
      const result = {
        ...data,
        validated: isValid,
        validationErrors: isValid ? [] : ['Missing required processing fields'],
        validatedAt: new Date().toISOString()
      };
      console.log(JSON.stringify(result));
    `,
      ],
      workingDir: "/app",
      environment: {
        VALIDATION_MODE: "strict",
      },
      mounts: [],
      resources: {
        memory: 64 * 1024 * 1024,
        cpu: 0.25,
      },
    },
    triggers: [
      {
        type: "function_completion",
        condition: "output.processed === true",
        description: "Validate data after processing",
        inputMapping: "full_data",
        enabled: true,
      },
    ],
    chains: [],
    inputSchema: {
      schema: {
        type: "object",
        properties: {
          processed: { type: "boolean" },
          timestamp: { type: "string" },
          processingId: { type: "string" },
        },
        required: ["processed", "timestamp", "processingId"],
      },
    },
    outputSchema: {
      schema: {
        type: "object",
        properties: {
          validated: { type: "boolean" },
          validationErrors: { type: "array", items: { type: "string" } },
          validatedAt: { type: "string" },
        },
      },
    },
    tags: ["validation", "quality-control"],
  };
}

function createNotificationFunction(): FunctionDefinition {
  return {
    id: "notification-" + generateRequestId().slice(0, 8),
    name: "Notification Service",
    description: "Sends notifications based on processing results",
    enabled: true,
    container: {
      image: "alpine:latest",
      command: [
        "sh",
        "-c",
        `
      echo "Processing notification..."
      DATA=$(echo "$INPUT_DATA" | sed 's/"/\\"/g')
      echo "Notification sent for: $DATA"
      echo '{"notified": true, "notificationId": "'$(date +%s)'", "sentAt": "'$(date -Iseconds)'"}'
    `,
      ],
      workingDir: "/app",
      environment: {
        NOTIFICATION_CHANNEL: "email",
      },
      mounts: [],
      resources: {
        memory: 32 * 1024 * 1024,
        cpu: 0.1,
      },
    },
    triggers: [
      {
        type: "function_completion",
        condition: "output.validated === true",
        description: "Send notification for successfully validated data",
        inputMapping: "full_data",
        enabled: true,
      },
    ],
    chains: [],
    inputSchema: {
      schema: {
        type: "object",
        properties: {
          validated: { type: "boolean" },
          validationErrors: { type: "array" },
        },
      },
    },
    outputSchema: {
      schema: {
        type: "object",
        properties: {
          notified: { type: "boolean" },
          notificationId: { type: "string" },
          sentAt: { type: "string" },
        },
      },
    },
    tags: ["notification", "communication"],
  };
}

// ============================================================================
// Main Demo Function
// ============================================================================

async function runFunctionDemo(): Promise<void> {
  console.log("🚀 Starting Function System Demo");
  console.log("=====================================\n");

  try {
    // Initialize SDK
    logInfo("Initializing Circuit Breaker SDK...");
    const sdk = new CircuitBreakerSDK(config);

    // Test connection
    logInfo("Testing SDK connection...");
    const sdkHealth = await sdk.getHealth();
    const sdkConfig = sdk.getConfig();
    logSuccess("SDK initialized successfully", {
      version: sdk.getVersion(),
      healthy: sdkHealth.healthy,
      endpoint: sdkConfig.graphqlEndpoint,
    });

    // ========================================================================
    // Create Functions
    // ========================================================================

    logInfo("\n📦 Creating Functions...");

    // Create data processor function
    const dataProcessor = createDataProcessorFunction();
    logInfo(`Creating data processor function: ${dataProcessor.name}`);
    const dataProcessorId = await sdk.functions.create(dataProcessor);
    logSuccess(`Data processor created with ID: ${dataProcessorId}`);

    // Create validator function
    const validator = createValidatorFunction();
    logInfo(`Creating validator function: ${validator.name}`);
    const validatorId = await sdk.functions.create(validator);
    logSuccess(`Validator created with ID: ${validatorId}`);

    // Create notification function
    const notification = createNotificationFunction();
    logInfo(`Creating notification function: ${notification.name}`);
    const notificationId = await sdk.functions.create(notification);
    logSuccess(`Notification function created with ID: ${notificationId}`);

    // ========================================================================
    // Function Management
    // ========================================================================

    logInfo("\n🔧 Function Management Operations...");

    // List all functions
    const functions = await sdk.functions.list();
    logInfo(`Total functions in system: ${functions.length}`);

    // Get function details
    const processorDetails = await sdk.functions.get(dataProcessorId);
    logSuccess("Retrieved function details", {
      id: processorDetails.id,
      name: processorDetails.name,
      enabled: processorDetails.enabled,
      triggers: processorDetails.triggers.length,
    });

    // Update function
    await sdk.functions.update(dataProcessorId, {
      description: "Enhanced data processor with additional capabilities",
      tags: [...(dataProcessor.tags || []), "enhanced"],
    });
    logSuccess("Function updated successfully");

    // ========================================================================
    // Function Execution
    // ========================================================================

    logInfo("\n⚡ Function Execution...");

    // Execute data processor directly
    const executionInput: FunctionExecuteInput = {
      functionId: dataProcessorId,
      input: {
        data: {
          type: "raw",
          content: "Sample data for processing",
          source: "demo",
        },
      },
    };

    logInfo("Executing data processor function...");
    const executionResult = await sdk.functions.execute(executionInput);
    logSuccess("Function executed successfully", {
      status: executionResult.status,
      duration: executionResult.duration,
      outputPreview: executionResult.output
        ? JSON.stringify(executionResult.output).substring(0, 100) + "..."
        : "No output",
    });

    // ========================================================================
    // Function Builder Usage
    // ========================================================================

    logInfo("\n🏗️  Using Function Builder...");

    // Create function using the function manager directly
    const builtFunction: FunctionDefinition = {
      id: "advanced-processor-" + generateRequestId().slice(0, 8),
      name: "Advanced Data Processor",
      description: "Built with the function builder API",
      enabled: true,
      container: {
        image: "python:3.9-slim",
        command: [
          "python",
          "-c",
          `
import json
import sys
import os

data = json.loads(os.environ.get('INPUT_DATA', '{}'))
result = {
    'original': data,
    'enhanced': True,
    'processing_method': 'python',
    'features_extracted': len(str(data)),
    'builder_created': True
}
print(json.dumps(result))
        `,
        ],
        workingDir: "/app",
        environment: {},
        mounts: [],
        resources: {
          memory: 256 * 1024 * 1024,
          cpu: 1,
        },
      },
      triggers: [
        {
          type: "resource_state",
          condition: "data.enhanced === undefined",
          inputMapping: "full_data",
          description: "Process unenhanced data",
          enabled: true,
        },
      ],
      chains: [],
      inputSchema: {
        schema: {
          type: "object",
          properties: {
            data: { type: "object" },
          },
        },
      },
      outputSchema: {
        schema: {
          type: "object",
          properties: {
            enhanced: { type: "boolean" },
            processing_method: { type: "string" },
          },
        },
      },
      tags: ["builder-created", "python-processor"],
    };

    const builtFunctionId = await sdk.functions.create(builtFunction);
    logSuccess(`Function built and created with ID: ${builtFunctionId}`);

    // ========================================================================
    // Function Templates
    // ========================================================================

    logInfo("\n📋 Using Function Templates...");

    // Create a function using a template pattern
    const templateFunction: FunctionDefinition = {
      id: "json-transformer-" + generateRequestId().slice(0, 8),
      name: "JSON Transformer",
      description: "Data transformer created from template",
      enabled: true,
      container: {
        image: "node:18-alpine",
        command: [
          "node",
          "-e",
          `
        const data = JSON.parse(process.env.INPUT_DATA || '{}');
        const transformed = {
          ...data,
          transformed: true,
          transformedAt: new Date().toISOString(),
          template: 'DataTransformer'
        };
        console.log(JSON.stringify(transformed));
      `,
        ],
        workingDir: "/app",
        environment: {},
        mounts: [],
        resources: {
          memory: 128 * 1024 * 1024,
          cpu: 0.5,
        },
      },
      triggers: [],
      chains: [],
      inputSchema: {
        schema: {
          type: "object",
        },
      },
      outputSchema: {
        schema: {
          type: "object",
          properties: {
            transformed: { type: "boolean" },
            transformedAt: { type: "string" },
          },
        },
      },
      tags: ["template", "json", "transformer"],
    };

    const templateFunctionId = await sdk.functions.create(templateFunction);
    logSuccess(`Template function created with ID: ${templateFunctionId}`);

    // ========================================================================
    // Function Monitoring
    // ========================================================================

    logInfo("\n📊 Function Monitoring...");

    // Get function statistics
    const stats = await sdk.functions.getStats(dataProcessorId);
    logInfo("Function statistics", {
      totalExecutions: stats.totalExecutions,
      successRate: stats.successRate,
      averageDuration: stats.averageDuration,
      lastExecution: stats.lastExecution,
    });

    // Get function health
    const health = await sdk.functions.getHealth(dataProcessorId);
    logInfo("Function health status", {
      status: health.status,
      isHealthy: health.isHealthy,
      lastHealthCheck: health.lastHealthCheck,
    });

    // ========================================================================
    // Cleanup
    // ========================================================================

    logInfo("\n🧹 Cleanup...");

    // Disable functions before cleanup
    await sdk.functions.update(dataProcessorId, { enabled: false });
    await sdk.functions.update(validatorId, { enabled: false });
    await sdk.functions.update(notificationId, { enabled: false });
    await sdk.functions.update(builtFunctionId, { enabled: false });
    await sdk.functions.update(templateFunctionId, { enabled: false });

    logSuccess("Functions disabled for cleanup");

    // Note: In a real scenario, you might want to delete functions
    // await sdk.functions.delete(dataProcessorId);
    // But for demo purposes, we'll leave them disabled

    console.log("\n✨ Function Demo completed successfully!");
    console.log("=====================================");
  } catch (error) {
    logError("Function demo failed", error);
    process.exit(1);
  }
}

// ============================================================================
// Run Demo
// ============================================================================

if (import.meta.url === `file://${process.argv[1]}`) {
  runFunctionDemo()
    .then(() => {
      logSuccess("Demo completed successfully");
      process.exit(0);
    })
    .catch((error) => {
      logError("Demo failed", error);
      process.exit(1);
    });
}

export { runFunctionDemo };
