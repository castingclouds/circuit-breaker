#!/usr/bin/env node

/**
 * vLLM Provider Test Example
 *
 * This example demonstrates how to use vLLM models through the Circuit Breaker server's OpenAI API.
 *
 * Prerequisites:
 * - Circuit Breaker server must be running (cargo run --bin server)
 * - vLLM must be configured and available through the server
 *
 * Usage:
 * ```bash
 * # Start the Circuit Breaker server (in another terminal)
 * cargo run --bin server
 *
 * # Run the example
 * npx tsx examples/typescript/vllm_provider_test.ts
 * ```
 */

import fetch from 'node-fetch';
import { Readable } from 'stream';
import { fileURLToPath } from 'url';
import { dirname } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

interface ChatMessage {
  role: string;
  content: string;
  name?: string;
}

interface ChatRequest {
  model: string;
  messages: ChatMessage[];
  temperature?: number;
  max_tokens?: number;
  stream?: boolean;
}

interface ChatChoice {
  message?: {
    content?: string;
  };
  delta?: {
    content?: string;
  };
}

interface ChatResponse {
  choices?: ChatChoice[];
  usage?: {
    prompt_tokens?: number;
    completion_tokens?: number;
    total_tokens?: number;
  };
}

interface ModelInfo {
  id: string;
  object?: string;
}

interface ModelsResponse {
  data: ModelInfo[];
}

interface EmbeddingRequest {
  model: string;
  input: string;
  encoding_format?: string;
}

interface EmbeddingData {
  embedding?: number[];
}

interface EmbeddingResponse {
  data?: EmbeddingData[];
}

