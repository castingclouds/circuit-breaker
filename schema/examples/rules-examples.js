const { GraphQLClient } = require("graphql-request");
const { loadSchemaSync } = require("@graphql-tools/load");
const { GraphQLFileLoader } = require("@graphql-tools/graphql-file-loader");
const fs = require("fs");
const path = require("path");

// Load the rules schema
const rulesSchema = loadSchemaSync(
  path.join(__dirname, "../rules.graphql"),
  {
    loaders: [new GraphQLFileLoader()],
  },
);

// Load GraphQL operations
const operationsFile = path.join(
  __dirname,
  "../operations/rules.graphql",
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
 * Rules Examples
 * These examples demonstrate how to use the rules engine operations
 * defined in ../rules.graphql
 */

// ============================================================================
// QUERY EXAMPLES
// ============================================================================

/**
 * Get a specific rule by ID
 */
async function getRule(ruleId) {
  const query = operationMap.GetRule;

  try {
    const data = await client.request(query, { ruleId });
    console.log("Rule details:", JSON.stringify(data, null, 2));
    return data.rule;
  } catch (error) {
    console.error("Error fetching rule:", error);
    throw error;
  }
}

/**
 * List all rules
 */
async function listRules(tags = null) {
  const query = operationMap.ListRules;

  try {
    const data = await client.request(query, { tags });
    console.log("All rules:", JSON.stringify(data, null, 2));
    return data.rules;
  } catch (error) {
    console.error("Error listing rules:", error);
    throw error;
  }
}

/**
 * Get rules for a specific workflow
 */
async function getWorkflowRules(workflowId) {
  const query = operationMap.GetWorkflowRules;

  try {
    const data = await client.request(query, { workflowId });
    console.log("Workflow rules:", JSON.stringify(data, null, 2));
    return data.workflowRules;
  } catch (error) {
    console.error("Error fetching workflow rules:", error);
    throw error;
  }
}

// ============================================================================
// MUTATION EXAMPLES
// ============================================================================

/**
 * Create a document validation rule
 */
async function createDocumentValidationRule() {
  const mutation = operationMap.CreateRule;

  const ruleInput = {
    name: "Document Validation Rule",
    description: "Validates that documents meet quality standards",
    condition: {
      conditionType: "and",
      rules: [
        {
          conditionType: "equals",
          field: "data.document.type",
          value: "annual_report",
        },
        {
          conditionType: "greater_than",
          field: "metadata.fileSize",
          value: 1000,
        },
        {
          conditionType: "contains",
          field: "data.document.content",
          substring: "financial summary",
        },
      ],
    },
    tags: ["validation", "document", "quality"],
  };

  try {
    const data = await client.request(mutation, { input: ruleInput });
    console.log("Created document validation rule:", JSON.stringify(data, null, 2));
    return data.createRule;
  } catch (error) {
    console.error("Error creating rule:", error);
    throw error;
  }
}

/**
 * Create a priority escalation rule
 */
async function createPriorityEscalationRule() {
  const mutation = operationMap.CreateRule;

  const ruleInput = {
    name: "Priority Escalation Rule",
    description: "Escalates high-priority documents that have been waiting too long",
    condition: {
      conditionType: "and",
      rules: [
        {
          conditionType: "equals",
          field: "metadata.priority",
          value: "high",
        },
        {
          conditionType: "script",
          script: `
            const waitingTime = Date.now() - new Date(data.createdAt).getTime();
            const hoursSinceCreated = waitingTime / (1000 * 60 * 60);
            return hoursSinceCreated > 24;
          `,
        },
      ],
    },
    tags: ["escalation", "priority", "sla"],
  };

  try {
    const data = await client.request(mutation, { input: ruleInput });
    console.log("Created priority escalation rule:", JSON.stringify(data, null, 2));
    return data.createRule;
  } catch (error) {
    console.error("Error creating escalation rule:", error);
    throw error;
  }
}

/**
 * Create a budget threshold rule
 */
async function createBudgetThresholdRule() {
  const mutation = operationMap.CreateRule;

  const ruleInput = {
    name: "Budget Threshold Alert",
    description: "Alerts when budget usage exceeds threshold",
    condition: {
      conditionType: "or",
      rules: [
        {
          conditionType: "greater_than",
          field: "budget.percentageUsed",
          value: 0.8,
        },
        {
          conditionType: "greater_than",
          field: "budget.used",
          value: 1000,
        },
      ],
    },
    tags: ["budget", "alert", "threshold"],
  };

  try {
    const data = await client.request(mutation, { input: ruleInput });
    console.log("Created budget threshold rule:", JSON.stringify(data, null, 2));
    return data.createRule;
  } catch (error) {
    console.error("Error creating budget rule:", error);
    throw error;
  }
}

/**
 * Update an existing rule
 */
async function updateRule(ruleId, updates) {
  const mutation = operationMap.UpdateRule;

  const updateInput = {
    name: updates.name,
    description: updates.description,
    condition: updates.condition,
    tags: updates.tags,
  };

  try {
    const data = await client.request(mutation, {
      id: ruleId,
      input: updateInput
    });
    console.log("Updated rule:", JSON.stringify(data, null, 2));
    return data.updateRule;
  } catch (error) {
    console.error("Error updating rule:", error);
    throw error;
  }
}

/**
 * Delete a rule
 */
async function deleteRule(ruleId) {
  const mutation = operationMap.DeleteRule;

  try {
    const data = await client.request(mutation, { id: ruleId });
    console.log("Rule deleted:", data.deleteRule);
    return data.deleteRule;
  } catch (error) {
    console.error("Error deleting rule:", error);
    throw error;
  }
}

/**
 * Evaluate a rule against test data
 */
async function evaluateRule(ruleId, testData, metadata = {}) {
  const mutation = operationMap.EvaluateRule;

  const evaluationInput = {
    ruleId: ruleId,
    data: testData,
    metadata: metadata,
  };

  try {
    const data = await client.request(mutation, { input: evaluationInput });
    console.log("Rule evaluation result:", JSON.stringify(data, null, 2));
    return data.evaluateRule;
  } catch (error) {
    console.error("Error evaluating rule:", error);
    throw error;
  }
}

// ============================================================================
// COMPLETE RULES EXAMPLE
// ============================================================================

/**
 * Complete example demonstrating rules engine lifecycle
 */
async function completeRulesExample() {
  console.log("\n=== Complete Rules Example ===\n");

  try {
    // 1. Create validation rule
    console.log("1. Creating document validation rule...");
    const validationRule = await createDocumentValidationRule();

    // 2. Create escalation rule
    console.log("\n2. Creating priority escalation rule...");
    const escalationRule = await createPriorityEscalationRule();

    // 3. Create budget rule
    console.log("\n3. Creating budget threshold rule...");
    const budgetRule = await createBudgetThresholdRule();

    // 4. List all rules
    console.log("\n4. Listing all rules...");
    await listRules();

    // 5. List rules by tag
    console.log("\n5. Listing validation rules...");
    await listRules(["validation"]);

    // 6. Test rule evaluation - document that passes validation
    console.log("\n6. Testing validation rule with valid document...");
    const validDocument = {
      document: {
        type: "annual_report",
        content: "This annual report contains a comprehensive financial summary of our operations...",
      },
      metadata: {
        fileSize: 2048576,
      },
    };
    await evaluateRule(validationRule.id, validDocument);

    // 7. Test rule evaluation - document that fails validation
    console.log("\n7. Testing validation rule with invalid document...");
    const invalidDocument = {
      document: {
        type: "memo",
        content: "This is a simple memo without financial information.",
      },
      metadata: {
        fileSize: 500,
      },
    };
    await evaluateRule(validationRule.id, invalidDocument);

    // 8. Test budget rule evaluation
    console.log("\n8. Testing budget rule...");
    const budgetData = {
      budget: {
        percentageUsed: 0.85,
        used: 850,
        limit: 1000,
      },
    };
    await evaluateRule(budgetRule.id, budgetData);

    // 9. Update a rule
    console.log("\n9. Updating validation rule...");
    await updateRule(validationRule.id, {
      name: "Enhanced Document Validation Rule",
      description: "Enhanced validation with additional checks",
      condition: validationRule.condition,
      tags: ["validation", "document", "quality", "enhanced"],
    });

    // 10. Get updated rule details
    console.log("\n10. Getting updated rule details...");
    await getRule(validationRule.id);

    console.log("\n✅ Rules example completed successfully!");

  } catch (error) {
    console.error("\n❌ Rules example failed:", error.message);
  }
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/**
 * Helper function to validate rule condition structure
 */
function validateRuleCondition(condition) {
  const requiredFields = ['conditionType'];
  const missing = requiredFields.filter(field => !(field in condition));
  if (missing.length > 0) {
    throw new Error(`Missing required condition fields: ${missing.join(', ')}`);
  }

  const validTypes = ['equals', 'not_equals', 'contains', 'not_contains', 'greater_than', 'less_than', 'and', 'or', 'not', 'script'];
  if (!validTypes.includes(condition.conditionType)) {
    throw new Error(`Invalid condition type: ${condition.conditionType}`);
  }

  return true;
}

/**
 * Helper function to create a simple field comparison rule
 */
function createFieldRule(field, operator, value) {
  return {
    conditionType: operator,
    field: field,
    value: value,
  };
}

/**
 * Helper function to create a logical rule (AND/OR)
 */
function createLogicalRule(operator, rules) {
  return {
    conditionType: operator,
    rules: rules,
  };
}

/**
 * Helper function to format evaluation results
 */
function formatEvaluationResult(result) {
  return {
    ruleId: result.ruleId,
    passed: result.passed ? "✅ PASSED" : "❌ FAILED",
    reason: result.reason,
    hasDetails: !!result.details,
    subResultsCount: result.subResults?.length || 0,
  };
}

// ============================================================================
// EXPORTS
// ============================================================================

module.exports = {
  // Query functions
  getRule,
  listRules,
  getWorkflowRules,

  // Mutation functions
  createDocumentValidationRule,
  createPriorityEscalationRule,
  createBudgetThresholdRule,
  updateRule,
  deleteRule,
  evaluateRule,

  // Complete examples
  completeRulesExample,

  // Utilities
  validateRuleCondition,
  createFieldRule,
  createLogicalRule,
  formatEvaluationResult,

  // Schema reference
  rulesSchema,
};

// Run example if this file is executed directly
if (require.main === module) {
  completeRulesExample()
    .catch(console.error)
    .finally(() => process.exit(0));
}
