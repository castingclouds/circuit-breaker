// TypeScript client demonstrating Circuit Breaker Rules Engine
// This shows complex rule evaluation for token transitions via GraphQL
// Run with: npm run start:rules

import { GraphQLClient, gql } from 'graphql-request';
import chalk from 'chalk';

// GraphQL endpoint (assumes Rust server is running)
const GRAPHQL_ENDPOINT = 'http://localhost:4000/graphql';

// Rules Engine Types (mirroring Rust implementation)
interface Rule {
  id: string;
  description: string;
  condition: RuleCondition;
}

interface RuleCondition {
  type: 'FieldExists' | 'FieldEquals' | 'FieldGreaterThan' | 'FieldLessThan' | 'FieldContains' | 'And' | 'Or' | 'Not' | 'Expression';
  field?: string;
  value?: any;
  threshold?: number;
  substring?: string;
  rules?: Rule[];
  rule?: Rule;
  script?: string;
}

interface WorkflowDefinition {
  name: string;
  places: string[];
  transitions: TransitionDefinition[];
  initialPlace: string;
}

interface TransitionDefinition {
  id: string;
  fromPlaces: string[];
  toPlace: string;
  conditions?: string[];
  rules?: Rule[];
}

interface Token {
  id: string;
  place: string;
  workflowId: string;
  data?: any;
  metadata?: any;
  history?: HistoryEvent[];
  createdAt?: string;
  updatedAt?: string;
}

interface HistoryEvent {
  transition: string;
  fromPlace: string;
  toPlace: string;
  timestamp: string;
}

interface RuleEvaluationResult {
  ruleId: string;
  passed: boolean;
  explanation: string;
  subResults: { ruleId: string; passed: boolean }[];
}

interface TransitionRuleEvaluation {
  transitionId: string;
  placeCompatible: boolean;
  rulesPassed: boolean;
  canFire: boolean;
  ruleResults: RuleEvaluationResult[];
  explanation: string;
}

interface WorkflowEvaluationResult {
  workflowId: string;
  tokenId: string;
  currentPlace: string;
  transitionResults: TransitionRuleEvaluation[];
  availableCount: number;
  blockedCount: number;
}

// GraphQL Response Types
interface CreateWorkflowResponse {
  createWorkflow: {
    id: string;
    name: string;
    places: string[];
    initialPlace: string;
  };
}

interface CreateTokenResponse {
  createToken: Token;
}

interface GetTokenResponse {
  token: Token;
}

interface AvailableTransitionsResponse {
  availableTransitions: {
    id: string;
    fromPlaces: string[];
    toPlace: string;
  }[];
}

// Rule Builder Functions (TypeScript equivalents of Rust builders)
class RuleBuilder {
  static fieldExists(id: string, field: string): Rule {
    return {
      id,
      description: `Field '${field}' must exist`,
      condition: {
        type: 'FieldExists',
        field
      }
    };
  }

  static fieldEquals(id: string, field: string, value: any): Rule {
    return {
      id,
      description: `Field '${field}' must equal ${JSON.stringify(value)}`,
      condition: {
        type: 'FieldEquals',
        field,
        value
      }
    };
  }

  static fieldGreaterThan(id: string, field: string, threshold: number): Rule {
    return {
      id,
      description: `Field '${field}' must be greater than ${threshold}`,
      condition: {
        type: 'FieldGreaterThan',
        field,
        threshold
      }
    };
  }

  static fieldLessThan(id: string, field: string, threshold: number): Rule {
    return {
      id,
      description: `Field '${field}' must be less than ${threshold}`,
      condition: {
        type: 'FieldLessThan',
        field,
        threshold
      }
    };
  }

  static fieldContains(id: string, field: string, substring: string): Rule {
    return {
      id,
      description: `Field '${field}' must contain '${substring}'`,
      condition: {
        type: 'FieldContains',
        field,
        substring
      }
    };
  }

  static and(id: string, description: string, rules: Rule[]): Rule {
    return {
      id,
      description,
      condition: {
        type: 'And',
        rules
      }
    };
  }

