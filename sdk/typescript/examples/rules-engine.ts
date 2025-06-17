/**
 * Rules Engine Example
 *
 * This example demonstrates comprehensive rule management using the
 * Circuit Breaker TypeScript SDK, including rule creation, evaluation,
 * composition, and advanced features.
 */

import {
  CircuitBreakerSDK,
  createRuleBuilder,
  createRuleTemplate,
  RulesEngine,
  RuleBuilder,
  RuleContext,
  CommonTemplates,
} from '../src/index.js';

// Initialize SDK
const sdk = new CircuitBreakerSDK({
  graphqlEndpoint: 'http://localhost:4000/graphql',
  debug: true,
});

// Sample context for rule evaluation
const sampleContext: RuleContext = {
  resource: {
    id: 'resource-001',
    workflowId: 'order-processing',
    state: 'pending',
    data: {
      orderId: 'ORD-123',
      customerId: 'CUST-456',
      totalAmount: 299.99,
      priority: 'high',
      items: [
        { productId: 'PROD-A', quantity: 2, price: 149.99 },
        { productId: 'PROD-B', quantity: 1, price: 149.99 }
      ]
    },
    metadata: {
      source: 'web-app',
      customerTier: 'gold',
      region: 'us-west'
    },
    history: [],
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  },
  activity: {
    id: 'validate_order',
    name: 'Validate Order',
    config: { timeout: 30000 }
  },
  timestamp: new Date(),
  metadata: {
    executionId: 'exec-789',
    sessionId: 'session-101'
  }
};

async function basicRuleOperations() {
  console.log('=== Basic Rule Operations ===');

  const rulesEngine = sdk.rules;

  try {
    // 1. Create a simple rule
    console.log('Creating a simple rule...');
    const stateRule = await rulesEngine.create({
      name: 'check-pending-state',
      type: 'simple',
      condition: "resource.state == 'pending'",
      description: 'Check if resource is in pending state',
      category: 'validation',
      priority: 10,
      enabled: true,
    });

    console.log('✅ Simple rule created:', {
      name: stateRule.name,
      type: stateRule.type,
      enabled: stateRule.enabled,
    });

    // 2. Create a JavaScript rule
    console.log('\nCreating a JavaScript rule...');
    const amountRule = await rulesEngine.create({
      name: 'high-value-order',
      type: 'javascript',
      condition: 'context.resource.data.totalAmount > 200',
      description: 'Check if order is high value (>$200)',
      category: 'business',
      priority: 20,
      enabled: true,
    });

    console.log('✅ JavaScript rule created:', {
      name: amountRule.name,
      condition: amountRule.condition,
    });

    // 3. Create a custom rule with evaluator
    console.log('\nCreating a custom rule...');
    const priorityRule = await rulesEngine.create({
      name: 'priority-customer',
      type: 'custom',
      description: 'Check if customer has priority status',
      category: 'customer',
      evaluator: async (context) => {
        const tier = context.resource.metadata.customerTier;
        const priority = context.resource.data.priority;
        return tier === 'gold' || tier === 'platinum' || priority === 'high';
      },
      enabled: true,
    });

    console.log('✅ Custom rule created:', {
      name: priorityRule.name,
      description: priorityRule.description,
    });

    // 4. Evaluate individual rules
    console.log('\nEvaluating rules...');

    const stateResult = await rulesEngine.evaluateRule('check-pending-state', sampleContext);
    console.log('State rule result:', {
      passed: stateResult.passed,
      evaluationTime: stateResult.evaluationTime,
    });

    const amountResult = await rulesEngine.evaluateRule('high-value-order', sampleContext);
    console.log('Amount rule result:', {
      passed: amountResult.passed,
      evaluationTime: amountResult.evaluationTime,
    });

    const priorityResult = await rulesEngine.evaluateRule('priority-customer', sampleContext);
    console.log('Priority rule result:', {
      passed: priorityResult.passed,
      evaluationTime: priorityResult.evaluationTime,
    });

    return [stateRule.name, amountRule.name, priorityRule.name];
  } catch (error) {
    console.error('❌ Error in basic operations:', error);
    throw error;
  }
}

