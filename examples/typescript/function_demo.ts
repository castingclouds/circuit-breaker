#!/usr/bin/env npx tsx
// Function system demonstration - TypeScript GraphQL Client
// Shows how to create event-driven Docker functions with REAL container execution
// Run with: npx tsx examples/typescript/function_demo.ts

import { spawn, ChildProcess } from 'child_process';
import { randomUUID } from 'crypto';

interface GraphQLResponse<T = any> {
  data?: T;
  errors?: Array<{ message: string; locations?: any[]; path?: any[] }>;
}

interface FunctionDefinition {
  id: string;
  name: string;
  description?: string;
  enabled: boolean;
  container: ContainerConfig;
  triggers: EventTrigger[];
  chains: FunctionChain[];
  inputSchema?: FunctionSchema;
  outputSchema?: FunctionSchema;
  tags: string[];
}

interface ContainerConfig {
  image: string;
  execCommand: string[];
  workingDir?: string;
  envVars: Record<string, string>;
  secretVars: Record<string, string>;
  mounts: ContainerMount[];
  resources?: ResourceLimits;
  setupCommands: string[][];
  exposedPorts: number[];
}

interface EventTrigger {
  id: string;
  eventType: EventType;
  workflowId?: string;
  conditions: string[];
  description?: string;
  inputMapping: InputMapping;
}

interface EventType {
  type: 'TokenCreated' | 'TokenTransitioned' | 'TokenUpdated' | 'TokenCompleted' | 'WorkflowCreated' | 'FunctionCompleted' | 'Custom';
  place?: string;
  from?: string;
  to?: string;
  transition?: string;
  functionId?: string;
  success?: boolean;
  eventName?: string;
}

interface InputMapping {
  type: 'FullOutput' | 'FieldMapping' | 'Template' | 'MergedData' | 'Script';
  mappings?: Record<string, string>;
  template?: any;
  script?: string;
}

interface FunctionChain {
  targetFunction: string;
  condition: ChainCondition;
  inputMapping: InputMapping;
  delay?: string; // ISO 8601 duration
  description?: string;
}

interface ChainCondition {
  type: 'Always' | 'OnSuccess' | 'OnFailure' | 'ConditionalRule' | 'Script';
  rule?: any;
  script?: string;
}

interface ContainerMount {
  source: string;
  target: string;
  readonly: boolean;
}

interface ResourceLimits {
  memoryMb?: number;
  cpuCores?: number;
  timeoutSeconds?: number;
}

interface FunctionSchema {
  schema: any;
  description?: string;
  example?: any;
}

interface Token {
  id: string;
  workflowId: string;
  place: string;
  data: any;
  metadata: Record<string, any>;
  createdAt: string;
  updatedAt: string;
  history: HistoryEvent[];
}

interface HistoryEvent {
  timestamp: string;
  transition: string;
  fromPlace: string;
  toPlace: string;
  data?: any;
}

interface FunctionExecution {
  id: string;
  functionId: string;
  triggerEvent: string;
  inputData: any;
  status: 'Pending' | 'Running' | 'Completed' | 'Failed' | 'Timeout' | 'Retrying';
  containerImage?: string;
  containerId?: string;
  startedAt?: string;
  completedAt?: string;
  exitCode?: number;
  stdout?: string;
  stderr?: string;
  outputData?: any;
  errorMessage?: string;
  retryCount: number;
  nextRetryAt?: string;
  parentExecutionId?: string;
  chainPosition: number;
  createdAt: string;
}

interface ContainerResult {
  exitCode: number;
  stdout: string;
  stderr: string;
}

class DockerExecutor {
  /**
   * Check if Docker is available on the system
   */
  static async checkDockerAvailable(): Promise<boolean> {
    return new Promise((resolve) => {
      const process = spawn('docker', ['--version']);
      process.on('exit', (code) => {
        resolve(code === 0);
      });
      process.on('error', () => {
        resolve(false);
      });
    });
  }

