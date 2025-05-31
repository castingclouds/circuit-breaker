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

  // Helper function to pause for demonstrations
  private async pauseForDemo(message: string): Promise<void> {
    console.log(`\n⏸️  ${message}`);
    console.log('   Press Enter to continue...');
    
    // Wait for user input
    await new Promise(resolve => {
      process.stdin.once('data', () => resolve(void 0));
    });
  }

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
  console.log('This demo will walk you through each step of NATS integration.\n');
  
  const client = new CircuitBreakerNATSClient();

  // Enable raw mode for better input handling
  if (process.stdin.isTTY) {
    process.stdin.setRawMode(false);
  }

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
    console.log('\n🔍 What just happened:');
    console.log('   • Workflow definition was sent via GraphQL to the Circuit Breaker server');
    console.log('   • Server stored the workflow in NATS JetStream with subject: cb.workflows.{id}.definition');
    console.log('   • NATS stream "CIRCUIT_BREAKER_GLOBAL" now contains this workflow definition');
    
    await (client as any).pauseForDemo('STEP 1 COMPLETE: Workflow created and stored in NATS');

    // Brief delay to ensure workflow is fully persisted in NATS
    console.log('⏳ Waiting for NATS persistence...');
    await new Promise(resolve => setTimeout(resolve, 500));

    // Step 2: Create workflow instances using NATS-enhanced mutations
    console.log('\n📄 Creating workflow instances with NATS tracking...');
    console.log('🔍 About to demonstrate:');
    console.log('   • NATS-enhanced GraphQL mutation: createWorkflowInstance');
    console.log('   • Each token will be stored as a message in NATS with metadata');
    console.log('   • Real-time event publishing to NATS subjects');
    
    await (client as any).pauseForDemo('Ready to create workflow instances with NATS tracking');

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
        
        console.log(`   🔍 Debug: Token ID added to list: ${token.id}`);
        console.log('   ✨ This token is now persisted in NATS JetStream!');
      } catch (error) {
        console.error(`❌ Failed to create instance for ${title}:`, error);
      }
    }

    // Add verification step
    console.log('\n🔍 Verifying all tokens were created successfully...');
    console.log(`📊 Created ${tokenIds.length} tokens with IDs:`);
    tokenIds.forEach((id, index) => {
      console.log(`   ${index + 1}. ${id}`);
    });

    await (client as any).pauseForDemo('STEP 2 COMPLETE: All workflow instances created and stored in NATS');

    // Step 3: Query tokens in specific places using NATS-optimized queries
    console.log('\n🔍 Querying tokens in \'draft\' place using NATS...');
    console.log('🔍 About to demonstrate:');
    console.log('   • NATS-optimized GraphQL query: tokensInPlace');
    console.log('   • Efficient filtering using NATS subject patterns');
    console.log('   • Retrieving tokens from specific workflow places');
    
    await (client as any).pauseForDemo('Ready to query tokens using NATS-optimized operations');

    try {
      const tokensInDraft = await client.getTokensInPlace(workflow.id, 'draft');
      console.log(`📊 Found ${tokensInDraft.length} tokens in 'draft' place`);

      for (const token of tokensInDraft) {
        const title = token.data?.title || 'Unknown';
        console.log(`   🎫 Token ${token.id}: ${title}`);
      }
      
      console.log('\n✨ These results came directly from NATS JetStream!');
      console.log('   • Query used NATS subject filtering: cb.workflows.{id}.places.draft.tokens');
      console.log('   • Much faster than scanning all tokens in traditional databases');
    } catch (error) {
      console.error('❌ Failed to query tokens in place:', error);
    }

    await (client as any).pauseForDemo('STEP 3 COMPLETE: Successfully queried tokens using NATS');

    // Step 4: Perform transitions with NATS event tracking
    console.log('\n⚡ Performing transitions with NATS event tracking...');
    console.log('🔍 About to demonstrate:');
    console.log('   • NATS-enhanced transition: transitionTokenWithNats');
    console.log('   • Real-time event publishing to transition event streams');
    console.log('   • Automatic NATS metadata tracking (sequences, timestamps)');
    console.log('   • Moving the FIRST token from "draft" to "review" place');
    console.log('   • (Note: Only transitioning one token to keep demo focused)');
    
    await (client as any).pauseForDemo('Ready to perform a NATS-tracked token transition on the first token');

    if (tokenIds.length > 0) {
      const firstTokenId = tokenIds[0];
      console.log(`🔍 Debug: Attempting to transition the first token ID: ${firstTokenId}`);
      
      // Add a small delay to ensure token is fully persisted
      await new Promise(resolve => setTimeout(resolve, 1000));
      
      try {
        // First, let's verify the token exists by querying it
        console.log('🔍 Verifying token exists before transition...');
        const existingToken = await client.getNATSToken(firstTokenId);
        
        let transitionedToken: NATSTokenGQL;
        let actualTokenId = firstTokenId;
        
        if (!existingToken) {
          console.log('❌ Token not found in NATS storage. Available tokens:');
          const allDraftTokens = await client.getTokensInPlace(workflow.id, 'draft');
          allDraftTokens.forEach(token => {
            console.log(`   🎫 Available token: ${token.id}`);
          });
          
          if (allDraftTokens.length > 0) {
            console.log('🔄 Using first available token from place query instead...');
            actualTokenId = allDraftTokens[0].id;
            tokenIds[0] = actualTokenId; // Update our list
            
            transitionedToken = await client.transitionTokenWithNats({
              tokenId: actualTokenId,
              transitionId: 'submit_for_review',
              newPlace: 'review',
              triggeredBy: 'typescript_nats_demo_transition',
              data: {
                reviewed_by: 'typescript_demo_reviewer',
                review_notes: 'Ready for review from TypeScript'
              }
            });
          } else {
            throw new Error('No tokens available for transition');
          }
        } else {
          console.log('✅ Token found, proceeding with transition...');
          transitionedToken = await client.transitionTokenWithNats({
            tokenId: firstTokenId,
            transitionId: 'submit_for_review',
            newPlace: 'review',
            triggeredBy: 'typescript_nats_demo_transition',
            data: {
              reviewed_by: 'typescript_demo_reviewer',
              review_notes: 'Ready for review from TypeScript'
            }
          });
        }

        console.log(`✅ Transitioned token ${actualTokenId} to place: ${transitionedToken.place}`);

        const history = transitionedToken.transitionHistory;
        if (history && history.length > 0) {
          const lastTransition = history[history.length - 1];
          console.log(`   📈 Transition: ${lastTransition.fromPlace} → ${lastTransition.toPlace}`);
          console.log(`   👤 Triggered by: ${lastTransition.triggeredBy || 'Unknown'}`);
          if (lastTransition.natsSequence) {
            console.log(`   📊 NATS Sequence: ${lastTransition.natsSequence}`);
          }
        }
        
        console.log('\n✨ Transition completed with full NATS event tracking!');
        console.log('   • Transition event published to: cb.workflows.{id}.events.transitions');
        console.log('   • Token moved to new NATS subject: cb.workflows.{id}.places.review.tokens');
        console.log('   • All changes are now persistent in NATS JetStream');
        console.log('   • NOTE: The other tokens remain in "draft" state (only first token was transitioned)');
      } catch (error) {
        console.error('❌ Failed to perform transition:', error);
        console.log('💡 This might be due to timing issues with NATS persistence or token ID mismatch');
      }
    }

    await (client as any).pauseForDemo('STEP 4 COMPLETE: Token transition with NATS event tracking');

    // Step 5: Demonstrate NATS-enhanced token retrieval
    console.log('\n🔎 Retrieving token with NATS metadata...');
    console.log('🔍 About to demonstrate:');
    console.log('   • Enhanced token retrieval with full NATS metadata');
    console.log('   • Complete transition history with NATS sequences');
    console.log('   • Real-time timestamps from NATS JetStream');
    
    await (client as any).pauseForDemo('Ready to retrieve token with complete NATS metadata');

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
            if (transition.natsSequence) {
              console.log(`         📊 NATS Sequence: ${transition.natsSequence}`);
            }
          });
          
          console.log('\n✨ Complete audit trail stored in NATS!');
          console.log('   • Every transition is immutably recorded');
          console.log('   • NATS sequence numbers provide ordering guarantees');
          console.log('   • Distributed teams can see real-time workflow progress');
        } else {
          console.log('❌ Token not found');
        }
      } catch (error) {
        console.error('❌ Failed to get NATS token:', error);
      }
    }

    await (client as any).pauseForDemo('STEP 5 COMPLETE: Retrieved token with full NATS metadata');

    console.log('\n🎉 TypeScript NATS Integration Demo Features Demonstrated:');
    console.log('   ✅ NATS JetStream storage backend (server-side)');
    console.log('   ✅ Automatic stream creation per workflow');
    console.log('   ✅ Enhanced token tracking with NATS metadata');
    console.log('   ✅ Event-driven transition recording');
    console.log('   ✅ Efficient place-based token queries');
    console.log('   ✅ Real-time transition history with NATS sequences');
    console.log('   ✅ GraphQL API integration with NATS storage');
    console.log('   ✅ TypeScript client library for NATS workflows');
    
    console.log('\n🚀 NATS Benefits Demonstrated:');
    console.log('   🔄 Distributed: Multiple services can connect to the same NATS cluster');
    console.log('   💾 Persistent: All data survives server restarts');
    console.log('   ⚡ Fast: Subject-based filtering is extremely efficient');
    console.log('   🔒 Reliable: Built-in acknowledgments and replay capability');
    console.log('   📈 Scalable: Handles millions of messages per second');
    
    await (client as any).pauseForDemo('DEMO COMPLETE: All NATS integration features demonstrated');

  } catch (error) {
    console.error('❌ Demo failed:', error);
    throw error;
  }
}