  static or(id: string, description: string, rules: Rule[]): Rule {
    return {
      id,
      description,
      condition: {
        type: 'Or',
        rules
      }
    };
  }

  static not(id: string, description: string, rule: Rule): Rule {
    return {
      id,
      description,
      condition: {
        type: 'Not',
        rule
      }
    };
  }
}

// Client-side rule evaluation (for demo purposes)
class ClientRulesEngine {
  private rules: Map<string, Rule> = new Map();

  constructor() {
    this.registerCommonRules();
  }

  private registerCommonRules() {
    // Content validation rules
    this.registerRule(RuleBuilder.fieldExists('has_content', 'content'));
    this.registerRule(RuleBuilder.fieldExists('has_title', 'title'));
    this.registerRule(RuleBuilder.fieldExists('has_description', 'description'));

    // Approval workflow rules
    this.registerRule(RuleBuilder.fieldExists('has_reviewer', 'reviewer'));
    this.registerRule(RuleBuilder.fieldExists('has_approver', 'approver'));
    this.registerRule(RuleBuilder.fieldEquals('status_approved', 'status', 'approved'));
    this.registerRule(RuleBuilder.fieldEquals('status_rejected', 'status', 'rejected'));
    this.registerRule(RuleBuilder.fieldEquals('status_pending', 'status', 'pending'));

    // Priority and urgency rules
    this.registerRule(RuleBuilder.fieldGreaterThan('high_priority', 'priority', 5));
    this.registerRule(RuleBuilder.fieldGreaterThan('critical_priority', 'priority', 8));
    this.registerRule(RuleBuilder.fieldEquals('emergency_flag', 'emergency', true));

    // Testing and deployment rules
    this.registerRule(RuleBuilder.fieldEquals('tests_passed', 'test_status', 'passed'));
    this.registerRule(RuleBuilder.fieldEquals('tests_failed', 'test_status', 'failed'));
    this.registerRule(RuleBuilder.fieldEquals('security_approved', 'security_status', 'approved'));

    // Custom rules for our demo
    this.registerRule(RuleBuilder.fieldEquals('document_type_article', 'document_type', 'article'));
    this.registerRule(RuleBuilder.fieldGreaterThan('word_count_sufficient', 'word_count', 500));
  }

  registerRule(rule: Rule) {
    this.rules.set(rule.id, rule);
  }

  listRuleIds(): string[] {
    return Array.from(this.rules.keys());
  }

  evaluateRule(rule: Rule, metadata: any, data: any): RuleEvaluationResult {
    const passed = this.evaluateCondition(rule.condition, metadata, data);
    return {
      ruleId: rule.id,
      passed,
      explanation: passed ? `Rule '${rule.id}' passed` : `Rule '${rule.id}' failed`,
      subResults: []
    };
  }

  private evaluateCondition(condition: RuleCondition, metadata: any, data: any): boolean {
    switch (condition.type) {
      case 'FieldExists':
        return (metadata && metadata[condition.field!] !== undefined) || 
               (data && data[condition.field!] !== undefined);
      
      case 'FieldEquals':
        const fieldValue = (metadata && metadata[condition.field!]) || 
                          (data && data[condition.field!]);
        return fieldValue === condition.value;
      
      case 'FieldGreaterThan':
        const numValue = (metadata && metadata[condition.field!]) || 
                        (data && data[condition.field!]);
        return typeof numValue === 'number' && numValue > condition.threshold!;
      
      case 'FieldLessThan':
        const numValue2 = (metadata && metadata[condition.field!]) || 
                         (data && data[condition.field!]);
        return typeof numValue2 === 'number' && numValue2 < condition.threshold!;
      
      case 'FieldContains':
        const strValue = (metadata && metadata[condition.field!]) || 
                        (data && data[condition.field!]);
        return typeof strValue === 'string' && strValue.includes(condition.substring!);
      
      case 'And':
        return condition.rules!.every(rule => this.evaluateCondition(rule.condition, metadata, data));
      
      case 'Or':
        return condition.rules!.some(rule => this.evaluateCondition(rule.condition, metadata, data));
      
      case 'Not':
        return !this.evaluateCondition(condition.rule!.condition, metadata, data);
      
      case 'Expression':
        // Not implemented in this demo
        return false;
      
      default:
        return false;
    }
  }

