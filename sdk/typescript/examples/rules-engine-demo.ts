#!/usr/bin/env tsx
/**
 * Rules Engine Demo - Circuit Breaker TypeScript SDK
 *
 * This example demonstrates advanced rules engine capabilities:
 * - Creating complex rule conditions
 * - Rule composition and evaluation
 * - Context-based rule processing
 * - Rule templates and builders
 * - Conditional workflow activities
 *
 * Run with: npx tsx examples/rules-engine-demo.ts
 */

/// <reference types="node" />

import {
  CircuitBreakerSDK,
  createRuleBuilder,
  createRuleTemplate,
  RulesEngine,
  RuleBuilder,
  RuleContext,
  CommonTemplates,
  Rule,
  RuleType,
  CompositeRule,
  RuleEvaluationResult,
  RuleCreateInput,
  RuleEvaluationOptions,
  CircuitBreakerError,
  RuleError,
  formatError,
  generateRequestId,
} from "../src/index.js";

// ============================================================================
// Configuration
// ============================================================================

const config = {
  graphqlEndpoint:
    process.env.CIRCUIT_BREAKER_ENDPOINT || "http://localhost:4000/graphql",
  timeout: 30000,
  debug: process.env.NODE_ENV === "development",
  logging: {
    level: "info" as const,
    structured: false,
  },
  headers: {
    "User-Agent": "CircuitBreaker-SDK-RulesDemo/0.1.0",
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
// Sample Data and Contexts
// ============================================================================

const sampleOrderContext: RuleContext = {
  resource: {
    id: "order-001",
    workflowId: "order-processing",
    state: "pending_approval",
    data: {
      orderId: "ORD-12345",
      customerId: "CUST-789",
      amount: 1250.0,
      currency: "USD",
      items: [
        { sku: "LAPTOP-001", price: 999.99, quantity: 1 },
        { sku: "MOUSE-001", price: 29.99, quantity: 2 },
        { sku: "KEYBOARD-001", price: 189.99, quantity: 1 },
      ],
      customerTier: "premium",
      region: "US-West",
      discountCode: "PREMIUM10",
    },
    metadata: {
      channel: "web",
      source: "marketing_campaign",
      priority: "high",
    },
    history: [],
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  },
  activity: {
    id: "approve_order",
    name: "Approve Order",
    fromStates: ["pending_approval"],
    toState: "approved",
    conditions: [],
  },
  metadata: {
    userId: "user-123",
    userRole: "customer",
    permissions: ["place_order", "view_order"],
    requestId: generateRequestId(),
  },
  timestamp: new Date(),
};

const sampleUserContext: RuleContext = {
  resource: {
    id: "user-profile-456",
    workflowId: "user-onboarding",
    state: "verification_pending",
    data: {
      userId: "USER-456",
      email: "jane.doe@example.com",
      age: 28,
      country: "US",
      accountType: "business",
      verificationLevel: "basic",
      creditScore: 750,
      monthlyIncome: 8500,
    },
    metadata: {
      registrationDate: "2024-01-15",
      lastLogin: "2024-01-20",
      deviceType: "desktop",
    },
    history: [],
    createdAt: "2024-01-15T00:00:00Z",
    updatedAt: new Date().toISOString(),
  },
  activity: {
    id: "verify_user",
    name: "Verify User",
    fromStates: ["verification_pending"],
    toState: "verified",
    conditions: [],
  },
  metadata: {
    userId: "USER-456",
    userRole: "user",
    permissions: ["edit_profile", "verify_account"],
    requestId: generateRequestId(),
  },
  timestamp: new Date(),
};

// ============================================================================
// Rule Factories
// ============================================================================

function createOrderApprovalRules(): RuleCreateInput[] {
  return [
    {
      name: "High Value Order",
      description: "Orders over $1000 require additional approval",
      type: "simple" as RuleType,
      condition: "context.resource.data.amount > 1000",
      metadata: { tags: ["order", "approval", "high-value"] },
    },
    {
      name: "Premium Customer Benefits",
      description: "Premium customers get expedited processing",
      type: "simple" as RuleType,
      condition: "context.resource.data.customerTier === 'premium'",
      metadata: { tags: ["customer", "premium", "benefits"] },
    },
    {
      name: "International Order Check",
      description: "Orders from certain regions need extra validation",
      type: "simple" as RuleType,
      condition:
        "['EU', 'APAC', 'International'].includes(context.resource.data.region)",
      metadata: { tags: ["region", "international", "validation"] },
    },
    {
      name: "Bulk Order Processing",
      description: "Orders with many items get bulk processing",
      type: "javascript" as RuleType,
      condition: `
        const itemCount = context.resource.data.items?.length || 0;
        return {
          passed: itemCount >= 5,
          reason: itemCount >= 5
            ? \`Bulk order with \${itemCount} items\`
            : \`Regular order with \${itemCount} items\`,
          data: { itemCount, isBulk: itemCount >= 5 }
        };
      `,
      metadata: { tags: ["bulk", "items", "processing"] },
    },
  ];
}

function createUserVerificationRules(): RuleCreateInput[] {
  return [
    {
      name: "Age Verification",
      description: "Users must be 18 or older",
      type: "simple" as RuleType,
      condition: "context.resource.data.age >= 18",
      metadata: { tags: ["age", "verification", "compliance"] },
    },
    {
      name: "Business Account Eligibility",
      description: "Business accounts require higher income threshold",
      type: "composite" as RuleType,
      operator: "AND",
      condition:
        "context.resource.data.accountType === 'business' && context.resource.data.monthlyIncome > 5000 && context.resource.data.creditScore > 650",
      metadata: { tags: ["business", "eligibility", "income"] },
    },
    {
      name: "Geographic Compliance",
      description: "Ensure compliance with regional regulations",
      type: "javascript" as RuleType,
      condition: `
        const country = context.resource.data.country;
        const age = context.resource.data.age;
        const accountType = context.resource.data.accountType;

        const regulations = {
          'US': { minAge: 18, businessMinAge: 21 },
          'EU': { minAge: 16, businessMinAge: 18 },
          'UK': { minAge: 16, businessMinAge: 18 }
        };

        const rules = regulations[country] || { minAge: 21, businessMinAge: 25 };
        const requiredAge = accountType === 'business' ? rules.businessMinAge : rules.minAge;

        return {
          passed: age >= requiredAge,
          reason: \`\${country} requires \${requiredAge}+ for \${accountType} accounts\`,
          data: { country, requiredAge, actualAge: age, accountType }
        };
      `,
      metadata: { tags: ["geographic", "compliance", "regulations"] },
    },
  ];
}

// ============================================================================
// Demo Functions
// ============================================================================

async function demonstrateBasicRules(sdk: CircuitBreakerSDK): Promise<void> {
  logInfo("\n🔧 Basic Rules Demonstration");
  console.log("=".repeat(50));

  // Create order approval rules
  const orderRules = createOrderApprovalRules();
  const createdOrderRules: string[] = [];

  for (const ruleInput of orderRules) {
    logInfo(`Creating rule: ${ruleInput.name}`);
    const ruleId = await sdk.rules.create(ruleInput);
    createdOrderRules.push(ruleId);
    logSuccess(`Rule created with ID: ${ruleId}`);
  }

  // Evaluate rules against sample order
  logInfo("\n📊 Evaluating rules against sample order...");
  for (const ruleId of createdOrderRules) {
    const result = await sdk.rules.evaluate(ruleId, sampleOrderContext);
    logInfo(`Rule ${ruleId}:`, {
      passed: result.passed,
      reason: result.reason,
      executionTime: result.executionTime,
    });
  }

  return;
}

async function demonstrateCompositeRules(
  sdk: CircuitBreakerSDK,
): Promise<void> {
  logInfo("\n🔗 Composite Rules Demonstration");
  console.log("=".repeat(50));

  // Create user verification rules
  const userRules = createUserVerificationRules();
  const createdUserRules: string[] = [];

  for (const ruleInput of userRules) {
    logInfo(`Creating rule: ${ruleInput.name}`);
    const ruleId = await sdk.rules.create(ruleInput);
    createdUserRules.push(ruleId);
    logSuccess(`Rule created with ID: ${ruleId}`);
  }

  // Evaluate composite rules
  logInfo("\n📊 Evaluating composite rules against sample user...");
  for (const ruleId of createdUserRules) {
    const result = await sdk.rules.evaluate(ruleId, sampleUserContext);
    logInfo(`Rule ${ruleId}:`, {
      passed: result.passed,
      reason: result.reason,
      data: result.data,
    });
  }

  return;
}

async function demonstrateRuleBuilder(sdk: CircuitBreakerSDK): Promise<void> {
  logInfo("\n🏗️  Rule Builder Demonstration");
  console.log("=".repeat(50));

  // Create a rule manually since the builder API differs
  const pricingRule: RuleCreateInput = {
    name: "Dynamic Pricing Rule",
    description: "Calculate dynamic pricing based on multiple factors",
    type: "javascript" as RuleType,
    condition: `
      const { amount, customerTier, region, discountCode } = context.resource.data;

      let finalPrice = amount;
      let appliedDiscounts = [];

      // Premium customer discount
      if (customerTier === 'premium') {
        finalPrice *= 0.95; // 5% discount
        appliedDiscounts.push('Premium Customer: 5%');
      }

      // Regional pricing
      const regionalMultipliers = {
        'US-West': 1.0,
        'US-East': 1.05,
        'EU': 1.15,
        'APAC': 1.10
      };

      const multiplier = regionalMultipliers[region] || 1.0;
      finalPrice *= multiplier;
      if (multiplier !== 1.0) {
        appliedDiscounts.push(\`Regional: \${((multiplier - 1) * 100).toFixed(1)}%\`);
      }

      // Discount code
      if (discountCode === 'PREMIUM10') {
        finalPrice *= 0.9; // 10% discount
        appliedDiscounts.push('Discount Code: 10%');
      }

      const discountAmount = amount - finalPrice;

      return {
        passed: true,
        reason: \`Dynamic pricing calculated: $\${finalPrice.toFixed(2)}\`,
        data: {
          originalPrice: amount,
          finalPrice: finalPrice.toFixed(2),
          discountAmount: discountAmount.toFixed(2),
          appliedDiscounts,
          savingsPercent: ((discountAmount / amount) * 100).toFixed(1)
        }
      };
    `,
    metadata: { tags: ["pricing", "dynamic", "discounts"] },
  };

  const pricingRuleId = await sdk.rules.create(pricingRule);
  logSuccess(`Pricing rule created with ID: ${pricingRuleId}`);

  // Evaluate the pricing rule
  const pricingResult = await sdk.rules.evaluate(
    pricingRuleId,
    sampleOrderContext,
  );
  logSuccess("Dynamic pricing calculation:", pricingResult.data);

  return;
}

async function demonstrateRuleTemplates(sdk: CircuitBreakerSDK): Promise<void> {
  logInfo("\n📋 Rule Templates Demonstration");
  console.log("=".repeat(50));

  // Create rules using manual definitions since template API differs
  const validationRule: RuleCreateInput = {
    name: "Order Data Validation",
    description: "Validate order data fields",
    type: "javascript" as RuleType,
    condition: `
      const { orderId, customerId, amount, items } = context.resource.data;
      const errors = [];

      if (!orderId || !/^ORD-\\d+$/.test(orderId)) errors.push('Invalid order ID');
      if (!customerId || !/^CUST-\\d+$/.test(customerId)) errors.push('Invalid customer ID');
      if (!amount || amount < 0.01) errors.push('Invalid amount');
      if (!items || items.length === 0) errors.push('No items in order');

      return {
        passed: errors.length === 0,
        reason: errors.length === 0 ? 'Validation passed' : errors.join(', '),
        data: { errors }
      };
    `,
    metadata: { tags: ["validation", "data-integrity"] },
  };

  const thresholdRule: RuleCreateInput = {
    name: "High Value Threshold",
    description: "Check if order value exceeds threshold",
    type: "simple" as RuleType,
    condition: "context.resource.data.amount > 500",
    metadata: { tags: ["threshold", "high-value"] },
  };

  // Create the rules
  const validationRuleId = await sdk.rules.create(validationRule);
  logSuccess(`Validation rule created with ID: ${validationRuleId}`);

  const thresholdRuleId = await sdk.rules.create(thresholdRule);
  logSuccess(`Threshold rule created with ID: ${thresholdRuleId}`);

  // Evaluate template-based rules
  logInfo("\n📊 Evaluating template-based rules...");

  const validationResult = await sdk.rules.evaluate(
    validationRuleId,
    sampleOrderContext,
  );
  logInfo("Validation rule result:", {
    passed: validationResult.passed,
    reason: validationResult.reason,
  });

  const thresholdResult = await sdk.rules.evaluate(
    thresholdRuleId,
    sampleOrderContext,
  );
  logInfo("Threshold rule result:", {
    passed: thresholdResult.passed,
    reason: thresholdResult.reason,
  });

  return;
}

async function demonstrateBatchEvaluation(
  sdk: CircuitBreakerSDK,
): Promise<void> {
  logInfo("\n📦 Batch Rule Evaluation Demonstration");
  console.log("=".repeat(50));

  // Get all rules
  const allRules = await sdk.rules.list();
  const ruleIds = allRules.map((rule: any) => rule.id).slice(0, 5); // Limit to first 5 rules

  if (ruleIds.length === 0) {
    logWarning("No rules available for batch evaluation");
    return;
  }

  // Batch evaluate multiple rules
  logInfo(`Batch evaluating ${ruleIds.length} rules...`);
  const batchResults = await sdk.rules.evaluateBatch({
    ruleIds,
    context: sampleOrderContext,
    options: {
      stopOnFirstFailure: false,
      includeExecutionDetails: true,
    },
  });

  logSuccess(`Batch evaluation completed. Results:`);
  batchResults.results.forEach((result: any, index: number) => {
    logInfo(`Rule ${index + 1}:`, {
      ruleId: result.ruleId,
      passed: result.passed,
      reason: result.reason,
      executionTime: result.executionTime,
    });
  });

  logInfo("Batch statistics:", {
    totalRules: batchResults.totalRules,
    passedRules: batchResults.passedRules,
    failedRules: batchResults.failedRules,
    totalExecutionTime: batchResults.totalExecutionTime,
  });

  return;
}

async function demonstrateRuleManagement(
  sdk: CircuitBreakerSDK,
): Promise<void> {
  logInfo("\n🔧 Rule Management Demonstration");
  console.log("=".repeat(50));

  // List all rules with search
  const searchResults = await sdk.rules.search({
    tags: ["order"],
    type: "simple",
    limit: 10,
  });

  logInfo(`Found ${searchResults.length} rules matching search criteria`);

  if (searchResults.length > 0) {
    const rule = searchResults[0];

    // Update rule
    await sdk.rules.update(rule.id, {
      description: "Updated description - " + new Date().toISOString(),
      tags: [...(rule.tags || []), "updated"],
    });
    logSuccess(`Updated rule: ${rule.id}`);

    // Get rule statistics
    const stats = await sdk.rules.getStats(rule.id);
    logInfo("Rule statistics:", {
      totalEvaluations: stats.totalEvaluations,
      successRate: stats.successRate,
      averageExecutionTime: stats.averageExecutionTime,
      lastEvaluation: stats.lastEvaluation,
    });

    // Get rule health
    const health = await sdk.rules.getHealth(rule.id);
    logInfo("Rule health:", {
      status: health.status,
      isHealthy: health.isHealthy,
      lastHealthCheck: health.lastHealthCheck,
    });
  }

  return;
}

// ============================================================================
// Main Demo Function
// ============================================================================

async function runRulesEngineDemo(): Promise<void> {
  console.log("🚀 Starting Rules Engine Demo");
  console.log("==============================\n");

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

    // Run demonstrations
    await demonstrateBasicRules(sdk);
    await demonstrateCompositeRules(sdk);
    await demonstrateRuleBuilder(sdk);
    await demonstrateRuleTemplates(sdk);
    await demonstrateBatchEvaluation(sdk);
    await demonstrateRuleManagement(sdk);

    // Final summary
    logInfo("\n📊 Demo Summary");
    console.log("=".repeat(50));

    const allRules = await sdk.rules.list();
    const rulesByType = allRules.reduce(
      (acc: Record<string, number>, rule: any) => {
        acc[rule.type] = (acc[rule.type] || 0) + 1;
        return acc;
      },
      {} as Record<string, number>,
    );

    logInfo("Rules by type:", rulesByType);

    console.log("\n✨ Rules Engine Demo completed successfully!");
    console.log("==========================================");
  } catch (error) {
    logError("Rules engine demo failed", error);
    process.exit(1);
  }
}

// ============================================================================
// Run Demo
// ============================================================================

if (import.meta.url === `file://${process.argv[1]}`) {
  runRulesEngineDemo()
    .then(() => {
      logSuccess("Demo completed successfully");
      process.exit(0);
    })
    .catch((error) => {
      logError("Demo failed", error);
      process.exit(1);
    });
}

export { runRulesEngineDemo };