async function compositeRules() {
  console.log('\n=== Composite Rules ===');

  const rulesEngine = sdk.rules;

  try {
    // Create individual rules for composition
    const rules = await Promise.all([
      rulesEngine.create({
        name: 'valid-customer',
        type: 'simple',
        condition: "resource.data.customerId != null",
        description: 'Check if customer ID is present',
        category: 'validation',
      }),
      rulesEngine.create({
        name: 'valid-amount',
        type: 'javascript',
        condition: 'context.resource.data.totalAmount > 0',
        description: 'Check if order amount is positive',
        category: 'validation',
      }),
      rulesEngine.create({
        name: 'us-region',
        type: 'simple',
        condition: "resource.metadata.region.startsWith('us-')",
        description: 'Check if order is from US region',
        category: 'location',
      }),
    ]);

    console.log('✅ Created component rules for composition');

    // Create AND composite rule
    console.log('\nCreating AND composite rule...');
    const andRule = await rulesEngine.create({
      name: 'valid-us-order',
      type: 'composite',
      operator: 'AND',
      rules: rules,
      description: 'Valid order from US region',
      category: 'composite',
      enabled: true,
    });

    console.log('✅ AND composite rule created:', {
      name: andRule.name,
      operator: (andRule as any).operator,
      childRules: (andRule as any).rules?.length,
    });

    // Create OR composite rule
    console.log('\nCreating OR composite rule...');
    const premiumRules = await Promise.all([
      rulesEngine.create({
        name: 'gold-customer',
        type: 'simple',
        condition: "resource.metadata.customerTier == 'gold'",
        description: 'Check if customer is gold tier',
        category: 'customer',
      }),
      rulesEngine.create({
        name: 'high-amount',
        type: 'javascript',
        condition: 'context.resource.data.totalAmount >= 500',
        description: 'Check if order amount is >= $500',
        category: 'business',
      }),
    ]);

    const orRule = await rulesEngine.create({
      name: 'premium-order',
      type: 'composite',
      operator: 'OR',
      rules: premiumRules,
      description: 'Premium order (gold customer OR high amount)',
      category: 'composite',
      enabled: true,
    });

    console.log('✅ OR composite rule created');

    // Evaluate composite rules
    console.log('\nEvaluating composite rules...');

    const andResult = await rulesEngine.evaluateRule('valid-us-order', sampleContext);
    console.log('AND rule result:', {
      passed: andResult.passed,
      rulesEvaluated: andResult.rulesEvaluated,
      rulesPassed: andResult.rulesPassed,
    });

    const orResult = await rulesEngine.evaluateRule('premium-order', sampleContext);
    console.log('OR rule result:', {
      passed: orResult.passed,
      rulesEvaluated: orResult.rulesEvaluated,
      rulesPassed: orResult.rulesPassed,
    });

    return [andRule.name, orRule.name];
  } catch (error) {
    console.error('❌ Error in composite rules:', error);
    throw error;
  }
}

