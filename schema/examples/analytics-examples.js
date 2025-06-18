const { GraphQLClient } = require("graphql-request");
const { loadSchemaSync } = require("@graphql-tools/load");
const { GraphQLFileLoader } = require("@graphql-tools/graphql-file-loader");
const fs = require("fs");
const path = require("path");

// Load the analytics schema
const analyticsSchema = loadSchemaSync(
  path.join(__dirname, "../analytics.graphql"),
  {
    loaders: [new GraphQLFileLoader()],
  },
);

// Load GraphQL operations
const operationsFile = path.join(
  __dirname,
  "../operations/analytics.graphql",
);
const operations = fs.readFileSync(operationsFile, "utf8");

// Parse operations to extract individual queries/mutations
const operationMap = {};
const operationRegex =
  /(query|mutation|subscription)\s+(\w+)[\s\S]*?(?=(?:query|mutation|subscription)\s+\w+|$)/g;
let match;
while ((match = operationRegex.exec(operations)) !== null) {
  operationMap[match[2]] = match[0].trim();
}

// GraphQL client setup
const endpoint = "http://localhost:4000/graphql";
const client = new GraphQLClient(endpoint);

/**
 * Analytics Examples
 * These examples demonstrate how to use the cost tracking and budget management operations
 * defined in ../analytics.graphql
 */

// ============================================================================
// QUERY EXAMPLES
// ============================================================================

/**
 * Get budget status for a specific user
 */
async function getUserBudgetStatus(userId) {
  const query = operationMap.GetBudgetStatus;

  try {
    const data = await client.request(query, { userId });
    console.log("User budget status:", JSON.stringify(data, null, 2));
    return data.budgetStatus;
  } catch (error) {
    console.error("Error fetching user budget status:", error);
    throw error;
  }
}

/**
 * Get budget status for a project
 */
async function getProjectBudgetStatus(projectId) {
  const query = operationMap.GetBudgetStatus;

  try {
    const data = await client.request(query, { projectId });
    console.log("Project budget status:", JSON.stringify(data, null, 2));
    return data.budgetStatus;
  } catch (error) {
    console.error("Error fetching project budget status:", error);
    throw error;
  }
}

/**
 * Get budget status for both user and project
 */
async function getCombinedBudgetStatus(userId, projectId) {
  const query = operationMap.GetBudgetStatus;

  try {
    const data = await client.request(query, { userId, projectId });
    console.log("Combined budget status:", JSON.stringify(data, null, 2));
    return data.budgetStatus;
  } catch (error) {
    console.error("Error fetching combined budget status:", error);
    throw error;
  }
}

/**
 * Get cost analytics for a time period
 */
async function getCostAnalytics(startDate, endDate, userId = null, projectId = null) {
  const query = operationMap.GetCostAnalytics;

  const analyticsInput = {
    startDate: startDate,
    endDate: endDate,
    userId: userId,
    projectId: projectId,
  };

  try {
    const data = await client.request(query, { input: analyticsInput });
    console.log("Cost analytics:", JSON.stringify(data, null, 2));
    return data.costAnalytics;
  } catch (error) {
    console.error("Error fetching cost analytics:", error);
    throw error;
  }
}

/**
 * Get monthly cost analytics for a user
 */
async function getMonthlyCostAnalytics(userId, year = new Date().getFullYear(), month = new Date().getMonth() + 1) {
  const startDate = new Date(year, month - 1, 1).toISOString();
  const endDate = new Date(year, month, 0).toISOString();

  return await getCostAnalytics(startDate, endDate, userId);
}

/**
 * Get weekly cost analytics
 */
async function getWeeklyCostAnalytics(weeksAgo = 0, projectId = null) {
  const endDate = new Date();
  endDate.setDate(endDate.getDate() - (weeksAgo * 7));

  const startDate = new Date(endDate);
  startDate.setDate(startDate.getDate() - 7);

  return await getCostAnalytics(
    startDate.toISOString(),
    endDate.toISOString(),
    null,
    projectId
  );
}

// ============================================================================
// MUTATION EXAMPLES
// ============================================================================

/**
 * Set daily budget for a user
 */
async function setUserDailyBudget(userId, limit, warningThreshold = 0.8) {
  const mutation = operationMap.SetBudget;

  const budgetInput = {
    userId: userId,
    limit: limit,
    period: "daily",
    warningThreshold: warningThreshold,
  };

  try {
    const data = await client.request(mutation, { input: budgetInput });
    console.log("Set user daily budget:", JSON.stringify(data, null, 2));
    return data.setBudget;
  } catch (error) {
    console.error("Error setting user daily budget:", error);
    throw error;
  }
}

