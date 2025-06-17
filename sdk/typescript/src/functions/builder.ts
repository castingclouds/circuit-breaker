/**
 * Function Builder for Circuit Breaker TypeScript SDK
 *
 * This file provides a fluent interface for creating and managing functions
 * with advanced features like container configuration, triggers, chains,
 * and deployment automation.
 */

import {
  FunctionDefinition,
  ContainerConfig,
  ResourceLimits,
  EventTrigger,
  FunctionChain,
  RetryConfig,
  ContainerMount,
  EventTriggerType,
  InputMapping,
  ChainCondition,
} from "../core/types.js";
import {
  FunctionError,
  FunctionValidationError,
} from "../core/errors.js";
import { FunctionManager, FunctionCreateInput } from "./manager.js";

// ============================================================================
// Types
// ============================================================================

export interface FunctionBuilderOptions {
  /** Whether to validate functions during building */
  validate?: boolean;

  /** Default container registry */
  defaultRegistry?: string;

  /** Default resource limits */
  defaultLimits?: ResourceLimits;

  /** Default metadata to apply to all functions */
  defaultMetadata?: Record<string, any>;

  /** Default tags to apply to all functions */
  defaultTags?: string[];

  /** Whether to auto-enable functions */
  autoEnable?: boolean;

  /** Whether to auto-build containers */
  autoBuild?: boolean;
}

export interface FunctionTemplate {
  /** Template name */
  name: string;

  /** Template description */
  description?: string;

  /** Base container image */
  baseImage: string;

  /** Default container configuration */
  containerConfig?: Partial<ContainerConfig>;

  /** Default triggers */
  defaultTriggers?: EventTrigger[];

  /** Default chains */
  defaultChains?: FunctionChain[];

  /** Template metadata */
  metadata?: Record<string, any>;

  /** Template parameters */
  parameters?: string[];

  /** Required environment variables */
  requiredEnv?: string[];
}

export interface FunctionGroup {
  /** Group name */
  name: string;

  /** Functions in the group */
  functions: FunctionDefinition[];

  /** Group-wide configuration */
  config?: {
    /** Shared environment variables */
    sharedEnv?: Record<string, string>;
    /** Shared resource limits */
    sharedLimits?: ResourceLimits;
    /** Shared network */
    sharedNetwork?: string;
  };

  /** Group metadata */
  metadata?: Record<string, any>;
}

export interface FunctionPipeline {
  /** Pipeline name */
  name: string;

  /** Pipeline stages (functions) */
  stages: {
    functionId: string;
    inputMapping?: InputMapping;
    condition?: ChainCondition;
    timeout?: number;
    retry?: RetryConfig;
  }[];

  /** Pipeline-wide configuration */
  config?: {
    /** Stop on first failure */
    stopOnFailure?: boolean;
    /** Parallel execution where possible */
    allowParallel?: boolean;
    /** Pipeline timeout */
    timeout?: number;
  };
}

// ============================================================================
// Function Builder
// ============================================================================

export class FunctionBuilder {
  private readonly functionManager: FunctionManager;
  private readonly options: FunctionBuilderOptions;
  private readonly templates = new Map<string, FunctionTemplate>();
  private readonly groups = new Map<string, FunctionGroup>();
  private readonly pipelines = new Map<string, FunctionPipeline>();

  constructor(functionManager: FunctionManager, options: FunctionBuilderOptions = {}) {
    this.functionManager = functionManager;
    this.options = {
      validate: true,
      autoEnable: true,
      autoBuild: false,
      ...options,
    };
  }

  // ============================================================================
  // Basic Function Creation
  // ============================================================================

  /**
   * Start building a function
   */
  function(name: string): FunctionBuilderInstance {
    return new FunctionBuilderInstance(name, this.functionManager, this.options);
  }

  /**
   * Create a function from Docker image
   */
  fromImage(name: string, image: string): FunctionBuilderInstance {
    return new FunctionBuilderInstance(name, this.functionManager, this.options)
      .image(image);
  }

