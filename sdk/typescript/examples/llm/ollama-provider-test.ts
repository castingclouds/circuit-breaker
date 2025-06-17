#!/usr/bin/env npx tsx
//! Ollama Provider Test Example - TypeScript Edition
//!
//! This TypeScript example demonstrates the same functionality as the Rust ollama_provider_test.rs
//! but shows how to integrate with Circuit Breaker from a TypeScript/JavaScript application.
//!
//! Key TypeScript Features Demonstrated:
//! - Modern ES modules with import/export syntax
//! - Full type safety with TypeScript interfaces
//! - Async/await patterns for HTTP requests
//! - Stream processing with Node.js ReadableStream
//! - Interactive CLI with readline interface
//! - Fetch API for HTTP requests (node-fetch)
//! - Type-safe GraphQL queries and mutations
//!
//! Architecture:
//! TypeScript Client → Circuit Breaker Server → Ollama Local Instance
//!
//! Prerequisites:
//! - Circuit Breaker server running on localhost:4000 (GraphQL) and localhost:3000 (OpenAI API)
//! - Ollama running locally (typically on http://localhost:11434)
//! - At least one model pulled (e.g., qwen2.5-coder:3b, gemma3:4b, nomic-embed-text:latest)
//!
//! Usage:
//! ```bash
//! # Start Ollama (if not already running)
//! ollama serve
//!
//! # Pull required models
//! ollama pull qwen2.5-coder:3b
//! ollama pull gemma3:4b
//! ollama pull nomic-embed-text:latest
//!
//! # Start Circuit Breaker server
//! cargo run --bin server
//!
//! # Run the TypeScript example
//! npx tsx ollama_provider_test.ts
//! # OR
//! npm run demo:ollama
//! ```

import fetch from 'node-fetch';
import * as readline from 'readline';

// TypeScript type definitions for type-safe API interactions
// These interfaces provide compile-time type checking and IntelliSense support
interface GraphQLResponse<T = any> {
  data?: T;
  errors?: Array<{ message: string }>;
}

interface LLMProvider {
  id: string;
  providerType: string;
  name: string;
  baseUrl: string;
  healthStatus: {
    isHealthy: boolean;
    errorRate: number;
    averageLatencyMs: number;
  };
  models: Array<{
    id: string;
    name: string;
    costPerInputToken: number;
    costPerOutputToken: number;
    supportsStreaming: boolean;
    contextWindow: number;
    maxTokens: number;
    capabilities: string[];
  }>;
}

interface ChatCompletionRequest {
  model: string;
  messages: Array<{
    role: 'system' | 'user' | 'assistant';
    content: string;
  }>;
  temperature?: number;
  max_tokens?: number;
  stream?: boolean;
}

interface ChatCompletionResponse {
  id: string;
  object: string;
  created: number;
  model: string;
  choices: Array<{
    index: number;
    message: {
      role: string;
      content: string;
    };
    finish_reason: string;
  }>;
  usage: {
    prompt_tokens: number;
    completion_tokens: number;
    total_tokens: number;
  };
}

interface EmbeddingsRequest {
  model: string;
  input: string | string[];
}

interface EmbeddingsResponse {
  object: string;
  data: Array<{
    object: string;
    embedding: number[];
    index: number;
  }>;
  model: string;
  usage: {
    prompt_tokens: number;
    total_tokens: number;
  };
}

interface StreamingChunk {
  id: string;
  object: string;
  created: number;
  model: string;
  choices: Array<{
    index: number;
    delta: {
      role?: string;
      content?: string;
    };
    finish_reason?: string;
  }>;
}

/**
 * TypeScript client for testing Ollama provider integration
 * Demonstrates modern TypeScript patterns and Node.js APIs
 */
class OllamaProviderTestClient {
  // TypeScript readonly properties ensure URLs cannot be accidentally modified
  private readonly graphqlUrl = 'http://localhost:4000/graphql';
  private readonly openaiApiUrl = 'http://localhost:3000/v1';
  private readonly ollamaDirectUrl = 'http://localhost:11434';

  // TypeScript async/await pattern for interactive demo pauses
  // Uses Node.js readline for cross-platform input handling
  private async waitForEnter(message: string): Promise<void> {
    console.log(`\n🎤 ${message}`);
    console.log('   Press Enter to continue...');
    
    const rl = readline.createInterface({
      input: process.stdin,
      output: process.stdout,
    });

    return new Promise((resolve) => {
      rl.question('', () => {
        rl.close();
        console.log();
        resolve();
      });
    });
  }

