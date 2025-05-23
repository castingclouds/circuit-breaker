// TypeScript client demonstrating Circuit Breaker token lifecycle management
// This shows rich token data and metadata flowing through state transitions
// Run with: npm run start:demo

import { GraphQLClient, gql } from 'graphql-request';
import chalk from 'chalk';

// GraphQL endpoint (assumes Rust server is running)
const GRAPHQL_ENDPOINT = 'http://localhost:4000/graphql';

interface WorkflowDefinition {
  id?: string;
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
  createdAt?: string;
  updatedAt?: string;
}

interface HistoryEvent {
  transition: string;
  fromPlace: string;
  toPlace: string;
  timestamp: string;
}

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

interface FireTransitionResponse {
  fireTransition: Token;
}

interface GetTokenResponse {
  token: Token;
}

async function main() {
  console.log(chalk.cyan('🌐 Circuit Breaker - TypeScript Token Demo'));
  console.log(chalk.cyan('==========================================='));
  console.log(chalk.green('Client-side token operations via GraphQL → Generic Rust Backend'));
  console.log();

  try {
    // Connect to the generic Rust backend
    const client = new GraphQLClient(GRAPHQL_ENDPOINT);
    console.log(chalk.blue('🔌 Connected to Rust backend at'), GRAPHQL_ENDPOINT);
    console.log();

    // 1. Create a TypeScript-specific workflow via GraphQL
    const workflowDefinition = createContentWorkflow();
    console.log(chalk.yellow('📋 TypeScript-Defined Content Workflow:'));
    console.log(chalk.gray('   Name:'), workflowDefinition.name);
    console.log(chalk.gray('   Places:'), workflowDefinition.places.join(' → '));
    console.log(chalk.gray('   Transitions:'), workflowDefinition.transitions.map(t => t.id).join(', '));
    console.log();

    // 2. Register workflow with the generic Rust backend
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

    console.log(chalk.green('✅ Workflow created with ID:'), workflowResult.createWorkflow.id);
    console.log();

    // 3. Create a token with TypeScript-specific data
    const createTokenMutation = gql`
      mutation CreateToken($input: TokenCreateInput!) {
        createToken(input: $input) {
          id
          place
          workflowId
          createdAt
        }
      }
    `;

    // Rich content creation data - this would come from your TypeScript application
    const contentData = {
      title: "The Future of State Managed Workflows",
      author: "Circuit Breaker Team",
      contentType: "technical_article",
      targetAudience: "developers",
      estimatedLength: 2500,
      keywords: ["workflow", "state management", "GraphQL", "Rust"],
      priority: "high",
      deadline: "2024-02-15",
      assignedTo: "content-team-alpha"
    };

    const contentMetadata = {
      department: "engineering",
      project: "circuit-breaker-docs",
      version: "1.0",
      language: "english",
      format: "markdown",
      reviewers: ["technical-lead", "content-manager"],
      distribution: ["website", "newsletter", "social"]
    };

    console.log(chalk.blue('🎯 Creating content token with rich TypeScript data...'));
    const tokenResult = await client.request<CreateTokenResponse>(createTokenMutation, {
      input: {
        workflowId: workflowResult.createWorkflow.id,
        data: contentData,
        metadata: contentMetadata
      }
    });

    console.log(chalk.green('✅ Content token created:'), tokenResult.createToken.id);
    console.log(chalk.gray('   Title:'), contentData.title);
    console.log(chalk.gray('   Current place:'), tokenResult.createToken.place);
    console.log(chalk.gray('   Created:'), new Date(tokenResult.createToken.createdAt!).toLocaleString());
    console.log();

    // 4. Execute content creation workflow
    const fireTransitionMutation = gql`
      mutation FireTransition($input: TransitionFireInput!) {
        fireTransition(input: $input) {
          id
          place
          updatedAt
          history {
            transition
            fromPlace
            toPlace
            timestamp
          }
        }
      }
    `;

    console.log(chalk.blue('🔄 Executing content creation workflow...'));
    
    // Define the workflow transitions with rich data updates
    const workflowSteps = [
      {
        transition: 'start_research',
        description: 'Begin research phase',
        data: {
          researchSources: ['technical_papers', 'github_repos', 'community_discussions'],
          researchProgress: 0,
          keyFindings: []
        }
      },
      {
        transition: 'create_outline',
        description: 'Create content outline',
        data: {
          researchProgress: 100,
          keyFindings: ['GraphQL flexibility', 'Rust performance', 'State management benefits'],
          outline: {
            sections: ['Introduction', 'Core Concepts', 'Implementation', 'Comparison', 'Conclusion'],
            estimatedSections: 5,
            approxWordsPerSection: 500
          }
        }
      },
      {
        transition: 'write_draft',
        description: 'Write initial draft',
        data: {
          wordCount: 1200,
          sectionsComplete: 3,
          codeExamples: ['Rust workflow definition', 'GraphQL mutation', 'TypeScript client'],
          reviewNotes: []
        }
      },
      {
        transition: 'ai_review',
        description: 'Submit for AI review',
        data: {
          wordCount: 2100,
          sectionsComplete: 5,
          aiReviewScore: 8.5,
          suggestedImprovements: ['Add more examples', 'Clarify performance claims'],
          grammarIssues: 3,
          readabilityScore: 'high'
        }
      }
    ];

    let currentToken = tokenResult.createToken;

    for (const step of workflowSteps) {
      console.log(chalk.yellow(`   ➡️  ${step.description} (${step.transition})`));
      
      try {
        const transitionResult = await client.request<FireTransitionResponse>(fireTransitionMutation, {
          input: {
            tokenId: currentToken.id,
            transitionId: step.transition,
            data: step.data // Use step data directly instead of spreading
          }
        });

        currentToken = transitionResult.fireTransition;
        console.log(chalk.green(`   ✅ Moved to place: ${currentToken.place}`));
        console.log(chalk.gray(`      Updated: ${new Date(currentToken.updatedAt!).toLocaleTimeString()}`));
      } catch (error: any) {
        console.log(chalk.red(`   ❌ Transition failed: ${error.message}`));
        break;
      }
    }

    console.log();

    // 5. Demonstrate revision cycle (AI suggests improvements)
    console.log(chalk.blue('🔄 Demonstrating revision cycle...'));
    try {
      console.log(chalk.yellow('   ➡️  AI requests revision (request_revision)'));
      const revisionResult = await client.request<FireTransitionResponse>(fireTransitionMutation, {
        input: {
          tokenId: currentToken.id,
          transitionId: 'request_revision',
          data: {
            revisionReason: 'Add performance benchmarks and more code examples',
            aiSuggestions: [
              'Include benchmark comparison with DAG systems',
              'Add TypeScript client integration example',
              'Expand on Petri Net mathematical foundations'
            ],
            priority: 'medium',
            estimatedRevisionTime: '4 hours'
          }
        }
      });

      currentToken = revisionResult.fireTransition;
      console.log(chalk.green(`   ✅ Back to place: ${currentToken.place} (revision cycle!)`));
    } catch (error: any) {
      console.log(chalk.red(`   ❌ Revision failed: ${error.message}`));
    }

    console.log();

    // 6. Query final token state with complete history
    const getTokenQuery = gql`
      query GetToken($id: ID!) {
        token(id: $id) {
          id
          place
          workflowId
          data
          metadata
          createdAt
          updatedAt
          history {
            transition
            fromPlace
            toPlace
            timestamp
          }
        }
      }
    `;

    console.log(chalk.blue('📚 Fetching complete token lifecycle...'));
    const finalResult = await client.request<GetTokenResponse>(getTokenQuery, {
      id: currentToken.id
    });

    const finalToken = finalResult.token;

    console.log(chalk.green('📈 Complete Workflow History:'));
    finalToken.history?.forEach((event: HistoryEvent, index: number) => {
      const time = new Date(event.timestamp).toLocaleTimeString();
      console.log(chalk.gray(`   ${index + 1}. ${event.fromPlace} → ${event.toPlace} via "${event.transition}" at ${time}`));
    });

    console.log();
    console.log(chalk.green('🎯 Final Token State:'));
    console.log(chalk.gray('   ID:'), finalToken.id);
    console.log(chalk.gray('   Current Place:'), finalToken.place);
    console.log(chalk.gray('   Workflow:'), finalToken.workflowId);
    console.log(chalk.gray('   Created:'), new Date(finalToken.createdAt!).toLocaleString());
    console.log(chalk.gray('   Last Updated:'), new Date(finalToken.updatedAt!).toLocaleString());

    console.log();
    console.log(chalk.green('📊 Token Data (Latest State):'));
    console.log(chalk.gray(JSON.stringify(finalToken.data, null, 2)));

    console.log();
    console.log(chalk.green('🏷️  Token Metadata:'));
    console.log(chalk.gray(JSON.stringify(finalToken.metadata, null, 2)));

    console.log();

    // 7. Architecture summary
    console.log(chalk.magenta('🏗️  TypeScript Token Demo Summary:'));
    console.log(chalk.gray('   📦 Rich Data:'), 'Tokens carry complex application state');
    console.log(chalk.gray('   🔄 State Transitions:'), 'Business logic flows through GraphQL');
    console.log(chalk.gray('   📚 Complete History:'), 'Full audit trail of all state changes');
    console.log(chalk.gray('   🔁 Cycles Supported:'), 'Revision loops work naturally');
    console.log(chalk.gray('   🌐 Language Agnostic:'), 'Same backend serves all languages');
    console.log();

    console.log(chalk.green('🎉 TypeScript token lifecycle demo complete!'));
    console.log(chalk.blue('Rich stateful workflows with GraphQL + Rust backend! 🚀'));

  } catch (error: any) {
    console.error(chalk.red('❌ Error:'), error.message);
    console.log();
    console.log(chalk.yellow('💡 Make sure the Rust server is running:'));
    console.log(chalk.gray('   cargo run --bin server'));
    process.exit(1);
  }
}

