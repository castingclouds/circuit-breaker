/**
 * Analytics and Budget Management Client
 *
 * This module provides functionality for managing budgets, tracking costs, and monitoring
 * analytics for the Circuit Breaker workflow automation server.
 */
import { Client } from "./client.js";
import { CostUpdateEvent } from "./subscriptions.js";
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
/**
 * Analytics client for budget and cost management operations
 */
export declare class AnalyticsClient {
    private client;
    constructor(client: Client);
    /**
     * Get budget status for a user or project
     */
    budgetStatus(): BudgetStatusBuilder;
    /**
     * Get cost analytics for a time period
     */
    costAnalytics(): CostAnalyticsBuilder;
    /**
     * Set budget limits
     */
    setBudget(): SetBudgetBuilder;
    /**
     * Subscribe to real-time cost updates
     * @param userId Optional user ID to filter updates
     * @returns Promise that resolves to a cost update stream
     */
    subscribeCostUpdates(userId?: string): Promise<CostUpdateStream>;
}
/**
 * Builder for budget status queries
 */
export declare class BudgetStatusBuilder {
    private client;
    private _userId?;
    private _projectId?;
    constructor(client: Client);
    /**
     * Set user ID for user-specific budget
     */
    userId(userId: string): this;
    /**
     * Set project ID for project-specific budget
     */
    projectId(projectId: string): this;
    /**
     * Execute the budget status query
     */
    get(): Promise<BudgetStatus>;
}
/**
 * Builder for cost analytics queries
 */
export declare class CostAnalyticsBuilder {
    private client;
    private _userId?;
    private _projectId?;
    private _startDate?;
    private _endDate?;
    constructor(client: Client);
    /**
     * Set user ID to filter analytics
     */
    userId(userId: string): this;
    /**
     * Set project ID to filter analytics
     */
    projectId(projectId: string): this;
    /**
     * Set date range for analytics
     */
    dateRange(startDate: string, endDate: string): this;
    /**
     * Set start date for analytics
     */
    startDate(startDate: string): this;
    /**
     * Set end date for analytics
     */
    endDate(endDate: string): this;
    /**
     * Execute the cost analytics query
     */
    get(): Promise<CostAnalytics>;
}
/**
 * Builder for setting budget limits
 */
export declare class SetBudgetBuilder {
    private client;
    private _userId?;
    private _projectId?;
    private _limit?;
    private _period?;
    private _warningThreshold?;
    constructor(client: Client);
    /**
     * Set user ID for user-specific budget
     */
    userId(userId: string): this;
    /**
     * Set project ID for project-specific budget
     */
    projectId(projectId: string): this;
    /**
     * Set budget limit amount
     */
    limit(limit: number): this;
    /**
     * Set budget period (daily, weekly, monthly)
     */
    period(period: string): this;
    /**
     * Set warning threshold (0.0 to 1.0)
     */
    warningThreshold(threshold: number): this;
    /**
     * Execute the set budget mutation
     */
    execute(): Promise<BudgetStatus>;
}
/**
 * Stream of cost updates with real subscription implementation
 */
export declare class CostUpdateStream {
    private callbacks;
    private subscriptionId?;
    /**
     * Listen to cost update events
     */
    onUpdate(callback: (event: CostUpdateEvent) => void): void;
    /**
     * Internal method to emit events to listeners
     */
    _emit(event: CostUpdateEvent): void;
    /**
     * Internal method to set subscription ID
     */
    _setSubscriptionId(id: string): void;
    /**
     * Get the subscription ID
     */
    getSubscriptionId(): string | undefined;
    /**
     * Close the stream
     */
    close(): void;
}
/**
 * Convenience function to create a cost analytics query
 */
export declare function costAnalytics(client: Client, startDate: string, endDate: string): CostAnalyticsBuilder;
/**
 * Convenience function to create a budget status query
 */
export declare function budgetStatus(client: Client): BudgetStatusBuilder;
/**
 * Convenience function to create a set budget operation
 */
export declare function setBudget(client: Client, limit: number, period: string): SetBudgetBuilder;
/**
 * Convenience function to get budget status for a user
 */
export declare function getUserBudgetStatus(client: Client, userId: string): Promise<BudgetStatus>;
/**
 * Convenience function to get budget status for a project
 */
export declare function getProjectBudgetStatus(client: Client, projectId: string): Promise<BudgetStatus>;
/**
 * Convenience function to get monthly cost analytics for a user
 */
export declare function getUserMonthlyCostAnalytics(client: Client, userId: string, year: number, month: number): Promise<CostAnalytics>;
/**
 * Convenience function to set a monthly budget for a user
 */
export declare function setUserMonthlyBudget(client: Client, userId: string, limit: number, warningThreshold?: number): Promise<BudgetStatus>;
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
//# sourceMappingURL=analytics.d.ts.map