#!/usr/bin/env npx tsx
// Simplified Places AI Agent Demo - TypeScript Client
// Demonstrates basic workflow operations with the current GraphQL schema
// Run with: npx tsx places_ai_agent_demo_simple.ts

import { config } from 'dotenv';
import { resolve } from 'path';

// Load environment variables from .env file in project root
config({ path: resolve(process.cwd(), '../../.env') });

interface GraphQLResponse<T = any> {
  data?: T;
  errors?: Array<{ message: string; locations?: any[]; path?: any[] }>;
}

interface Workflow {
  id: string;
  name: string;
  places: string[];
  transitions: Transition[];
}

interface Transition {
  id: string;
  fromPlaces: string[];
  toPlace: string;
  conditions: string[];
}

interface Token {
  id: string;
  workflowId: string;
  place: string;
  data: Record<string, any>;
  metadata: Record<string, any>;
  history: HistoryEvent[];
}

interface HistoryEvent {
  transition: string;
  fromPlace: string;
  toPlace: string;
  timestamp: string;
}

class SimplePlacesAIClient {
  private baseUrl: string;
  private headers: Record<string, string>;

  constructor(baseUrl?: string) {
    this.baseUrl = baseUrl || process.env.GRAPHQL_ENDPOINT || 'http://localhost:4000';
    this.headers = {
      'Content-Type': 'application/json',
      'User-Agent': 'Circuit-Breaker-Simple-Client/1.0',
    };
  }

  async request<T = any>(query: string, variables?: any): Promise<GraphQLResponse<T>> {
    try {
      const url = this.baseUrl.endsWith('/graphql') ? this.baseUrl : `${this.baseUrl}/graphql`;
      console.log(`🌐 Making GraphQL request to: ${url}`);
      console.log(`📤 Query: ${query.substring(0, 100)}...`);
      
      const response = await fetch(url, {
        method: 'POST',
        headers: this.headers,
        body: JSON.stringify({ query, variables }),
      });

      console.log(`📥 Response status: ${response.status} ${response.statusText}`);

      if (!response.ok) {
        const errorText = await response.text();
        console.error(`❌ HTTP error details: ${errorText}`);
        throw new Error(`HTTP error! status: ${response.status} - ${errorText}`);
      }

      const result = await response.json();
      
      if (result.errors) {
        console.error('❌ GraphQL errors:', result.errors);
      } else {
        console.log('✅ GraphQL request successful');
      }

      return result;
    } catch (error) {
      console.error('❌ GraphQL request failed:', error);
      throw error;
    }
  }