  /**
   * Create a function from template
   */
  fromTemplate(
    templateName: string,
    functionName: string,
    parameters: Record<string, any> = {}
  ): FunctionBuilderInstance {
    const template = this.templates.get(templateName);
    if (!template) {
      throw new FunctionValidationError(
        [`Template not found: ${templateName}`],
        { templateName }
      );
    }

    let image = template.baseImage;

    // Replace parameters in image name
    if (template.parameters) {
      for (const param of template.parameters) {
        if (parameters[param] === undefined) {
          throw new FunctionValidationError(
            [`Missing required parameter: ${param}`],
            { templateName, parameters: template.parameters }
          );
        }
        image = image.replace(
          new RegExp(`\\{\\{${param}\\}\\}`, "g"),
          parameters[param]
        );
      }
    }

    const builder = new FunctionBuilderInstance(functionName, this.functionManager, this.options)
      .image(image)
      .description(template.description || `Function created from template: ${templateName}`);

    // Apply template configuration
    if (template.containerConfig) {
      if (template.containerConfig.environment) {
        builder.env(template.containerConfig.environment);
      }
      if (template.containerConfig.resources) {
        builder.resources(template.containerConfig.resources);
      }
      if (template.containerConfig.workingDir) {
        builder.workingDir(template.containerConfig.workingDir);
      }
      if (template.containerConfig.command) {
        builder.command(...template.containerConfig.command);
      }
    }

    // Apply default triggers
    if (template.defaultTriggers) {
      for (const trigger of template.defaultTriggers) {
        builder.addTrigger(trigger);
      }
    }

    // Apply default chains
    if (template.defaultChains) {
      for (const chain of template.defaultChains) {
        builder.addChain(chain);
      }
    }

    // Apply template metadata
    if (template.metadata) {
      builder.metadata({
        ...template.metadata,
        templateName,
        templateParameters: parameters,
      });
    }

    return builder;
  }

  // ============================================================================
  // Template Management
  // ============================================================================

  /**
   * Register a function template
   */
  registerTemplate(template: FunctionTemplate): this {
    this.templates.set(template.name, template);
    return this;
  }

  /**
   * Get a registered template
   */
  getTemplate(name: string): FunctionTemplate | undefined {
    return this.templates.get(name);
  }

  /**
   * List all registered templates
   */
  listTemplates(): FunctionTemplate[] {
    return Array.from(this.templates.values());
  }

  // ============================================================================
  // Function Groups
  // ============================================================================

  /**
   * Create a function group
   */
  group(name: string): FunctionGroupBuilder {
    return new FunctionGroupBuilder(name, this);
  }

  /**
   * Register a function group
   */
  registerGroup(group: FunctionGroup): this {
    this.groups.set(group.name, group);
    return this;
  }

  /**
   * Deploy a function group
   */
  async deployGroup(groupName: string): Promise<FunctionDefinition[]> {
    const group = this.groups.get(groupName);
    if (!group) {
      throw new FunctionValidationError(
        [`Function group not found: ${groupName}`],
        { groupName }
      );
    }

    const deployedFunctions: FunctionDefinition[] = [];

    for (const functionDef of group.functions) {
      try {
        // Apply group-wide configuration
        const functionInput: FunctionCreateInput = {
          name: functionDef.name,
          container: {
            ...functionDef.container,
            environment: {
              ...group.config?.sharedEnv,
              ...functionDef.container.environment,
            },
            resources: {
              ...group.config?.sharedLimits,
              ...functionDef.container.resources,
            },
            networkMode: group.config?.sharedNetwork || functionDef.container.networkMode,
          },
          triggers: functionDef.triggers,
          chains: functionDef.chains,
          description: functionDef.description,
          tags: [...(functionDef.tags || []), `group:${groupName}`],
          metadata: {
            ...functionDef.metadata,
            ...group.metadata,
            groupName,
          },
          enabled: functionDef.enabled,
        };

        const deployed = await this.functionManager.create(functionInput, {
          validate: this.options.validate,
          build: this.options.autoBuild,
        });

        deployedFunctions.push(deployed);
      } catch (error) {
        throw new FunctionError(
          `Failed to deploy function ${functionDef.name} in group ${groupName}`,
          "GROUP_DEPLOYMENT_ERROR",
          { functionName: functionDef.name, groupName, error }
        );
      }
    }

    return deployedFunctions;
  }

  // ============================================================================
  // Function Pipelines
  // ============================================================================

  /**
   * Create a function pipeline
   */
  pipeline(name: string): FunctionPipelineBuilder {
    return new FunctionPipelineBuilder(name, this);
  }

  /**
   * Register a function pipeline
   */
  registerPipeline(pipeline: FunctionPipeline): this {
    this.pipelines.set(pipeline.name, pipeline);
    return this;
  }

