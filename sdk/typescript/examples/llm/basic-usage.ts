/**
 * Basic LLM Router Usage Examples
 *
 * This file demonstrates how to use the Circuit Breaker LLM Router
 * for multi-provider AI integration with intelligent routing.
 */

import {
  LLMBuilder,
  createLLMBuilder,
  createMultiProviderBuilder,
  createCostOptimizedBuilder,
  LLMRouter,
  StreamingHandler,
  ChatCompletionRequest,
} from '../../src/index.js';

// ============================================================================
// Example 1: Basic Single Provider Setup
// ============================================================================

async function basicSingleProvider() {
  console.log('🔧 Setting up basic OpenAI provider...');

  const router = await createLLMBuilder()
    .addOpenAI({
      apiKey: process.env.OPENAI_API_KEY,
      models: ['gpt-4', 'gpt-3.5-turbo']
    })
    .setDefaultProvider('openai-provider')
    .enableHealthChecks()
    .build();

  // Simple chat completion
  const request: ChatCompletionRequest = {
    model: 'gpt-3.5-turbo',
    messages: [
      { role: 'system', content: 'You are a helpful assistant.' },
      { role: 'user', content: 'Hello! How are you today?' }
    ],
    temperature: 0.7,
    max_tokens: 150
  };

  try {
    const response = await router.router.chatCompletion(request);
    console.log('✅ Response:', response.choices[0].message.content);
    console.log('📊 Usage:', response.usage);
  } catch (error) {
    console.error('❌ Error:', error);
  }

  await router.router.destroy();
}

// ============================================================================
// Example 2: Multi-Provider with Failover
// ============================================================================

async function multiProviderFailover() {
  console.log('🔧 Setting up multi-provider with failover...');

  const router = await createMultiProviderBuilder({
    openai: process.env.OPENAI_API_KEY,
    anthropic: process.env.ANTHROPIC_API_KEY,
    ollama: 'http://localhost:11434'
  }).build();

  const request: ChatCompletionRequest = {
    model: 'gpt-4',
    messages: [
      { role: 'user', content: 'Explain quantum computing in simple terms.' }
    ],
    temperature: 0.5
  };

  try {
    const response = await router.router.chatCompletion(request);
    console.log('✅ Provider used:', (response as any).routingInfo?.selectedProvider);
    console.log('✅ Response:', response.choices[0].message.content);
  } catch (error) {
    console.error('❌ Error:', error);
  }

  await router.router.destroy();
}

// ============================================================================
// Example 3: Cost-Optimized Routing
// ============================================================================

async function costOptimizedRouting() {
  console.log('🔧 Setting up cost-optimized routing...');

  const router = await createCostOptimizedBuilder({
    openai: process.env.OPENAI_API_KEY,
    anthropic: process.env.ANTHROPIC_API_KEY
  }).build();

  const requests = [
    {
      model: 'gpt-3.5-turbo',
      messages: [{ role: 'user', content: 'Write a short poem about AI.' }]
    },
    {
      model: 'claude-3-haiku',
      messages: [{ role: 'user', content: 'Summarize the benefits of renewable energy.' }]
    }
  ];

  for (const request of requests) {
    try {
      const estimatedCost = router.router.estimateCost(request);
      console.log(`💰 Estimated cost for ${request.model}: $${estimatedCost.toFixed(4)}`);

      const response = await router.router.chatCompletion(request);
      console.log(`✅ ${request.model} response:`, response.choices[0].message.content.substring(0, 100) + '...');
    } catch (error) {
      console.error(`❌ Error with ${request.model}:`, error);
    }
  }

  // Get cost statistics
  const stats = router.router.getStats();
  console.log('📊 Total cost:', `$${stats.totalCost.toFixed(4)}`);

  await router.router.destroy();
}

// ============================================================================
// Example 4: Streaming Responses
// ============================================================================

