/**
 * Analytics and Budget Management Client
 *
 * This module provides functionality for managing budgets, tracking costs, and monitoring
 * analytics for the Circuit Breaker workflow automation server.
 */

import { Client } from './client.js';

// ============================================================================
// Types
// ============================================================================

/**
 * Budget status information
 */
export interface BudgetStatus {
  /** Unique budget identifier */
  budgetId: string;
  /** Budget limit amount */
  limit: number;
  /** Amount already used */
  used: number;
  /** Percentage of budget used (0.0 to 100.0) */
  percentageUsed: number;
  /** Whether budget is exhausted */
  isExhausted: boolean;
  /** Whether budget is in warning state */
  isWarning: boolean;
  /** Remaining budget amount */
  remaining: number;
  /** Status message */
  message: string;
}

/**
 * Cost analytics data
 */
export interface CostAnalytics {
  /** Total cost for the period */
  totalCost: number;
  /** Total tokens used */
  totalTokens: number;
  /** Average cost per token */
  averageCostPerToken: number;
  /** Cost breakdown by provider */
  providerBreakdown: Record<string, number>;
  /** Cost breakdown by model */
  modelBreakdown: Record<string, number>;
  /** Daily costs over the period */
  dailyCosts: Record<string, number>;
  /** Start of the analytics period */
  periodStart: string;
  /** End of the analytics period */
  periodEnd: string;
}

/**
 * Budget input for setting limits
 */
export interface BudgetInput {
  /** User ID for user-specific budget */
  userId?: string;
  /** Project ID for project-specific budget */
  projectId?: string;
  /** Budget limit amount */
  limit: number;
  /** Budget period (daily, weekly, monthly) */
  period: string;
  /** Warning threshold (0.0 to 1.0) */
  warningThreshold: number;
}

/**
 * Cost analytics input parameters
 */
export interface CostAnalyticsInput {
  /** User ID to filter analytics */
  userId?: string;
  /** Project ID to filter analytics */
  projectId?: string;
  /** Start date for analytics (ISO 8601) */
  startDate: string;
  /** End date for analytics (ISO 8601) */
  endDate: string;
}

/**
 * Cost update event (for future subscription implementation)
 */
export interface CostUpdateEvent {
  userId?: string;
  projectId?: string;
  cost: number;
  timestamp: string;
  details: Record<string, any>;
}

// ============================================================================
// Analytics Client
// ============================================================================

/**
 * Analytics client for budget and cost management operations
 */
export class AnalyticsClient {
  constructor(private client: Client) {}

  /**
   * Get budget status for a user or project
   */
  budgetStatus(): BudgetStatusBuilder {
    return new BudgetStatusBuilder(this.client);
  }

  /**
   * Get cost analytics for a time period
   */
  costAnalytics(): CostAnalyticsBuilder {
    return new CostAnalyticsBuilder(this.client);
  }

  /**
   * Set budget limits
   */
  setBudget(): SetBudgetBuilder {
    return new SetBudgetBuilder(this.client);
  }

  /**
   * Subscribe to real-time cost updates
   * @param userId Optional user ID to filter updates
   * @returns Promise that resolves to a cost update stream
   */
  async subscribeCostUpdates(userId?: string): Promise<CostUpdateStream> {
    // This would need WebSocket/SSE implementation
    // For now, throw an error indicating subscriptions aren't implemented
    throw new Error('Real-time subscriptions not yet implemented');
  }
}

// ============================================================================
// Builders
// ============================================================================

/**
 * Builder for budget status queries
 */
export class BudgetStatusBuilder {
  private _userId?: string;
  private _projectId?: string;

  constructor(private client: Client) {}

  /**
   * Set user ID for user-specific budget
   */
  userId(userId: string): this {
    this._userId = userId;
    return this;
  }

  /**
   * Set project ID for project-specific budget
   */
  projectId(projectId: string): this {
    this._projectId = projectId;
    return this;
  }

  /**
   * Execute the budget status query
   */
  async get(): Promise<BudgetStatus> {
    const query = `
      query BudgetStatus($userId: String, $projectId: String) {
        budgetStatus(userId: $userId, projectId: $projectId) {
          budgetId
          limit
          used
          percentageUsed
          isExhausted
          isWarning
          remaining
          message
        }
      }
    `;

    const variables = {
      userId: this._userId,
      projectId: this._projectId,
    };

    const response = await this.client.graphql<{
      budgetStatus: {
        budgetId: string;
        limit: number;
        used: number;
        percentageUsed: number;
        isExhausted: boolean;
        isWarning: boolean;
        remaining: number;
        message: string;
      };
    }>(query, variables);

    return {
      budgetId: response.budgetStatus.budgetId,
      limit: response.budgetStatus.limit,
      used: response.budgetStatus.used,
      percentageUsed: response.budgetStatus.percentageUsed,
      isExhausted: response.budgetStatus.isExhausted,
      isWarning: response.budgetStatus.isWarning,
      remaining: response.budgetStatus.remaining,
      message: response.budgetStatus.message,
    };
  }
}