  /**
   * Execute a function pipeline
   */
  async executePipeline(
    pipelineName: string,
    input: any,
    options: { timeout?: number } = {}
  ): Promise<any> {
    const pipeline = this.pipelines.get(pipelineName);
    if (!pipeline) {
      throw new FunctionValidationError(
        [`Function pipeline not found: ${pipelineName}`],
        { pipelineName }
      );
    }

    let currentInput = input;
    const results: any[] = [];

    for (let i = 0; i < pipeline.stages.length; i++) {
      const stage = pipeline.stages[i];

      try {
        // Check stage condition
        if (stage.condition && stage.condition !== 'always') {
          const previousResult = results[i - 1];
          if (stage.condition === 'success' && !previousResult?.success) {
            continue;
          }
          if (stage.condition === 'failure' && previousResult?.success) {
            continue;
          }
        }

        // Map input for this stage
        let stageInput = currentInput;
        if (stage.inputMapping) {
          switch (stage.inputMapping) {
            case 'metadata_only':
              stageInput = { pipelineStage: i, previousResults: results };
              break;
            case 'full_data':
              stageInput = currentInput;
              break;
            default:
              stageInput = currentInput;
          }
        }

        // Execute the function
        const result = await this.functionManager.execute({
          functionId: stage.functionId,
          input: stageInput,
          timeout: stage.timeout || options.timeout,
          retry: stage.retry,
          metadata: {
            pipelineName,
            stageIndex: i,
            stageFunctionId: stage.functionId,
          },
        });

        results.push(result);
        currentInput = result.output;

        // Stop on failure if configured
        if (pipeline.config?.stopOnFailure && result.status !== 'success') {
          throw new FunctionError(
            `Pipeline stopped due to failure in stage ${i}`,
            "PIPELINE_STAGE_FAILURE",
            { pipelineName, stageIndex: i, result }
          );
        }

      } catch (error) {
        if (pipeline.config?.stopOnFailure) {
          throw error;
        }
        // Continue with next stage on error
        results.push({ success: false, error });
      }
    }

    return {
      pipelineName,
      input,
      results,
      output: currentInput,
      success: results.every(r => r.success !== false),
    };
  }

  // ============================================================================
  // Utility Methods
  // ============================================================================

  /**
   * Create multiple functions with common properties
   */
  batch(
    functions: Array<{
      name: string;
      image: string;
      config?: Partial<ContainerConfig>;
    }>,
    commonConfig: Partial<FunctionCreateInput> = {}
  ): BatchFunctionBuilder {
    return new BatchFunctionBuilder(functions, commonConfig, this.functionManager, this.options);
  }

  /**
   * Clone an existing function with modifications
   */
  async clone(
    sourceFunctionId: string,
    newName: string,
    modifications: Partial<FunctionCreateInput> = {}
  ): Promise<FunctionBuilderInstance> {
    const sourceFunction = await this.functionManager.get(sourceFunctionId);

    const functionInput: FunctionCreateInput = {
      name: newName,
      container: sourceFunction.container,
      triggers: sourceFunction.triggers,
      chains: sourceFunction.chains,
      description: sourceFunction.description,
      tags: [...(sourceFunction.tags || []), 'cloned'],
      metadata: {
        ...sourceFunction.metadata,
        clonedFrom: sourceFunctionId,
        clonedAt: new Date().toISOString(),
      },
      version: sourceFunction.version,
      enabled: sourceFunction.enabled,
      ...modifications,
    };

    return new FunctionBuilderResult(functionInput, this.functionManager);
  }
}

// ============================================================================
// Function Builder Instance
// ============================================================================

export class FunctionBuilderInstance {
  private functionInput: Partial<FunctionCreateInput>;
  private containerConfig: Partial<ContainerConfig> = {};

  constructor(
    name: string,
    private functionManager: FunctionManager,
    private options: FunctionBuilderOptions
  ) {
    this.functionInput = {
      name,
      enabled: this.options.autoEnable,
      tags: [...(this.options.defaultTags || [])],
      metadata: { ...this.options.defaultMetadata },
    };
  }

  // Container Configuration
  image(image: string): this {
    this.containerConfig.image = this.options.defaultRegistry
      ? `${this.options.defaultRegistry}/${image}`
      : image;
    return this;
  }

  command(...command: string[]): this {
    this.containerConfig.command = command;
    return this;
  }

  env(environment: Record<string, string>): this {
    this.containerConfig.environment = {
      ...this.containerConfig.environment,
      ...environment,
    };
    return this;
  }

