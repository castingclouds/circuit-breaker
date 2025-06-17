/**
 * Function Management Example
 *
 * This example demonstrates comprehensive function management using the
 * Circuit Breaker TypeScript SDK, including function creation, execution,
 * container management, and advanced orchestration features.
 */

import {
  CircuitBreakerSDK,
  createFunctionBuilder,
  createFunctionTemplate,
  FunctionManager,
  FunctionBuilder,
  CommonTemplates,
} from '../src/index.js';

// Initialize SDK
const sdk = new CircuitBreakerSDK({
  graphqlEndpoint: 'http://localhost:4000/graphql',
  debug: true,
});

async function basicFunctionOperations() {
  console.log('=== Basic Function Operations ===');

  const functionManager = sdk.functions;

  try {
    // 1. Create a simple HTTP API function
    console.log('Creating HTTP API function...');
    const apiFunction = await functionManager.create({
      name: 'user-api',
      container: {
        image: 'node:18-alpine',
        command: ['node', 'server.js'],
        environment: {
          NODE_ENV: 'production',
          PORT: '3000',
        },
        resources: {
          cpu: 1,
          memory: 512 * 1024 * 1024, // 512MB
        },
        workingDir: '/app',
        labels: {
          'function.type': 'api',
          'function.language': 'nodejs',
        },
      },
      triggers: [
        {
          type: 'webhook',
          condition: '/api/users/*',
          enabled: true,
        },
      ],
      description: 'User management API',
      tags: ['api', 'users', 'nodejs'],
      metadata: {
        author: 'development-team',
        version: '1.0.0',
      },
      enabled: true,
    });

    console.log('✅ HTTP API function created:', {
      id: apiFunction.id,
      name: apiFunction.name,
      image: apiFunction.container.image,
    });

    // 2. Create a data processing function
    console.log('\nCreating data processing function...');
    const processingFunction = await functionManager.create({
      name: 'data-processor',
      container: {
        image: 'python:3.11-slim',
        command: ['python', 'processor.py'],
        environment: {
          PYTHONPATH: '/app',
          WORKER_THREADS: '4',
        },
        resources: {
          cpu: 2,
          memory: 1024 * 1024 * 1024, // 1GB
        },
        mounts: [
          {
            source: '/data',
            target: '/app/data',
            type: 'bind',
            readonly: true,
          },
        ],
      },
      triggers: [
        {
          type: 'workflow_event',
          condition: "event.type == 'data_uploaded'",
          enabled: true,
        },
      ],
      chains: [
        {
          targetFunction: 'user-api',
          condition: 'success',
          inputMapping: 'metadata_only',
          description: 'Notify API of processing completion',
        },
      ],
      description: 'Process uploaded data files',
      tags: ['processing', 'data', 'python'],
      enabled: true,
    });

    console.log('✅ Data processing function created:', {
      id: processingFunction.id,
      name: processingFunction.name,
      triggers: processingFunction.triggers?.length,
      chains: processingFunction.chains?.length,
    });

    // 3. Execute the processing function
    console.log('\nExecuting data processing function...');
    const executionResult = await functionManager.execute({
      functionId: processingFunction.id,
      input: {
        filePath: '/data/sample.csv',
        outputFormat: 'json',
        validationRules: ['required_fields', 'data_types'],
      },
      timeout: 60000, // 1 minute
      metadata: {
        triggeredBy: 'manual-test',
        priority: 'high',
      },
    });

    console.log('✅ Function executed:', {
      executionId: executionResult.executionId,
      status: executionResult.status,
      executionTime: executionResult.executionTime,
      outputSize: JSON.stringify(executionResult.output || {}).length,
    });

    return [apiFunction.id, processingFunction.id];
  } catch (error) {
    console.error('❌ Error in basic operations:', error);
    throw error;
  }
}