  canTransition(token: Token, transition: TransitionDefinition): boolean {
    // Check place compatibility
    if (!transition.fromPlaces.includes(token.place)) {
      return false;
    }

    // Check rules if any
    if (transition.rules) {
      return transition.rules.every(rule => 
        this.evaluateCondition(rule.condition, token.metadata, token.data)
      );
    }

    return true;
  }

  availableTransitions(token: Token, workflow: WorkflowDefinition): TransitionDefinition[] {
    return workflow.transitions.filter(transition => 
      this.canTransition(token, transition)
    );
  }
}

// Demo workflow creation
function createPublishingWorkflow(): WorkflowDefinition {
  // Complex rule: Ready to publish if (high quality AND sufficient length) OR emergency override
  const publishRule = RuleBuilder.or(
    'publish_ready',
    'Ready to publish',
    [
      // Normal publishing criteria
      RuleBuilder.and(
        'quality_criteria',
        'High quality article with sufficient content',
        [
          RuleBuilder.fieldExists('has_content', 'content'),
          RuleBuilder.fieldExists('has_title', 'title'),
          RuleBuilder.fieldExists('has_reviewer', 'reviewer'),
          RuleBuilder.fieldEquals('status_approved', 'status', 'approved'),
          RuleBuilder.fieldEquals('document_type_article', 'document_type', 'article'),
          RuleBuilder.fieldGreaterThan('word_count_sufficient', 'word_count', 500),
        ]
      ),
      // Emergency override
      RuleBuilder.fieldEquals('emergency_flag', 'emergency', true),
    ]
  );

  // Rule for starting review: must have basic content
  const reviewRule = RuleBuilder.and(
    'review_ready',
    'Ready for review',
    [
      RuleBuilder.fieldExists('has_content', 'content'),
      RuleBuilder.fieldExists('has_title', 'title'),
      RuleBuilder.fieldGreaterThan('word_count_sufficient', 'word_count', 100), // Lower threshold for review
    ]
  );

  return {
    name: 'Article Publishing Workflow',
    places: ['draft', 'review', 'approved', 'published', 'rejected'],
    transitions: [
      {
        id: 'submit_for_review',
        fromPlaces: ['draft'],
        toPlace: 'review',
        rules: [reviewRule]
      },
      {
        id: 'approve_article',
        fromPlaces: ['review'],
        toPlace: 'approved',
        rules: [RuleBuilder.fieldExists('has_reviewer', 'reviewer')]
      },
      {
        id: 'publish_article',
        fromPlaces: ['approved'],
        toPlace: 'published',
        rules: [publishRule]
      },
      {
        id: 'reject_article',
        fromPlaces: ['review'],
        toPlace: 'rejected',
        rules: [RuleBuilder.fieldExists('has_reviewer', 'reviewer')]
      },
      {
        id: 'revise_article',
        fromPlaces: ['rejected'],
        toPlace: 'draft',
        rules: [] // No rules - can always revise
      }
    ],
    initialPlace: 'draft'
  };
}

// Test token creation functions
function createReadyToken(): Partial<Token> {
  return {
    place: 'approved',
    data: {
      content: 'This is a comprehensive article about the new features in our platform. '.repeat(50),
      title: 'New Platform Features: A Comprehensive Guide',
      document_type: 'article',
      word_count: 750
    },
    metadata: {
      status: 'approved',
      reviewer: 'senior_editor',
      priority: 8
    }
  };
}

function createIncompleteToken(): Partial<Token> {
  return {
    place: 'draft',
    data: {
      content: 'Just a short draft...',
      title: 'Draft Article',
      document_type: 'article',
      word_count: 50
    },
    metadata: {
      status: 'draft'
      // Missing reviewer
    }
  };
}

function createEmergencyToken(): Partial<Token> {
  return {
    place: 'approved',
    data: {
      content: 'Emergency security announcement.',
      title: 'URGENT: Security Update Required',
      document_type: 'article',
      word_count: 100
    },
    metadata: {
      emergency: true, // Emergency override
      status: 'pending'
    }
  };
}