async function streamingExample() {
  console.log('🔧 Setting up streaming example...');

  const router = await createLLMBuilder()
    .addOpenAI({
      apiKey: process.env.OPENAI_API_KEY,
      models: ['gpt-3.5-turbo']
    })
    .enableHealthChecks()
    .build();

  const request: ChatCompletionRequest = {
    model: 'gpt-3.5-turbo',
    messages: [
      { role: 'user', content: 'Write a detailed explanation of machine learning algorithms.' }
    ],
    stream: true,
    temperature: 0.7
  };

  try {
    console.log('🌊 Starting stream...');
    let fullContent = '';

    const stream = router.router.chatCompletionStream(request);
    for await (const chunk of stream) {
      const content = chunk.choices[0]?.delta?.content || '';
      if (content) {
        process.stdout.write(content);
        fullContent += content;
      }
    }

    console.log('\n✅ Stream completed!');
    console.log(`📝 Total characters: ${fullContent.length}`);
  } catch (error) {
    console.error('❌ Streaming error:', error);
  }

  await router.router.destroy();
}

// ============================================================================
// Example 5: Advanced Configuration
// ============================================================================

async function advancedConfiguration() {
  console.log('🔧 Setting up advanced configuration...');

  const router = await createLLMBuilder({
    timeout: 15000,
    maxRetries: 2,
    debug: true
  })
    .addOpenAI({
      name: 'openai-primary',
      apiKey: process.env.OPENAI_API_KEY,
      models: ['gpt-4', 'gpt-3.5-turbo'],
      priority: 1,
      rateLimit: {
        requestsPerMinute: 100,
        tokensPerMinute: 50000
      }
    })
    .addAnthropic({
      name: 'anthropic-secondary',
      apiKey: process.env.ANTHROPIC_API_KEY,
      models: ['claude-3-sonnet', 'claude-3-haiku'],
      priority: 2,
      rateLimit: {
        requestsPerMinute: 50,
        tokensPerMinute: 25000
      }
    })
    .setRoutingStrategy('performance-first')
    .enableHealthChecks({
      interval: 30,
      timeout: 5000,
      retries: 2
    })
    .enableCostTracking({
      budgetLimit: 50,
      alertThreshold: 80,
      trackPerUser: true
    })
    .setFailover({
      enabled: true,
      maxRetries: 3,
      backoffStrategy: 'exponential'
    })
    .build();

  // Test with multiple models
  const models = ['gpt-4', 'gpt-3.5-turbo', 'claude-3-sonnet', 'claude-3-haiku'];

  for (const model of models) {
    if (router.router.supportsModel(model)) {
      const request: ChatCompletionRequest = {
        model,
        messages: [
          { role: 'user', content: 'What are the latest trends in artificial intelligence?' }
        ],
        temperature: 0.3,
        max_tokens: 100
      };

      try {
        const startTime = Date.now();
        const response = await router.router.chatCompletion(request);
        const latency = Date.now() - startTime;

        console.log(`✅ ${model}:`);
        console.log(`   Provider: ${(response as any).routingInfo?.selectedProvider}`);
        console.log(`   Latency: ${latency}ms`);
        console.log(`   Content: ${response.choices[0].message.content.substring(0, 80)}...`);
        console.log('');
      } catch (error) {
        console.error(`❌ ${model} failed:`, error);
      }
    }
  }

  // Health check status
  const healthStatus = router.router.getProviderHealth();
  console.log('🏥 Provider Health:');
  if (Array.isArray(healthStatus)) {
    healthStatus.forEach(health => {
      console.log(`   ${health.provider}: ${health.isHealthy ? '✅' : '❌'} (${health.averageLatency}ms avg)`);
    });
  }

  // Statistics
  const stats = router.router.getStats();
  console.log('📊 Router Statistics:');
  console.log(`   Total requests: ${stats.totalRequests}`);
  console.log(`   Success rate: ${((stats.successfulRequests / stats.totalRequests) * 100).toFixed(1)}%`);
  console.log(`   Average latency: ${stats.averageLatency}ms`);
  console.log(`   Total cost: $${stats.totalCost.toFixed(4)}`);

  await router.router.destroy();
}

// ============================================================================
// Example 6: Custom Provider Configuration
// ============================================================================