/**
 * Builder for cost analytics queries
 */
export class CostAnalyticsBuilder {
  private _userId?: string;
  private _projectId?: string;
  private _startDate?: string;
  private _endDate?: string;

  constructor(private client: Client) {}

  /**
   * Set user ID to filter analytics
   */
  userId(userId: string): this {
    this._userId = userId;
    return this;
  }

  /**
   * Set project ID to filter analytics
   */
  projectId(projectId: string): this {
    this._projectId = projectId;
    return this;
  }

  /**
   * Set date range for analytics
   */
  dateRange(startDate: string, endDate: string): this {
    this._startDate = startDate;
    this._endDate = endDate;
    return this;
  }

  /**
   * Set start date for analytics
   */
  startDate(startDate: string): this {
    this._startDate = startDate;
    return this;
  }

  /**
   * Set end date for analytics
   */
  endDate(endDate: string): this {
    this._endDate = endDate;
    return this;
  }

  /**
   * Execute the cost analytics query
   */
  async get(): Promise<CostAnalytics> {
    if (!this._startDate) {
      throw new Error('startDate is required');
    }
    if (!this._endDate) {
      throw new Error('endDate is required');
    }

    const query = `
      query CostAnalytics($input: CostAnalyticsInput!) {
        costAnalytics(input: $input) {
          totalCost
          totalTokens
          averageCostPerToken
          providerBreakdown
          modelBreakdown
          dailyCosts
          periodStart
          periodEnd
        }
      }
    `;

    const input: CostAnalyticsInput = {
      userId: this._userId,
      projectId: this._projectId,
      startDate: this._startDate,
      endDate: this._endDate,
    };

    const variables = { input };

    const response = await this.client.graphql<{
      costAnalytics: {
        totalCost: number;
        totalTokens: number;
        averageCostPerToken: number;
        providerBreakdown: any;
        modelBreakdown: any;
        dailyCosts: any;
        periodStart: string;
        periodEnd: string;
      };
    }>(query, variables);

    // Convert JSON values to Records
    const providerBreakdown =
      typeof response.costAnalytics.providerBreakdown === 'object'
        ? response.costAnalytics.providerBreakdown as Record<string, number>
        : {};

    const modelBreakdown =
      typeof response.costAnalytics.modelBreakdown === 'object'
        ? response.costAnalytics.modelBreakdown as Record<string, number>
        : {};

    const dailyCosts =
      typeof response.costAnalytics.dailyCosts === 'object'
        ? response.costAnalytics.dailyCosts as Record<string, number>
        : {};

    return {
      totalCost: response.costAnalytics.totalCost,
      totalTokens: response.costAnalytics.totalTokens,
      averageCostPerToken: response.costAnalytics.averageCostPerToken,
      providerBreakdown,
      modelBreakdown,
      dailyCosts,
      periodStart: response.costAnalytics.periodStart,
      periodEnd: response.costAnalytics.periodEnd,
    };
  }
}

/**
 * Builder for setting budget limits
 */
export class SetBudgetBuilder {
  private _userId?: string;
  private _projectId?: string;
  private _limit?: number;
  private _period?: string;
  private _warningThreshold?: number;

  constructor(private client: Client) {}

  /**
   * Set user ID for user-specific budget
   */
  userId(userId: string): this {
    this._userId = userId;
    return this;
  }

  /**
   * Set project ID for project-specific budget
   */
  projectId(projectId: string): this {
    this._projectId = projectId;
    return this;
  }

  /**
   * Set budget limit amount
   */
  limit(limit: number): this {
    this._limit = limit;
    return this;
  }

  /**
   * Set budget period (daily, weekly, monthly)
   */
  period(period: string): this {
    this._period = period;
    return this;
  }

  /**
   * Set warning threshold (0.0 to 1.0)
   */
  warningThreshold(threshold: number): this {
    this._warningThreshold = threshold;
    return this;
  }