  envVar(key: string, value: string): this {
    this.containerConfig.environment = {
      ...this.containerConfig.environment,
      [key]: value,
    };
    return this;
  }

  resources(limits: ResourceLimits): this {
    this.containerConfig.resources = {
      ...this.options.defaultLimits,
      ...limits,
    };
    return this;
  }

  cpu(cores: number): this {
    this.containerConfig.resources = {
      ...this.containerConfig.resources,
      cpu: cores,
    };
    return this;
  }

  memory(bytes: number): this {
    this.containerConfig.resources = {
      ...this.containerConfig.resources,
      memory: bytes,
    };
    return this;
  }

  gpu(count: number): this {
    this.containerConfig.resources = {
      ...this.containerConfig.resources,
      gpu: count,
    };
    return this;
  }

  mount(source: string, target: string, options?: Partial<ContainerMount>): this {
    this.containerConfig.mounts = this.containerConfig.mounts || [];
    this.containerConfig.mounts.push({
      source,
      target,
      type: 'bind',
      readonly: false,
      ...options,
    });
    return this;
  }

  volume(name: string, target: string): this {
    return this.mount(name, target, { type: 'volume' });
  }

  workingDir(dir: string): this {
    this.containerConfig.workingDir = dir;
    return this;
  }

  network(mode: string): this {
    this.containerConfig.networkMode = mode;
    return this;
  }

  label(key: string, value: string): this {
    this.containerConfig.labels = {
      ...this.containerConfig.labels,
      [key]: value,
    };
    return this;
  }

  // Function Configuration
  description(description: string): this {
    this.functionInput.description = description;
    return this;
  }

  version(version: string): this {
    this.functionInput.version = version;
    return this;
  }

  tags(...tags: string[]): this {
    this.functionInput.tags = [...(this.functionInput.tags || []), ...tags];
    return this;
  }

  metadata(metadata: Record<string, any>): this {
    this.functionInput.metadata = { ...this.functionInput.metadata, ...metadata };
    return this;
  }

  enabled(enabled: boolean): this {
    this.functionInput.enabled = enabled;
    return this;
  }

  // Triggers and Chains
  trigger(type: EventTriggerType, condition: string, options?: Partial<EventTrigger>): this {
    this.functionInput.triggers = this.functionInput.triggers || [];
    this.functionInput.triggers.push({
      type,
      condition,
      enabled: true,
      ...options,
    });
    return this;
  }

  addTrigger(trigger: EventTrigger): this {
    this.functionInput.triggers = this.functionInput.triggers || [];
    this.functionInput.triggers.push(trigger);
    return this;
  }

  onWorkflowEvent(condition: string): this {
    return this.trigger('workflow_event', condition);
  }

  onResourceState(condition: string): this {
    return this.trigger('resource_state', condition);
  }

  onSchedule(schedule: string): this {
    return this.trigger('schedule', schedule);
  }

  onWebhook(path: string): this {
    return this.trigger('webhook', path);
  }

  chain(targetFunction: string, condition: ChainCondition = 'success', options?: Partial<FunctionChain>): this {
    this.functionInput.chains = this.functionInput.chains || [];
    this.functionInput.chains.push({
      targetFunction,
      condition,
      inputMapping: 'full_data',
      ...options,
    });
    return this;
  }

  addChain(chain: FunctionChain): this {
    this.functionInput.chains = this.functionInput.chains || [];
    this.functionInput.chains.push(chain);
    return this;
  }

  // Input/Output Schema
  inputSchema(schema: any): this {
    this.functionInput.inputSchema = schema;
    return this;
  }

  outputSchema(schema: any): this {
    this.functionInput.outputSchema = schema;
    return this;
  }

  // Build and Create
  build(): FunctionBuilderResult {
    if (!this.containerConfig.image) {
      throw new FunctionValidationError(
        ["Container image is required"],
        { functionInput: this.functionInput }
      );
    }

    this.functionInput.container = this.containerConfig as ContainerConfig;

    return new FunctionBuilderResult(this.functionInput as FunctionCreateInput, this.functionManager);
  }

  async create(): Promise<FunctionDefinition> {
    return this.build().create();
  }

  async deploy(): Promise<FunctionDefinition> {
    return this.build().deploy();
  }
}

// ============================================================================
// Builder Results and Utilities
// ============================================================================

export class FunctionBuilderResult {
  constructor(
    private functionInput: FunctionCreateInput,
    private functionManager: FunctionManager
  ) {}