async function functionBuilderExample() {
  console.log('\n=== Function Builder Example ===');

  try {
    const functionManager = sdk.functions;

    // Create a function builder with default options
    const builder = createFunctionBuilder(functionManager, {
      validate: true,
      defaultRegistry: 'myregistry.com',
      defaultLimits: {
        cpu: 0.5,
        memory: 256 * 1024 * 1024, // 256MB
      },
      defaultTags: ['circuit-breaker', 'auto-generated'],
      autoEnable: true,
      autoBuild: false,
    });

    // Using fluent interface for Node.js microservice
    console.log('Creating Node.js microservice with fluent interface...');
    const microservice = await builder
      .function('notification-service')
      .image('node:18-alpine')
      .command('npm', 'start')
      .env({
        NODE_ENV: 'production',
        LOG_LEVEL: 'info',
        PORT: '3001',
      })
      .envVar('SERVICE_NAME', 'notification-service')
      .resources({
        cpu: 1,
        memory: 512 * 1024 * 1024,
      })
      .workingDir('/app')
      .mount('/logs', '/app/logs', { type: 'volume' })
      .label('service.type', 'notification')
      .description('Handles email and SMS notifications')
      .version('2.1.0')
      .tags('notifications', 'email', 'sms')
      .onWebhook('/notifications/*')
      .onSchedule('0 */5 * * *') // Every 5 minutes
      .chain('user-api', 'success', {
        inputMapping: 'metadata_only',
        delay: 1000,
        description: 'Update user notification preferences',
      })
      .create();

    console.log('✅ Microservice created with fluent interface:', {
      id: microservice.id,
      name: microservice.name,
      version: microservice.version,
      triggers: microservice.triggers?.length,
    });

    // Using Python ML model function
    console.log('\nCreating Python ML model function...');
    const mlFunction = await builder
      .function('ml-predictor')
      .image('tensorflow/tensorflow:2.13.0-gpu')
      .command('python', 'predict.py')
      .env({
        PYTHONPATH: '/model',
        MODEL_PATH: '/model/trained_model.h5',
        BATCH_SIZE: '32',
      })
      .gpu(1) // Request 1 GPU
      .cpu(4)
      .memory(4 * 1024 * 1024 * 1024) // 4GB
      .mount('/models', '/model', { readonly: true })
      .mount('/tmp', '/tmp', { type: 'tmpfs' })
      .network('ml-network')
      .description('ML model for predictions')
      .trigger('resource_state', "resource.state == 'ready_for_prediction'")
      .inputSchema({
        type: 'object',
        properties: {
          data: { type: 'array' },
          model_version: { type: 'string' },
        },
        required: ['data'],
      })
      .outputSchema({
        type: 'object',
        properties: {
          predictions: { type: 'array' },
          confidence: { type: 'number' },
        },
      })
      .create();

    console.log('✅ ML function created:', {
      id: mlFunction.id,
      name: mlFunction.name,
      gpu: mlFunction.container.resources?.gpu,
      memory: mlFunction.container.resources?.memory,
    });

    return [microservice.id, mlFunction.id];
  } catch (error) {
    console.error('❌ Error in function builder example:', error);
    throw error;
  }
}