async function customProviderConfiguration() {
  console.log('🔧 Setting up custom provider configuration...');

  const router = await createLLMBuilder()
    .addProvider({
      name: 'custom-openai',
      type: 'openai',
      endpoint: 'https://api.openai.com/v1',
      apiKey: process.env.OPENAI_API_KEY,
      models: ['gpt-4'],
      priority: 1,
      timeout: 20000,
      maxRetries: 2
    })
    .addProvider({
      name: 'local-ollama',
      type: 'ollama',
      endpoint: 'http://localhost:11434',
      models: ['llama2:7b', 'mistral:7b'],
      priority: 2
    })
    .setRoutingStrategy('failover-chain')
    .enableHealthChecks()
    .build();

  // Test provider capabilities
  const availableModels = router.router.getAvailableModels();
  console.log('📋 Available models by provider:');
  Object.entries(availableModels).forEach(([provider, models]) => {
    console.log(`   ${provider}: ${models.join(', ')}`);
  });

  const request: ChatCompletionRequest = {
    model: 'gpt-4',
    messages: [
      { role: 'user', content: 'Compare the advantages of cloud vs edge computing.' }
    ]
  };

  try {
    const response = await router.router.chatCompletion(request);
    console.log('✅ Response received from:', (response as any).routingInfo?.selectedProvider);
    console.log('✅ Content:', response.choices[0].message.content);
  } catch (error) {
    console.error('❌ Error:', error);
  }

  await router.router.destroy();
}

// ============================================================================
// Example 7: Error Handling and Monitoring
// ============================================================================

async function errorHandlingAndMonitoring() {
  console.log('🔧 Setting up error handling and monitoring...');

  const router = await createLLMBuilder()
    .addOpenAI({
      apiKey: 'invalid-key', // Intentionally invalid for testing
      models: ['gpt-3.5-turbo']
    })
    .addOllama({
      endpoint: 'http://localhost:11434',
      models: ['llama2']
    })
    .setRoutingStrategy('failover-chain')
    .enableHealthChecks({ interval: 10 })
    .setFailover({ enabled: true, maxRetries: 2 })
    .build();

  // Set up event listeners
  router.router.on('requestComplete', (data: any) => {
    console.log(`✅ Request completed: ${data.provider} (${data.latency}ms)`);
  });

  router.router.on('requestFailed', (data: any) => {
    console.log(`❌ Request failed: ${data.error} (${data.latency}ms)`);
  });

  router.router.on('healthCheck', (data: any) => {
    console.log(`🏥 Health check: ${data.provider} - ${data.isHealthy ? 'healthy' : 'unhealthy'}`);
  });

  router.router.on('providerError', (data: any) => {
    console.log(`⚠️  Provider error: ${data.provider} - ${data.error}`);
  });

  const request: ChatCompletionRequest = {
    model: 'gpt-3.5-turbo',
    messages: [
      { role: 'user', content: 'Hello, test message for error handling.' }
    ]
  };

  try {
    const response = await router.router.chatCompletion(request);
    console.log('✅ Fallback succeeded:', response.choices[0].message.content);
  } catch (error) {
    console.error('❌ All providers failed:', error);
  }

  // Wait a bit to see health checks
  console.log('⏳ Waiting for health checks...');
  await new Promise(resolve => setTimeout(resolve, 15000));

  await router.router.destroy();
}

// ============================================================================
// Example 8: Batch Processing
// ============================================================================

