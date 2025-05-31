#!/usr/bin/env npx tsx
// NATS Integration Demo - TypeScript GraphQL Client
// This demonstrates using GraphQL API with NATS storage backend
// Assumes Circuit Breaker server is running with NATS storage
// Run with: npx tsx examples/typescript/nats_demo.ts

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

interface NATSTokenGQL {
  id: string;
  workflowId: string;
  place: string;
  data: any;
  metadata: any;
  createdAt: string;
  updatedAt: string;
  history: HistoryEventGQL[];
  natsSequence?: string;
  natsTimestamp?: string;
  natsSubject?: string;
  transitionHistory: TransitionRecordGQL[];
}

interface TransitionRecordGQL {
  fromPlace: string;
  toPlace: string;
  transitionId: string;
  timestamp: string;
  triggeredBy?: string;
  natsSequence?: string;
  metadata?: any;
}

interface HistoryEventGQL {
  timestamp: string;
  transition: string;
  fromPlace: string;
  toPlace: string;
  data?: any;
}

interface CreateWorkflowInstanceInput {
  workflowId: string;
  initialData?: any;
  metadata?: any;
  triggeredBy?: string;
}

interface TransitionTokenWithNATSInput {
  tokenId: string;
  transitionId: string;
  newPlace: string;
  triggeredBy?: string;
  data?: any;
}

class CircuitBreakerNATSClient {
  constructor(private baseUrl: string = 'http://localhost:4000') {}

  private async graphqlRequest<T>(query: string, variables?: any): Promise<GraphQLResponse<T>> {
    const response = await fetch(`${this.baseUrl}/graphql`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ query, variables }),
    });

    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }

    return response.json() as Promise<GraphQLResponse<T>>;
  }

  private handleErrors<T>(response: GraphQLResponse<T>): T {
    if (response.errors) {
      throw new Error(`GraphQL errors: ${JSON.stringify(response.errors)}`);
    }
    if (!response.data) {
      throw new Error('No data in GraphQL response');
    }
    return response.data;
  }

  async createWorkflow(input: {
    name: string;
    description?: string;
    places: string[];
    initialPlace: string;
    transitions: Array<{
      id: string;
      fromPlaces: string[];
      toPlace: string;
      conditions?: string[];
      description?: string;
    }>;
  }): Promise<WorkflowGQL> {
    const query = `
      mutation CreateWorkflow($input: WorkflowDefinitionInput!) {
        createWorkflow(input: $input) {
          id
          name
          places
          initialPlace
          transitions {
            id
            fromPlaces
            toPlace
            conditions
            description
          }
          createdAt
          updatedAt
        }
      }
    `;

    const response = await this.graphqlRequest<{ createWorkflow: WorkflowGQL }>(query, { input });
    return this.handleErrors(response).createWorkflow;
  }

  async createWorkflowInstance(input: CreateWorkflowInstanceInput): Promise<NATSTokenGQL> {
    const query = `
      mutation CreateWorkflowInstance($input: CreateWorkflowInstanceInput!) {
        createWorkflowInstance(input: $input) {
          id
          workflowId
          place
          data
          metadata
          createdAt
          updatedAt
          natsSequence
          natsTimestamp
          natsSubject
          transitionHistory {
            fromPlace
            toPlace
            transitionId
            timestamp
            triggeredBy
            natsSequence
            metadata
          }
        }
      }
    `;

    const response = await this.graphqlRequest<{ createWorkflowInstance: NATSTokenGQL }>(query, { input });
    return this.handleErrors(response).createWorkflowInstance;
  }

  async transitionTokenWithNats(input: TransitionTokenWithNATSInput): Promise<NATSTokenGQL> {
    const query = `
      mutation TransitionTokenWithNATS($input: TransitionTokenWithNATSInput!) {
        transitionTokenWithNats(input: $input) {
          id
          place
          data
          natsSequence
          natsTimestamp
          transitionHistory {
            fromPlace
            toPlace
            transitionId
            timestamp
            triggeredBy
            natsSequence
          }
        }
      }
    `;

    const response = await this.graphqlRequest<{ transitionTokenWithNats: NATSTokenGQL }>(query, { input });
    return this.handleErrors(response).transitionTokenWithNats;
  }

  async getTokensInPlace(workflowId: string, placeId: string): Promise<NATSTokenGQL[]> {
    const query = `
      query TokensInPlace($workflowId: String!, $placeId: String!) {
        tokensInPlace(workflowId: $workflowId, placeId: $placeId) {
          id
          place
          data
          natsSequence
          natsSubject
          transitionHistory {
            fromPlace
            toPlace
            timestamp
            triggeredBy
          }
        }
      }
    `;

    const response = await this.graphqlRequest<{ tokensInPlace: NATSTokenGQL[] }>(query, { 
      workflowId, 
      placeId 
    });
    return this.handleErrors(response).tokensInPlace;
  }

  async getNATSToken(id: string): Promise<NATSTokenGQL | null> {
    const query = `
      query GetNATSToken($id: String!) {
        natsToken(id: $id) {
          id
          workflowId
          place
          data
          natsSequence
          natsTimestamp
          natsSubject
          transitionHistory {
            fromPlace
            toPlace
            transitionId
            timestamp
            triggeredBy
            natsSequence
            metadata
          }
        }
      }
    `;

    const response = await this.graphqlRequest<{ natsToken: NATSTokenGQL | null }>(query, { id });
    return this.handleErrors(response).natsToken;
  }
}