async function templatesAndBatchOperations() {
  console.log('\n=== Templates and Batch Operations ===');

  try {
    const functionManager = sdk.functions;
    const builder = createFunctionBuilder(functionManager);

    // Register custom templates
    console.log('Registering custom templates...');

    const webApiTemplate = createFunctionTemplate(
      'web-api-template',
      'node:{{version}}-alpine',
      {
        description: 'Template for web API services',
        containerConfig: {
          command: ['npm', 'start'],
          environment: {
            NODE_ENV: 'production',
            PORT: '{{port}}',
          },
          workingDir: '/app',
          resources: {
            cpu: 1,
            memory: 512 * 1024 * 1024,
          },
        },
        defaultTriggers: [
          {
            type: 'webhook',
            condition: '/api/{{service}}/*',
            enabled: true,
          },
        ],
        parameters: ['version', 'port', 'service'],
        requiredEnv: ['NODE_ENV', 'PORT'],
      }
    );

    const workerTemplate = createFunctionTemplate(
      'background-worker',
      'python:{{version}}-slim',
      {
        description: 'Template for background worker services',
        containerConfig: {
          command: ['python', 'worker.py'],
          environment: {
            WORKER_TYPE: '{{worker_type}}',
            CONCURRENCY: '{{concurrency}}',
          },
          workingDir: '/app',
          resources: {
            cpu: 2,
            memory: 1024 * 1024 * 1024,
          },
        },
        defaultTriggers: [
          {
            type: 'schedule',
            condition: '{{schedule}}',
            enabled: true,
          },
        ],
        parameters: ['version', 'worker_type', 'concurrency', 'schedule'],
      }
    );

    builder.registerTemplate(webApiTemplate);
    builder.registerTemplate(workerTemplate);

    // Create functions from templates
    console.log('\nCreating functions from templates...');

    const apiFromTemplate = await builder.fromTemplate(
      'web-api-template',
      'products-api',
      {
        version: '18',
        port: '3002',
        service: 'products',
      }
    ).create();

    const workerFromTemplate = await builder.fromTemplate(
      'background-worker',
      'email-worker',
      {
        version: '3.11',
        worker_type: 'email',
        concurrency: '4',
        schedule: '0 */2 * * *', // Every 2 hours
      }
    ).create();

    console.log('✅ Functions created from templates:', {
      api: { id: apiFromTemplate.id, name: apiFromTemplate.name },
      worker: { id: workerFromTemplate.id, name: workerFromTemplate.name },
    });

    // Batch function creation
    console.log('\nCreating functions in batch...');

    const batchFunctions = await builder.batch([
      {
        name: 'redis-cache',
        image: 'redis:7-alpine',
        config: {
          command: ['redis-server'],
          environment: { REDIS_PORT: '6379' },
        },
      },
      {
        name: 'postgres-db',
        image: 'postgres:15-alpine',
        config: {
          environment: {
            POSTGRES_DB: 'circuit_breaker',
            POSTGRES_USER: 'admin',
            POSTGRES_PASSWORD: 'secure_password',
          },
        },
      },
      {
        name: 'nginx-proxy',
        image: 'nginx:alpine',
        config: {
          command: ['nginx', '-g', 'daemon off;'],
          environment: { NGINX_PORT: '80' },
        },
      },
    ], {
      description: 'Infrastructure services',
      tags: ['infrastructure', 'support'],
      enabled: true,
    }).create();

    console.log('✅ Batch functions created:', {
      count: batchFunctions.length,
      names: batchFunctions.map(f => f.name),
    });

    // Test batch execution
    console.log('\nTesting batch execution...');

    const batchExecution = await functionManager.executeBatch({
      executions: [
        {
          functionId: apiFromTemplate.id,
          input: { action: 'health_check' },
          timeout: 10000,
        },
        {
          functionId: workerFromTemplate.id,
          input: { task: 'test_email', recipient: 'test@example.com' },
          timeout: 15000,
        },
      ],
      options: {
        concurrency: 2,
        continueOnError: true,
        batchTimeout: 30000,
      },
    });

    console.log('✅ Batch execution completed:', {
      success: batchExecution.success,
      successful: batchExecution.successful,
      failed: batchExecution.failed,
      totalTime: batchExecution.totalTime,
    });

    return [
      apiFromTemplate.id,
      workerFromTemplate.id,
      ...batchFunctions.map(f => f.id),
    ];
  } catch (error) {
    console.error('❌ Error in templates and batch operations:', error);
    throw error;
  }
}

async function containerManagement() {
  console.log('\n=== Container Management ===');

  const functionManager = sdk.functions;

  try {
    // Create a function that requires custom container management
    const customFunction = await functionManager.create({
      name: 'custom-container-function',
      container: {
        image: 'alpine:latest',
        command: ['sh', '-c', 'while true; do echo "Running..."; sleep 30; done'],
        environment: {
          LOG_LEVEL: 'debug',
        },
        resources: {
          cpu: 0.5,
          memory: 128 * 1024 * 1024, // 128MB
        },
      },
      description: 'Function for container management testing',
      enabled: true,
    });

    console.log('✅ Custom function created for container management:', {
      id: customFunction.id,
      name: customFunction.name,
    });

    // Build container image
    console.log('\nBuilding container image...');
    const buildId = await functionManager.buildContainer(customFunction.id);
    console.log('Container build initiated:', { buildId });

    // Start container
    console.log('\nStarting container...');
    const containerId = await functionManager.startContainer(customFunction.id);
    console.log('Container started:', { containerId });

    // Get container status
    console.log('\nGetting container status...');
    const containerStatus = await functionManager.getContainerStatus(customFunction.id);
    console.log('Container status:', {
      status: containerStatus.status,
      containerId: containerStatus.containerId,
      startedAt: containerStatus.startedAt,
    });

    // Get container logs
    console.log('\nGetting container logs...');
    const logs = await functionManager.getContainerLogs(customFunction.id, {
      lines: 10,
      timestamps: true,
    });
    console.log('Container logs (last 10 lines):', logs.slice(0, 3)); // Show first 3 for brevity

    // Stop container
    console.log('\nStopping container...');
    const stopped = await functionManager.stopContainer(customFunction.id);
    console.log('Container stopped:', { success: stopped });

    return customFunction.id;
  } catch (error) {
    console.error('❌ Error in container management:', error);
    throw error;
  }
}