// Demo evaluation function
function demoTokenEvaluation(
  engine: ClientRulesEngine,
  token: Partial<Token>,
  workflow: WorkflowDefinition,
  scenarioName: string
) {
  console.log(chalk.cyan(`🔍 Scenario: ${scenarioName}`));
  console.log(chalk.gray(`Token is in place: '${token.place}'`));

  // Get available transitions (client-side evaluation for demo)
  const fullToken = { ...token, id: 'demo', workflowId: 'demo' } as Token;
  const available = engine.availableTransitions(fullToken, workflow);
  console.log(chalk.blue(`Available transitions: ${available.length}`));

  for (const transition of available) {
    console.log(chalk.green(`  ✅ Can fire: '${transition.id}' -> '${transition.toPlace}'`));
  }

  // Show detailed evaluation for all transitions
  console.log(chalk.blue('\nDetailed evaluation:'));
  const allTransitions = workflow.transitions;
  let availableCount = 0;
  let blockedCount = 0;

  for (const transition of allTransitions) {
    const canFire = engine.canTransition(fullToken, transition);
    const status = canFire ? '✅' : '❌';
    const placeOk = transition.fromPlaces.includes(token.place!);
    
    if (canFire) availableCount++;
    else blockedCount++;

    console.log(chalk.gray(`  ${status} ${transition.id} (${placeOk ? 'place ok' : 'wrong place'})`));

    // Show rule details for complex transitions
    if (transition.id === 'publish_article' && transition.rules && transition.rules.length > 0) {
      console.log(chalk.gray('    Rule details:'));
      for (const rule of transition.rules) {
        const ruleResult = engine.evaluateRule(rule, token.metadata, token.data);
        const ruleStatus = ruleResult.passed ? '✅' : '❌';
        console.log(chalk.gray(`      ${ruleStatus} ${rule.id}: ${rule.description}`));

        // Show logical structure for complex rules
        if (rule.condition.type === 'Or' && rule.condition.rules) {
          console.log(chalk.gray('        OR conditions:'));
          for (const subRule of rule.condition.rules) {
            const subResult = engine.evaluateRule(subRule, token.metadata, token.data);
            const subStatus = subResult.passed ? '✅' : '❌';
            console.log(chalk.gray(`          ${subStatus} ${subRule.id}`));
            
            if (subRule.condition.type === 'And' && subRule.condition.rules) {
              console.log(chalk.gray('            AND conditions:'));
              for (const andRule of subRule.condition.rules) {
                const andResult = engine.evaluateRule(andRule, token.metadata, token.data);
                const andStatus = andResult.passed ? '✅' : '❌';
                console.log(chalk.gray(`              ${andStatus} ${andRule.id}`));
              }
            }
          }
        }
      }
    }
  }

  console.log(chalk.blue(`  Available: ${availableCount}, Blocked: ${blockedCount}`));
  console.log(chalk.gray('─'.repeat(60)));
  console.log();
}