async function ruleBuilderExample() {
  console.log('\n=== Rule Builder Example ===');

  try {
    const rulesEngine = sdk.rules;

    // Create a rule builder with default options
    const builder = createRuleBuilder(rulesEngine, {
      validate: true,
      defaultCategory: 'auto-generated',
      defaultPriority: 50,
      autoEnable: true,
      defaultMetadata: {
        generatedBy: 'rule-builder',
        version: '1.0',
      },
    });

    // Using fluent interface for simple rule
    console.log('Creating rule with fluent interface...');
    const fluentRule = await builder
      .simple('fluent-validation')
      .condition("resource.data.orderId.startsWith('ORD-')")
      .description('Validate order ID format using fluent interface')
      .category('format-validation')
      .priority(100)
      .metadata({ pattern: 'ORD-*' })
      .create();

    console.log('✅ Fluent rule created:', {
      name: fluentRule.name,
      condition: fluentRule.condition,
      priority: fluentRule.priority,
    });

    // Using JavaScript rule builder
    console.log('\nCreating JavaScript rule with builder...');
    const jsRule = await builder
      .javascript('complex-validation')
      .condition(`
        const { resource } = context;
        const isValidCustomer = resource.data.customerId && resource.data.customerId.length > 0;
        const isValidAmount = resource.data.totalAmount > 0;
        const hasItems = resource.data.items && resource.data.items.length > 0;
        return isValidCustomer && isValidAmount && hasItems;
      `)
      .description('Complex order validation with JavaScript')
      .category('business-logic')
      .create();

    console.log('✅ JavaScript rule created:', {
      name: jsRule.name,
      type: jsRule.type,
    });

    // Using custom rule builder
    console.log('\nCreating custom rule with builder...');
    const customRule = await builder
      .custom('advanced-customer-check')
      .evaluator(async (context) => {
        const customer = context.resource.data.customerId;
        const tier = context.resource.metadata.customerTier;
        const amount = context.resource.data.totalAmount;

        // Simulate complex business logic
        if (tier === 'platinum') return true;
        if (tier === 'gold' && amount > 100) return true;
        if (tier === 'silver' && amount > 200) return true;
        if (!tier && amount > 500) return true;

        return false;
      })
      .description('Advanced customer eligibility check')
      .category('customer-logic')
      .create();

    console.log('✅ Custom rule created:', {
      name: customRule.name,
      type: customRule.type,
    });

    // Test rules before evaluation
    console.log('\nTesting rules...');
    const testResult = await builder
      .simple('test-rule')
      .condition("resource.data.totalAmount > 100")
      .build()
      .test(sampleContext);

    console.log('Test result:', {
      passed: testResult.passed,
      errors: testResult.errors,
    });

    return [fluentRule.name, jsRule.name, customRule.name];
  } catch (error) {
    console.error('❌ Error in rule builder example:', error);
    throw error;
  }
}

async function templatesAndBatch() {
  console.log('\n=== Templates and Batch Operations ===');

  try {
    const rulesEngine = sdk.rules;
    const builder = createRuleBuilder(rulesEngine);

    // Register templates
    console.log('Registering rule templates...');

    const stateTemplate = createRuleTemplate('state-check', 'simple', {
      condition: "resource.state == '{{state}}'",
      description: 'Check if resource is in specific state',
      parameters: ['state'],
      category: 'state-validation',
    });

    const amountTemplate = createRuleTemplate('amount-threshold', 'javascript', {
      condition: 'context.resource.data.totalAmount {{operator}} {{threshold}}',
      description: 'Check amount against threshold',
      parameters: ['operator', 'threshold'],
      category: 'amount-validation',
    });

    builder.registerTemplate(stateTemplate);
    builder.registerTemplate(amountTemplate);

    // Create rules from templates
    console.log('\nCreating rules from templates...');

    const pendingStateRule = await builder.fromTemplate(
      'state-check',
      'check-pending-state-template',
      { state: 'pending' }
    ).create();

    const highAmountRule = await builder.fromTemplate(
      'amount-threshold',
      'check-high-amount-template',
      { operator: '>', threshold: '1000' }
    ).create();

    console.log('✅ Rules created from templates:', {
      stateRule: pendingStateRule.name,
      amountRule: highAmountRule.name,
    });

    // Batch rule creation
    console.log('\nCreating rules in batch...');

    const batchRules = await builder.createBatch([
      {
        name: 'batch-rule-1',
        type: 'simple',
        condition: "resource.metadata.source == 'web-app'",
      },
      {
        name: 'batch-rule-2',
        type: 'simple',
        condition: "resource.metadata.source == 'mobile-app'",
      },
      {
        name: 'batch-rule-3',
        type: 'javascript',
        condition: 'context.resource.data.items.length > 1',
      },
    ], {
      category: 'batch-created',
      description: 'Rule created in batch operation',
      enabled: true,
    }).create();

    console.log('✅ Batch rules created:', {
      count: batchRules.length,
      names: batchRules.map(r => r.name),
    });

    // Multiple rule evaluation
    console.log('\nEvaluating multiple rules...');

    const ruleNames = [
      pendingStateRule.name,
      highAmountRule.name,
      ...batchRules.map(r => r.name),
    ];

    const multiResult = await rulesEngine.evaluateRules(ruleNames, sampleContext);
    console.log('Multiple rules evaluation:', {
      passed: multiResult.passed,
      rulesEvaluated: multiResult.rulesEvaluated,
      rulesPassed: multiResult.rulesPassed,
      evaluationTime: multiResult.evaluationTime,
    });

    return batchRules.map(r => r.name);
  } catch (error) {
    console.error('❌ Error in templates and batch:', error);
    throw error;
  }
}

