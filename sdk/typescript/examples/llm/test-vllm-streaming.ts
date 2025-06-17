#!/usr/bin/env node

/**
 * Simple vLLM Streaming Test
 * 
 * This is a focused test for vLLM streaming functionality through Circuit Breaker.
 * Run with: npx tsx examples/typescript/test_vllm_streaming.ts
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
}

interface ChatRequest {
  model: string;
  messages: ChatMessage[];
  temperature: number;
  max_tokens: number;
  stream: boolean;
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
}

async function main(): Promise<void> {
  console.log('🔍 vLLM Streaming Test');
  console.log('=====================');

  // Configuration
  const baseUrl = process.env.CIRCUIT_BREAKER_URL || 'http://localhost:3000';
  const openaiEndpoint = `${baseUrl}/v1`;
  const apiKey = process.env.CIRCUIT_BREAKER_API_KEY || '';

  console.log(`🌐 Testing against: ${openaiEndpoint}`);

  // Test models to try
  const testModels = [
    'codellama/CodeLlama-7b-Instruct-hf',
    'microsoft/DialoGPT-medium',
    'meta-llama/Llama-2-7b-chat-hf',
  ];

  // Test 1: Non-streaming first
  console.log('\n1️⃣  Testing NON-STREAMING chat completion...');
  for (const model of testModels) {
    console.log(`   Trying model: ${model}`);

    const request: ChatRequest = {
      model,
      messages: [
        {
          role: 'user',
          content: 'Say hello in exactly 5 words.'
        }
      ],
      temperature: 0.7,
      max_tokens: 50,
      stream: false
    };

    try {
      const response = await testChatCompletion(openaiEndpoint, apiKey, request);
      console.log(`   ✅ SUCCESS with model: ${model}`);
      
      if (response.choices?.[0]?.message?.content) {
        console.log(`   🤖 Response: ${response.choices[0].message.content.trim()}`);
      }
      break;
    } catch (error) {
      console.log(`   ❌ Failed: ${error}`);
      continue;
    }
  }

  // Test 2: Streaming
  console.log('\n2️⃣  Testing STREAMING chat completion...');
  for (const model of testModels) {
    console.log(`   Trying streaming with model: ${model}`);

    const request: ChatRequest = {
      model,
      messages: [
        {
          role: 'user',
          content: 'Write a slow elevator pitch for GitLab. Use exactly 3 short sentences, each on a new line.'
        }
      ],
      temperature: 0.1,
      max_tokens: 150,
      stream: true
    };

    try {
      await testStreamingChat(openaiEndpoint, apiKey, request);
      console.log(`\n   ✅ STREAMING SUCCESS with model: ${model}`);
      break;
    } catch (error) {
      console.log(`   ❌ Streaming failed: ${error}`);
      continue;
    }
  }

  console.log('\n🎉 Test completed!');
}

async function testChatCompletion(
  endpoint: string,
  apiKey: string,
  requestBody: ChatRequest
): Promise<ChatResponse> {
  const url = `${endpoint}/chat/completions`;

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
    throw new Error(`HTTP ${response.status}: ${errorText}`);
  }

  return await response.json() as ChatResponse;
}

async function testStreamingChat(
  endpoint: string,
  apiKey: string,
  requestBody: ChatRequest
): Promise<void> {
  const url = `${endpoint}/chat/completions`;

  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    'Accept': 'text/event-stream',
    'Cache-Control': 'no-cache',
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
    throw new Error(`HTTP ${response.status}: ${errorText}`);
  }

  console.log('   🌊 Real-time streaming output:');
  process.stdout.write('   ');

  const startTime = Date.now();
  let chunkCount = 0;
  let totalChars = 0;

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
            const elapsed = Date.now() - startTime;
            console.log();
            console.log(`   📊 Streaming stats: ${chunkCount} chunks, ${totalChars} chars in ${elapsed}ms`);
            resolve();
            return;
          }

          try {
            const jsonChunk = JSON.parse(data) as ChatResponse;
            const content = jsonChunk.choices?.[0]?.delta?.content;
            
            if (content) {
              // Display each character with a small delay for visualization
              for (const char of content) {
                process.stdout.write(char);
                // Add artificial delay to see streaming effect
                setTimeout(() => {}, 25);
              }
              
              chunkCount++;
              totalChars += content.length;
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
      console.log(); // Add newline after streaming
      resolve();
    });
  });
}

// Helper function to add delay (for visualization)
function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}

// Run the main function
if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch(console.error);
}