  async createWorkflow(id: string, name: string): Promise<Workflow> {
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
        }
      }
    `;

    const input = {
      name,
      initialPlace: 'pending_classification',
      places: [
        'pending_classification',
        'classified', 
        'pending_review',
        'reviewed',
        'published'
      ],
      transitions: [
        {
          id: 'classify',
          fromPlaces: ['pending_classification'],
          toPlace: 'classified',
          conditions: []
        },
        {
          id: 'review',
          fromPlaces: ['classified'],
          toPlace: 'pending_review',
          conditions: []
        },
        {
          id: 'approve',
          fromPlaces: ['pending_review'],
          toPlace: 'reviewed',
          conditions: []
        },
        {
          id: 'publish',
          fromPlaces: ['reviewed'],
          toPlace: 'published',
          conditions: []
        }
      ]
    };

    const response = await this.request<{ createWorkflow: Workflow }>(mutation, { input });

    if (response.errors) {
      throw new Error(`Failed to create workflow: ${response.errors[0].message}`);
    }

    return response.data!.createWorkflow;
  }

  async createToken(workflowId: string, data: Record<string, any>, metadata: Record<string, any> = {}): Promise<Token> {
    const mutation = `
      mutation CreateToken($input: TokenCreateInput!) {
        createToken(input: $input) {
          id
          workflowId
          place
          data
          metadata
          history {
            transition
            fromPlace
            toPlace
            timestamp
          }
        }
      }
    `;

    const response = await this.request<{ createToken: Token }>(mutation, {
      input: { workflowId, data, metadata },
    });

    if (response.errors) {
      throw new Error(`Failed to create token: ${response.errors[0].message}`);
    }

    return response.data!.createToken;
  }

  async getToken(tokenId: string): Promise<Token | null> {
    const query = `
      query GetToken($id: String!) {
        token(id: $id) {
          id
          workflowId
          place
          data
          metadata
          history {
            transition
            fromPlace
            toPlace
            timestamp
          }
        }
      }
    `;

    const response = await this.request<{ token: Token | null }>(query, { id: tokenId });
    return response.data?.token || null;
  }

  async fireTransition(tokenId: string, transitionId: string): Promise<Token> {
    const mutation = `
      mutation FireTransition($input: TransitionFireInput!) {
        fireTransition(input: $input) {
          id
          workflowId
          place
          data
          metadata
          history {
            transition
            fromPlace
            toPlace
            timestamp
          }
        }
      }
    `;

    const response = await this.request<{ fireTransition: Token }>(mutation, {
      input: { tokenId, transitionId },
    });

    if (response.errors) {
      throw new Error(`Failed to fire transition: ${response.errors[0].message}`);
    }

    return response.data!.fireTransition;
  }

  async listWorkflows(): Promise<Workflow[]> {
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
          }
        }
      }
    `;

    const response = await this.request<{ workflows: Workflow[] }>(query);
    return response.data?.workflows || [];
  }
}