async function main() {
  console.log(chalk.cyan('🤖 Circuit Breaker Rules Engine Demo (TypeScript)'));
  console.log(chalk.cyan('===================================================='));
  console.log(chalk.green('Demonstrating complex rule evaluation for token transitions'));
  console.log();

  try {
    // 1. Create a client-side rules engine for demonstration
    const rulesEngine = new ClientRulesEngine();
    
    console.log(chalk.blue(`📋 Registered ${rulesEngine.listRuleIds().length} rules in the engine`));
    console.log(chalk.gray('Rules available:'), rulesEngine.listRuleIds().slice(0, 10).join(', '), '...');
    console.log();

    // 2. Create a complex workflow with sophisticated rules
    const workflow = createPublishingWorkflow();
    console.log(chalk.yellow(`📄 Created workflow: ${workflow.name}`));
    console.log(chalk.gray('Places:'), workflow.places.join(' → '));
    console.log(chalk.gray(`Transitions: ${workflow.transitions.length}`));
    console.log();

    // 3. Create test tokens with different scenarios
    console.log(chalk.blue('🎯 Creating test tokens with different scenarios...'));
    console.log();

    // Scenario 1: Ready to publish
    const readyToken = createReadyToken();
    demoTokenEvaluation(rulesEngine, readyToken, workflow, 'Ready Article');

    // Scenario 2: Incomplete article  
    const incompleteToken = createIncompleteToken();
    demoTokenEvaluation(rulesEngine, incompleteToken, workflow, 'Incomplete Article');

    // Scenario 3: Emergency override
    const emergencyToken = createEmergencyToken();
    demoTokenEvaluation(rulesEngine, emergencyToken, workflow, 'Emergency Override');

    // 4. Try to connect to backend for real evaluation (if available)
    console.log(chalk.blue('🌐 Attempting to connect to Rust backend for real evaluation...'));
    
    try {
      const client = new GraphQLClient(GRAPHQL_ENDPOINT);
      
      // Create workflow in backend
      const createWorkflowMutation = gql`
        mutation CreateWorkflow($input: WorkflowDefinitionInput!) {
          createWorkflow(input: $input) {
            id
            name
            places
            initialPlace
          }
        }
      `;

      // Note: The backend would need to support rules in the GraphQL schema
      // For now, we'll just create a basic workflow without rules
      const basicWorkflow = {
        name: workflow.name,
        places: workflow.places,
        transitions: workflow.transitions.map(t => ({
          id: t.id,
          fromPlaces: t.fromPlaces,
          toPlace: t.toPlace,
          conditions: [] // Backend doesn't support rules in GraphQL yet
        })),
        initialPlace: workflow.initialPlace
      };

      const workflowResult = await client.request<CreateWorkflowResponse>(createWorkflowMutation, {
        input: basicWorkflow
      });

      console.log(chalk.green('✅ Connected to backend! Workflow created:'), workflowResult.createWorkflow.id);
      
      // Create a token in the backend
      const createTokenMutation = gql`
        mutation CreateToken($input: TokenCreateInput!) {
          createToken(input: $input) {
            id
            place
            workflowId
          }
        }
      `;

      const tokenResult = await client.request<CreateTokenResponse>(createTokenMutation, {
        input: {
          workflowId: workflowResult.createWorkflow.id,
          data: readyToken.data,
          metadata: readyToken.metadata
        }
      });

      console.log(chalk.green('✅ Token created in backend:'), tokenResult.createToken.id);
      
      // Query available transitions from backend
      const availableTransitionsQuery = gql`
        query AvailableTransitions($tokenId: ID!) {
          availableTransitions(tokenId: $tokenId) {
            id
            fromPlaces
            toPlace
          }
        }
      `;

      const transitionsResult = await client.request<AvailableTransitionsResponse>(availableTransitionsQuery, {
        tokenId: tokenResult.createToken.id
      });

      console.log(chalk.blue('🔄 Backend available transitions:'));
      for (const transition of transitionsResult.availableTransitions) {
        console.log(chalk.green(`  ✅ ${transition.id}: ${transition.fromPlaces.join(', ')} → ${transition.toPlace}`));
      }

    } catch (backendError: any) {
      console.log(chalk.yellow('⚠️  Backend not available or rules not implemented in GraphQL yet'));
      console.log(chalk.gray('   Using client-side evaluation only'));
    }

    console.log();
    console.log(chalk.magenta('🏗️  Rules Engine Demo Summary:'));
    console.log(chalk.gray('   ✅ Complex logical expressions (AND, OR, NOT)'));
    console.log(chalk.gray('   ✅ Token state evaluation against rules'));
    console.log(chalk.gray('   ✅ Detailed debugging information'));
    console.log(chalk.gray('   ✅ Emergency override scenarios'));
    console.log(chalk.gray('   ✅ TypeScript-to-Rust polyglot architecture'));
    console.log();
    console.log(chalk.green('✅ Rules engine demo completed successfully!'));

  } catch (error: any) {
    console.error(chalk.red('❌ Demo failed:'), error.message);
    if (error.stack) {
      console.error(chalk.gray(error.stack));
    }
  }
}

// Run the demo
main().catch(console.error); 