  /**
   * Execute the set budget mutation
   */
  async execute(): Promise<BudgetStatus> {
    if (this._limit === undefined) {
      throw new Error('limit is required');
    }
    if (!this._period) {
      throw new Error('period is required');
    }
    if (this._warningThreshold === undefined) {
      throw new Error('warningThreshold is required');
    }

    const query = `
      mutation SetBudget($input: BudgetInput!) {
        setBudget(input: $input) {
          budgetId
          limit
          used
          percentageUsed
          isExhausted
          isWarning
          remaining
          message
        }
      }
    `;

    const input: BudgetInput = {
      userId: this._userId,
      projectId: this._projectId,
      limit: this._limit,
      period: this._period,
      warningThreshold: this._warningThreshold,
    };

    const variables = { input };

    const response = await this.client.graphql<{
      setBudget: {
        budgetId: string;
        limit: number;
        used: number;
        percentageUsed: number;
        isExhausted: boolean;
        isWarning: boolean;
        remaining: number;
        message: string;
      };
    }>(query, variables);

    return {
      budgetId: response.setBudget.budgetId,
      limit: response.setBudget.limit,
      used: response.setBudget.used,
      percentageUsed: response.setBudget.percentageUsed,
      isExhausted: response.setBudget.isExhausted,
      isWarning: response.setBudget.isWarning,
      remaining: response.setBudget.remaining,
      message: response.setBudget.message,
    };
  }
}

// ============================================================================
// Subscription Stream (Placeholder)
// ============================================================================

/**
 * Stream of cost updates (placeholder for future subscription implementation)
 */
export class CostUpdateStream {
  // This would contain WebSocket/SSE stream implementation

  /**
   * Listen to cost update events
   */
  onUpdate(callback: (event: CostUpdateEvent) => void): void {
    // Placeholder implementation
    throw new Error('Subscription streams not yet implemented');
  }

  /**
   * Close the stream
   */
  close(): void {
    // Placeholder implementation
  }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/**
 * Convenience function to create a cost analytics query
 */
export function costAnalytics(
  client: Client,
  startDate: string,
  endDate: string,
): CostAnalyticsBuilder {
  return client.analytics().costAnalytics().dateRange(startDate, endDate);
}

/**
 * Convenience function to create a budget status query
 */
export function budgetStatus(client: Client): BudgetStatusBuilder {
  return client.analytics().budgetStatus();
}

/**
 * Convenience function to create a set budget operation
 */
export function setBudget(
  client: Client,
  limit: number,
  period: string,
): SetBudgetBuilder {
  return client
    .analytics()
    .setBudget()
    .limit(limit)
    .period(period)
    .warningThreshold(0.8); // Default warning threshold
}

/**
 * Convenience function to get budget status for a user
 */
export async function getUserBudgetStatus(
  client: Client,
  userId: string,
): Promise<BudgetStatus> {
  return client.analytics().budgetStatus().userId(userId).get();
}

/**
 * Convenience function to get budget status for a project
 */
export async function getProjectBudgetStatus(
  client: Client,
  projectId: string,
): Promise<BudgetStatus> {
  return client.analytics().budgetStatus().projectId(projectId).get();
}

/**
 * Convenience function to get monthly cost analytics for a user
 */
export async function getUserMonthlyCostAnalytics(
  client: Client,
  userId: string,
  year: number,
  month: number,
): Promise<CostAnalytics> {
  const startDate = `${year}-${month.toString().padStart(2, '0')}-01`;
  const endDate = new Date(year, month, 0).toISOString().split('T')[0]; // Last day of month

  return client
    .analytics()
    .costAnalytics()
    .userId(userId)
    .dateRange(startDate, endDate)
    .get();
}

/**
 * Convenience function to set a monthly budget for a user
 */
export async function setUserMonthlyBudget(
  client: Client,
  userId: string,
  limit: number,
  warningThreshold: number = 0.8,
): Promise<BudgetStatus> {
  return client
    .analytics()
    .setBudget()
    .userId(userId)
    .limit(limit)
    .period('monthly')
    .warningThreshold(warningThreshold)
    .execute();
}

// ============================================================================
// Usage Examples
// ============================================================================

/**
 * Example usage of the Analytics client
 *
 * ```typescript
 * import { Client } from './client.js';
 * import { getUserBudgetStatus, setUserMonthlyBudget } from './analytics.js';
 *
 * const client = Client.builder()
 *   .baseUrl('http://localhost:4000')
 *   .build();
 *
 * // Get budget status
 * const budget = await getUserBudgetStatus(client, 'user123');
 * console.log(`Budget used: ${budget.percentageUsed.toFixed(2)}%`);
 *
 * // Set monthly budget
 * const newBudget = await setUserMonthlyBudget(client, 'user123', 100.0);
 * console.log(`Budget set: $${newBudget.limit}`);
 *
 * // Get cost analytics
 * const analytics = await client.analytics()
 *   .costAnalytics()
 *   .userId('user123')
 *   .dateRange('2024-01-01', '2024-01-31')
 *   .get();
 *
 * console.log(`Total cost: $${analytics.totalCost.toFixed(2)}`);
 * ```
 */