async function functionGroupsAndPipelines() {
  console.log('\n=== Function Groups and Pipelines ===');

  try {
    const functionManager = sdk.functions;
    const builder = createFunctionBuilder(functionManager);

    // Create a function group for microservices
    console.log('Creating microservices function group...');

    const microservicesGroup = await builder
      .group('microservices-stack')
      .addFunctions([
        {
          id: 'gateway',
          name: 'api-gateway',
          container: {
            image: 'nginx:alpine',
            environment: { NGINX_PORT: '80' },
          },
          description: 'API Gateway',
          enabled: true,
        } as any,
        {
          id: 'auth',
          name: 'auth-service',
          container: {
            image: 'node:18-alpine',
            environment: { PORT: '3001' },
          },
          description: 'Authentication Service',
          enabled: true,
        } as any,
        {
          id: 'user',
          name: 'user-service',
          container: {
            image: 'node:18-alpine',
            environment: { PORT: '3002' },
          },
          description: 'User Management Service',
          enabled: true,
        } as any,
      ])
      .sharedEnv({
        NODE_ENV: 'production',
        LOG_LEVEL: 'info',
        DATABASE_URL: 'postgresql://localhost:5432/microservices',
      })
      .sharedLimits({
        cpu: 1,
        memory: 512 * 1024 * 1024,
      })
      .sharedNetwork('microservices-network')
      .metadata({
        project: 'microservices-demo',
        team: 'platform',
      })
      .deploy();

    console.log('✅ Microservices group deployed:', {
      count: microservicesGroup.length,
      services: microservicesGroup.map(f => f.name),
    });

    // Create a data processing pipeline
    console.log('\nCreating data processing pipeline...');

    // First create the pipeline functions
    const ingestionFunction = await builder
      .function('data-ingestion')
      .image('python:3.11-slim')
      .command('python', 'ingest.py')
      .description('Data ingestion stage')
      .create();

    const validationFunction = await builder
      .function('data-validation')
      .image('python:3.11-slim')
      .command('python', 'validate.py')
      .description('Data validation stage')
      .create();

    const transformFunction = await builder
      .function('data-transform')
      .image('python:3.11-slim')
      .command('python', 'transform.py')
      .description('Data transformation stage')
      .create();

    const outputFunction = await builder
      .function('data-output')
      .image('python:3.11-slim')
      .command('python', 'output.py')
      .description('Data output stage')
      .create();

    // Create the pipeline
    const processingPipeline = await builder
      .pipeline('data-processing-pipeline')
      .addStage(ingestionFunction.id, {
        condition: 'always',
        timeout: 30000,
      })
      .addStage(validationFunction.id, {
        condition: 'success',
        inputMapping: 'full_data',
        timeout: 20000,
      })
      .addStage(transformFunction.id, {
        condition: 'success',
        inputMapping: 'full_data',
        timeout: 60000,
      })
      .addStage(outputFunction.id, {
        condition: 'success',
        inputMapping: 'full_data',
        timeout: 15000,
      })
      .stopOnFailure(true)
      .timeout(180000) // 3 minutes total
      .execute({
        sourceFile: '/data/input/dataset.csv',
        outputFormat: 'parquet',
        validationRules: ['schema_check', 'data_quality'],
        transformations: ['clean', 'normalize', 'enrich'],
      });

    console.log('✅ Data processing pipeline executed:', {
      pipelineName: processingPipeline.pipelineName,
      success: processingPipeline.success,
      stagesExecuted: processingPipeline.results.length,
    });

    return [
      ...microservicesGroup.map(f => f.id),
      ingestionFunction.id,
      validationFunction.id,
      transformFunction.id,
      outputFunction.id,
    ];
  } catch (error) {
    console.error('❌ Error in function groups and pipelines:', error);
    throw error;
  }
}