async function conditionalGroupsAndChains() {
  console.log('\n=== Conditional Groups and Chains ===');

  try {
    const rulesEngine = sdk.rules;
    const builder = createRuleBuilder(rulesEngine);

    // Create rules for grouping
    const groupRules = await Promise.all([
      rulesEngine.create({
        name: 'group-rule-1',
        type: 'simple',
        condition: "resource.data.priority == 'high'",
        description: 'High priority check',
        category: 'priority',
      }),
      rulesEngine.create({
        name: 'group-rule-2',
        type: 'simple',
        condition: "resource.metadata.customerTier == 'gold'",
        description: 'Gold customer check',
        category: 'customer',
      }),
      rulesEngine.create({
        name: 'group-rule-3',
        type: 'javascript',
        condition: 'context.resource.data.totalAmount > 500',
        description: 'High value check',
        category: 'amount',
      }),
    ]);

    // Create conditional group
    console.log('Creating conditional group...');

    const conditionalGroup = builder
      .conditionalGroup('high-priority-group')
      .addRules(groupRules)
      .enableWhen(async (context) => {
        // Only enable this group during business hours (simplified)
        const hour = new Date().getHours();
        return hour >= 9 && hour <= 17;
      })
      .metadata({ type: 'business-hours-only' })
      .register();

    // Evaluate conditional group
    const groupResult = await builder.evaluateConditionalGroup(
      'high-priority-group',
      sampleContext
    );

    if (groupResult) {
      console.log('✅ Conditional group evaluated:', {
        passed: groupResult.passed,
        rulesEvaluated: groupResult.rulesEvaluated,
      });
    } else {
      console.log('ℹ️ Conditional group was not enabled');
    }

    // Create rule chain
    console.log('\nCreating rule chain...');

    const chainRules = await Promise.all([
      rulesEngine.create({
        name: 'chain-step-1',
        type: 'simple',
        condition: "resource.data.customerId != null",
        description: 'Validate customer exists',
        category: 'validation',
      }),
      rulesEngine.create({
        name: 'chain-step-2',
        type: 'javascript',
        condition: 'context.resource.data.totalAmount > 0',
        description: 'Validate positive amount',
        category: 'validation',
      }),
      rulesEngine.create({
        name: 'chain-step-3',
        type: 'simple',
        condition: "resource.data.items.length > 0",
        description: 'Validate has items',
        category: 'validation',
      }),
    ]);

    const chain = builder
      .chain('validation-chain')
      .addRules(chainRules)
      .stopOnFailure(true)
      .transformContext((context, ruleIndex) => {
        // Add step information to context
        return {
          ...context,
          metadata: {
            ...context.metadata,
            validationStep: ruleIndex + 1,
          },
        };
      })
      .register();

    // Execute rule chain
    const chainResult = await builder.executeChain('validation-chain', sampleContext);
    console.log('✅ Rule chain executed:', {
      passed: chainResult.passed,
      rulesEvaluated: chainResult.rulesEvaluated,
      evaluationTime: chainResult.evaluationTime,
    });

    return [...groupRules.map(r => r.name), ...chainRules.map(r => r.name)];
  } catch (error) {
    console.error('❌ Error in conditional groups and chains:', error);
    throw error;
  }
}