async function main(): Promise<void> {
  console.log('🚀 Circuit Breaker - vLLM Provider Test');
  console.log('=======================================');

  // Configuration
  const baseUrl = process.env.CIRCUIT_BREAKER_URL || 'http://localhost:3000';
  const openaiEndpoint = `${baseUrl}/v1`;
  const apiKey = process.env.CIRCUIT_BREAKER_API_KEY || '';

  console.log(`🔍 Testing vLLM through Circuit Breaker at: ${openaiEndpoint}`);

  // Test server health
  console.log('\n🏥 Testing server health...');
  try {
    await testHealth(baseUrl);
    console.log('✅ Server is healthy');
  } catch (error) {
    console.error(`❌ Server health check failed: ${error}`);
    return;
  }

  // List available models
  console.log('\n📋 Fetching available models...');
  try {
    const models = await listModels(openaiEndpoint, apiKey);
    console.log('✅ Available models:');
    for (const model of models) {
      console.log(`  - ${model.id}`);
    }
  } catch (error) {
    console.error(`❌ Failed to list models: ${error}`);
    console.log('   This might mean vLLM is not properly configured.');
  }

  // Test chat completion with fallback models
  const model = process.env.VLLM_MODEL || 'microsoft/DialoGPT-medium';
  const fallbackModels = [
    model,
    'microsoft/DialoGPT-medium',
    'codellama/CodeLlama-7b-Instruct-hf',
    'meta-llama/Llama-2-7b-chat-hf',
  ];

  let chatSuccess = false;
  for (const testModel of fallbackModels) {
    console.log(`\n💬 Testing chat completion with model: ${testModel}`);

    const chatRequest: ChatRequest = {
      model: testModel,
      messages: [
        {
          role: 'system',
          content: 'You are a helpful assistant. Be explicit in your responses.'
        },
        {
          role: 'user',
          content: 'Write me an elevator pitch for GitLab'
        }
      ],
      temperature: 0.7,
      max_tokens: 2048,
      stream: false
    };

    try {
      const response = await chatCompletion(openaiEndpoint, apiKey, chatRequest);
      console.log(`✅ Chat completion successful with model: ${testModel}`);
      chatSuccess = true;

      if (response.usage) {
        const { prompt_tokens, completion_tokens, total_tokens } = response.usage;
        if (prompt_tokens && completion_tokens && total_tokens) {
          console.log('📊 Token usage:');
          console.log(`  Prompt tokens: ${prompt_tokens}`);
          console.log(`  Completion tokens: ${completion_tokens}`);
          console.log(`  Total tokens: ${total_tokens}`);
        }
      }

      if (response.choices?.[0]?.message?.content) {
        console.log('\n🤖 Assistant response:');
        console.log(`  ${response.choices[0].message.content}`);
      }
      break; // Success, no need to try other models
    } catch (error) {
      console.error(`❌ Chat completion failed with model '${testModel}': ${error}`);
      if (testModel === fallbackModels[fallbackModels.length - 1]) {
        console.error('   All fallback models failed. Common issues:');
        console.error('   1. No models available in vLLM server');
        console.error('   2. vLLM server not running or misconfigured');
        console.error('   3. Circuit Breaker routing configuration issue');
        console.error('   4. Try setting VLLM_MODEL environment variable to a specific model');
      }
    }
  }

  // Test embeddings (if supported)
  console.log('\n🔮 Testing embeddings...');
  const embeddingModels = [
    process.env.VLLM_EMBEDDING_MODEL || 'sentence-transformers/all-MiniLM-L6-v2',
    'sentence-transformers/all-MiniLM-L6-v2',
    'sentence-transformers/all-mpnet-base-v2',
  ];

  let embeddingSuccess = false;
  for (const embeddingModel of embeddingModels) {
    console.log(`  Trying embedding model: ${embeddingModel}`);

    const embeddingRequest: EmbeddingRequest = {
      model: embeddingModel,
      input: 'This is a test sentence for embeddings.',
      encoding_format: 'float'
    };

    try {
      const response = await embeddings(openaiEndpoint, apiKey, embeddingRequest);
      console.log(`✅ Embeddings successful with model: ${embeddingModel}`);
      embeddingSuccess = true;

      if (response.data?.[0]?.embedding) {
        const embedding = response.data[0].embedding;
        console.log('📊 Embedding details:');
        console.log(`  Model: ${embeddingModel}`);
        console.log(`  Embedding dimension: ${embedding.length}`);

        // Show first 5 values
        const preview = embedding.slice(0, 5);
        console.log(`  First 5 values: [${preview.join(', ')}]`);
      }
      break; // Success, no need to try other models
    } catch (error) {
      console.error(`❌ Embeddings failed with model '${embeddingModel}': ${error}`);
      if (embeddingModel === embeddingModels[embeddingModels.length - 1]) {
        console.error('   All embedding models failed. This might mean:');
        console.error('   1. No embedding models are available in vLLM');
        console.error("   2. vLLM server doesn't support embeddings endpoint");
      }
    }
  }

  // Test streaming (optional)
  if (process.env.TEST_STREAMING === 'true' && chatSuccess) {
    console.log('\n🌊 Testing streaming chat completion...');

    // Use the first successful model from the fallback list
    const streamingModel = fallbackModels[0];

    const streamingRequest: ChatRequest = {
      model: streamingModel,
      messages: [
        {
          role: 'user',
          content: 'Count from 1 to 5, one number per line.'
        }
      ],
      temperature: 0.3,
      max_tokens: 50,
      stream: true
    };

    try {
      await streamingChat(openaiEndpoint, apiKey, streamingRequest);
      console.log('✅ Streaming completed successfully');
    } catch (error) {
      console.error(`❌ Streaming failed: ${error}`);
    }
  } else if (process.env.TEST_STREAMING === 'true') {
    console.log('\n🌊 Skipping streaming test (no successful chat model found)');
  }

  console.log('\n🎉 vLLM provider test completed!');

  // Summary
  console.log('\n📋 Test Summary:');
  console.log(`  Chat completion: ${chatSuccess ? '✅ PASSED' : '❌ FAILED'}`);
  console.log(`  Embeddings: ${embeddingSuccess ? '✅ PASSED' : '❌ FAILED'}`);

  console.log('\n💡 Tips:');
  console.log('  - Set VLLM_MODEL to test specific chat models (default: microsoft/DialoGPT-medium)');
  console.log('  - Set VLLM_EMBEDDING_MODEL to test specific embedding models');
  console.log('  - Set TEST_STREAMING=true to test streaming responses');
  console.log('  - Set CIRCUIT_BREAKER_URL to test remote instances');
  console.log('  - Set CIRCUIT_BREAKER_API_KEY if authentication is required');
  console.log('  - Available fallback models: microsoft/DialoGPT-medium, codellama/CodeLlama-7b-Instruct-hf');
  console.log('  - Make sure vLLM server is running with at least one model loaded');
}