async function batchProcessing() {
  console.log('🔧 Setting up batch processing...');

  const router = await createLLMBuilder()
    .addOpenAI({
      apiKey: process.env.OPENAI_API_KEY,
      models: ['gpt-3.5-turbo']
    })
    .setRoutingStrategy('load-balanced')
    .enableCostTracking()
    .build();

  const batchRequests = [
    'Summarize the benefits of renewable energy.',
    'Explain the basics of blockchain technology.',
    'What are the main principles of sustainable development?',
    'Describe the impact of artificial intelligence on healthcare.',
    'How does quantum computing differ from classical computing?'
  ].map(content => ({
    model: 'gpt-3.5-turbo',
    messages: [{ role: 'user', content }],
    max_tokens: 100
  }));

  console.log(`🚀 Processing ${batchRequests.length} requests...`);

  const startTime = Date.now();
  const results = await Promise.allSettled(
    batchRequests.map((request, index) =>
      router.router.chatCompletion(request).then(response => ({
        index,
        request: request.messages[0].content,
        response: response.choices[0].message.content,
        usage: response.usage
      }))
    )
  );

  const totalTime = Date.now() - startTime;

  console.log(`⏱️  Total time: ${totalTime}ms`);
  console.log(`📊 Average time per request: ${(totalTime / batchRequests.length).toFixed(0)}ms`);

  let totalTokens = 0;
  results.forEach((result, index) => {
    if (result.status === 'fulfilled') {
      const data = result.value;
      totalTokens += data.usage?.total_tokens || 0;
      console.log(`\n✅ Request ${index + 1}:`);
      console.log(`   Q: ${data.request.substring(0, 50)}...`);
      console.log(`   A: ${data.response.substring(0, 80)}...`);
      console.log(`   Tokens: ${data.usage?.total_tokens || 0}`);
    } else {
      console.log(`\n❌ Request ${index + 1} failed:`, result.reason);
    }
  });

  const stats = router.router.getStats();
  console.log(`\n📈 Final Statistics:`);
  console.log(`   Total tokens processed: ${totalTokens}`);
  console.log(`   Total cost: $${stats.totalCost.toFixed(4)}`);
  console.log(`   Success rate: ${((stats.successfulRequests / stats.totalRequests) * 100).toFixed(1)}%`);

  await router.router.destroy();
}

// ============================================================================
// Main Function - Run Examples
// ============================================================================

async function main() {
  console.log('🚀 Circuit Breaker LLM Router Examples\n');

  const examples = [
    { name: 'Basic Single Provider', fn: basicSingleProvider },
    { name: 'Multi-Provider Failover', fn: multiProviderFailover },
    { name: 'Cost-Optimized Routing', fn: costOptimizedRouting },
    { name: 'Streaming Responses', fn: streamingExample },
    { name: 'Advanced Configuration', fn: advancedConfiguration },
    { name: 'Custom Provider Configuration', fn: customProviderConfiguration },
    { name: 'Error Handling and Monitoring', fn: errorHandlingAndMonitoring },
    { name: 'Batch Processing', fn: batchProcessing },
  ];

  // Check environment variables
  const missingEnvVars = [];
  if (!process.env.OPENAI_API_KEY) missingEnvVars.push('OPENAI_API_KEY');
  if (!process.env.ANTHROPIC_API_KEY) missingEnvVars.push('ANTHROPIC_API_KEY');

  if (missingEnvVars.length > 0) {
    console.log('⚠️  Missing environment variables:', missingEnvVars.join(', '));
    console.log('   Some examples may not work properly.\n');
  }

  // Run specific example or all
  const exampleToRun = process.argv[2];

  if (exampleToRun) {
    const example = examples.find(e =>
      e.name.toLowerCase().replace(/\s+/g, '-') === exampleToRun.toLowerCase()
    );

    if (example) {
      console.log(`Running example: ${example.name}\n`);
      await example.fn();
    } else {
      console.log('Available examples:');
      examples.forEach((example, index) => {
        const slug = example.name.toLowerCase().replace(/\s+/g, '-');
        console.log(`   ${index + 1}. ${example.name} (${slug})`);
      });
    }
  } else {
    // Run all examples
    for (const example of examples) {
      try {
        console.log(`\n${'='.repeat(60)}`);
        console.log(`🔹 ${example.name}`);
        console.log('='.repeat(60));
        await example.fn();
        console.log(`✅ ${example.name} completed successfully!`);
      } catch (error) {
        console.error(`❌ ${example.name} failed:`, error);
      }

      // Wait between examples
      await new Promise(resolve => setTimeout(resolve, 2000));
    }
  }

  console.log('\n🎉 All examples completed!');
}

// Error handling
process.on('unhandledRejection', (reason, promise) => {
  console.error('Unhandled Rejection at:', promise, 'reason:', reason);
  process.exit(1);
});

process.on('uncaughtException', (error) => {
  console.error('Uncaught Exception:', error);
  process.exit(1);
});

// Run if this file is executed directly
if (require.main === module) {
  main().catch(console.error);
}

export {
  basicSingleProvider,
  multiProviderFailover,
  costOptimizedRouting,
  streamingExample,
  advancedConfiguration,
  customProviderConfiguration,
  errorHandlingAndMonitoring,
  batchProcessing,
};