async function analyticsAndMonitoring() {
  console.log('\n=== Analytics and Monitoring ===');

  const rulesEngine = sdk.rules;

  try {
    // Search for rules
    console.log('Searching for rules...');
    const searchResults = await rulesEngine.search({
      category: 'validation',
      enabled: true,
      includeStats: true,
      limit: 10,
    });

    console.log('✅ Rules found:', {
      count: searchResults.items.length,
      totalCount: searchResults.totalCount,
      categories: [...new Set(searchResults.items.map(r => r.category))],
    });

    // Get rule statistics
    console.log('\nGetting rule statistics...');
    const stats = await rulesEngine.getStats();
    console.log('✅ Rule statistics:', {
      totalRules: stats.totalRules,
      enabled: stats.enabled,
      disabled: stats.disabled,
      byType: stats.byType,
      averageEvaluationTime: stats.averageEvaluationTime,
    });

    // Get health status
    console.log('\nChecking rule health...');
    const health = await rulesEngine.getHealth();
    console.log('✅ Rule health:', {
      healthy: health.healthy,
      issues: health.issues.length,
      failingRules: health.failingRules,
      errorRate: health.errorRate,
    });

    // Validate a rule
    console.log('\nValidating rule definition...');
    const validationResult = await rulesEngine.validateRule({
      name: 'test-validation',
      type: 'javascript',
      condition: 'context.resource.data.amount > 0',
      description: 'Test rule for validation',
    });

    console.log('✅ Rule validation:', {
      valid: validationResult.valid,
      errors: validationResult.errors,
      warnings: validationResult.warnings,
    });

  } catch (error) {
    console.error('❌ Error in analytics and monitoring:', error);
    throw error;
  }
}

async function cleanupRules(ruleNames: string[]) {
  console.log('\n=== Cleanup ===');

  const rulesEngine = sdk.rules;

  try {
    for (const ruleName of ruleNames) {
      try {
        await rulesEngine.delete(ruleName, { force: true });
        console.log(`✅ Deleted rule: ${ruleName}`);
      } catch (error) {
        console.warn(`⚠️ Could not delete rule ${ruleName}:`, error);
      }
    }
  } catch (error) {
    console.warn('⚠️ Cleanup errors:', error);
  }
}

async function runAllExamples() {
  console.log('🚀 Starting Rules Engine Examples\n');

  const allRuleNames: string[] = [];

  try {
    // Initialize the SDK
    await sdk.initialize();
    console.log('✅ SDK initialized\n');

    // Run all examples
    const basicRules = await basicRuleOperations();
    allRuleNames.push(...basicRules);

    const compositeRules = await compositeRules();
    allRuleNames.push(...compositeRules);

    const builderRules = await ruleBuilderExample();
    allRuleNames.push(...builderRules);

    const batchRules = await templatesAndBatch();
    allRuleNames.push(...batchRules);

    const groupRules = await conditionalGroupsAndChains();
    allRuleNames.push(...groupRules);

    await analyticsAndMonitoring();

    console.log('\n🎉 All examples completed successfully!');

  } catch (error) {
    console.error('💥 Example execution failed:', error);
    throw error;
  } finally {
    // Clean up created rules
    await cleanupRules(allRuleNames);

    // Dispose of SDK resources
    await sdk.dispose();
    console.log('👋 SDK disposed');
  }
}

// Export for use in other examples or tests
export {
  basicRuleOperations,
  compositeRules,
  ruleBuilderExample,
  templatesAndBatch,
  conditionalGroupsAndChains,
  analyticsAndMonitoring,
  runAllExamples,
};

// Run examples if this file is executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  runAllExamples().catch(console.error);
}