/**
 * Set monthly budget for a project
 */
async function setProjectMonthlyBudget(projectId, limit, warningThreshold = 0.9) {
  const mutation = operationMap.SetBudget;

  const budgetInput = {
    projectId: projectId,
    limit: limit,
    period: "monthly",
    warningThreshold: warningThreshold,
  };

  try {
    const data = await client.request(mutation, { input: budgetInput });
    console.log("Set project monthly budget:", JSON.stringify(data, null, 2));
    return data.setBudget;
  } catch (error) {
    console.error("Error setting project monthly budget:", error);
    throw error;
  }
}

/**
 * Set weekly budget for a user
 */
async function setUserWeeklyBudget(userId, limit, warningThreshold = 0.75) {
  const mutation = operationMap.SetBudget;

  const budgetInput = {
    userId: userId,
    limit: limit,
    period: "weekly",
    warningThreshold: warningThreshold,
  };

  try {
    const data = await client.request(mutation, { input: budgetInput });
    console.log("Set user weekly budget:", JSON.stringify(data, null, 2));
    return data.setBudget;
  } catch (error) {
    console.error("Error setting user weekly budget:", error);
    throw error;
  }
}

/**
 * Set enterprise project budget with high limit
 */
async function setEnterpriseProjectBudget(projectId, limit = 5000, warningThreshold = 0.85) {
  const mutation = operationMap.SetBudget;

  const budgetInput = {
    projectId: projectId,
    limit: limit,
    period: "monthly",
    warningThreshold: warningThreshold,
  };

  try {
    const data = await client.request(mutation, { input: budgetInput });
    console.log("Set enterprise project budget:", JSON.stringify(data, null, 2));
    return data.setBudget;
  } catch (error) {
    console.error("Error setting enterprise budget:", error);
    throw error;
  }
}

// ============================================================================
// SUBSCRIPTION EXAMPLES
// ============================================================================

/**
 * Subscribe to cost updates for a user using WebSocket
 * Note: This requires a WebSocket client like graphql-ws
 */
function subscribeToCostUpdates(userId, callback) {
  const subscription = operationMap.CostUpdates;

  // This would require WebSocket setup
  console.log(`Subscription query for user ${userId}:`, subscription);

  // In a real implementation, you'd use something like:
  // const wsClient = createClient({ url: 'ws://localhost:4000/graphql' });
  // wsClient.subscribe({ query: subscription, variables: { userId } }, callback);
}

/**
 * Subscribe to general cost updates
 */
function subscribeToAllCostUpdates(callback) {
  const subscription = operationMap.CostUpdates;

  console.log("Subscription query for all cost updates:", subscription);

  // In a real implementation, you'd use something like:
  // const wsClient = createClient({ url: 'ws://localhost:4000/graphql' });
  // wsClient.subscribe({ query: subscription, variables: {} }, callback);
}

// ============================================================================
// COMPLETE ANALYTICS EXAMPLE
// ============================================================================

/**
 * Complete example demonstrating analytics and budget management lifecycle
 */