async function main(): Promise<void> {
  console.log('🚀 Circuit Breaker NATS Integration Demo (TypeScript Client)');
  console.log('=============================================================');
  console.log('This interactive demo will walk you through NATS integration step-by-step.');
  console.log('');
  console.log('📋 Prerequisites:');
  console.log('   1. NATS server running: docker run -p 4222:4222 -p 8222:8222 nats:alpine --jetstream --http_port 8222');
  console.log('   2. Circuit Breaker server with NATS: export STORAGE_BACKEND=nats && cargo run --bin server');
  console.log('   3. Server should be running on localhost:4000');
  console.log('');
  console.log('🎯 What you\'ll see:');
  console.log('   • Live workflow creation and storage in NATS JetStream');
  console.log('   • Real-time token operations with NATS metadata tracking');
  console.log('   • Efficient place-based queries using NATS subject patterns');
  console.log('   • Event-driven transitions with complete audit trails');
  console.log('   • Polyglot architecture: TypeScript client → GraphQL → NATS-powered Rust backend');
  console.log('');
  console.log('⏸️  Ready to begin the demo?');
  console.log('   Press Enter to start...');
  
  // Wait for user input to start
  await new Promise(resolve => {
    process.stdin.once('data', () => resolve(void 0));
  });

  try {
    await runNATSWorkflowDemo();
    console.log('\n✅ TypeScript NATS integration demo completed successfully!');
    console.log('');
    console.log('🎓 What you learned:');
    console.log('   • How NATS JetStream provides distributed workflow storage');
    console.log('   • Real-time event publishing and consumption patterns');
    console.log('   • Efficient querying using NATS subject hierarchies');
    console.log('   • Complete audit trails with immutable event sequences');
    console.log('   • Polyglot workflow architecture benefits');
    console.log('');
    console.log('🔗 Next Steps:');
    console.log('   • Explore the NATS admin interface: http://localhost:8222');
    console.log('   • Try the Rust demo: cargo run --example nats_demo');
    console.log('   • Check the documentation: docs/NATS_IMPLEMENTATION.md');
    console.log('');
    console.log('⏸️  Demo session ending...');
    console.log('   Press Enter to exit.');
    
    // Final pause
    await new Promise(resolve => {
      process.stdin.once('data', () => resolve(void 0));
    });
    
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