async function runNATSWorkflowDemo(): Promise<void> {
  console.log('📋 Creating workflow with NATS storage backend...');
  
  const client = new CircuitBreakerNATSClient();

  try {
    // Step 1: Create a workflow definition
    const workflow = await client.createWorkflow({
      name: 'NATS Document Review Process (TypeScript)',
      description: 'A document review workflow using NATS streaming backend',
      places: ['draft', 'review', 'approved', 'published', 'rejected'],
      initialPlace: 'draft',
      transitions: [
        {
          id: 'submit_for_review',
          fromPlaces: ['draft'],
          toPlace: 'review',
          conditions: [],
          description: 'Submit document for review'
        },
        {
          id: 'approve',
          fromPlaces: ['review'],
          toPlace: 'approved',
          conditions: [],
          description: 'Approve the document'
        },
        {
          id: 'reject',
          fromPlaces: ['review'],
          toPlace: 'rejected',
          conditions: [],
          description: 'Reject the document'
        },
        {
          id: 'publish',
          fromPlaces: ['approved'],
          toPlace: 'published',
          conditions: [],
          description: 'Publish the document'
        }
      ]
    });

    console.log(`✅ Created workflow: "${workflow.name}" (ID: ${workflow.id})`);

    // Brief delay to ensure workflow is fully persisted in NATS
    console.log('⏳ Waiting for NATS persistence...');
    await new Promise(resolve => setTimeout(resolve, 500));

    // Step 2: Create workflow instances using NATS-enhanced mutations
    console.log('\n📄 Creating workflow instances with NATS tracking...');

    const instances = [
      { title: 'TypeScript Technical Specification', department: 'engineering' },
      { title: 'TypeScript Marketing Proposal', department: 'marketing' },
      { title: 'TypeScript Legal Contract', department: 'legal' }
    ];

    const tokenIds: string[] = [];

    for (const { title, department } of instances) {
      try {
        const token = await client.createWorkflowInstance({
          workflowId: workflow.id,
          initialData: {
            title,
            content: `This is the TypeScript content for ${title}`,
            priority: 'medium'
          },
          metadata: {
            department,
            created_by: 'typescript_demo_user',
            urgency: 'normal'
          },
          triggeredBy: 'typescript_nats_demo'
        });

        tokenIds.push(token.id);

        console.log(`📝 Created instance: ${title} (Token: ${token.id})`);
        console.log(`   📍 Place: ${token.place}`);
        console.log(`   🔗 NATS Subject: ${token.natsSubject || 'N/A'}`);
        
        if (token.natsSequence) {
          console.log(`   📊 NATS Sequence: ${token.natsSequence}`);
        }
      } catch (error) {
        console.error(`❌ Failed to create instance for ${title}:`, error);
      }
    }

    // Step 3: Query tokens in specific places using NATS-optimized queries
    console.log('\n🔍 Querying tokens in \'draft\' place using NATS...');

    try {
      const tokensInDraft = await client.getTokensInPlace(workflow.id, 'draft');
      console.log(`📊 Found ${tokensInDraft.length} tokens in 'draft' place`);

      for (const token of tokensInDraft) {
        const title = token.data?.title || 'Unknown';
        console.log(`   🎫 Token ${token.id}: ${title}`);
      }
    } catch (error) {
      console.error('❌ Failed to query tokens in place:', error);
    }

    // Step 4: Perform transitions with NATS event tracking
    console.log('\n⚡ Performing transitions with NATS event tracking...');

    if (tokenIds.length > 0) {
      const firstTokenId = tokenIds[0];
      try {
        const transitionedToken = await client.transitionTokenWithNats({
          tokenId: firstTokenId,
          transitionId: 'submit_for_review',
          newPlace: 'review',
          triggeredBy: 'typescript_nats_demo_transition',
          data: {
            reviewed_by: 'typescript_demo_reviewer',
            review_notes: 'Ready for review from TypeScript'
          }
        });

        console.log(`✅ Transitioned token ${firstTokenId} to place: ${transitionedToken.place}`);

        const history = transitionedToken.transitionHistory;
        if (history && history.length > 0) {
          const lastTransition = history[history.length - 1];
          console.log(`   📈 Transition: ${lastTransition.fromPlace} → ${lastTransition.toPlace}`);
          console.log(`   👤 Triggered by: ${lastTransition.triggeredBy || 'Unknown'}`);
          if (lastTransition.natsSequence) {
            console.log(`   📊 NATS Sequence: ${lastTransition.natsSequence}`);
          }
        }
      } catch (error) {
        console.error('❌ Failed to perform transition:', error);
      }
    }

    // Step 5: Demonstrate NATS-enhanced token retrieval
    console.log('\n🔎 Retrieving token with NATS metadata...');

    if (tokenIds.length > 0) {
      const tokenId = tokenIds[0];
      try {
        const natsToken = await client.getNATSToken(tokenId);
        if (natsToken) {
          console.log('🎫 NATS Token Details:');
          console.log(`   📋 ID: ${natsToken.id}`);
          console.log(`   📍 Current Place: ${natsToken.place}`);
          console.log(`   🔗 NATS Subject: ${natsToken.natsSubject || 'N/A'}`);
          
          if (natsToken.natsSequence) {
            console.log(`   📊 NATS Sequence: ${natsToken.natsSequence}`);
          }
          
          if (natsToken.natsTimestamp) {
            console.log(`   ⏰ NATS Timestamp: ${natsToken.natsTimestamp}`);
          }
          
          const history = natsToken.transitionHistory || [];
          console.log(`   📈 Transition History (${history.length} events):`);
          
          history.forEach((transition, i) => {
            console.log(`      ${i + 1}. ${transition.fromPlace} → ${transition.toPlace} (${transition.transitionId})`);
            if (transition.triggeredBy) {
              console.log(`         👤 Triggered by: ${transition.triggeredBy}`);
            }
          });
        } else {
          console.log('❌ Token not found');
        }
      } catch (error) {
        console.error('❌ Failed to get NATS token:', error);
      }
    }

    console.log('\n🎉 TypeScript NATS Integration Demo Features Demonstrated:');
    console.log('   ✅ NATS JetStream storage backend (server-side)');
    console.log('   ✅ Automatic stream creation per workflow');
    console.log('   ✅ Enhanced token tracking with NATS metadata');
    console.log('   ✅ Event-driven transition recording');
    console.log('   ✅ Efficient place-based token queries');
    console.log('   ✅ Real-time transition history with NATS sequences');
    console.log('   ✅ GraphQL API integration with NATS storage');
    console.log('   ✅ TypeScript client library for NATS workflows');

  } catch (error) {
    console.error('❌ Demo failed:', error);
    throw error;
  }
}

async function main(): Promise<void> {
  console.log('🚀 Circuit Breaker NATS Integration Demo (TypeScript Client)');
  console.log('=============================================================');
  console.log('This demo assumes the Circuit Breaker server is running with NATS storage');
  console.log('Start the server with NATS storage before running this demo\n');

  // Give user time to read the message
  await new Promise(resolve => setTimeout(resolve, 2000));

  try {
    await runNATSWorkflowDemo();
    console.log('\n✅ TypeScript NATS integration demo completed successfully!');
  } catch (error) {
    console.error('\n❌ Demo failed:', error);
    console.log('💡 Make sure the Circuit Breaker server is running on localhost:4000 with NATS storage');
    process.exit(1);
  }
}

// Run the demo
if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch(console.error);
}