async function completeAnalyticsExample() {
  console.log("\n=== Complete Analytics Example ===\n");

  const userId = "user-demo-123";
  const projectId = "project-demo-456";

  try {
    // 1. Set up budgets
    console.log("1. Setting up user daily budget...");
    await setUserDailyBudget(userId, 50.0, 0.8);

    console.log("\n2. Setting up project monthly budget...");
    await setProjectMonthlyBudget(projectId, 1000.0, 0.9);

    console.log("\n3. Setting up enterprise project budget...");
    await setEnterpriseProjectBudget("enterprise-project", 5000.0, 0.85);

    // 4. Check budget statuses
    console.log("\n4. Checking user budget status...");
    const userBudget = await getUserBudgetStatus(userId);

    console.log("\n5. Checking project budget status...");
    const projectBudget = await getProjectBudgetStatus(projectId);

    console.log("\n6. Checking combined budget status...");
    await getCombinedBudgetStatus(userId, projectId);

    // 7. Get cost analytics
    console.log("\n7. Getting monthly cost analytics...");
    await getMonthlyCostAnalytics(userId);

    console.log("\n8. Getting weekly cost analytics for project...");
    await getWeeklyCostAnalytics(0, projectId);

    console.log("\n9. Getting cost analytics for last 30 days...");
    const thirtyDaysAgo = new Date();
    thirtyDaysAgo.setDate(thirtyDaysAgo.getDate() - 30);
    await getCostAnalytics(
      thirtyDaysAgo.toISOString(),
      new Date().toISOString(),
      userId
    );

    // 10. Budget alerts simulation
    console.log("\n10. Simulating budget alerts...");
    if (userBudget.isWarning) {
      console.log("⚠️  User budget warning threshold reached!");
    }
    if (userBudget.isExhausted) {
      console.log("🚨 User budget exhausted!");
    }

    console.log("\n✅ Analytics example completed successfully!");

  } catch (error) {
    console.error("\n❌ Analytics example failed:", error.message);
  }
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/**
 * Helper function to format budget status for display
 */
function formatBudgetStatus(budget) {
  const status = budget.isExhausted ? "🚨 EXHAUSTED" :
                 budget.isWarning ? "⚠️  WARNING" : "✅ OK";

  return {
    status: status,
    usage: `$${budget.used.toFixed(2)} / $${budget.limit.toFixed(2)}`,
    percentage: `${budget.percentageUsed.toFixed(1)}%`,
    remaining: `$${budget.remaining.toFixed(2)}`,
    message: budget.message,
  };
}

/**
 * Helper function to calculate cost trends
 */
function calculateCostTrends(analytics) {
  if (!analytics.dailyCosts || Object.keys(analytics.dailyCosts).length < 2) {
    return { trend: "insufficient_data", change: 0 };
  }

  const dailyValues = Object.values(analytics.dailyCosts).map(Number);
  const recent = dailyValues.slice(-7); // Last 7 days
  const previous = dailyValues.slice(-14, -7); // Previous 7 days

  const recentAvg = recent.reduce((a, b) => a + b, 0) / recent.length;
  const previousAvg = previous.reduce((a, b) => a + b, 0) / previous.length;

  const change = ((recentAvg - previousAvg) / previousAvg) * 100;

  return {
    trend: change > 5 ? "increasing" : change < -5 ? "decreasing" : "stable",
    change: change.toFixed(1),
    recentAvg: recentAvg.toFixed(2),
    previousAvg: previousAvg.toFixed(2),
  };
}

/**
 * Helper function to get budget recommendations
 */
function getBudgetRecommendations(budget, analytics) {
  const recommendations = [];

  if (budget.percentageUsed > 0.9) {
    recommendations.push("Consider increasing budget limit");
  }

  if (budget.percentageUsed > 0.8) {
    recommendations.push("Monitor usage closely");
  }

  if (analytics && analytics.averageCostPerToken > 0.0001) {
    recommendations.push("Review LLM provider costs - consider switching to more cost-effective models");
  }

  if (recommendations.length === 0) {
    recommendations.push("Budget usage is within normal parameters");
  }

  return recommendations;
}

/**
 * Helper function to generate cost report
 */
function generateCostReport(analytics, budget) {
  const trends = calculateCostTrends(analytics);
  const recommendations = getBudgetRecommendations(budget, analytics);

  return {
    summary: {
      totalCost: `$${analytics.totalCost.toFixed(2)}`,
      totalTokens: analytics.totalTokens.toLocaleString(),
      avgCostPerToken: analytics.averageCostPerToken.toFixed(6),
      period: `${analytics.periodStart} to ${analytics.periodEnd}`,
    },
    budget: formatBudgetStatus(budget),
    trends: trends,
    recommendations: recommendations,
    topProviders: Object.entries(analytics.providerBreakdown || {})
      .sort(([,a], [,b]) => b - a)
      .slice(0, 3)
      .map(([provider, cost]) => ({ provider, cost: `$${cost.toFixed(2)}` })),
  };
}

// ============================================================================
// EXPORTS
// ============================================================================

module.exports = {
  // Query functions
  getUserBudgetStatus,
  getProjectBudgetStatus,
  getCombinedBudgetStatus,
  getCostAnalytics,
  getMonthlyCostAnalytics,
  getWeeklyCostAnalytics,

  // Mutation functions
  setUserDailyBudget,
  setProjectMonthlyBudget,
  setUserWeeklyBudget,
  setEnterpriseProjectBudget,

  // Subscription functions
  subscribeToCostUpdates,
  subscribeToAllCostUpdates,

  // Complete examples
  completeAnalyticsExample,

  // Utilities
  formatBudgetStatus,
  calculateCostTrends,
  getBudgetRecommendations,
  generateCostReport,

  // Schema reference
  analyticsSchema,
};

// Run example if this file is executed directly
if (require.main === module) {
  completeAnalyticsExample()
    .catch(console.error)
    .finally(() => process.exit(0));
}