  async create(): Promise<FunctionDefinition> {
    return this.functionManager.create(this.functionInput);
  }

  async deploy(): Promise<FunctionDefinition> {
    return this.functionManager.create(this.functionInput, {
      validate: true,
      build: true,
    });
  }

  async test(input: any): Promise<any> {
    const functionDef = await this.create();

    // Execute a test run
    const result = await this.functionManager.execute({
      functionId: functionDef.id,
      input,
      metadata: { test: true },
    });

    return result;
  }

  getFunctionInput(): FunctionCreateInput {
    return { ...this.functionInput };
  }
}

export class BatchFunctionBuilder {
  constructor(
    private functions: Array<{
      name: string;
      image: string;
      config?: Partial<ContainerConfig>;
    }>,
    private commonConfig: Partial<FunctionCreateInput>,
    private functionManager: FunctionManager,
    private options: FunctionBuilderOptions
  ) {}

  async create(): Promise<FunctionDefinition[]> {
    const createdFunctions: FunctionDefinition[] = [];

    for (const func of this.functions) {
      const functionInput: FunctionCreateInput = {
        name: func.name,
        container: {
          image: func.image,
          ...func.config,
        },
        ...this.commonConfig,
        enabled: this.options.autoEnable,
        tags: [...(this.options.defaultTags || []), ...(this.commonConfig.tags || [])],
        metadata: {
          ...this.options.defaultMetadata,
          ...this.commonConfig.metadata,
        },
      };

      const created = await this.functionManager.create(functionInput);
      createdFunctions.push(created);
    }

    return createdFunctions;
  }

  async deploy(): Promise<FunctionDefinition[]> {
    const deployedFunctions: FunctionDefinition[] = [];

    for (const func of this.functions) {
      const functionInput: FunctionCreateInput = {
        name: func.name,
        container: {
          image: func.image,
          ...func.config,
        },
        ...this.commonConfig,
        enabled: this.options.autoEnable,
      };

      const deployed = await this.functionManager.create(functionInput, {
        validate: true,
        build: true,
      });
      deployedFunctions.push(deployed);
    }

    return deployedFunctions;
  }
}

export class FunctionGroupBuilder {
  private group: Partial<FunctionGroup>;

  constructor(
    name: string,
    private builder: FunctionBuilder
  ) {
    this.group = { name, functions: [] };
  }

  addFunction(functionDef: FunctionDefinition): this {
    this.group.functions = this.group.functions || [];
    this.group.functions.push(functionDef);
    return this;
  }

  addFunctions(functionDefs: FunctionDefinition[]): this {
    this.group.functions = this.group.functions || [];
    this.group.functions.push(...functionDefs);
    return this;
  }

  sharedEnv(environment: Record<string, string>): this {
    this.group.config = this.group.config || {};
    this.group.config.sharedEnv = environment;
    return this;
  }

  sharedLimits(limits: ResourceLimits): this {
    this.group.config = this.group.config || {};
    this.group.config.sharedLimits = limits;
    return this;
  }

  sharedNetwork(network: string): this {
    this.group.config = this.group.config || {};
    this.group.config.sharedNetwork = network;
    return this;
  }

  metadata(metadata: Record<string, any>): this {
    this.group.metadata = metadata;
    return this;
  }

  build(): FunctionGroup {
    if (!this.group.functions || this.group.functions.length === 0) {
      throw new FunctionValidationError(
        ["At least one function is required for function groups"],
        { group: this.group }
      );
    }

    return this.group as FunctionGroup;
  }

  register(): this {
    this.builder.registerGroup(this.build());
    return this;
  }

  async deploy(): Promise<FunctionDefinition[]> {
    const group = this.build();
    this.builder.registerGroup(group);
    return this.builder.deployGroup(group.name);
  }
}

export class FunctionPipelineBuilder {
  private pipeline: Partial<FunctionPipeline>;

  constructor(
    name: string,
    private builder: FunctionBuilder
  ) {
    this.pipeline = { name, stages: [] };
  }

  addStage(
    functionId: string,
    options?: {
      inputMapping?: InputMapping;
      condition?: ChainCondition;
      timeout?: number;
      retry?: RetryConfig;
    }
  ): this {
    this.pipeline.stages = this.pipeline.stages || [];
    this.pipeline.stages.push({
      functionId,
      inputMapping: options?.inputMapping || 'full_data',
      condition: options?.condition || 'always',
      timeout: options?.timeout,
      retry: options?.retry,
    });
    return this;
  }