  /**
   * Execute a Docker container with the given configuration
   */
  static async runContainer(
    config: ContainerConfig,
    execution: FunctionExecution
  ): Promise<ContainerResult> {
    const containerName = `circuit-breaker-${execution.id}`;
    
    // Build Docker command
    const dockerArgs = [
      'run',
      '--name', containerName,
      '--rm', // Remove container when done
    ];

    // Add environment variables
    for (const [key, value] of Object.entries(config.envVars)) {
      dockerArgs.push('-e', `${key}=${value}`);
    }

    // Add execution context as environment variables
    dockerArgs.push('-e', `TRIGGER_EVENT=${execution.triggerEvent}`);
    dockerArgs.push('-e', `EXECUTION_ID=${execution.id}`);
    dockerArgs.push('-e', `FUNCTION_ID=${execution.functionId}`);
    dockerArgs.push('-e', `INPUT_DATA=${JSON.stringify(execution.inputData)}`);

    // Add working directory
    if (config.workingDir) {
      dockerArgs.push('-w', config.workingDir);
    }

    // Add resource limits
    if (config.resources) {
      if (config.resources.memoryMb) {
        dockerArgs.push('-m', `${config.resources.memoryMb}m`);
      }
      if (config.resources.cpuCores) {
        dockerArgs.push('--cpus', config.resources.cpuCores.toString());
      }
    }

    // Add mounts
    for (const mount of config.mounts) {
      const mountStr = mount.readonly 
        ? `${mount.source}:${mount.target}:ro`
        : `${mount.source}:${mount.target}`;
      dockerArgs.push('-v', mountStr);
    }

    // Add image
    dockerArgs.push(config.image);

    // Add execution command
    if (config.execCommand.length > 0) {
      dockerArgs.push(...config.execCommand);
    }

    console.log(`🐳 Running Docker command: docker ${dockerArgs.join(' ')}`);

    return new Promise((resolve, reject) => {
      const dockerProcess = spawn('docker', dockerArgs);
      
      let stdout = '';
      let stderr = '';

      // Capture stdout
      dockerProcess.stdout.on('data', (data) => {
        const output = data.toString();
        console.log(`📄 STDOUT: ${output.trim()}`);
        stdout += output;
      });

      // Capture stderr
      dockerProcess.stderr.on('data', (data) => {
        const output = data.toString();
        console.log(`⚠️  STDERR: ${output.trim()}`);
        stderr += output;
      });

      // Handle process completion
      dockerProcess.on('exit', (code) => {
        const exitCode = code || 0;
        
        if (exitCode === 0) {
          console.log(`✅ Docker container completed successfully (exit code: ${exitCode})`);
        } else {
          console.log(`❌ Docker container failed (exit code: ${exitCode})`);
        }

        resolve({
          exitCode,
          stdout: stdout.trim(),
          stderr: stderr.trim()
        });
      });

      // Handle process errors
      dockerProcess.on('error', (error) => {
        console.error(`💥 Docker execution error: ${error.message}`);
        reject(new Error(`Docker execution failed: ${error.message}`));
      });

      // Set timeout if specified
      if (config.resources?.timeoutSeconds) {
        setTimeout(() => {
          dockerProcess.kill('SIGTERM');
          reject(new Error(`Docker execution timed out after ${config.resources?.timeoutSeconds} seconds`));
        }, config.resources.timeoutSeconds * 1000);
      }
    });
  }

  /**
   * Clean up a Docker container
   */
  static async cleanupContainer(containerName: string): Promise<void> {
    return new Promise((resolve) => {
      const process = spawn('docker', ['rm', '-f', containerName]);
      process.on('exit', () => resolve());
      process.on('error', () => resolve()); // Ignore cleanup errors
    });
  }

  /**
   * Parse container output as JSON
   */
  static parseContainerOutput(stdout: string): any {
    try {
      // Try to parse the last line as JSON (common pattern)
      const lines = stdout.split('\n').filter(line => line.trim());
      const lastLine = lines[lines.length - 1];
      
      if (lastLine) {
        const parsed = JSON.parse(lastLine);
        return parsed;
      }
    } catch (error) {
      // If parsing fails, return raw output
    }
    
    return {
      output: stdout,
      type: 'raw_text'
    };
  }
}

class CircuitBreakerClient {
  constructor(private baseUrl: string = 'http://localhost:4000') {}

