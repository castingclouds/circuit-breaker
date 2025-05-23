// TypeScript client demonstrating Circuit Breaker polyglot architecture
// This shows how ANY language can define workflows via GraphQL against the generic Rust backend
// Run with: npm run start:basic

import { GraphQLClient, gql } from 'graphql-request';
import chalk from 'chalk';

// GraphQL endpoint (assumes Rust server is running)
const GRAPHQL_ENDPOINT = 'http://localhost:4000/graphql';

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
}

interface Token {
  id: string;
  place: string;
  workflowId: string;
  data?: any;
  metadata?: any;
  history?: HistoryEvent[];
}

interface HistoryEvent {
  transition: string;
  fromPlace: string;
  toPlace: string;
  timestamp: string;
}

interface WorkflowResponse {
  id: string;
  name: string;
  places: string[];
  initialPlace: string;
}

interface CreateWorkflowResponse {
  createWorkflow: WorkflowResponse;
}

interface CreateTokenResponse {
  createToken: Token;
}

interface FireTransitionResponse {
  fireTransition: Token;
}

interface GetTokenResponse {
  token: Token;
}

async function main() {
  console.log(chalk.cyan('🌐 Circuit Breaker - TypeScript Client Demo'));
  console.log(chalk.cyan('=============================================='));
  console.log(chalk.green('Polyglot Architecture: TypeScript → GraphQL → Rust Backend'));
  console.log();

  try {
    // 1. Connect to the generic Rust backend
    const client = new GraphQLClient(GRAPHQL_ENDPOINT);
    console.log(chalk.blue('🔌 Connected to Rust backend at'), GRAPHQL_ENDPOINT);
    console.log();

    // 2. Define a TypeScript workflow via GraphQL (client-defined domain logic!)
    const workflowDefinition = createTypeScriptWorkflow();
    console.log(chalk.yellow('📋 TypeScript-Defined Workflow:'), workflowDefinition.name);
    console.log(chalk.gray('   Places:'), workflowDefinition.places.join(' → '));
    console.log(chalk.gray('   Transitions:'), workflowDefinition.transitions.map(t => t.id).join(', '));
    console.log();

    // 3. Create workflow in the generic Rust backend
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

    console.log(chalk.blue('🚀 Creating workflow in generic Rust backend...'));
    const workflowResult = await client.request<CreateWorkflowResponse>(createWorkflowMutation, {
      input: workflowDefinition
    });

    console.log(chalk.green('✅ Workflow created:'), workflowResult.createWorkflow.id);
    console.log();

    // 4. Create a token via GraphQL
    const createTokenMutation = gql`
      mutation CreateToken($input: TokenCreateInput!) {
        createToken(input: $input) {
          id
          place
          workflowId
        }
      }
    `;

    console.log(chalk.blue('🎯 Creating token via GraphQL...'));
    const tokenResult = await client.request<CreateTokenResponse>(createTokenMutation, {
      input: {
        workflowId: workflowResult.createWorkflow.id,
        data: {
          title: "TypeScript Application",
          framework: "React + Node.js",
          priority: "high"
        }
      }
    });

    console.log(chalk.green('✅ Token created:'), tokenResult.createToken.id);
    console.log(chalk.gray('   Initial place:'), tokenResult.createToken.place);
    console.log();

    // 5. Execute transitions through GraphQL
    const fireTransitionMutation = gql`
      mutation FireTransition($input: TransitionFireInput!) {
        fireTransition(input: $input) {
          id
          place
          history {
            transition
            fromPlace
            toPlace
            timestamp
          }
        }
      }
    `;

    console.log(chalk.blue('🔄 Executing TypeScript-defined state transitions...'));
    const transitions = ['start_development', 'submit_pr', 'approve'];
    let currentToken = tokenResult.createToken;

    for (const transitionId of transitions) {
      console.log(chalk.yellow(`   ➡️  Firing transition: ${transitionId}`));
      try {
        const transitionResult = await client.request<FireTransitionResponse>(fireTransitionMutation, {
          input: {
            tokenId: currentToken.id,
            transitionId
          }
        });

        currentToken = transitionResult.fireTransition;
        console.log(chalk.green(`   ✅ New place: ${currentToken.place}`));
      } catch (error: any) {
        console.log(chalk.red(`   ❌ Transition failed: ${error.message}`));
      }
    }

    console.log();

    // 6. Query token history
    const getTokenQuery = gql`
      query GetToken($id: ID!) {
        token(id: $id) {
          id
          place
          workflowId
          history {
            transition
            fromPlace
            toPlace
            timestamp
          }
        }
      }
    `;

    console.log(chalk.blue('📚 Fetching complete token history...'));
    const historyResult = await client.request<GetTokenResponse>(getTokenQuery, {
      id: currentToken.id
    });

    const token = historyResult.token;
    console.log(chalk.green('📈 Transition History:'));
    token.history?.forEach((event: HistoryEvent, index: number) => {
      console.log(chalk.gray(`   ${index + 1}. ${event.fromPlace} → ${event.toPlace} via ${event.transition}`));
    });

    console.log();

    // 7. Demonstrate architecture benefits
    console.log(chalk.magenta('🏗️  Architecture Demonstration Complete:'));
    console.log(chalk.gray('   🦀 Rust Backend:'), 'Generic engine, zero TypeScript knowledge');
    console.log(chalk.gray('   🌐 GraphQL API:'), 'Language-agnostic workflow definition');
    console.log(chalk.gray('   📜 TypeScript:'), 'Defines domain logic through GraphQL');
    console.log(chalk.gray('   🔄 State Management:'), 'Supports cycles, concurrent places');
    console.log();

    console.log(chalk.green('🎉 TypeScript client demo complete!'));
    console.log(chalk.blue('The same Rust backend serves any language via GraphQL! 🚀'));

  } catch (error: any) {
    console.error(chalk.red('❌ Error:'), error.message);
    console.log();
    console.log(chalk.yellow('💡 Make sure the Rust server is running:'));
    console.log(chalk.gray('   cargo run --bin server'));
    process.exit(1);
  }
}

