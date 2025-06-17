/**
 * Resource Builder for Circuit Breaker TypeScript SDK
 *
 * This file provides a fluent interface for creating and managing resources
 * with advanced features like validation, batch operations, and state management.
 */

import {
  Resource,
  ResourceCreateInput,
  ActivityExecuteInput,
} from "../core/types.js";
import { ResourceValidationError } from "../core/errors.js";
import { ResourceManager, StateTransitionInput } from "./manager.js";

// ============================================================================
// Types
// ============================================================================

export interface ResourceBuilderOptions {
  /** Whether to validate resources during building */
  validate?: boolean;

  /** Default metadata to apply to all resources */
  defaultMetadata?: Record<string, any>;

  /** Workflow ID to use for all resources (can be overridden per resource) */
  defaultWorkflowId?: string;

  /** Whether to auto-initialize resources after creation */
  autoInitialize?: boolean;
}

export interface BatchResourceInput {
  /** Unique identifier for this resource in the batch */
  batchId: string;

  /** Resource creation input */
  input: ResourceCreateInput;

  /** Optional metadata specific to this batch item */
  batchMetadata?: Record<string, any>;
}

export interface ConditionalTransition {
  /** Condition to evaluate */
  condition: (resource: Resource) => boolean | Promise<boolean>;

  /** Target state if condition is true */
  targetState: string;

  /** Optional activity to execute for the transition */
  activityId?: string;

  /** Optional data to include with the transition */
  data?: any;

  /** Optional metadata for the transition */
  metadata?: Record<string, any>;
}

export interface ResourceTemplate {
  /** Template name */
  name: string;

  /** Template description */
  description?: string;

  /** Workflow ID */
  workflowId: string;

  /** Initial state (optional, uses workflow default) */
  initialState?: string;

  /** Default data structure */
  defaultData: any;

  /** Default metadata */
  defaultMetadata?: Record<string, any>;

  /** Data validation schema (optional) */
  dataSchema?: any;

  /** Required fields in data */
  requiredFields?: string[];
}

export interface ResourceChain {
  /** Source resource ID */
  sourceResourceId: string;

  /** Target workflow ID */
  targetWorkflowId: string;

  /** Data mapping function */
  dataMapper: (sourceData: any) => any;

  /** Metadata mapping function */
  metadataMapper?: (sourceMetadata: any) => any;

  /** Condition to trigger chain creation */
  condition?: (sourceResource: Resource) => boolean | Promise<boolean>;
}

// ============================================================================
// Resource Builder
// ============================================================================

export class ResourceBuilder {
  private readonly resourceManager: ResourceManager;
  private readonly options: ResourceBuilderOptions;
  private readonly templates = new Map<string, ResourceTemplate>();
  private readonly chains = new Map<string, ResourceChain[]>();

  constructor(
    resourceManager: ResourceManager,
    options: ResourceBuilderOptions = {},
  ) {
    this.resourceManager = resourceManager;
    this.options = {
      validate: true,
      autoInitialize: false,
      ...options,
    };
  }

  // ============================================================================
  // Template Management
  // ============================================================================

  /**
   * Register a resource template
   */
  registerTemplate(template: ResourceTemplate): this {
    this.templates.set(template.name, template);
    return this;
  }

  /**
   * Get a registered template
   */
  getTemplate(name: string): ResourceTemplate | undefined {
    return this.templates.get(name);
  }

  /**
   * List all registered templates
   */
  listTemplates(): ResourceTemplate[] {
    return Array.from(this.templates.values());
  }

  /**
   * Create a resource from a template
   */
  async fromTemplate(
    templateName: string,
    data: any,
    overrides: Partial<ResourceCreateInput> = {},
  ): Promise<Resource> {
    const template = this.templates.get(templateName);
    if (!template) {
      throw new ResourceValidationError(
        `Template not found: ${templateName}`,
        "TEMPLATE_NOT_FOUND",
        { templateName },
      );
    }

    // Validate required fields
    if (template.requiredFields) {
      const missingFields = template.requiredFields.filter(
        (field) => !(field in data),
      );
      if (missingFields.length > 0) {
        throw new ResourceValidationError(
          `Missing required fields: ${missingFields.join(", ")}`,
          "MISSING_REQUIRED_FIELDS",
          { templateName, missingFields, providedData: data },
        );
      }
    }

    // Merge template data with provided data
    const mergedData = { ...template.defaultData, ...data };
    const mergedMetadata = {
      ...this.options.defaultMetadata,
      ...template.defaultMetadata,
      ...overrides.metadata,
      templateName,
    };

    const input: ResourceCreateInput = {
      workflowId: overrides.workflowId || template.workflowId,
      data: mergedData,
      metadata: mergedMetadata,
      ...(overrides.initialState || template.initialState
        ? { initialState: overrides.initialState || template.initialState }
        : {}),
    };

    const result = await this.create(input);
    return result;
  }