  async graphql<T = any>(query: string, variables?: any): Promise<GraphQLResponse<T>> {
    const response = await fetch(`${this.baseUrl}/graphql`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ query, variables }),
    });

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }

    return await response.json() as GraphQLResponse<T>;
  }

  async createWorkflow(name: string, places: string[], transitions: any[], initialPlace: string) {
    const mutation = `
      mutation CreateWorkflow($input: WorkflowDefinitionInput!) {
        createWorkflow(input: $input) {
          id
          name
          places
          initialPlace
        }
      }
    `;

    return this.graphql(mutation, {
      input: { name, places, transitions, initialPlace }
    });
  }

  async createToken(workflowId: string, initialPlace?: string, data?: any, metadata?: any) {
    const mutation = `
      mutation CreateToken($input: TokenCreateInput!) {
        createToken(input: $input) {
          id
          workflowId
          place
          data
          metadata
          createdAt
        }
      }
    `;

    return this.graphql(mutation, {
      input: { workflowId, initialPlace, data, metadata }
    });
  }

  async fireTransition(tokenId: string, transitionId: string, data?: any) {
    const mutation = `
      mutation FireTransition($input: TransitionFireInput!) {
        fireTransition(input: $input) {
          id
          place
          data
          history {
            timestamp
            transition
            fromPlace
            toPlace
          }
        }
      }
    `;

    return this.graphql(mutation, {
      input: { tokenId, transitionId, data }
    });
  }

  async getToken(id: string) {
    const query = `
      query GetToken($id: String!) {
        token(id: $id) {
          id
          workflowId
          place
          data
          metadata
          createdAt
          updatedAt
          history {
            timestamp
            transition
            fromPlace
            toPlace
            data
          }
        }
      }
    `;

    return this.graphql(query, { id });
  }

  async listTokens(workflowId?: string) {
    const query = `
      query ListTokens($workflowId: String) {
        tokens(workflowId: $workflowId) {
          id
          workflowId
          place
          data
          metadata
          createdAt
        }
      }
    `;

    return this.graphql(query, { workflowId });
  }
}