  stopOnFailure(stop: boolean = true): this {
    this.pipeline.config = this.pipeline.config || {};
    this.pipeline.config.stopOnFailure = stop;
    return this;
  }

  allowParallel(allow: boolean = true): this {
    this.pipeline.config = this.pipeline.config || {};
    this.pipeline.config.allowParallel = allow;
    return this;
  }

  timeout(timeout: number): this {
    this.pipeline.config = this.pipeline.config || {};
    this.pipeline.config.timeout = timeout;
    return this;
  }

  build(): FunctionPipeline {
    if (!this.pipeline.stages || this.pipeline.stages.length === 0) {
      throw new FunctionValidationError(
        ["At least one stage is required for function pipelines"],
        { pipeline: this.pipeline }
      );
    }

    return this.pipeline as FunctionPipeline;
  }

  register(): this {
    this.builder.registerPipeline(this.build());
    return this;
  }

  async execute(input: any, options?: { timeout?: number }): Promise<any> {
    const pipeline = this.build();
    this.builder.registerPipeline(pipeline);
    return this.builder.executePipeline(pipeline.name, input, options);
  }
}

// ============================================================================
// Factory Functions
// ============================================================================

/**
 * Create a new FunctionBuilder instance
 */
export function createFunctionBuilder(
  functionManager: FunctionManager,
  options?: FunctionBuilderOptions
): FunctionBuilder {
  return new FunctionBuilder(functionManager, options);
}

/**
 * Create a function template
 */
export function createFunctionTemplate(
  name: string,
  baseImage: string,
  options: Partial<FunctionTemplate> = {}
): FunctionTemplate {
  return {
    name,
    baseImage,
    ...options,
  };
}

/**
 * Common function templates
 */
export const CommonTemplates = {
  /**
   * Node.js function template
   */
  nodejs: (version = "18"): FunctionTemplate => ({
    name: "nodejs-function",
    baseImage: `node:${version}-alpine`,
    containerConfig: {
      command: ["node", "index.js"],
      workingDir: "/app",
      environment: {
        NODE_ENV: "production",
      },
    },
    parameters: ["version"],
    requiredEnv: ["NODE_ENV"],
    description: "Node.js function template",
  }),

  /**
   * Python function template
   */
  python: (version = "3.11"): FunctionTemplate => ({
    name: "python-function",
    baseImage: `python:${version}-slim`,
    containerConfig: {
      command: ["python", "main.py"],
      workingDir: "/app",
      environment: {
        PYTHONPATH: "/app",
        PYTHONUNBUFFERED: "1",
      },
    },
    parameters: ["version"],
    requiredEnv: ["PYTHONPATH"],
    description: "Python function template",
  }),

  /**
   * Go function template
   */
  golang: (version = "1.21"): FunctionTemplate => ({
    name: "go-function",
    baseImage: `golang:${version}-alpine`,
    containerConfig: {
      command: ["./main"],
      workingDir: "/app",
      environment: {
        CGO_ENABLED: "0",
        GOOS: "linux",
      },
    },
    parameters: ["version"],
    description: "Go function template",
  }),

  /**
   * Rust function template
   */
  rust: (): FunctionTemplate => ({
    name: "rust-function",
    baseImage: "rust:alpine",
    containerConfig: {
      command: ["./target/release/main"],
      workingDir: "/app",
      environment: {
        RUST_LOG: "info",
      },
    },
    description: "Rust function template",
  }),

  /**
   * Generic HTTP API template
   */
  httpApi: (port = 8080): FunctionTemplate => ({
    name: "http-api",
    baseImage: "{{base_image}}",
    containerConfig: {
      environment: {
        PORT: port.toString(),
      },
      labels: {
        "function.type": "http-api",
        "function.port": port.toString(),
      },
    },
    defaultTriggers: [
      {
        type: "webhook",
        condition: "/api/*",
        enabled: true,
      },
    ],
    parameters: ["base_image"],
    description: "HTTP API function template",
  }),

  /**
   * Scheduled job template
   */
  scheduledJob: (schedule = "0 */1 * * *"): FunctionTemplate => ({
    name: "scheduled-job",
    baseImage: "{{base_image}}",
    defaultTriggers: [
      {
        type: "schedule",
        condition: schedule,
        enabled: true,
      },
    ],
    parameters: ["base_image"],
    description: "Scheduled job function template",
  }),
};