  // ============================================================================
  // Single Resource Operations
  // ============================================================================

  /**
   * Create a single resource with fluent options
   */
  async create(input: ResourceCreateInput): Promise<ResourceBuilderResult> {
    // Apply default options
    const finalInput = this.applyDefaults(input);

    // Create the resource
    const resource = await this.resourceManager.create(
      finalInput,
      this.options.validate !== undefined
        ? { validate: this.options.validate }
        : {},
    );

    // Auto-initialize if configured
    if (this.options.autoInitialize) {
      // Could add initialization logic here
    }

    return new ResourceBuilderResult(resource, this.resourceManager);
  }

  /**
   * Create a resource and immediately transition to a state
   */
  async createAndTransition(
    input: ResourceCreateInput,
    targetState: string,
    transitionData?: any,
  ): Promise<ResourceBuilderResult> {
    const result = await this.create(input);

    if (result.resource.state !== targetState) {
      await result.transitionTo(targetState, transitionData);
    }

    return result;
  }

  /**
   * Create a resource and execute an activity
   */
  async createAndExecute(
    input: ResourceCreateInput,
    activityId: string,
    activityData?: any,
  ): Promise<ResourceBuilderResult> {
    const result = await this.create(input);
    await result.executeActivity(activityId, activityData);
    return result;
  }

  // ============================================================================
  // Batch Operations
  // ============================================================================

  /**
   * Create multiple resources from batch input
   */
  async createBatch(
    inputs: BatchResourceInput[],
  ): Promise<BatchResourceBuilderResult> {
    // Prepare resource inputs
    const resourceInputs = inputs.map((batchInput) => {
      const finalInput = this.applyDefaults(batchInput.input);

      // Add batch metadata
      if (batchInput.batchMetadata) {
        finalInput.metadata = {
          ...finalInput.metadata,
          ...batchInput.batchMetadata,
          batchId: batchInput.batchId,
        };
      }

      return finalInput;
    });

    // Create resources in batch
    const batchResult = await this.resourceManager.createBatch(resourceInputs);

    return new BatchResourceBuilderResult(
      batchResult,
      this.resourceManager,
      inputs,
    );
  }

  /**
   * Create resources from template in batch
   */
  async createBatchFromTemplate(
    templateName: string,
    dataArray: {
      batchId: string;
      data: any;
      overrides?: Partial<ResourceCreateInput>;
    }[],
  ): Promise<BatchResourceBuilderResult> {
    const batchInputs: BatchResourceInput[] = [];

    for (const item of dataArray) {
      const template = this.templates.get(templateName);
      if (!template) {
        throw new ResourceValidationError(
          `Template not found: ${templateName}`,
          "TEMPLATE_NOT_FOUND",
          { templateName },
        );
      }

      const mergedData = { ...template.defaultData, ...item.data };
      const mergedMetadata = {
        ...this.options.defaultMetadata,
        ...template.defaultMetadata,
        ...item.overrides?.metadata,
        templateName,
      };

      batchInputs.push({
        batchId: item.batchId,
        input: {
          workflowId: item.overrides?.workflowId || template.workflowId,
          data: mergedData,
          metadata: mergedMetadata,
          ...(item.overrides?.initialState || template.initialState
            ? {
                initialState:
                  item.overrides?.initialState || template.initialState,
              }
            : {}),
        },
      });
    }

    return this.createBatch(batchInputs);
  }

  // ============================================================================
  // Chain Operations
  // ============================================================================

  /**
   * Register a resource chain
   */
  registerChain(sourceWorkflowId: string, chain: ResourceChain): this {
    if (!this.chains.has(sourceWorkflowId)) {
      this.chains.set(sourceWorkflowId, []);
    }
    this.chains.get(sourceWorkflowId)!.push(chain);
    return this;
  }