async function analyticsAndMonitoring() {
  console.log('\n=== Analytics and Monitoring ===');

  const functionManager = sdk.functions;

  try {
    // Search for functions
    console.log('Searching for functions...');
    const searchResults = await functionManager.search({
      tags: ['api'],
      enabled: true,
      includeStats: true,
      includeContainerStatus: true,
      limit: 10,
    });

    console.log('✅ Functions found:', {
      count: searchResults.data.length,
      totalCount: searchResults.totalCount,
      types: [...new Set(searchResults.data.map(f => f.container.image.split(':')[0]))],
    });

    // Get function statistics
    console.log('\nGetting function statistics...');
    const stats = await functionManager.getStats();
    console.log('✅ Function statistics:', {
      totalFunctions: stats.totalFunctions,
      enabled: stats.enabled,
      disabled: stats.disabled,
      totalExecutions: stats.totalExecutions,
      successRate: stats.successRate,
      averageExecutionTime: stats.averageExecutionTime,
      resourceUsage: stats.resourceUsage,
    });

    // Get function health
    console.log('\nChecking function health...');
    const health = await functionManager.getHealth();
    console.log('✅ Function health:', {
      healthy: health.healthy,
      issues: health.issues.length,
      failingFunctions: health.failingFunctions,
      containerErrors: health.containerErrors,
      resourceUtilization: health.resourceUtilization,
    });

    // Get individual function execution
    if (searchResults.data.length > 0) {
      const testFunction = searchResults.data[0];
      console.log('\nTesting function execution...');

      const execution = await functionManager.execute({
        functionId: testFunction.id,
        input: { test: true, timestamp: new Date().toISOString() },
        timeout: 30000,
        metadata: { test: 'analytics-monitoring' },
      });

      console.log('✅ Test execution completed:', {
        functionId: testFunction.id,
        executionId: execution.executionId,
        status: execution.status,
        executionTime: execution.executionTime,
      });

      // Get execution details
      const executionDetails = await functionManager.getExecution(execution.executionId);
      console.log('Execution details:', {
        status: executionDetails.status,
        startedAt: executionDetails.startedAt,
        completedAt: executionDetails.completedAt,
        logs: executionDetails.logs?.length || 0,
      });
    }

  } catch (error) {
    console.error('❌ Error in analytics and monitoring:', error);
    throw error;
  }
}

async function cleanupFunctions(functionIds: string[]) {
  console.log('\n=== Cleanup ===');

  const functionManager = sdk.functions;

  try {
    for (const functionId of functionIds) {
      try {
        // Stop and remove containers first
        await functionManager.stopContainer(functionId);
        await functionManager.removeContainer(functionId);

        // Delete the function
        await functionManager.delete(functionId, {
          force: true,
          removeContainer: true,
        });

        console.log(`✅ Deleted function: ${functionId}`);
      } catch (error) {
        console.warn(`⚠️ Could not delete function ${functionId}:`, error);
      }
    }
  } catch (error) {
    console.warn('⚠️ Cleanup errors:', error);
  }
}

async function runAllExamples() {
  console.log('🚀 Starting Function Management Examples\n');

  const allFunctionIds: string[] = [];

  try {
    // Initialize the SDK
    await sdk.initialize();
    console.log('✅ SDK initialized\n');

    // Run all examples
    const basicFunctions = await basicFunctionOperations();
    allFunctionIds.push(...basicFunctions);

    const builderFunctions = await functionBuilderExample();
    allFunctionIds.push(...builderFunctions);

    const templateFunctions = await templatesAndBatchOperations();
    allFunctionIds.push(...templateFunctions);

    const containerFunction = await containerManagement();
    allFunctionIds.push(containerFunction);

    const groupFunctions = await functionGroupsAndPipelines();
    allFunctionIds.push(...groupFunctions);

    await analyticsAndMonitoring();

    console.log('\n🎉 All examples completed successfully!');

  } catch (error) {
    console.error('💥 Example execution failed:', error);
    throw error;
  } finally {
    // Clean up created functions
    await cleanupFunctions(allFunctionIds);

    // Dispose of SDK resources
    await sdk.dispose();
    console.log('👋 SDK disposed');
  }
}

// Export for use in other examples or tests
export {
  basicFunctionOperations,
  functionBuilderExample,
  templatesAndBatchOperations,
  containerManagement,
  functionGroupsAndPipelines,
  analyticsAndMonitoring,
  runAllExamples,
};

// Run examples if this file is executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  runAllExamples().catch(console.error);
}
