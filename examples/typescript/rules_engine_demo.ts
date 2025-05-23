#!/usr/bin/env npx tsx
// Rules engine demonstration - TypeScript GraphQL Client
// Shows advanced rules evaluation for conditional workflow transitions
// Run with: npx tsx examples/typescript/rules_engine_demo.ts

import { CircuitBreakerClient } from './basic_workflow.js';

interface Rule {
  id: string;
  name: string;
  description: string;
  condition: RuleCondition;
}

interface RuleCondition {
  type: 'And' | 'Or' | 'Not' | 'FieldExists' | 'FieldEquals' | 'FieldGreaterThan' | 'FieldLessThan' | 'FieldContains';
  field?: string;
  value?: any;
  conditions?: RuleCondition[];
}

interface RuleEvaluationResult {
  rule_id: string;
  passed: boolean;
  reason: string;
  details?: Record<string, any>;
}

class RuleBuilder {
  static fieldExists(id: string, description: string, field: string): Rule {
    return {
      id,
      name: `Field Exists: ${field}`,
      description,
      condition: {
        type: 'FieldExists',
        field
      }
    };
  }

  static fieldEquals(id: string, description: string, field: string, value: any): Rule {
    return {
      id,
      name: `Field Equals: ${field} = ${value}`,
      description,
      condition: {
        type: 'FieldEquals',
        field,
        value
      }
    };
  }

  static fieldGreaterThan(id: string, description: string, field: string, value: number): Rule {
    return {
      id,
      name: `Field Greater Than: ${field} > ${value}`,
      description,
      condition: {
        type: 'FieldGreaterThan',
        field,
        value
      }
    };
  }

  static fieldContains(id: string, description: string, field: string, value: string): Rule {
    return {
      id,
      name: `Field Contains: ${field} contains "${value}"`,
      description,
      condition: {
        type: 'FieldContains',
        field,
        value
      }
    };
  }

  static and(id: string, description: string, conditions: Rule[]): Rule {
    return {
      id,
      name: `AND: ${conditions.map(c => c.name).join(' AND ')}`,
      description,
      condition: {
        type: 'And',
        conditions: conditions.map(c => c.condition)
      }
    };
  }

  static or(id: string, description: string, conditions: Rule[]): Rule {
    return {
      id,
      name: `OR: ${conditions.map(c => c.name).join(' OR ')}`,
      description,
      condition: {
        type: 'Or',
        conditions: conditions.map(c => c.condition)
      }
    };
  }
}

class ClientRuleEngine {
  /**
   * Client-side rule evaluation for immediate UI feedback
   * Note: This should always be validated on the server for authoritative results
   */
  static evaluateRule(rule: Rule, tokenData: any, tokenMetadata: any): RuleEvaluationResult {
    return this.evaluateCondition(rule.condition, tokenData, tokenMetadata, rule.id);
  }

  private static evaluateCondition(
    condition: RuleCondition, 
    data: any, 
    metadata: any, 
    ruleId: string
  ): RuleEvaluationResult {
    const combinedData = { ...data, ...metadata };

    switch (condition.type) {
      case 'FieldExists':
        const exists = combinedData[condition.field!] !== undefined && combinedData[condition.field!] !== null;
        return {
          rule_id: ruleId,
          passed: exists,
          reason: exists ? `Field '${condition.field}' exists` : `Field '${condition.field}' does not exist`,
          details: { field: condition.field, value: combinedData[condition.field!] }
        };

      case 'FieldEquals':
        const fieldValue = combinedData[condition.field!];
        const equals = fieldValue === condition.value;
        return {
          rule_id: ruleId,
          passed: equals,
          reason: equals 
            ? `Field '${condition.field}' equals ${condition.value}` 
            : `Field '${condition.field}' (${fieldValue}) does not equal ${condition.value}`,
          details: { field: condition.field, expected: condition.value, actual: fieldValue }
        };

      case 'FieldGreaterThan':
        const numValue = Number(combinedData[condition.field!]);
        const greater = !isNaN(numValue) && numValue > condition.value;
        return {
          rule_id: ruleId,
          passed: greater,
          reason: greater 
            ? `Field '${condition.field}' (${numValue}) is greater than ${condition.value}`
            : `Field '${condition.field}' (${numValue}) is not greater than ${condition.value}`,
          details: { field: condition.field, threshold: condition.value, actual: numValue }
        };

      case 'FieldContains':
        const strValue = String(combinedData[condition.field!] || '');
        const contains = strValue.includes(String(condition.value));
        return {
          rule_id: ruleId,
          passed: contains,
          reason: contains 
            ? `Field '${condition.field}' contains "${condition.value}"`
            : `Field '${condition.field}' does not contain "${condition.value}"`,
          details: { field: condition.field, searchValue: condition.value, actualValue: strValue }
        };

      case 'And':
        const andResults = condition.conditions!.map(c => this.evaluateCondition(c, data, metadata, ruleId));
        const allPassed = andResults.every(r => r.passed);
        return {
          rule_id: ruleId,
          passed: allPassed,
          reason: allPassed ? 'All AND conditions passed' : 'One or more AND conditions failed',
          details: { subResults: andResults }
        };

      case 'Or':
        const orResults = condition.conditions!.map(c => this.evaluateCondition(c, data, metadata, ruleId));
        const anyPassed = orResults.some(r => r.passed);
        return {
          rule_id: ruleId,
          passed: anyPassed,
          reason: anyPassed ? 'At least one OR condition passed' : 'All OR conditions failed',
          details: { subResults: orResults }
        };

      default:
        return {
          rule_id: ruleId,
          passed: false,
          reason: `Unknown rule type: ${condition.type}`,
          details: { condition }
        };
    }
  }
}