  /**
   * Execute chains for a resource
   */
  async executeChains(resource: Resource): Promise<Resource[]> {
    const chains = this.chains.get(resource.workflowId);
    if (!chains) return [];
    const chainedResources: Resource[] = [];

    for (const chain of chains) {
      if (chain.condition) {
        const shouldChain = await chain.condition(resource);
        if (!shouldChain) continue;
      }

      const chainedData = chain.dataMapper(resource.data);
      const chainedMetadata = chain.metadataMapper
        ? chain.metadataMapper(resource.metadata)
        : { sourceResourceId: resource.id, ...resource.metadata };

      const chainedResource = await this.resourceManager.create({
        workflowId: chain.targetWorkflowId,
        data: chainedData,
        metadata: chainedMetadata,
      });

      chainedResources.push(chainedResource);
    }

    return chainedResources;
  }

  // ============================================================================
  // Conditional Operations
  // ============================================================================

  /**
   * Create a conditional transition
   */
  createConditionalTransition(
    transition: ConditionalTransition,
  ): ConditionalTransitionBuilder {
    return new ConditionalTransitionBuilder(transition, this.resourceManager);
  }

  // ============================================================================
  // Utility Methods
  // ============================================================================

  /**
   * Apply default options to resource input
   */
  private applyDefaults(input: ResourceCreateInput): ResourceCreateInput {
    return {
      ...input,
      workflowId:
        input.workflowId || this.options.defaultWorkflowId || input.workflowId,
      metadata: {
        ...this.options.defaultMetadata,
        ...input.metadata,
      },
    };
  }
}

// ============================================================================
// Result Classes
// ============================================================================

export class ResourceBuilderResult {
  constructor(
    public readonly resource: Resource,
    private readonly resourceManager: ResourceManager,
  ) {}

  /**
   * Transition the resource to a new state
   */
  async transitionTo(
    targetState: string,
    data?: any,
    metadata?: Record<string, any>,
  ): Promise<ResourceBuilderResult> {
    const input: StateTransitionInput = {
      resourceId: this.resource.id,
      toState: targetState,
      ...(data !== undefined ? { data } : {}),
      ...(metadata !== undefined ? { metadata } : {}),
    };

    const result = await this.resourceManager.transitionState(input);
    return new ResourceBuilderResult(result.resource, this.resourceManager);
  }

  /**
   * Execute an activity on the resource
   */
  async executeActivity(
    activityId: string,
    data?: any,
    metadata?: Record<string, any>,
  ): Promise<ResourceBuilderResult> {
    const input: ActivityExecuteInput = {
      resourceId: this.resource.id,
      activityId,
      ...(data !== undefined ? { data } : {}),
      ...(metadata !== undefined ? { metadata } : {}),
    };

    const result = await this.resourceManager.executeActivity(input);
    return new ResourceBuilderResult(result.resource, this.resourceManager);
  }

  /**
   * Update the resource data
   */
  async update(
    data?: any,
    metadata?: Record<string, any>,
  ): Promise<ResourceBuilderResult> {
    const updatedResource = await this.resourceManager.update(
      this.resource.id,
      {
        ...(data !== undefined ? { data } : {}),
        ...(metadata !== undefined ? { metadata } : {}),
      },
    );
    return new ResourceBuilderResult(updatedResource, this.resourceManager);
  }

  /**
   * Delete the resource
   */
  async delete(force = false): Promise<boolean> {
    return this.resourceManager.delete(this.resource.id, { force });
  }

  /**
   * Refresh the resource from the server
   */
  async refresh(): Promise<ResourceBuilderResult> {
    const refreshedResource = await this.resourceManager.get(this.resource.id, {
      useCache: false,
    });
    return new ResourceBuilderResult(refreshedResource, this.resourceManager);
  }

  /**
   * Check if resource is in a specific state
   */
  isInState(state: string): boolean {
    return this.resource.state === state;
  }

  /**
   * Check if resource is in any of the provided states
   */
  isInStates(states: string[]): boolean {
    return states.includes(this.resource.state);
  }

  /**
   * Get resource age in milliseconds
   */
  getAge(): number {
    return Date.now() - new Date(this.resource.createdAt).getTime();
  }

  /**
   * Get time since last update in milliseconds
   */
  getTimeSinceUpdate(): number {
    return Date.now() - new Date(this.resource.updatedAt).getTime();
  }
}

export class BatchResourceBuilderResult {
  constructor(
    public readonly batchResult: any,
    private readonly resourceManager: ResourceManager,
    public readonly originalInputs: BatchResourceInput[],
  ) {}

