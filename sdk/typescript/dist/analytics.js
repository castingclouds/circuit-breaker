/**
 * Analytics and Budget Management Client
 *
 * This module provides functionality for managing budgets, tracking costs, and monitoring
 * analytics for the Circuit Breaker workflow automation server.
 */
// ============================================================================
// Analytics Client
// ============================================================================
/**
 * Analytics client for budget and cost management operations
 */
export class AnalyticsClient {
    constructor(client) {
        this.client = client;
    }
    /**
     * Get budget status for a user or project
     */
    budgetStatus() {
        return new BudgetStatusBuilder(this.client);
    }
    /**
     * Get cost analytics for a time period
     */
    costAnalytics() {
        return new CostAnalyticsBuilder(this.client);
    }
    /**
     * Set budget limits
     */
    setBudget() {
        return new SetBudgetBuilder(this.client);
    }
    /**
     * Subscribe to real-time cost updates
     * @param userId Optional user ID to filter updates
     * @returns Promise that resolves to a cost update stream
     */
    async subscribeCostUpdates(userId) {
        const subscriptions = this.client.subscriptions();
        const builder = subscriptions.costUpdates();
        if (userId) {
            builder.userId(userId);
        }
        const stream = new CostUpdateStream();
        const subscriptionId = await builder.subscribe((update) => {
            stream._emit(update);
        });
        stream._setSubscriptionId(subscriptionId);
        return stream;
    }
}
// ============================================================================
// Builders
// ============================================================================
/**
 * Builder for budget status queries
 */
export class BudgetStatusBuilder {
    constructor(client) {
        this.client = client;
    }
    /**
     * Set user ID for user-specific budget
     */
    userId(userId) {
        this._userId = userId;
        return this;
    }
    /**
     * Set project ID for project-specific budget
     */
    projectId(projectId) {
        this._projectId = projectId;
        return this;
    }
    /**
     * Execute the budget status query
     */
    async get() {
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
        const response = await this.client.graphql(query, variables);
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
    constructor(client) {
        this.client = client;
    }
    /**
     * Set user ID to filter analytics
     */
    userId(userId) {
        this._userId = userId;
        return this;
    }
    /**
     * Set project ID to filter analytics
     */
    projectId(projectId) {
        this._projectId = projectId;
        return this;
    }
    /**
     * Set date range for analytics
     */
    dateRange(startDate, endDate) {
        this._startDate = startDate;
        this._endDate = endDate;
        return this;
    }
    /**
     * Set start date for analytics
     */
    startDate(startDate) {
        this._startDate = startDate;
        return this;
    }
    /**
     * Set end date for analytics
     */
    endDate(endDate) {
        this._endDate = endDate;
        return this;
    }
    /**
     * Execute the cost analytics query
     */
    async get() {
        if (!this._startDate) {
            throw new Error("startDate is required");
        }
        if (!this._endDate) {
            throw new Error("endDate is required");
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
        const input = {
            userId: this._userId,
            projectId: this._projectId,
            startDate: this._startDate,
            endDate: this._endDate,
        };
        const variables = { input };
        const response = await this.client.graphql(query, variables);
        // Convert JSON values to Records
        const providerBreakdown = typeof response.costAnalytics.providerBreakdown === "object"
            ? response.costAnalytics.providerBreakdown
            : {};
        const modelBreakdown = typeof response.costAnalytics.modelBreakdown === "object"
            ? response.costAnalytics.modelBreakdown
            : {};
        const dailyCosts = typeof response.costAnalytics.dailyCosts === "object"
            ? response.costAnalytics.dailyCosts
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
    constructor(client) {
        this.client = client;
    }
    /**
     * Set user ID for user-specific budget
     */
    userId(userId) {
        this._userId = userId;
        return this;
    }
    /**
     * Set project ID for project-specific budget
     */
    projectId(projectId) {
        this._projectId = projectId;
        return this;
    }
    /**
     * Set budget limit amount
     */
    limit(limit) {
        this._limit = limit;
        return this;
    }
    /**
     * Set budget period (daily, weekly, monthly)
     */
    period(period) {
        this._period = period;
        return this;
    }
    /**
     * Set warning threshold (0.0 to 1.0)
     */
    warningThreshold(threshold) {
        this._warningThreshold = threshold;
        return this;
    }
    /**
     * Execute the set budget mutation
     */
    async execute() {
        if (this._limit === undefined) {
            throw new Error("limit is required");
        }
        if (!this._period) {
            throw new Error("period is required");
        }
        if (this._warningThreshold === undefined) {
            throw new Error("warningThreshold is required");
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
        const input = {
            userId: this._userId,
            projectId: this._projectId,
            limit: this._limit,
            period: this._period,
            warningThreshold: this._warningThreshold,
        };
        const variables = { input };
        const response = await this.client.graphql(query, variables);
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
 * Stream of cost updates with real subscription implementation
 */
export class CostUpdateStream {
    constructor() {
        this.callbacks = [];
    }
    /**
     * Listen to cost update events
     */
    onUpdate(callback) {
        this.callbacks.push(callback);
    }
    /**
     * Internal method to emit events to listeners
     */
    _emit(event) {
        this.callbacks.forEach((callback) => {
            try {
                callback(event);
            }
            catch (error) {
                console.error("Error in cost update callback:", error);
            }
        });
    }
    /**
     * Internal method to set subscription ID
     */
    _setSubscriptionId(id) {
        this.subscriptionId = id;
    }
    /**
     * Get the subscription ID
     */
    getSubscriptionId() {
        return this.subscriptionId;
    }
    /**
     * Close the stream
     */
    close() {
        this.callbacks = [];
        // TODO: Unsubscribe from the actual subscription
    }
}
// ============================================================================
// Convenience Functions
// ============================================================================
/**
 * Convenience function to create a cost analytics query
 */
export function costAnalytics(client, startDate, endDate) {
    return client.analytics().costAnalytics().dateRange(startDate, endDate);
}
/**
 * Convenience function to create a budget status query
 */
export function budgetStatus(client) {
    return client.analytics().budgetStatus();
}
/**
 * Convenience function to create a set budget operation
 */
export function setBudget(client, limit, period) {
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
export async function getUserBudgetStatus(client, userId) {
    return client.analytics().budgetStatus().userId(userId).get();
}
/**
 * Convenience function to get budget status for a project
 */
export async function getProjectBudgetStatus(client, projectId) {
    return client.analytics().budgetStatus().projectId(projectId).get();
}
/**
 * Convenience function to get monthly cost analytics for a user
 */
export async function getUserMonthlyCostAnalytics(client, userId, year, month) {
    const startDate = `${year}-${month.toString().padStart(2, "0")}-01`;
    const endDate = new Date(year, month, 0).toISOString().split("T")[0]; // Last day of month
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
export async function setUserMonthlyBudget(client, userId, limit, warningThreshold = 0.8) {
    return client
        .analytics()
        .setBudget()
        .userId(userId)
        .limit(limit)
        .period("monthly")
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
//# sourceMappingURL=analytics.js.map