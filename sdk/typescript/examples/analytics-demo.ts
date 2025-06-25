/**
 * Analytics and Budget Management Demo
 *
 * This example demonstrates the Circuit Breaker SDK's analytics and budget management
 * capabilities, showing how to track costs, set budgets, and monitor spending across
 * different users and projects.
 */

import { Client } from '../src/client.js';
import {
  budgetStatus,
  costAnalytics,
  setBudget,
  getUserBudgetStatus,
  setUserMonthlyBudget,
  type BudgetStatus,
  type CostAnalytics,
} from '../src/analytics.js';

async function main(): Promise<void> {
  console.log('🔍 Circuit Breaker Analytics & Budget Management Demo');
  console.log('====================================================');

  // Initialize the client
  const baseUrl = process.env.CIRCUIT_BREAKER_URL || 'http://localhost:4000';
  const apiKey = process.env.CIRCUIT_BREAKER_API_KEY;

  let client = Client.builder().baseUrl(baseUrl);
  if (apiKey) {
    client = client.apiKey(apiKey);
  }
  const circuitBreakerClient = client.build();

  // Test connection
  try {
    const ping = await circuitBreakerClient.ping();
    console.log(`✅ Connected to Circuit Breaker server: ${ping.message}`);
  } catch (error) {
    console.log(`❌ Failed to connect to server: ${error}`);
    console.log(
      `   Make sure the Circuit Breaker server is running at ${baseUrl}`,
    );
    return;
  }

  console.log('\n📊 Analytics & Budget Management Features:');
  console.log('==========================================');

  // 1. Budget Management Demo
  console.log('\n1. 💰 Budget Management');
  console.log('   ---------------------');

  const userId = 'demo_user_123';
  const projectId = 'demo_project_456';

  // Set a monthly budget for a user
  console.log(`   Setting monthly budget for user: ${userId}`);
  try {
    const budget = await circuitBreakerClient
      .analytics()
      .setBudget()
      .userId(userId)
      .limit(100.0)
      .period('monthly')
      .warningThreshold(0.8)
      .execute();

    console.log('   ✅ Budget set successfully:');
    console.log(`      • Budget ID: ${budget.budgetId}`);
    console.log(`      • Limit: $${budget.limit.toFixed(2)}`);
    console.log(
      `      • Used: $${budget.used.toFixed(2)} (${budget.percentageUsed.toFixed(1)}%)`,
    );
    console.log(`      • Remaining: $${budget.remaining.toFixed(2)}`);
    console.log(`      • Status: ${budget.message}`);

    if (budget.isWarning) {
      console.log('      ⚠️  Warning: Budget is in warning state!');
    }
    if (budget.isExhausted) {
      console.log('      🚨 Alert: Budget is exhausted!');
    }
  } catch (error) {
    console.log(
      `   ⚠️  Could not set budget (server may not be running): ${error}`,
    );
  }

  // Set a project budget
  console.log(`\n   Setting project budget for: ${projectId}`);
  try {
    const budget = await circuitBreakerClient
      .analytics()
      .setBudget()
      .projectId(projectId)
      .limit(500.0)
      .period('monthly')
      .warningThreshold(0.75)
      .execute();

    console.log('   ✅ Project budget set:');
    console.log(`      • Limit: $${budget.limit.toFixed(2)}`);
    console.log(`      • Warning at: ${(0.75 * 100).toFixed(0)}%`);
  } catch (error) {
    console.log(`   ⚠️  Could not set project budget: ${error}`);
  }

  // 2. Budget Status Monitoring
  console.log('\n2. 📈 Budget Status Monitoring');
  console.log('   ----------------------------');

  // Check user budget status
  console.log(`   Checking budget status for user: ${userId}`);
  try {
    const status = await circuitBreakerClient
      .analytics()
      .budgetStatus()
      .userId(userId)
      .get();

    console.log('   ✅ User budget status:');
    displayBudgetStatus(status);
  } catch (error) {
    console.log(`   ⚠️  Could not get user budget status: ${error}`);
  }

  // Check project budget status
  console.log(`\n   Checking budget status for project: ${projectId}`);
  try {
    const status = await circuitBreakerClient
      .analytics()
      .budgetStatus()
      .projectId(projectId)
      .get();

    console.log('   ✅ Project budget status:');
    displayBudgetStatus(status);
  } catch (error) {
    console.log(`   ⚠️  Could not get project budget status: ${error}`);
  }

  // 3. Cost Analytics
  console.log('\n3. 📊 Cost Analytics');
  console.log('   -----------------');

  const startDate = '2024-01-01';
  const endDate = '2024-01-31';

  console.log(
    `   Getting cost analytics for user: ${userId} (${startDate} to ${endDate})`,
  );
  try {
    const analytics = await circuitBreakerClient
      .analytics()
      .costAnalytics()
      .userId(userId)
      .dateRange(startDate, endDate)
      .get();

    console.log('   ✅ Cost analytics retrieved:');
    displayCostAnalytics(analytics);
  } catch (error) {
    console.log(`   ⚠️  Could not get cost analytics: ${error}`);
  }

  // Get project analytics
  console.log(`\n   Getting cost analytics for project: ${projectId}`);
  try {
    const analytics = await circuitBreakerClient
      .analytics()
      .costAnalytics()
      .projectId(projectId)
      .dateRange(startDate, endDate)
      .get();

    console.log('   ✅ Project cost analytics:');
    displayCostAnalytics(analytics);
  } catch (error) {
    console.log(`   ⚠️  Could not get project analytics: ${error}`);
  }

  // 4. Convenience Functions Demo
  console.log('\n4. 🛠️  Convenience Functions');
  console.log('   -------------------------');

  // Using convenience functions
  console.log('   Using convenience functions for common operations:');

  // Budget status convenience function
  try {
    const status = await budgetStatus(circuitBreakerClient)
      .userId(userId)
      .get();

    console.log(
      `   ✅ Convenience budget status: $${status.used.toFixed(2)} used of $${status.limit.toFixed(2)}`,
    );
  } catch (error) {
    console.log(`   ⚠️  Convenience budget status failed: ${error}`);
  }

  // Cost analytics convenience function
  try {
    const analytics = await costAnalytics(
      circuitBreakerClient,
      startDate,
      endDate,
    )
      .userId(userId)
      .get();

    console.log(
      `   ✅ Convenience analytics: $${analytics.totalCost.toFixed(2)} total cost`,
    );
  } catch (error) {
    console.log(`   ⚠️  Convenience analytics failed: ${error}`);
  }

  // Set budget convenience function
  try {
    const budget = await setBudget(circuitBreakerClient, 200.0, 'monthly')
      .userId('convenience_user')
      .execute();

    console.log(`   ✅ Convenience budget set: $${budget.limit.toFixed(2)} limit`);
  } catch (error) {
    console.log(`   ⚠️  Convenience budget set failed: ${error}`);
  }

  // High-level convenience functions
  try {
    const userBudget = await getUserBudgetStatus(circuitBreakerClient, userId);
    console.log(
      `   ✅ User budget convenience: ${userBudget.percentageUsed.toFixed(1)}% used`,
    );
  } catch (error) {
    console.log(`   ⚠️  User budget convenience failed: ${error}`);
  }

  try {
    const monthlyBudget = await setUserMonthlyBudget(
      circuitBreakerClient,
      'monthly_user',
      150.0,
      0.9,
    );
    console.log(
      `   ✅ Monthly budget convenience: $${monthlyBudget.limit.toFixed(2)} with 90% warning`,
    );
  } catch (error) {
    console.log(`   ⚠️  Monthly budget convenience failed: ${error}`);
  }

  // 5. Real-time Cost Monitoring (Future Feature)
  console.log('\n5. ⏰ Real-time Cost Monitoring');
  console.log('   -----------------------------');
  console.log(
    '   Real-time cost monitoring via subscriptions will be available in a future release.',
  );
  console.log('   This will allow you to:');
  console.log('   • Subscribe to cost updates as they happen');
  console.log('   • Get real-time alerts when budgets are exceeded');
  console.log('   • Monitor spending patterns in real-time');

  // Demonstrate that subscription isn't implemented yet
  try {
    await circuitBreakerClient.analytics().subscribeCostUpdates(userId);
    console.log('   ✅ Subscribed to cost updates');
  } catch (error) {
    console.log(`   ⚠️  Cost update subscriptions: ${error}`);
  }

  // 6. Advanced Analytics Scenarios
  console.log('\n6. 🔬 Advanced Analytics Scenarios');
  console.log('   --------------------------------');

  // Multi-date range analysis
  const dateRanges = [
    { start: '2024-01-01', end: '2024-01-31', month: 'January' },
    { start: '2024-02-01', end: '2024-02-29', month: 'February' },
    { start: '2024-03-01', end: '2024-03-31', month: 'March' },
  ];

  console.log('   Analyzing costs across multiple months:');
  for (const { start, end, month } of dateRanges) {
    try {
      const analytics = await circuitBreakerClient
        .analytics()
        .costAnalytics()
        .userId(userId)
        .dateRange(start, end)
        .get();

      console.log(
        `   • ${month}: $${analytics.totalCost.toFixed(2)} total, ${analytics.totalTokens} tokens used`,
      );
    } catch (error) {
      console.log(`   • ${month}: No data available`);
    }
  }

  // Budget health check
  console.log('\n   Budget Health Check:');
  try {
    const status = await circuitBreakerClient
      .analytics()
      .budgetStatus()
      .userId(userId)
      .get();

    let health: string;
    if (status.isExhausted) {
      health = '🚨 CRITICAL';
    } else if (status.isWarning) {
      health = '⚠️  WARNING';
    } else if (status.percentageUsed > 50.0) {
      health = '🟡 MODERATE';
    } else {
      health = '✅ HEALTHY';
    }

    console.log(
      `   Budget Health: ${health} (${status.percentageUsed.toFixed(1)}% used)`,
    );

    // Recommendations
    if (status.isExhausted) {
      console.log('   💡 Recommendation: Increase budget limit or optimize usage');
    } else if (status.isWarning) {
      console.log('   💡 Recommendation: Monitor usage closely, consider optimizations');
    } else if (status.percentageUsed > 50.0) {
      console.log(
        '   💡 Recommendation: Review usage patterns for optimization opportunities',
      );
    } else {
      console.log('   💡 Budget is healthy - continue current usage patterns');
    }
  } catch (error) {
    console.log(`   ⚠️  Could not perform health check: ${error}`);
  }

  // 7. TypeScript-specific Features
  console.log('\n7. 🔷 TypeScript-Specific Features');
  console.log('   --------------------------------');

  console.log('   Type-safe analytics operations:');

  // Type-safe budget operations
  try {
    const budget = await circuitBreakerClient
      .analytics()
      .setBudget()
      .userId('typescript_user')
      .limit(75.0)
      .period('weekly')
      .warningThreshold(0.6)
      .execute();

    // TypeScript ensures type safety
    const isOverBudget: boolean = budget.isExhausted;
    const remainingBudget: number = budget.remaining;
    const budgetMessage: string = budget.message;

    console.log('   ✅ Type-safe budget operations:');
    console.log(`      • Over budget: ${isOverBudget}`);
    console.log(`      • Remaining: $${remainingBudget.toFixed(2)}`);
    console.log(`      • Status: ${budgetMessage}`);
  } catch (error) {
    console.log(`   ⚠️  Type-safe budget operations failed: ${error}`);
  }

  // Type-safe analytics with destructuring
  try {
    const {
      totalCost,
      totalTokens,
      averageCostPerToken,
      providerBreakdown,
      modelBreakdown,
    } = await circuitBreakerClient
      .analytics()
      .costAnalytics()
      .userId('typescript_user')
      .dateRange('2024-01-01', '2024-01-31')
      .get();

    console.log('   ✅ Type-safe analytics with destructuring:');
    console.log(`      • Total cost: $${totalCost.toFixed(2)}`);
    console.log(`      • Total tokens: ${totalTokens}`);
    console.log(`      • Avg cost/token: $${averageCostPerToken.toFixed(6)}`);
    console.log(`      • Provider breakdown entries: ${Object.keys(providerBreakdown).length}`);
    console.log(`      • Model breakdown entries: ${Object.keys(modelBreakdown).length}`);
  } catch (error) {
    console.log(`   ⚠️  Type-safe analytics failed: ${error}`);
  }

  console.log('\n🎉 Analytics Demo Complete!');
  console.log('============================');
  console.log('This demo showcased:');
  console.log('• Budget management for users and projects');
  console.log('• Real-time budget status monitoring');
  console.log('• Comprehensive cost analytics');
  console.log('• Convenience functions for common operations');
  console.log('• Advanced analytics scenarios and health checks');
  console.log('• TypeScript-specific type safety features');
  console.log('\nThe Analytics client provides powerful tools for:');
  console.log('• Cost control and budget management');
  console.log('• Usage optimization and monitoring');
  console.log('• Financial planning and reporting');
  console.log('• Multi-tenant cost tracking');
  console.log('• Type-safe analytics operations');
}

