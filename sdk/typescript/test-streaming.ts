#!/usr/bin/env node

/**
 * Simple streaming test for Circuit Breaker TypeScript SDK
 * This test isolates the streaming functionality to identify issues
 */

import { Client, COMMON_MODELS } from "./src/index.js";

async function testStreaming() {
  console.log("🌊 Circuit Breaker Streaming Test");
  console.log("=================================");

  const client = new Client({
    baseUrl: "http://localhost:3000",
    timeout: 15000,
  });

  const llm = client.llm();

  // Test 1: Simple streaming request
  console.log("\n1. Testing simple streaming request");
  console.log("   --------------------------------");

  try {
    const simpleRequest = {
      model: "gpt-3.5-turbo", // Use a regular model first
      messages: [
        { role: "user" as const, content: "Count from 1 to 5, one number per response chunk." }
      ],
      stream: true as const,
      max_tokens: 50,
    };

    console.log("   📤 Request:", JSON.stringify(simpleRequest, null, 2));
    console.log("   🌊 Starting stream...");

    let chunkCount = 0;
    for await (const chunk of llm.streamChatCompletionIterator(simpleRequest)) {
      chunkCount++;
      const content = chunk.choices[0]?.delta?.content || "";
      if (content) {
        process.stdout.write(content);
      }
      console.log(`\n   📦 Chunk ${chunkCount}:`, JSON.stringify(chunk, null, 2));
    }
    console.log(`\n   ✅ Simple streaming completed (${chunkCount} chunks)`);
  } catch (error) {
    console.log(`   ❌ Simple streaming failed:`, error);
  }

  // Test 2: Virtual model streaming
  console.log("\n2. Testing virtual model streaming");
  console.log("   -------------------------------");

  try {
    const virtualRequest = {
      model: COMMON_MODELS.SMART_CREATIVE,
      messages: [
        { role: "user" as const, content: "Write a very short poem about computers." }
      ],
      stream: true as const,
      max_tokens: 100,
    };

    console.log("   📤 Request:", JSON.stringify(virtualRequest, null, 2));
    console.log("   🌊 Starting virtual model stream...");

    let chunkCount = 0;
    process.stdout.write("   📝 Response: ");
    for await (const chunk of llm.streamChatCompletionIterator(virtualRequest)) {
      chunkCount++;
      const content = chunk.choices[0]?.delta?.content || "";
      if (content) {
        process.stdout.write(content);
      }
    }
    console.log(`\n   ✅ Virtual model streaming completed (${chunkCount} chunks)`);
  } catch (error) {
    console.log(`   ❌ Virtual model streaming failed:`, error);
  }

  // Test 3: Advanced streaming with Circuit Breaker options
  console.log("\n3. Testing advanced streaming with Circuit Breaker options");
  console.log("   -------------------------------------------------------");

  try {
    const advancedRequest = {
      model: COMMON_MODELS.SMART_FAST,
      messages: [
        { role: "user" as const, content: "Explain what a circuit breaker is in one sentence." }
      ],
      stream: true as const,
      max_tokens: 50,
      temperature: 0.7,
      circuit_breaker: {
        routing_strategy: "performance_first" as const,
        max_latency_ms: 3000,
        require_streaming: true,
        fallback_models: ["gpt-3.5-turbo"],
      },
    };

    console.log("   📤 Request:", JSON.stringify(advancedRequest, null, 2));
    console.log("   🌊 Starting advanced stream...");

    let chunkCount = 0;
    process.stdout.write("   📝 Response: ");
    for await (const chunk of llm.streamChatCompletionIterator(advancedRequest)) {
      chunkCount++;
      const content = chunk.choices[0]?.delta?.content || "";
      if (content) {
        process.stdout.write(content);
      }
    }
    console.log(`\n   ✅ Advanced streaming completed (${chunkCount} chunks)`);
  } catch (error) {
    console.log(`   ❌ Advanced streaming failed:`, error);
  }

  // Test 4: Raw fetch streaming (bypass SDK)
  console.log("\n4. Testing raw fetch streaming (bypass SDK)");
  console.log("   -----------------------------------------");

  try {
    const rawRequest = {
      model: "gpt-3.5-turbo",
      messages: [
        { role: "user", content: "Say 'Hello from raw fetch!'" }
      ],
      stream: true,
      max_tokens: 20,
    };

    console.log("   📤 Raw request:", JSON.stringify(rawRequest, null, 2));

    const response = await fetch("http://localhost:3000/v1/chat/completions", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "Accept": "text/event-stream",
        "Cache-Control": "no-cache",
      },
      body: JSON.stringify(rawRequest),
    });

    console.log("   📡 Response status:", response.status, response.statusText);
    console.log("   📡 Response headers:", Object.fromEntries(response.headers.entries()));

    if (!response.ok) {
      const errorText = await response.text();
      console.log("   ❌ Response error:", errorText);
      return;
    }

    if (!response.body) {
      console.log("   ❌ No response body");
      return;
    }

    const reader = response.body.getReader();
    const decoder = new TextDecoder();
    let chunkCount = 0;

    console.log("   🌊 Reading raw stream...");
    process.stdout.write("   📝 Raw response: ");

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;

      chunkCount++;
      const chunk = decoder.decode(value, { stream: true });
      console.log(`\n   📦 Raw chunk ${chunkCount}:`, JSON.stringify(chunk));

      // Parse SSE events
      const lines = chunk.split('\n');
      for (const line of lines) {
        if (line.startsWith('data: ') && !line.includes('[DONE]')) {
          try {
            const data = JSON.parse(line.substring(6));
            const content = data.choices?.[0]?.delta?.content || "";
            if (content) {
              process.stdout.write(content);
            }
          } catch (e) {
            // Ignore JSON parse errors for incomplete chunks
          }
        }
      }
    }

    console.log(`\n   ✅ Raw streaming completed (${chunkCount} raw chunks)`);
  } catch (error) {
    console.log(`   ❌ Raw streaming failed:`, error);
  }

  console.log("\n📋 Streaming Test Summary");
  console.log("   ======================");
  console.log("   If all tests pass, streaming is working correctly.");
  console.log("   If some tests fail, check the server logs for more details.");
}

// Run the test
testStreaming().catch((error) => {
  console.error("Streaming test failed:", error);
  process.exit(1);
});
