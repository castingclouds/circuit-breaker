#!/usr/bin/env npx tsx
// Basic workflow demonstration - TypeScript GraphQL Client
// Shows core workflow operations using GraphQL API
// Run with: npx tsx examples/typescript/basic_workflow.ts

interface GraphQLResponse<T = any> {
  data?: T;
  errors?: Array<{ message: string; locations?: any[]; path?: any[] }>;
}

interface WorkflowGQL {
  id: string;
  name: string;
  places: string[];
  transitions: TransitionGQL[];
  initialPlace: string;
  createdAt: string;
  updatedAt: string;
}

interface TransitionGQL {
  id: string;
  name?: string;
  fromPlaces: string[];
  toPlace: string;
  conditions: string[];
  description?: string;
}

interface TokenGQL {
  id: string;
  workflowId: string;
  place: string;
  data: any;
  metadata: any;
  createdAt: string;
  updatedAt: string;
  history: HistoryEventGQL[];
}

interface HistoryEventGQL {
  timestamp: string;
  transition: string;
  fromPlace: string;
  toPlace: string;
  data?: any;
}

class CircuitBreakerClient {
  constructor(private baseUrl: string = 'http://localhost:4000') {}

  async graphql<T = any>(query: string, variables?: any): Promise<GraphQLResponse<T>> {
    const response = await fetch(`${this.baseUrl}/graphql`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ query, variables }),
    });

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }

    return await response.json() as GraphQLResponse<T>;
  }

  async createWorkflow(input: any) {
    const mutation = `
      mutation CreateWorkflow($input: WorkflowDefinitionInput!) {
        createWorkflow(input: $input) {
          id
          name
          places
          transitions {
            id
            fromPlaces
            toPlace
            conditions
          }
          initialPlace
          createdAt
          updatedAt
        }
      }
    `;

    return this.graphql<{ createWorkflow: WorkflowGQL }>(mutation, { input });
  }

  async createToken(input: any) {
    const mutation = `
      mutation CreateToken($input: TokenCreateInput!) {
        createToken(input: $input) {
          id
          workflowId
          place
          data
          metadata
          createdAt
          updatedAt
          history {
            timestamp
            transition
            fromPlace
            toPlace
            data
          }
        }
      }
    `;

    return this.graphql<{ createToken: TokenGQL }>(mutation, { input });
  }

  async fireTransition(input: any) {
    const mutation = `
      mutation FireTransition($input: TransitionFireInput!) {
        fireTransition(input: $input) {
          id
          workflowId
          place
          data
          metadata
          createdAt
          updatedAt
          history {
            timestamp
            transition
            fromPlace
            toPlace
            data
          }
        }
      }
    `;

    return this.graphql<{ fireTransition: TokenGQL }>(mutation, { input });
  }

  async getToken(id: string) {
    const query = `
      query GetToken($id: String!) {
        token(id: $id) {
          id
          workflowId
          place
          data
          metadata
          createdAt
          updatedAt
          history {
            timestamp
            transition
            fromPlace
            toPlace
            data
          }
        }
      }
    `;

    return this.graphql<{ token: TokenGQL | null }>(query, { id });
  }

  async listWorkflows() {
    const query = `
      query ListWorkflows {
        workflows {
          id
          name
          places
          transitions {
            id
            fromPlaces
            toPlace
            conditions
          }
          initialPlace
          createdAt
        }
      }
    `;

    return this.graphql<{ workflows: WorkflowGQL[] }>(query);
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

async function main() {
  console.log('🚀 Circuit Breaker Basic Workflow Demo - TypeScript Client');
  console.log('==========================================================');
  console.log();

  const client = new CircuitBreakerClient();

  try {
    // Create a simple application development workflow
    logInfo('Creating Application Development Workflow...');
    
    const workflowInput = {
      name: 'Application Development Process',
      places: ['planning', 'development', 'testing', 'staging', 'production', 'maintenance'],
      transitions: [
        {
          id: 'start_development',
          fromPlaces: ['planning'],
          toPlace: 'development',
          conditions: []
        },
        {
          id: 'submit_for_testing',
          fromPlaces: ['development'],
          toPlace: 'testing',
          conditions: []
        },
        {
          id: 'back_to_development',
          fromPlaces: ['testing'],
          toPlace: 'development',
          conditions: []
        },
        {
          id: 'promote_to_staging',
          fromPlaces: ['testing'],
          toPlace: 'staging',
          conditions: []
        },
        {
          id: 'deploy_to_production',
          fromPlaces: ['staging'],
          toPlace: 'production',
          conditions: []
        },
        {
          id: 'enter_maintenance',
          fromPlaces: ['production'],
          toPlace: 'maintenance',
          conditions: []
        },
        {
          id: 'back_to_planning',
          fromPlaces: ['maintenance'],
          toPlace: 'planning',
          conditions: []
        }
      ],
      initialPlace: 'planning'
    };

    const workflowResult = await client.createWorkflow(workflowInput);
    
    if (workflowResult.errors) {
      logError(`Failed to create workflow: ${workflowResult.errors.map(e => e.message).join(', ')}`);
      return;
    }

    const workflow = workflowResult.data!.createWorkflow;
    logSuccess(`Created workflow: ${workflow.name} (${workflow.id})`);
    logInfo(`Places: ${workflow.places.join(' → ')}`);
    logInfo(`Transitions: ${workflow.transitions.length} defined`);
    console.log();

    // Create a feature development token
    logInfo('Creating Feature Development Token...');
    
    const tokenInput = {
      workflowId: workflow.id,
      initialPlace: 'planning',
      data: {
        featureName: 'User Authentication',
        assignedDeveloper: 'Alice Smith',
        priority: 'high',
        estimatedHours: 40,
        requirements: [
          'Login/logout functionality',
          'Password reset capability',
          'Session management',
          'Security audit'
        ]
      },
      metadata: {
        createdBy: 'project-manager',
        project: 'main-application',
        sprint: 'sprint-2024-01'
      }
    };

    const tokenResult = await client.createToken(tokenInput);
    
    if (tokenResult.errors) {
      logError(`Failed to create token: ${tokenResult.errors.map(e => e.message).join(', ')}`);
      return;
    }

    const token = tokenResult.data!.createToken;
    logSuccess(`Created token: ${token.id}`);
    logInfo(`Feature: ${token.data.featureName}`);
    logInfo(`Developer: ${token.data.assignedDeveloper}`);
    logInfo(`Current place: ${token.place}`);
    console.log();

    // Simulate development lifecycle
    const transitions = [
      { id: 'start_development', description: 'Start Development Phase' },
      { id: 'submit_for_testing', description: 'Submit for Testing' },
      { id: 'back_to_development', description: 'Back to Development (Bug Found)' },
      { id: 'submit_for_testing', description: 'Resubmit for Testing' },
      { id: 'promote_to_staging', description: 'Promote to Staging' },
      { id: 'deploy_to_production', description: 'Deploy to Production' }
    ];

    let currentToken = token;
    
    for (const transition of transitions) {
      logInfo(`Firing transition: ${transition.description}`);
      
      const transitionInput = {
        tokenId: currentToken.id,
        transitionId: transition.id,
        data: {
          timestamp: new Date().toISOString(),
          performedBy: 'automated-system',
          notes: `Executed ${transition.description}`
        }
      };

      const transitionResult = await client.fireTransition(transitionInput);
      
      if (transitionResult.errors) {
        logError(`Failed to fire transition: ${transitionResult.errors.map(e => e.message).join(', ')}`);
        continue;
      }

      currentToken = transitionResult.data!.fireTransition;
      logSuccess(`Transition completed: ${currentToken.place}`);
      
      // Add a small delay to make the demo more realistic
      await new Promise(resolve => setTimeout(resolve, 100));
    }

    console.log();
    logInfo('Development Lifecycle Complete!');
    logInfo(`Final state: ${currentToken.place}`);
    
    // Show history
    console.log();
    logInfo('Complete Development History:');
    currentToken.history.forEach((event, index) => {
      const timestamp = new Date(event.timestamp).toLocaleTimeString();
      console.log(`  ${index + 1}. ${event.fromPlace} → ${event.toPlace} via ${event.transition} (${timestamp})`);
    });

    console.log();
    logInfo('Workflow demonstrates:');
    console.log('  • Complex state transitions with cycles');
    console.log('  • Rich token data for application features');
    console.log('  • Complete audit trail of state changes');
    console.log('  • GraphQL API integration from TypeScript');
    console.log('  • Production-ready development workflow');

  } catch (error) {
    logError(`Demo failed: ${error}`);
    process.exit(1);
  }
}

if (require.main === module) {
  main().catch(console.error);
}

export { CircuitBreakerClient, type WorkflowGQL, type TokenGQL, type HistoryEventGQL }; 