// TypeScript defines its own domain workflow - sent to generic Rust backend via GraphQL
function createTypeScriptWorkflow(): WorkflowDefinition {
  return {
    name: 'TypeScript Application Development',
    places: [
      'planning',
      'development', 
      'code_review',
      'testing',
      'deployed',
      'maintenance'
    ],
    transitions: [
      {
        id: 'start_development',
        fromPlaces: ['planning'],
        toPlace: 'development',
        conditions: ['requirements_defined', 'tech_stack_chosen']
      },
      {
        id: 'submit_pr',
        fromPlaces: ['development'],
        toPlace: 'code_review',
        conditions: ['code_complete', 'tests_written']
      },
      {
        id: 'approve',
        fromPlaces: ['code_review'],
        toPlace: 'testing',
        conditions: ['code_approved', 'no_conflicts']
      },
      {
        id: 'reject',
        fromPlaces: ['code_review'],
        toPlace: 'development',
        conditions: ['changes_requested']
      },
      {
        id: 'deploy',
        fromPlaces: ['testing'],
        toPlace: 'deployed',
        conditions: ['all_tests_pass', 'security_scan_clean']
      },
      {
        id: 'rollback',
        fromPlaces: ['deployed'],
        toPlace: 'testing',
        conditions: ['production_issues']
      },
      {
        id: 'maintain',
        fromPlaces: ['deployed'],
        toPlace: 'maintenance',
        conditions: ['stable_release']
      },
      {
        id: 'enhance',
        fromPlaces: ['maintenance'],
        toPlace: 'planning',
        conditions: ['new_requirements'] // Cycle back to planning!
      }
    ],
    initialPlace: 'planning'
  };
}

// Run the demo
main().catch(console.error); 