  // TypeScript generic method for type-safe GraphQL requests
  // Generic type T ensures return data matches expected interface
  private async graphqlRequest<T>(query: string, variables?: any): Promise<GraphQLResponse<T>> {
    const response = await fetch(this.graphqlUrl, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ query, variables }),
    });

    if (!response.ok) {
      throw new Error(`GraphQL request failed: ${response.statusText}`);
    }

    return response.json() as Promise<GraphQLResponse<T>>;
  }

  // Check server connectivity
  private async checkServerConnectivity(): Promise<void> {
    console.log('🔗 Testing server connectivity...');
    
    try {
      // Test GraphQL endpoint
      const graphqlResponse = await fetch(this.graphqlUrl, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ query: '{ __typename }' }),
      });

      // Test OpenAI API endpoint
      const openaiResponse = await fetch(`${this.openaiApiUrl}/models`);

      if (graphqlResponse.ok && openaiResponse.ok) {
        console.log('✅ Circuit Breaker server is running on both endpoints');
      } else {
        throw new Error('One or more endpoints not responding');
      }
    } catch (error) {
      console.log('❌ Circuit Breaker server not responding. Please ensure:');
      console.log('   1. Server is running: cargo run --bin server');
      console.log('   2. GraphQL endpoint: http://localhost:4000/graphql');
      console.log('   3. OpenAI API endpoint: http://localhost:3000/v1');
      throw error;
    }
  }

  // Check Ollama availability
  private async checkOllamaAvailability(): Promise<boolean> {
    console.log('🔍 Checking Ollama availability...');
    
    try {
      const response = await fetch(`${this.ollamaDirectUrl}/api/tags`);
      if (response.ok) {
        console.log('✅ Ollama is available');
        return true;
      } else {
        console.log('❌ Ollama not responding. Please ensure:');
        console.log('   1. Ollama is installed and running: ollama serve');
        console.log('   2. Ollama is accessible at: http://localhost:11434');
        return false;
      }
    } catch (error) {
      console.log('❌ Cannot connect to Ollama directly:', error);
      return false;
    }
  }

  // Check LLM providers via GraphQL
  private async checkLLMProviders(): Promise<void> {
    console.log('\n📊 Checking LLM providers via Circuit Breaker...');
    
    const query = `
      query {
        llmProviders {
          id
          providerType
          name
          baseUrl
          healthStatus {
            isHealthy
            errorRate
            averageLatencyMs
          }
          models {
            id
            name
            costPerInputToken
            costPerOutputToken
            supportsStreaming
            contextWindow
            maxTokens
            capabilities
          }
        }
      }
    `;

    try {
      const response = await this.graphqlRequest<{ llmProviders: LLMProvider[] }>(query);
      
      if (response.errors) {
        console.log('❌ GraphQL error:', response.errors[0].message);
        return;
      }

      const providers = response.data?.llmProviders || [];
      const ollamaProvider = providers.find(p => p.providerType === 'ollama');

      if (ollamaProvider) {
        console.log('✅ Ollama provider found in Circuit Breaker');
        console.log(`   Provider ID: ${ollamaProvider.id}`);
        console.log(`   Base URL: ${ollamaProvider.baseUrl}`);
        console.log(`   Health: ${ollamaProvider.healthStatus.isHealthy ? 'Healthy' : 'Unhealthy'}`);
        console.log(`   Models available: ${ollamaProvider.models.length}`);
        
        // Show first few models
        ollamaProvider.models.slice(0, 3).forEach(model => {
          console.log(`     • ${model.name} (${model.id})`);
          console.log(`       Context: ${model.contextWindow} tokens, Max: ${model.maxTokens} tokens, Capabilities: ${model.capabilities.join(', ')}`);
        });
        
        if (ollamaProvider.models.length > 3) {
          console.log(`     ... and ${ollamaProvider.models.length - 3} more models`);
        }
      } else {
        console.log('⚠️  Ollama provider not found in Circuit Breaker configuration');
        console.log('   The server may not have Ollama configured yet');
      }
    } catch (error) {
      console.log('❌ Failed to check LLM providers:', error);
    }
  }

  // Test available models via OpenAI API
  private async testAvailableModels(): Promise<void> {
    console.log('\n📋 Testing available models via OpenAI API...');
    
    try {
      const response = await fetch(`${this.openaiApiUrl}/models`);
      
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      const data = await response.json();
      const models = (data as any).data || [];
      
      // Filter for our specific Ollama models
      const ollamaModels = models.filter((model: any) => 
        model.id.includes('qwen2.5-coder') || 
        model.id.includes('gemma3') || 
        model.id.includes('nomic-embed')
      );

      console.log(`✅ Found ${models.length} total models, ${ollamaModels.length} Ollama models`);
      
      if (ollamaModels.length > 0) {
        console.log('📋 Available Ollama models:');
        ollamaModels.forEach((model: any) => {
          console.log(`   • ${model.id} (${model.owned_by || 'ollama'})`);
        });
      } else {
        console.log('⚠️  No Ollama models found. Expected models:');
        console.log('   • qwen2.5-coder:3b (for coding)');
        console.log('   • gemma3:4b (for text generation)');
        console.log('   • nomic-embed-text:latest (for embeddings)');
      }
    } catch (error) {
      console.log('❌ Failed to fetch models:', error);
    }
  }

  // Test chat completion
  private async testChatCompletion(): Promise<void> {
    console.log('\n💬 Testing chat completion with Ollama models...');
    
    const testCases = [
      {
        name: 'Coding Model (qwen2.5-coder:3b)',
        model: 'qwen2.5-coder:3b',
        messages: [
          {
            role: 'system' as const,
            content: 'You are a helpful coding assistant. Be concise in your responses.',
          },
          {
            role: 'user' as const,
            content: 'Write a simple Python function to calculate fibonacci numbers.',
          },
        ],
      },
      {
        name: 'Text Model (gemma3:4b)',
        model: 'gemma3:4b',
        messages: [
          {
            role: 'system' as const,
            content: 'You are a helpful assistant. Be concise in your responses.',
          },
          {
            role: 'user' as const,
            content: 'Explain what artificial intelligence is in simple terms.',
          },
        ],
      },
    ];

    for (const testCase of testCases) {
      console.log(`\n🧪 Testing ${testCase.name}...`);
      
      const request: ChatCompletionRequest = {
        model: testCase.model,
        messages: testCase.messages,
        temperature: 0.7,
        max_tokens: 150,
        stream: false,
      };

      try {
        const startTime = Date.now();
        const response = await fetch(`${this.openaiApiUrl}/chat/completions`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(request),
        });

        if (!response.ok) {
          const errorText = await response.text();
          console.log(`❌ ${testCase.name} failed: ${response.status} ${response.statusText}`);
          console.log(`   Error: ${errorText.substring(0, 200)}...`);
          continue;
        }

        const result: ChatCompletionResponse = await response.json() as ChatCompletionResponse;
        const latency = Date.now() - startTime;

        console.log(`✅ ${testCase.name} successful!`);
        console.log(`   ⏱️  Latency: ${latency}ms`);
        console.log(`   📊 Tokens: ${result.usage.total_tokens} (${result.usage.prompt_tokens} + ${result.usage.completion_tokens})`);
        console.log(`   💰 Estimated cost: $0.00 (local inference)`);
        console.log(`\n🤖 Response:`);
        console.log(`   ${result.choices[0].message.content}`);
        
      } catch (error) {
        console.log(`❌ ${testCase.name} error:`, error);
      }
    }
  }

  // Test embeddings
  private async testEmbeddings(): Promise<void> {
    console.log('\n🔮 Testing embeddings with nomic-embed-text...');
    
    const testTexts = [
      'Hello, this is a test sentence for embeddings.',
      'Artificial intelligence and machine learning are fascinating fields.',
      'TypeScript is a great language for building scalable applications.',
    ];

    // Test single embedding
    console.log('\n📄 Testing single text embedding...');
    try {
      const singleRequest: EmbeddingsRequest = {
        model: 'nomic-embed-text:latest',
        input: testTexts[0],
      };

      const response = await fetch(`${this.openaiApiUrl}/embeddings`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(singleRequest),
      });

      if (!response.ok) {
        const errorText = await response.text();
        console.log(`❌ Single embedding failed: ${response.status} ${response.statusText}`);
        console.log(`   Error: ${errorText.substring(0, 200)}...`);
      } else {
        const result: EmbeddingsResponse = await response.json() as EmbeddingsResponse;
        console.log('✅ Single text embedding successful!');
        console.log(`   📊 Embedding dimensions: ${result.data[0].embedding.length}`);
        console.log(`   🔢 First 5 values: [${result.data[0].embedding.slice(0, 5).map(v => v.toFixed(4)).join(', ')}...]`);
        console.log(`   📝 Tokens used: ${result.usage.total_tokens}`);
        console.log(`   💰 Estimated cost: $0.00 (local inference)`);
      }
    } catch (error) {
      console.log('❌ Single embedding error:', error);
    }

    // Test batch embeddings
    console.log('\n📚 Testing batch embeddings...');
    try {
      const batchRequest: EmbeddingsRequest = {
        model: 'nomic-embed-text:latest',
        input: testTexts,
      };

      const response = await fetch(`${this.openaiApiUrl}/embeddings`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(batchRequest),
      });

      if (!response.ok) {
        const errorText = await response.text();
        console.log(`❌ Batch embeddings failed: ${response.status} ${response.statusText}`);
        console.log(`   Error: ${errorText.substring(0, 200)}...`);
      } else {
        const result: EmbeddingsResponse = await response.json() as EmbeddingsResponse;
        console.log('✅ Batch embeddings successful!');
        console.log(`   📊 Number of embeddings: ${result.data.length}`);
        console.log(`   📝 Total tokens: ${result.usage.total_tokens}`);
        
        result.data.forEach((embedding, index) => {
          console.log(`   ${index + 1}. Text "${testTexts[index].substring(0, 30)}..." → ${embedding.embedding.length} dimensions`);
        });
      }
    } catch (error) {
      console.log('❌ Batch embeddings error:', error);
    }
  }

  // Test streaming
  private async testStreaming(): Promise<void> {
    console.log('\n🌊 Testing streaming chat completion...');
    
    const request: ChatCompletionRequest = {
      model: 'qwen2.5-coder:3b',
      messages: [
        {
          role: 'user',
          content: 'Create an elevator pitch for selling GitLab in a creative way.',
        },
      ],
      temperature: 0.7,
      max_tokens: 200,
      stream: true,
    };

    try {
      const response = await fetch(`${this.openaiApiUrl}/chat/completions`, {
        method: 'POST',
        headers: { 
          'Content-Type': 'application/json',
          'Accept': 'text/event-stream',
        },
        body: JSON.stringify(request),
      });

      if (!response.ok) {
        const errorText = await response.text();
        console.log(`❌ Streaming failed: ${response.status} ${response.statusText}`);
        console.log(`   Error: ${errorText.substring(0, 200)}...`);
        return;
      }

      console.log('✅ Streaming started...');
      console.log('\n🤖 Streamed response:');
      process.stdout.write('   ');

      let chunkCount = 0;
      let totalContent = '';

      if (response.body) {
        // Node.js-compatible stream processing
        const decoder = new TextDecoder();
        let buffer = '';

        try {
          // Handle different Node.js fetch implementations
          if ((response.body as any).getReader) {
            // ReadableStream with getReader (Node.js 18+)
            const reader = (response.body as any).getReader();
            
            while (true) {
              const { done, value } = await reader.read();
              if (done) break;

              buffer += decoder.decode(value, { stream: true });
              const lines = buffer.split('\n');
              buffer = lines.pop() || '';

              for (const line of lines) {
                if (line.startsWith('data: ')) {
                  const data = line.slice(6).trim();
                  if (data === '[DONE]') {
                    break;
                  }

                  try {
                    const chunk: StreamingChunk = JSON.parse(data);
                    chunkCount++;
                    
                    if (chunk.choices[0]?.delta?.content) {
                      const content = chunk.choices[0].delta.content;
                      process.stdout.write(content);
                      totalContent += content;
                    }

                    if (chunk.choices[0]?.finish_reason === 'stop') {
                      break;
                    }
                  } catch (e) {
                    // Skip invalid JSON chunks
                  }
                }
              }
            }
            reader.releaseLock();
          } else if ((response.body as any)[Symbol.asyncIterator]) {
            // AsyncIterable (Node.js streams)
            for await (const chunk of response.body as any) {
              buffer += decoder.decode(chunk, { stream: true });
              const lines = buffer.split('\n');
              buffer = lines.pop() || '';

              for (const line of lines) {
                if (line.startsWith('data: ')) {
                  const data = line.slice(6).trim();
                  if (data === '[DONE]') {
                    break;
                  }

                  try {
                    const chunk: StreamingChunk = JSON.parse(data);
                    chunkCount++;
                    
                    if (chunk.choices[0]?.delta?.content) {
                      const content = chunk.choices[0].delta.content;
                      process.stdout.write(content);
                      totalContent += content;
                    }

                    if (chunk.choices[0]?.finish_reason === 'stop') {
                      break;
                    }
                  } catch (e) {
                    // Skip invalid JSON chunks
                  }
                }
              }
            }
          } else {
            throw new Error('Unsupported stream type');
          }
        } catch (streamError) {
          console.log(`\n   ❌ Streaming error: ${streamError}`);
          throw streamError;
        }
      }

      console.log('\n');
      console.log(`✅ Streaming completed!`);
      console.log(`   📊 Chunks received: ${chunkCount}`);
      console.log(`   📝 Content length: ${totalContent.length} characters`);

    } catch (error) {
      console.log('❌ Streaming error:', error);
    }
  }

  // Main demo orchestration method - showcases TypeScript async patterns
  async main(): Promise<void> {
    console.log('🦙 Circuit Breaker Ollama Provider Test - TypeScript Edition');
    console.log('============================================================');
    console.log();

    console.log('💡 This demo tests Ollama integration via Circuit Breaker server');
    console.log('🔧 TypeScript Features Demonstrated:');
    console.log('   • Type-safe API calls with interfaces');
    console.log('   • Modern async/await patterns');
    console.log('   • Stream processing with ReadableStream');
    console.log('   • Interactive CLI with readline');
    console.log('   • ES modules and import/export');
    console.log();
    console.log('📋 Prerequisites:');
    console.log('   • Circuit Breaker server running (cargo run --bin server)');
    console.log('   • Ollama running with models pulled');
    console.log('   • Expected models: qwen2.5-coder:3b, gemma3:4b, nomic-embed-text:latest');
    console.log();

    try {
      // Step 1: Check server connectivity
      await this.checkServerConnectivity();

      await this.waitForEnter('Server connectivity verified! Ready to check Ollama?');

      // Step 2: Check Ollama availability (direct)
      await this.checkOllamaAvailability();

      await this.waitForEnter('Ollama check complete! Ready to test Circuit Breaker integration?');

      // Step 3: Check LLM providers via Circuit Breaker
      await this.checkLLMProviders();

      await this.waitForEnter('Provider check complete! Ready to test available models?');

      // Step 4: Test available models
      await this.testAvailableModels();

      await this.waitForEnter('Model discovery complete! Ready to test chat completion?');

      // Step 5: Test chat completion
      await this.testChatCompletion();

      await this.waitForEnter('Chat completion tests done! Ready to test embeddings?');

      // Step 6: Test embeddings
      await this.testEmbeddings();

      await this.waitForEnter('Embeddings tests complete! Ready to test streaming?');

      // Step 7: Test streaming
      await this.testStreaming();

      // Final summary
      console.log('\n🎉 Ollama provider test completed!');
      console.log('=====================================');
      console.log();
      console.log('✅ Successfully demonstrated:');
      console.log('   • Circuit Breaker server connectivity');
      console.log('   • Ollama provider integration');
      console.log('   • Chat completion with coding and text models');
      console.log('   • Embeddings with nomic-embed-text');
      console.log('   • Real-time streaming responses');
      console.log('   • OpenAI API compatibility');
      console.log();
      console.log('💡 Key benefits:');
      console.log('   • $0 cost for local inference');
      console.log('   • Privacy-preserving (no data leaves your machine)');
      console.log('   • Fast inference for smaller models');
      console.log('   • Unified API across all providers');
      console.log('   • TypeScript type safety and IntelliSense');
      console.log('   • Modern JavaScript/Node.js ecosystem integration');
      console.log();
      console.log('🔗 Next steps:');
      console.log('   • Try different models: ollama pull <model-name>');
      console.log('   • Explore model management: ollama list');
      console.log('   • Test with your own prompts and use cases');
      console.log('   • Integrate with your TypeScript/JavaScript applications');
      console.log('   • Use with React, Vue, or other frontend frameworks');
      console.log('   • Deploy with Node.js servers or edge functions');

    } catch (error) {
      console.error('\n❌ Demo failed:', error);
      console.log('\n💡 Common troubleshooting:');
      console.log('   • Ensure Circuit Breaker server is running: cargo run --bin server');
      console.log('   • Ensure Ollama is running: ollama serve');
      console.log('   • Pull required models: ollama pull qwen2.5-coder:3b');
      process.exit(1);
    }
  }
}

// TypeScript module pattern - async function with proper error handling
async function run(): Promise<void> {
  const client = new OllamaProviderTestClient();
  await client.main();
}

// Always run when this file is executed directly
run().catch(console.error);

export { OllamaProviderTestClient };