function logSuccess(message: string) {
  console.log(`✅ ${message}`);
}

function logInfo(message: string) {
  console.log(`ℹ️  ${message}`);
}

function logError(message: string) {
  console.log(`❌ ${message}`);
}

function logWarning(message: string) {
  console.log(`⚠️  ${message}`);
}

function logRule(rule: Rule, result: RuleEvaluationResult) {
  const icon = result.passed ? '✅' : '❌';
  console.log(`${icon} ${rule.name}: ${result.reason}`);
  if (result.details && !result.passed) {
    console.log(`   Details: ${JSON.stringify(result.details, null, 2)}`);
  }
}

async function main() {
  console.log('🚀 Circuit Breaker Rules Engine Demo - TypeScript Client');
  console.log('========================================================');
  console.log();

  const client = new CircuitBreakerClient();

  try {
    // Create article publishing workflow with quality gates
    logInfo('Creating Article Publishing Workflow...');
    
    const workflowInput = {
      name: 'Article Publishing with Quality Gates',
      places: ['draft', 'review', 'revision', 'quality_check', 'approved', 'published', 'rejected'],
      transitions: [
        {
          id: 'submit_for_review',
          fromPlaces: ['draft'],
          toPlace: 'review',
          conditions: []
        },
        {
          id: 'request_revision',
          fromPlaces: ['review'],
          toPlace: 'revision',
          conditions: []
        },
        {
          id: 'back_to_draft',
          fromPlaces: ['revision'],
          toPlace: 'draft',
          conditions: []
        },
        {
          id: 'quality_check',
          fromPlaces: ['review'],
          toPlace: 'quality_check',
          conditions: []
        },
        {
          id: 'approve_article',
          fromPlaces: ['quality_check'],
          toPlace: 'approved',
          conditions: []
        },
        {
          id: 'reject_article',
          fromPlaces: ['quality_check'],
          toPlace: 'rejected',
          conditions: []
        },
        {
          id: 'publish_article',
          fromPlaces: ['approved'],
          toPlace: 'published',
          conditions: []
        },
        {
          id: 'emergency_publish',
          fromPlaces: ['review'],
          toPlace: 'published',
          conditions: []
        }
      ],
      initialPlace: 'draft'
    };

    const workflowResult = await client.createWorkflow(workflowInput);
    
    if (workflowResult.errors) {
      logError(`Failed to create workflow: ${workflowResult.errors.map(e => e.message).join(', ')}`);
      return;
    }

    const workflow = workflowResult.data!.createWorkflow;
    logSuccess(`Created workflow: ${workflow.name} (${workflow.id})`);
    console.log();

    // Define quality rules for article publishing
    const qualityRules = [
      RuleBuilder.fieldExists('has_content', 'Article must have content', 'content'),
      RuleBuilder.fieldExists('has_reviewer', 'Article must have a reviewer assigned', 'reviewer'),
      RuleBuilder.fieldGreaterThan('word_count_sufficient', 'Article must have at least 500 words', 'word_count', 500),
      RuleBuilder.fieldGreaterThan('quality_score_high', 'Article must have quality score > 7', 'quality_score', 7),
      RuleBuilder.fieldContains('has_urgent_tag', 'Article tagged as urgent', 'tags', 'urgent')
    ];

    const publishReadyRule = RuleBuilder.or('publish_ready', 'Ready to publish', [
      RuleBuilder.and('quality_criteria', 'High quality article', [
        qualityRules[0], // has_content
        qualityRules[1], // has_reviewer 
        qualityRules[2], // word_count_sufficient
        qualityRules[3]  // quality_score_high
      ]),
      RuleBuilder.fieldEquals('emergency_flag', 'Emergency override', 'emergency', true)
    ]);

    logInfo('Defined Quality Rules:');
    qualityRules.forEach(rule => {
      console.log(`  • ${rule.name}: ${rule.description}`);
    });
    console.log(`  • ${publishReadyRule.name}: ${publishReadyRule.description}`);
    console.log();

    // Test scenarios
    const testScenarios = [
      {
        name: 'High Quality Article',
        data: {
          title: 'Complete Guide to Rust Programming',
          content: 'This is a comprehensive guide covering all aspects of Rust programming...',
          word_count: 1250,
          quality_score: 9,
          reviewer: 'senior-editor',
          tags: ['programming', 'rust', 'tutorial'],
          author: 'tech-writer'
        },
        metadata: {
          priority: 'high',
          deadline: '2024-12-31',
          category: 'technical'
        }
      },
      {
        name: 'Incomplete Article',
        data: {
          title: 'Short Article',
          content: 'Brief content...',
          word_count: 150,
          quality_score: 5,
          tags: ['draft'],
          author: 'junior-writer'
        },
        metadata: {
          priority: 'medium',
          category: 'general'
        }
      },
      {
        name: 'Emergency Article',
        data: {
          title: 'Breaking News Update',
          content: 'Urgent breaking news content...',
          word_count: 300,
          quality_score: 6,
          emergency: true,
          tags: ['breaking', 'urgent'],
          author: 'news-writer'
        },
        metadata: {
          priority: 'urgent',
          category: 'news'
        }
      }
    ];

    for (const [index, scenario] of testScenarios.entries()) {
      console.log();
      logInfo(`Testing Scenario ${index + 1}: ${scenario.name}`);
      
      // Create token for scenario
      const tokenResult = await client.createToken({
        workflowId: workflow.id,
        initialPlace: 'draft',
        data: scenario.data,
        metadata: scenario.metadata
      });
      
      if (tokenResult.errors) {
        logError(`Failed to create token: ${tokenResult.errors.map(e => e.message).join(', ')}`);
        continue;
      }

      const token = tokenResult.data!.createToken;
      logSuccess(`Created article token: ${token.data.title}`);
      
      // Move to review stage
      const reviewResult = await client.fireTransition({
        tokenId: token.id,
        transitionId: 'submit_for_review',
        data: {
          action: 'submit_for_review',
          timestamp: new Date().toISOString()
        }
      });
      
      if (reviewResult.errors) {
        logError(`Failed to submit for review: ${reviewResult.errors.map(e => e.message).join(', ')}`);
        continue;
      }

      const reviewToken = reviewResult.data!.fireTransition;
      
      // Evaluate rules client-side
      logInfo('Client-side Rule Evaluation:');
      
      // Test individual quality rules
      qualityRules.forEach(rule => {
        const result = ClientRuleEngine.evaluateRule(rule, reviewToken.data, reviewToken.metadata);
        logRule(rule, result);
      });
      
      // Test composite publish-ready rule
      const publishResult = ClientRuleEngine.evaluateRule(publishReadyRule, reviewToken.data, reviewToken.metadata);
      console.log();
      logInfo('Overall Publishing Decision:');
      logRule(publishReadyRule, publishResult);
      
      // Determine next action based on rules
      let nextTransition: string;
      let nextData: any = {
        timestamp: new Date().toISOString(),
        rulesEvaluated: true
      };
      
      if (publishResult.passed) {
        if (reviewToken.data.emergency) {
          nextTransition = 'emergency_publish';
          nextData.publishReason = 'Emergency override - bypassing quality check';
          logWarning('Using emergency publish due to override flag');
        } else {
          nextTransition = 'quality_check';
          nextData.qualityCheckPassed = true;
          logInfo('Proceeding to quality check - all criteria met');
        }
      } else {
        nextTransition = 'request_revision';
        nextData.revisionReason = 'Quality criteria not met';
        nextData.failedRules = publishResult.details;
        logWarning('Requesting revision due to failed quality criteria');
      }
      
      // Execute the determined transition
      const finalResult = await client.fireTransition({
        tokenId: reviewToken.id,
        transitionId: nextTransition,
        data: nextData
      });
      
      if (finalResult.errors) {
        logError(`Failed to execute transition: ${finalResult.errors.map(e => e.message).join(', ')}`);
        continue;
      }

      const finalToken = finalResult.data!.fireTransition;
      logSuccess(`Article moved to: ${finalToken.place}`);
      
      // If approved, publish it
      if (finalToken.place === 'quality_check') {
        const approveResult = await client.fireTransition({
          tokenId: finalToken.id,
          transitionId: 'approve_article',
          data: {
            timestamp: new Date().toISOString(),
            approvedBy: 'quality-team'
          }
        });
        
        if (!approveResult.errors) {
          const approvedToken = approveResult.data!.fireTransition;
          
          const publishFinalResult = await client.fireTransition({
            tokenId: approvedToken.id,
            transitionId: 'publish_article',
            data: {
              timestamp: new Date().toISOString(),
              publishUrl: `https://blog.example.com/${token.data.title.toLowerCase().replace(/\s+/g, '-')}`
            }
          });
          
          if (!publishFinalResult.errors) {
            logSuccess(`Article published: ${token.data.title}`);
          }
        }
      }
    }

    console.log();
    logInfo('Rules Engine Demo Summary:');
    console.log('  • Complex logical rules with AND, OR operations');
    console.log('  • Field-based conditions (exists, equals, greater than, contains)');
    console.log('  • Client-side rule evaluation for immediate feedback');
    console.log('  • Conditional workflow transitions based on rule results');
    console.log('  • Emergency override scenarios bypassing normal rules');
    console.log('  • Detailed rule evaluation results and debugging');

  } catch (error) {
    logError(`Demo failed: ${error}`);
    process.exit(1);
  }
}

if (require.main === module) {
  main().catch(console.error);
}

export { RuleBuilder, ClientRuleEngine, type Rule, type RuleCondition, type RuleEvaluationResult }; 