async function testHealth(baseUrl: string): Promise<void> {
  const healthUrl = `${baseUrl}/health`;
  const response = await fetch(healthUrl);

  if (!response.ok) {
    throw new Error(`Health check failed with status: ${response.status}`);
  }
}

async function listModels(baseUrl: string, apiKey: string): Promise<ModelInfo[]> {
  const url = `${baseUrl}/models`;

  const headers: Record<string, string> = {};
  if (apiKey) {
    headers['Authorization'] = `Bearer ${apiKey}`;
  }

  const response = await fetch(url, { headers });

  if (!response.ok) {
    throw new Error(`Models request failed with status: ${response.status}`);
  }

  const json = await response.json() as ModelsResponse;
  return json.data || [];
}

async function chatCompletion(
  baseUrl: string,
  apiKey: string,
  requestBody: ChatRequest
): Promise<ChatResponse> {
  const url = `${baseUrl}/chat/completions`;

  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
  };

  if (apiKey) {
    headers['Authorization'] = `Bearer ${apiKey}`;
  }

  const response = await fetch(url, {
    method: 'POST',
    headers,
    body: JSON.stringify(requestBody),
  });

  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(`Chat completion failed: ${errorText}`);
  }

  return await response.json() as ChatResponse;
}

async function embeddings(
  baseUrl: string,
  apiKey: string,
  requestBody: EmbeddingRequest
): Promise<EmbeddingResponse> {
  const url = `${baseUrl}/embeddings`;

  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
  };

  if (apiKey) {
    headers['Authorization'] = `Bearer ${apiKey}`;
  }

  const response = await fetch(url, {
    method: 'POST',
    headers,
    body: JSON.stringify(requestBody),
  });

  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(`Embeddings failed: ${errorText}`);
  }

  return await response.json() as EmbeddingResponse;
}

async function streamingChat(
  baseUrl: string,
  apiKey: string,
  requestBody: ChatRequest
): Promise<void> {
  const url = `${baseUrl}/chat/completions`;

  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
  };

  if (apiKey) {
    headers['Authorization'] = `Bearer ${apiKey}`;
  }

  const response = await fetch(url, {
    method: 'POST',
    headers,
    body: JSON.stringify(requestBody),
  });

  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(`Streaming chat failed: ${errorText}`);
  }

  const stream = response.body as Readable;
  let buffer = '';

  return new Promise((resolve, reject) => {
    stream.on('data', (chunk: Buffer) => {
      const chunkStr = chunk.toString();
      buffer += chunkStr;

      // Process complete lines from buffer
      let lineEnd;
      while ((lineEnd = buffer.indexOf('\n')) !== -1) {
        const line = buffer.slice(0, lineEnd).trim();
        buffer = buffer.slice(lineEnd + 1);

        if (line.startsWith('data: ')) {
          const data = line.slice(6); // Remove "data: " prefix
          
          if (data === '[DONE]') {
            resolve();
            return;
          }

          try {
            const chunk = JSON.parse(data) as ChatResponse;
            const content = chunk.choices?.[0]?.delta?.content;
            
            if (content) {
              process.stdout.write(content);
            }
          } catch (parseError) {
            // Ignore JSON parse errors for malformed chunks
          }
        }
      }
    });

    stream.on('error', (error) => {
      reject(new Error(`Stream error: ${error.message}`));
    });

    stream.on('end', () => {
      console.log(); // New line after streaming
      resolve();
    });
  });
}

// Run the main function
if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch(console.error);
}