function displayBudgetStatus(status: BudgetStatus): void {
  console.log(`      • Budget ID: ${status.budgetId}`);
  console.log(`      • Limit: $${status.limit.toFixed(2)}`);
  console.log(`      • Used: $${status.used.toFixed(2)}`);
  console.log(`      • Percentage Used: ${status.percentageUsed.toFixed(1)}%`);
  console.log(`      • Remaining: $${status.remaining.toFixed(2)}`);
  console.log(`      • Is Warning: ${status.isWarning}`);
  console.log(`      • Is Exhausted: ${status.isExhausted}`);
  console.log(`      • Message: ${status.message}`);
}

function displayCostAnalytics(analytics: CostAnalytics): void {
  console.log(`      • Period: ${analytics.periodStart} to ${analytics.periodEnd}`);
  console.log(`      • Total Cost: $${analytics.totalCost.toFixed(2)}`);
  console.log(`      • Total Tokens: ${analytics.totalTokens}`);
  console.log(`      • Avg Cost/Token: $${analytics.averageCostPerToken.toFixed(6)}`);

  if (Object.keys(analytics.providerBreakdown).length > 0) {
    console.log('      • Provider Breakdown:');
    for (const [provider, cost] of Object.entries(analytics.providerBreakdown)) {
      console.log(`        - ${provider}: $${cost.toFixed(2)}`);
    }
  }

  if (Object.keys(analytics.modelBreakdown).length > 0) {
    console.log('      • Model Breakdown:');
    for (const [model, cost] of Object.entries(analytics.modelBreakdown)) {
      console.log(`        - ${model}: $${cost.toFixed(2)}`);
    }
  }

  if (Object.keys(analytics.dailyCosts).length > 0) {
    console.log('      • Daily Costs (last 5 days):');
    const dailyCosts = Object.entries(analytics.dailyCosts).sort(
      ([dateA], [dateB]) => dateA.localeCompare(dateB),
    );
    for (const [date, cost] of dailyCosts.reverse().slice(0, 5)) {
      console.log(`        - ${date}: $${cost.toFixed(2)}`);
    }
  }
}

// Run the demo
main().catch((error) => {
  console.error('Demo failed:', error);
  process.exit(1);
});