// Demo functions
async function runSimpleDemo() {
  console.log('🚀 Simple Places AI Agent Demo (TypeScript)');
  console.log('===========================================');

  // Check for API key
  if (!process.env.ANTHROPIC_API_KEY || process.env.ANTHROPIC_API_KEY === 'your_anthropic_api_key_here') {
    console.warn('⚠️  Note: ANTHROPIC_API_KEY not configured in .env');
    console.warn('Agent functionality will use placeholder responses');
    console.warn('Configure your API key for real agent execution\n');
  } else {
    console.log('✅ Anthropic API key configured for agent execution\n');
  }

  const client = new SimplePlacesAIClient();

  try {
    // Test connectivity
    console.log('🔍 Testing GraphQL server connectivity...');
    console.log(`   Server URL: ${client['baseUrl']}`);
    
    const workflows = await client.listWorkflows();
    console.log(`✅ Connected! Found ${workflows.length} existing workflows\n`);

    // Test Anthropic agent creation
    console.log('🤖 Testing Anthropic agent creation...');
    const testAgentResult = await client.request(`
      mutation CreateAgent($input: AgentDefinitionInput!) {
        createAgent(input: $input) {
          id
          name
          description
          llmProvider {
            providerType
            model
          }
        }
      }
    `, {
      input: {
        name: "Simple Test Agent",
        description: "Test agent for Anthropic integration",
        llmProvider: {
          providerType: "anthropic",
          model: process.env.ANTHROPIC_DEFAULT_MODEL || "claude-3-sonnet-20240229",
          apiKey: process.env.ANTHROPIC_API_KEY || "demo-key",
          ...(process.env.ANTHROPIC_BASE_URL && { baseUrl: process.env.ANTHROPIC_BASE_URL })
        },
        llmConfig: {
          temperature: 0.7,
          maxTokens: 100,
          topP: 0.9,
          frequencyPenalty: 0.0,
          presencePenalty: 0.0,
          stopSequences: []
        },
        prompts: {
          system: "You are a helpful assistant.",
          userTemplate: "Please respond to: {input}",
          contextInstructions: "Be concise and helpful."
        },
        capabilities: ["text_generation"],
        tools: []
      }
    });

    if (testAgentResult.errors) {
      console.error('❌ Failed to create test agent:', testAgentResult.errors[0].message);
      console.log('Continuing with workflow demo...\n');
    } else {
      console.log(`✅ Created test agent: ${testAgentResult.data.createAgent.id}`);
      console.log(`   Provider: ${testAgentResult.data.createAgent.llmProvider.providerType}`);
      console.log(`   Model: ${testAgentResult.data.createAgent.llmProvider.model}\n`);
    }

    // Create a demo workflow for AI agent processing
    console.log('📋 Creating AI-enabled document workflow...');
    const workflow = await client.createWorkflow(
      'ai_document_workflow',
      'AI-Enabled Document Processing'
    );
    console.log(`✅ Created workflow: ${workflow.id}`);
    console.log(`   Places: ${workflow.places.join(' → ')}\n`);

    // Create a document token that would trigger AI agents
    console.log('📄 Creating document token for AI processing...');
    const documentToken = await client.createToken(
      workflow.id,
      {
        content: 'This is a technical document about Rust programming and async/await patterns.',
        type: 'technical_document'
      },
      {
        status: 'unclassified',
        priority: 'high',
        author: 'demo_user'
      }
    );
    console.log(`✅ Created token: ${documentToken.id}`);
    console.log(`   Current place: ${documentToken.place}`);
    console.log(`   Content preview: "${documentToken.data.content.substring(0, 50)}..."`);

    // Simulate AI agent processing by firing transitions
    console.log('\n🤖 Simulating AI agent workflow...');
    
    console.log('   1. Classification Agent would process the document...');
    console.log('      (In full implementation: AI analyzes content and classifies it)');
    await new Promise(resolve => setTimeout(resolve, 1000)); // Simulate processing
    
    const classifiedToken = await client.fireTransition(documentToken.id, 'classify');
    console.log(`   ✅ Token moved to: ${classifiedToken.place}`);

    console.log('   2. Moving to review stage...');
    const reviewToken = await client.fireTransition(classifiedToken.id, 'review');
    console.log(`   ✅ Token moved to: ${reviewToken.place}`);

    console.log('   3. Review Agent would analyze quality...');
    console.log('      (In full implementation: AI reviews content quality and accuracy)');
    await new Promise(resolve => setTimeout(resolve, 1000)); // Simulate processing

    const approvedToken = await client.fireTransition(reviewToken.id, 'approve');
    console.log(`   ✅ Token moved to: ${approvedToken.place}`);

    // Show final token state
    console.log('\n📊 Final token state:');
    const finalToken = await client.getToken(approvedToken.id);
    if (finalToken) {
      console.log(`   ID: ${finalToken.id}`);
      console.log(`   Current place: ${finalToken.place}`);
      console.log(`   Workflow transitions: ${finalToken.history.length}`);
      console.log('   History:');
      finalToken.history.forEach((event, index) => {
        console.log(`     ${index + 1}. ${event.fromPlace} → ${event.toPlace} (${event.transition})`);
      });
    }

    console.log('\n🎯 What this demonstrates:');
    console.log('   • Basic workflow operations via GraphQL');
    console.log('   • Token state management and transitions');
    console.log('   • Places where AI agents would be triggered');
    console.log('   • Document processing pipeline structure');

    console.log('\n📝 Next steps for full AI integration:');
    console.log('   • Implement GraphQL resolvers for agent operations');
    console.log('   • Add Places AI Agent configurations');
    console.log('   • Enable real-time agent execution and streaming');
    console.log('   • Connect with Anthropic Claude for content processing');

    console.log('\n✨ Demo completed successfully!');

  } catch (error) {
    console.error('❌ Demo failed:', error);
    if (error instanceof Error) {
      console.error('Error details:', error.message);
    }
    process.exit(1);
  }
}

// Run the demo
if (import.meta.url === `file://${process.argv[1]}`) {
  runSimpleDemo().catch(console.error);
}

export {
  SimplePlacesAIClient,
  type Workflow,
  type Token,
  type Transition,
  type HistoryEvent,
};