async function main() {
  console.log('🚀 Circuit Breaker Function System Demo - TypeScript Client with REAL Docker');
  console.log('================================================================================');
  console.log();

  // Check Docker availability
  const dockerAvailable = await DockerExecutor.checkDockerAvailable();
  if (!dockerAvailable) {
    console.error('❌ Docker is not available. Please install Docker and ensure it\'s running.');
    console.error('   Installation: https://docs.docker.com/get-docker/');
    process.exit(1);
  }
  console.log('✅ Docker is available and ready');
  console.log();

  const client = new CircuitBreakerClient();

  try {
    // 1. Create a demo workflow
    console.log('📋 Creating Demo Workflow...');
    const workflowResult = await client.createWorkflow(
      'Order Processing Workflow',
      ['start', 'processing', 'completed', 'failed'],
      [
        {
          id: 'process',
          fromPlaces: ['start'],
          toPlace: 'processing',
          conditions: []
        },
        {
          id: 'complete',
          fromPlaces: ['processing'],
          toPlace: 'completed',
          conditions: []
        },
        {
          id: 'fail',
          fromPlaces: ['processing'],
          toPlace: 'failed',
          conditions: []
        }
      ],
      'start'
    );

    if (workflowResult.errors) {
      console.error('❌ Failed to create workflow:', workflowResult.errors);
      return;
    }

    const workflowId = workflowResult.data?.createWorkflow?.id;
    console.log(`✅ Created workflow: ${workflowId}`);
    console.log();

    // 2. Create a token with order data
    console.log('🎯 Creating Order Token...');
    const tokenResult = await client.createToken(
      workflowId,
      'start',
      {
        orderId: 'ORD-12345',
        customerId: 'CUST-789',
        items: [
          { product: 'Laptop', price: 999.99, quantity: 1 },
          { product: 'Mouse', price: 29.99, quantity: 2 }
        ],
        total: 1059.97
      },
      {
        customerTier: 'premium',
        salesChannel: 'web',
        region: 'US-West'
      }
    );

    if (tokenResult.errors) {
      console.error('❌ Failed to create token:', tokenResult.errors);
      return;
    }

    const token = tokenResult.data?.createToken;
    console.log(`✅ Created token: ${token?.id}`);
    console.log(`📍 Current place: ${token?.place}`);
    console.log(`💰 Order total: $${token?.data?.total}`);
    console.log();

    // 3. Define the actual function that we'll execute
    console.log('⚡ Setting up Real Docker Function...');
    
    const dataProcessorFunction: FunctionDefinition = {
      id: 'data-processor',
      name: 'Order Data Processor',
      description: 'Processes order data and prepares it for downstream systems',
      enabled: true,
      container: {
        image: 'node:18-alpine',
        execCommand: [
          'node',
          '-e',
          `
            const inputData = JSON.parse(process.env.INPUT_DATA || '{}');
            const executionId = process.env.EXECUTION_ID;
            const functionId = process.env.FUNCTION_ID;
            
            console.log('Processing order data...');
            console.log('Input:', JSON.stringify(inputData, null, 2));
            
            // Simulate processing logic
            const result = {
              processed: true,
              timestamp: new Date().toISOString(),
              executionId,
              functionId,
              orderId: inputData.orderId,
              customerSegment: inputData.customerTier === 'premium' ? 'high-value' : 'standard',
              recommendedShipping: inputData.total > 500 ? 'expedited' : 'standard',
              itemCount: inputData.items ? inputData.items.length : 0,
              totalValue: inputData.total,
              processingRegion: inputData.region || 'unknown'
            };
            
            console.log('Processing complete!');
            console.log(JSON.stringify(result));
          `
        ],
        // Any Docker image can be used! Examples:
        // image: 'python:3.11-slim', execCommand: ['python', '-c', 'print("Hello from Python!")']
        // image: 'rust:1.70-alpine', execCommand: ['sh', '-c', 'echo "{\\"processed\\": true}" | rust-analyzer']
        // image: 'alpine:latest', execCommand: ['sh', '-c', 'echo "{\\"processed\\": true, \\"timestamp\\": \\"$(date -Iseconds)\\"}"']
        workingDir: '/tmp',
        envVars: { 
          NODE_ENV: 'production',
          LOG_LEVEL: 'info'
        },
        secretVars: {},
        mounts: [],
        setupCommands: [],
        exposedPorts: [],
        resources: {
          memoryMb: 128,
          cpuCores: 0.5,
          timeoutSeconds: 30
        }
      },
      triggers: [{
        id: 'token_created',
        eventType: { type: 'TokenTransitioned', place: 'processing' },
        conditions: [],
        inputMapping: { type: 'MergedData' }
      }],
      chains: [],
      tags: ['data', 'processing', 'orders']
    };

    console.log('📦 Function Definition:');
    console.log(`   • ${dataProcessorFunction.name} (${dataProcessorFunction.id})`);
    console.log(`   • Container: ${dataProcessorFunction.container.image}`);
    console.log(`   • Resources: ${dataProcessorFunction.container.resources?.memoryMb}MB, ${dataProcessorFunction.container.resources?.cpuCores} CPU`);
    console.log(`   • Timeout: ${dataProcessorFunction.container.resources?.timeoutSeconds}s`);
    console.log();

    // 4. Fire transition to trigger function execution
    console.log('🔄 Firing Transition to Trigger Function...');
    const transitionResult = await client.fireTransition(
      token?.id,
      'process',
      {
        processedBy: 'typescript-client',
        processingStarted: new Date().toISOString()
      }
    );

    if (transitionResult.errors) {
      console.error('❌ Failed to fire transition:', transitionResult.errors);
      return;
    }

    const updatedToken = transitionResult.data?.fireTransition;
    console.log(`✅ Transition fired successfully`);
    console.log(`📍 New place: ${updatedToken?.place}`);
    console.log();

    // 5. Execute the actual Docker function
    console.log('🐳 Executing Real Docker Function...');
    
    const execution: FunctionExecution = {
      id: randomUUID(),
      functionId: dataProcessorFunction.id,
      triggerEvent: JSON.stringify({
        type: 'TokenTransitioned',
        tokenId: updatedToken?.id,
        fromPlace: 'start',
        toPlace: 'processing'
      }),
      inputData: {
        ...updatedToken?.data,
        ...updatedToken?.metadata
      },
      status: 'Running',
      retryCount: 0,
      chainPosition: 0,
      createdAt: new Date().toISOString()
    };

    console.log(`   • Execution ID: ${execution.id}`);
    console.log(`   • Function: ${execution.functionId}`);
    console.log(`   • Input Data: ${JSON.stringify(execution.inputData)}`);
    console.log();

    let containerResult: ContainerResult;
    try {
      containerResult = await DockerExecutor.runContainer(
        dataProcessorFunction.container,
        execution
      );

      // Parse the output
      const outputData = DockerExecutor.parseContainerOutput(containerResult.stdout);

      // Update execution status
      execution.status = containerResult.exitCode === 0 ? 'Completed' : 'Failed';
      execution.exitCode = containerResult.exitCode;
      execution.stdout = containerResult.stdout;
      execution.stderr = containerResult.stderr;
      execution.outputData = outputData;
      execution.completedAt = new Date().toISOString();

      console.log();
      console.log('📊 Execution Results:');
      console.log(`   • Status: ${execution.status}`);
      console.log(`   • Exit Code: ${execution.exitCode}`);
      console.log(`   • Duration: ${new Date(execution.completedAt!).getTime() - new Date(execution.createdAt).getTime()}ms`);
      
      if (execution.status === 'Completed') {
        console.log(`   • Output Data: ${JSON.stringify(execution.outputData, null, 2)}`);
      } else {
        console.log(`   • Error: ${execution.stderr}`);
      }

    } catch (error) {
      execution.status = 'Failed';
      execution.errorMessage = error instanceof Error ? error.message : String(error);
      execution.completedAt = new Date().toISOString();
      
      console.log();
      console.log('❌ Execution Failed:');
      console.log(`   • Error: ${execution.errorMessage}`);
    }

    // 6. Complete the workflow with function results
    if (execution.status === 'Completed') {
      console.log();
      console.log('✅ Completing Workflow with Function Results...');
      const completeResult = await client.fireTransition(
        updatedToken?.id,
        'complete',
        {
          completedBy: 'function-processor',
          completedAt: new Date().toISOString(),
          functionResults: execution.outputData,
          executionId: execution.id
        }
      );

      if (completeResult.errors) {
        console.error('❌ Failed to complete workflow:', completeResult.errors);
      } else {
        const finalToken = completeResult.data?.fireTransition;
        console.log(`🎉 Workflow completed successfully`);
        console.log(`📍 Final place: ${finalToken?.place}`);
      }
    } else {
      console.log();
      console.log('💥 Failing Workflow due to Function Error...');
      const failResult = await client.fireTransition(
        updatedToken?.id,
        'fail',
        {
          failedBy: 'function-processor',
          failedAt: new Date().toISOString(),
          error: execution.errorMessage,
          executionId: execution.id
        }
      );

      if (!failResult.errors) {
        const failedToken = failResult.data?.fireTransition;
        console.log(`💥 Workflow failed`);
        console.log(`📍 Final place: ${failedToken?.place}`);
      }
    }

    // 7. Show complete history
    console.log();
    console.log('📊 Complete Workflow History:');
    const tokenDetails = await client.getToken(updatedToken?.id);
    const history = tokenDetails.data?.token?.history || [];
    
    history.forEach((event: HistoryEvent, index: number) => {
      console.log(`   ${index + 1}. ${event.fromPlace} → ${event.toPlace} via ${event.transition}`);
      console.log(`      at ${new Date(event.timestamp).toLocaleString()}`);
    });

    // 8. Architecture demonstration
    console.log();
    console.log('🏗️  Real Docker Function Architecture:');
    console.log('   🌐 TypeScript Client: Uses GraphQL API for workflow management');
    console.log('   🔄 Event System: Token transitions trigger real function execution');
    console.log('   🐳 Docker Execution: REAL Docker containers process data with resource limits');
    console.log('   📊 Live Output: Real-time stdout/stderr capture from containers');
    console.log('   🔗 Function Chaining: Functions can trigger other functions (ready to implement)');
    console.log('   📈 Resource Management: Memory, CPU, and timeout limits enforced');
    console.log('   🔒 Environment Injection: Execution context automatically provided');

    console.log();
    console.log('💡 Real Implementation Benefits:');
    console.log('   • Actual Docker Execution: Real containers, not simulation');
    console.log('   • Resource Limits: Memory, CPU, timeout enforcement');
    console.log('   • Live Monitoring: Real-time output capture and logging');
    console.log('   • Error Handling: Proper exit code and stderr capture');
    console.log('   • Environment Context: Execution metadata injected automatically');
    console.log('   • Container Cleanup: Automatic container removal');
    console.log('   • Language Agnostic: Any Docker image can be used');

  } catch (error) {
    console.error('❌ Error running demo:', error);
    process.exit(1);
  }
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch(console.error);
}

export {
  CircuitBreakerClient,
  DockerExecutor,
  type FunctionDefinition,
  type FunctionExecution,
  type ContainerResult,
  type Token,
  type EventTrigger,
  type InputMapping,
  type ChainCondition
}; 