  /**
   * Get successful resources
   */
  getSuccessful(): ResourceBuilderResult[] {
    return this.batchResult.results
      .filter((r: any) => r.success)
      .map((r: any) => new ResourceBuilderResult(r.data, this.resourceManager));
  }

  /**
   * Get failed results
   */
  getFailed(): { batchId: string; error: Error }[] {
    const failedResults: { batchId: string; error: Error }[] = [];

    this.batchResult.results.forEach((result: any, index: number) => {
      if (!result.success) {
        failedResults.push({
          batchId: this.originalInputs[index].batchId,
          error: result.error,
        });
      }
    });

    return failedResults;
  }

  /**
   * Get result by batch ID
   */
  getByBatchId(batchId: string): ResourceBuilderResult | undefined {
    const index = this.originalInputs.findIndex(
      (input) => input.batchId === batchId,
    );
    if (index === -1) return undefined;

    const result = this.batchResult.results[index];
    if (!result.success) return undefined;

    return new ResourceBuilderResult(result.data, this.resourceManager);
  }

  /**
   * Execute activity on all successful resources
   */
  async executeActivityOnAll(
    activityId: string,
    data?: any,
  ): Promise<ResourceBuilderResult[]> {
    const successful = this.getSuccessful();
    const results: ResourceBuilderResult[] = [];

    for (const resource of successful) {
      try {
        const result = await resource.executeActivity(activityId, data);
        results.push(result);
      } catch (error) {
        // Continue with others even if one fails
        console.warn(
          `Failed to execute activity on resource ${resource.resource.id}:`,
          error,
        );
      }
    }

    return results;
  }

  /**
   * Transition all successful resources to a state
   */
  async transitionAllTo(
    targetState: string,
    data?: any,
  ): Promise<ResourceBuilderResult[]> {
    const successful = this.getSuccessful();
    const results: ResourceBuilderResult[] = [];

    for (const resource of successful) {
      try {
        const result = await resource.transitionTo(targetState, data);
        results.push(result);
      } catch (error) {
        // Continue with others even if one fails
        console.warn(
          `Failed to transition resource ${resource.resource.id}:`,
          error,
        );
      }
    }

    return results;
  }
}

export class ConditionalTransitionBuilder {
  constructor(
    private readonly transition: ConditionalTransition,
    private readonly resourceManager: ResourceManager,
  ) {}

  /**
   * Execute the conditional transition on a resource
   */
  async execute(resource: Resource): Promise<ResourceBuilderResult | null> {
    const shouldTransition = await this.transition.condition(resource);

    if (!shouldTransition) {
      return null;
    }

    const input: StateTransitionInput = {
      resourceId: resource.id,
      toState: this.transition.targetState,
      ...(this.transition.activityId !== undefined
        ? { activityId: this.transition.activityId }
        : {}),
      ...(this.transition.data !== undefined
        ? { data: this.transition.data }
        : {}),
      ...(this.transition.metadata !== undefined
        ? { metadata: this.transition.metadata }
        : {}),
    };

    const result = await this.resourceManager.transitionState(input);
    return new ResourceBuilderResult(result.resource, this.resourceManager);
  }

  /**
   * Execute the conditional transition on multiple resources
   */
  async executeOnResources(
    resources: Resource[],
  ): Promise<ResourceBuilderResult[]> {
    const results: ResourceBuilderResult[] = [];

    for (const resource of resources) {
      const result = await this.execute(resource);
      if (result) {
        results.push(result);
      }
    }

    return results;
  }
}

// ============================================================================
// Factory Functions
// ============================================================================

/**
 * Create a new ResourceBuilder instance
 */
export function createResourceBuilder(
  resourceManager: ResourceManager,
  options?: ResourceBuilderOptions,
): ResourceBuilder {
  return new ResourceBuilder(resourceManager, options);
}

/**
 * Create a resource template
 */
export function createResourceTemplate(
  name: string,
  workflowId: string,
  defaultData: any,
  options: Partial<ResourceTemplate> = {},
): ResourceTemplate {
  return {
    name,
    workflowId,
    defaultData,
    ...options,
  };
}

/**
 * Create a resource chain configuration
 */
export function createResourceChain(
  sourceResourceId: string,
  targetWorkflowId: string,
  dataMapper: (sourceData: any) => any,
  options: Partial<ResourceChain> = {},
): ResourceChain {
  return {
    sourceResourceId,
    targetWorkflowId,
    dataMapper,
    ...options,
  };
}