// TypeScript-specific workflow definition (sent to generic Rust backend)
function createContentWorkflow(): WorkflowDefinition {
  return {
    name: 'AI-Powered Content Creation Workflow',
    places: [
      'planning',
      'research',
      'outline',
      'draft',
      'ai_review',
      'human_review',
      'published',
      'archived'
    ],
    transitions: [
      {
        id: 'start_research',
        fromPlaces: ['planning'],
        toPlace: 'research',
        conditions: ['topic_defined', 'audience_identified']
      },
      {
        id: 'create_outline',
        fromPlaces: ['research'],
        toPlace: 'outline',
        conditions: ['research_complete', 'sources_validated']
      },
      {
        id: 'write_draft',
        fromPlaces: ['outline'],
        toPlace: 'draft',
        conditions: ['outline_approved', 'style_guide_reviewed']
      },
      {
        id: 'ai_review',
        fromPlaces: ['draft'],
        toPlace: 'ai_review',
        conditions: ['draft_complete', 'word_count_met']
      },
      {
        id: 'request_revision',
        fromPlaces: ['ai_review'],
        toPlace: 'draft',
        conditions: ['ai_feedback_provided'] // Cycle back for revisions!
      },
      {
        id: 'human_review',
        fromPlaces: ['ai_review'],
        toPlace: 'human_review',
        conditions: ['ai_approval', 'fact_check_passed']
      },
      {
        id: 'approve_content',
        fromPlaces: ['human_review'],
        toPlace: 'published',
        conditions: ['editorial_approval', 'legal_cleared']
      },
      {
        id: 'reject_content',
        fromPlaces: ['human_review'],
        toPlace: 'draft',
        conditions: ['revision_required'] // Another cycle!
      },
      {
        id: 'archive',
        fromPlaces: ['published'],
        toPlace: 'archived',
        conditions: ['content_outdated']
      }
    ],
    initialPlace: 'planning'
  };
}

// Run the demo
main().catch(console.error); 