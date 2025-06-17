/**
 * Agent Builder Examples for Circuit Breaker SDK
 *
 * This file demonstrates how to create and use various types of agents
 * with the Circuit Breaker AgentBuilder, including conversational agents,
 * state machine agents, and complex workflow integrations.
 */

import {
  createAgent,
  createConversationalAgent,
  createWorkflowAgent,
  AgentTemplates,
  ConversationalTemplates,
  StateMachineTemplates,
  createLLMRouter,
  createMultiProviderBuilder,
  AgentContext,
  ToolImplementation,
} from '../../src/index.js';

// ============================================================================
// Example 1: Basic Conversational Agent
// ============================================================================

async function basicConversationalAgent() {
  console.log('🤖 Creating basic conversational agent...');

  // Set up LLM router
  const llmRouter = await createMultiProviderBuilder({
    openai: process.env.OPENAI_API_KEY,
    anthropic: process.env.ANTHROPIC_API_KEY,
  }).build();

  // Create a basic conversational agent
  const agent = await createConversationalAgent(
    'You are a helpful assistant that can answer questions and provide information.',
    {
      name: 'Basic Assistant',
      defaultLLMProvider: 'openai-primary',
    }
  )
    .enableMemory({ type: 'both', maxSize: 500 })
    .setConversationConfig({
      maxTurns: 20,
      temperature: 0.7,
      maxTokens: 500,
    })
    .build(llmRouter.router);

  // Have a conversation
  const response1 = await agent.agent.chat('Hello! What can you help me with?', {
    conversationId: 'demo-conversation',
    userId: 'user-123',
  });
  console.log('Assistant:', response1);

  const response2 = await agent.agent.chat('Tell me about machine learning.', {
    conversationId: 'demo-conversation',
    userId: 'user-123',
  });
  console.log('Assistant:', response2);

  // Get conversation statistics
  const stats = agent.agent.getStats();
  console.log('Agent stats:', stats);

  await agent.agent.destroy();
  await llmRouter.router.destroy();
}

// ============================================================================
// Example 2: Customer Support Agent with Tools
// ============================================================================

async function customerSupportAgent() {
  console.log('🎧 Creating customer support agent with tools...');

  const llmRouter = await createMultiProviderBuilder({
    openai: process.env.OPENAI_API_KEY,
  }).build();

  // Define tool implementations
  const searchKnowledgeBase: ToolImplementation = async (params, context) => {
    console.log('🔍 Searching knowledge base for:', params.query);
    // Mock knowledge base search
    return {
      results: [
        {
          title: 'How to reset your password',
          content: 'To reset your password, go to the login page and click "Forgot Password"...',
          relevance: 0.9,
        },
        {
          title: 'Account billing information',
          content: 'You can view your billing information in the account settings...',
          relevance: 0.7,
        },
      ],
      totalResults: 2,
    };
  };

  const createSupportTicket: ToolImplementation = async (params, context) => {
    console.log('🎫 Creating support ticket:', params);
    // Mock ticket creation
    const ticketId = `TICKET-${Date.now()}`;
    return {
      ticketId,
      status: 'created',
      priority: params.priority || 'medium',
      assignedTo: 'support-team',
      estimatedResolution: '24 hours',
    };
  };

  const getUserInfo: ToolImplementation = async (params, context) => {
    console.log('👤 Getting user info for:', context.userId);
    // Mock user lookup
    return {
      userId: context.userId,
      name: 'John Doe',
      email: 'john.doe@example.com',
      accountType: 'premium',
      joinDate: '2023-01-15',
      ticketHistory: ['TICKET-123', 'TICKET-456'],
    };
  };

  // Create customer support agent
  const agent = await createConversationalAgent(
    `You are a professional customer support agent. Your goal is to help customers resolve their issues quickly and efficiently.

Guidelines:
- Be empathetic and understanding
- Listen carefully to customer concerns
- Search the knowledge base before creating tickets
- Escalate complex issues appropriately
- Always ensure customer satisfaction

You have access to tools for searching our knowledge base, creating support tickets, and looking up user information.`,
    {
      name: 'Customer Support Agent',
      defaultLLMProvider: 'openai-primary',
    }
  )
    .addTool(
      'search_knowledge_base',
      'Search the knowledge base for relevant articles and solutions',
      {
        type: 'object',
        properties: {
          query: { type: 'string', description: 'Search query' },
          category: { type: 'string', description: 'Optional category filter' },
        },
        required: ['query'],
      },
      searchKnowledgeBase
    )
    .addTool(
      'create_support_ticket',
      'Create a support ticket for issues that require human intervention',
      {
        type: 'object',
        properties: {
          title: { type: 'string', description: 'Ticket title' },
          description: { type: 'string', description: 'Detailed issue description' },
          priority: { type: 'string', enum: ['low', 'medium', 'high'], description: 'Ticket priority' },
          category: { type: 'string', description: 'Issue category' },
        },
        required: ['title', 'description'],
      },
      createSupportTicket
    )
    .addTool(
      'get_user_info',
      'Get information about the current user',
      {
        type: 'object',
        properties: {
          includeHistory: { type: 'boolean', description: 'Include ticket history' },
        },
      },
      getUserInfo
    )
    .enableMemory({ type: 'both', maxSize: 1000, persistent: true })
    .setConversationConfig({
      maxTurns: 30,
      temperature: 0.3,
      maxTokens: 800,
    })
    .build(llmRouter.router);

  // Simulate customer support conversation
  console.log('\n--- Customer Support Conversation ---');

  const messages = [
    "Hi, I'm having trouble logging into my account.",
    "I tried resetting my password but I'm not receiving the email.",
    "My email is john.doe@example.com and I've been a customer since last year.",
    "This is really urgent as I need to access my account for work.",
  ];

  for (const message of messages) {
    console.log('\nCustomer:', message);
    const response = await agent.agent.chat(message, {
      conversationId: 'support-session-001',
      userId: 'user-john-doe',
      session: { supportLevel: 'premium', urgency: 'high' },
    });
    console.log('Support Agent:', response);
  }

  await agent.agent.destroy();
  await llmRouter.router.destroy();
}

// ============================================================================
// Example 3: Order Processing State Machine Agent
// ============================================================================

async function orderProcessingStateMachine() {
  console.log('📦 Creating order processing state machine agent...');

  const llmRouter = await createMultiProviderBuilder({
    openai: process.env.OPENAI_API_KEY,
  }).build();

  // Define state machine tools
  const validateOrder: ToolImplementation = async (params, context) => {
    console.log('✅ Validating order:', params);
    // Mock order validation
    const orderData = context.session.orderData || {};
    const isValid = orderData.product && orderData.quantity && orderData.address;

    // Update context variables
    await context.memory.store('validationResult', isValid ? 'valid' : 'invalid');
    await context.memory.store('hasAllInfo', Boolean(isValid));

    return {
      isValid,
      errors: isValid ? [] : ['Missing required information'],
      estimatedTotal: isValid ? orderData.quantity * 99.99 : 0,
    };
  };

  const processPayment: ToolImplementation = async (params, context) => {
    console.log('💳 Processing payment:', params);
    // Mock payment processing
    const success = Math.random() > 0.2; // 80% success rate

    await context.memory.store('paymentStatus', success ? 'success' : 'failed');

    return {
      success,
      transactionId: success ? `TXN-${Date.now()}` : null,
      message: success ? 'Payment processed successfully' : 'Payment failed - please try again',
    };
  };

  const fulfillOrder: ToolImplementation = async (params, context) => {
    console.log('📋 Fulfilling order:', params);
    // Mock order fulfillment
    const orderId = `ORDER-${Date.now()}`;

    await context.memory.store('orderId', orderId);
    await context.memory.store('orderStatus', 'fulfilled');

    return {
      orderId,
      trackingNumber: `TRACK-${Date.now()}`,
      estimatedDelivery: '2-3 business days',
    };
  };

  // Create state machine agent
  const agent = await createWorkflowAgent({
    name: 'Order Processing Agent',
    defaultLLMProvider: 'openai-primary',
  })
    .setStateMachine({
      initialState: 'greeting',
      states: {
        greeting: {
          name: 'greeting',
          prompt: 'Welcome! I can help you place an order. What would you like to purchase today?',
          availableTransitions: ['collecting_info'],
          onEntry: [],
          onExit: [],
        },
        collecting_info: {
          name: 'collecting_info',
          prompt: `I need some information to process your order:
- Product name
- Quantity
- Delivery address

Please provide these details.`,
          availableTransitions: ['validating', 'collecting_info'],
          onEntry: [],
          onExit: [],
        },
        validating: {
          name: 'validating',
          prompt: 'Let me validate your order information...',
          availableTransitions: ['payment', 'collecting_info'],
          onEntry: [
            {
              type: 'function_call',
              config: { functionName: 'validateOrder' },
            },
          ],
          onExit: [],
        },
        payment: {
          name: 'payment',
          prompt: 'Your order looks good! The total is ${{estimatedTotal}}. Please confirm to proceed with payment.',
          availableTransitions: ['processing', 'payment_failed'],
          onEntry: [],
          onExit: [],
        },
        processing: {
          name: 'processing',
          prompt: 'Processing your payment and preparing your order...',
          availableTransitions: ['fulfillment', 'payment_failed'],
          onEntry: [
            {
              type: 'function_call',
              config: { functionName: 'processPayment' },
            },
          ],
          onExit: [],
        },
        fulfillment: {
          name: 'fulfillment',
          prompt: 'Payment successful! Preparing your order for shipment...',
          availableTransitions: ['completed'],
          onEntry: [
            {
              type: 'function_call',
              config: { functionName: 'fulfillOrder' },
            },
          ],
          onExit: [],
        },
        completed: {
          name: 'completed',
          prompt: 'Order completed! Your order {{orderId}} will be delivered in {{estimatedDelivery}}. Tracking: {{trackingNumber}}',
          defaultResponse: 'Thank you for your order!',
          availableTransitions: [],
          onEntry: [],
          onExit: [],
        },
        payment_failed: {
          name: 'payment_failed',
          prompt: 'Payment failed. Would you like to try a different payment method?',
          availableTransitions: ['payment', 'cancelled'],
          onEntry: [],
          onExit: [],
        },
        cancelled: {
          name: 'cancelled',
          prompt: 'Order cancelled. Is there anything else I can help you with?',
          availableTransitions: ['greeting'],
          onEntry: [],
          onExit: [],
        },
      },
      transitions: [
        { fromState: 'greeting', toState: 'collecting_info', trigger: 'order' },
        { fromState: 'collecting_info', toState: 'validating', condition: 'hasAllInfo == true' },
        { fromState: 'validating', toState: 'payment', condition: 'validationResult == "valid"' },
        { fromState: 'validating', toState: 'collecting_info', condition: 'validationResult == "invalid"' },
        { fromState: 'payment', toState: 'processing', trigger: 'confirm' },
        { fromState: 'payment', toState: 'cancelled', trigger: 'cancel' },
        { fromState: 'processing', toState: 'fulfillment', condition: 'paymentStatus == "success"' },
        { fromState: 'processing', toState: 'payment_failed', condition: 'paymentStatus == "failed"' },
        { fromState: 'fulfillment', toState: 'completed', condition: 'orderStatus == "fulfilled"' },
        { fromState: 'payment_failed', toState: 'payment', trigger: 'retry' },
        { fromState: 'payment_failed', toState: 'cancelled', trigger: 'cancel' },
      ],
      variables: {
        hasAllInfo: false,
        validationResult: '',
        paymentStatus: '',
        orderStatus: '',
        estimatedTotal: 0,
        orderId: '',
        trackingNumber: '',
        estimatedDelivery: '',
      },
    })
    .addTool('validateOrder', 'Validate order information', {}, validateOrder)
    .addTool('processPayment', 'Process customer payment', {}, processPayment)
    .addTool('fulfillOrder', 'Fulfill and ship the order', {}, fulfillOrder)
    .enableMemory({ type: 'both', persistent: true })
    .build(llmRouter.router);

  // Simulate order processing conversation
  console.log('\n--- Order Processing Conversation ---');

  const orderMessages = [
    'Hi, I want to place an order.',
    'I want to buy a laptop, quantity 1, and deliver to 123 Main St, Anytown USA.',
    'Yes, that looks correct. Please proceed.',
    'Confirmed, please process the payment.',
  ];

  for (const message of orderMessages) {
    console.log('\nCustomer:', message);
    const response = await agent.agent.chat(message, {
      conversationId: 'order-session-001',
      userId: 'customer-456',
      session: {
        orderData: {
          product: 'laptop',
          quantity: 1,
          address: '123 Main St, Anytown USA',
        },
      },
    });
    console.log('Order Agent:', response);

    // Show current state
    const currentState = agent.agent.getSessionState('order-session-001');
    console.log(`Current State: ${currentState?.currentState}`);
  }

  await agent.agent.destroy();
  await llmRouter.router.destroy();
}

// ============================================================================
// Example 4: Using Pre-built Agent Templates
// ============================================================================

async function agentTemplates() {
  console.log('📋 Using pre-built agent templates...');

  const llmRouter = await createMultiProviderBuilder({
    openai: process.env.OPENAI_API_KEY,
  }).build();

  // Customer Support Template
  console.log('\n--- Customer Support Template ---');
  const supportAgent = AgentTemplates.customerSupport().addCustomerSupportTools();
  const supportResult = await supportAgent.build(llmRouter.router);

  const supportResponse = await supportResult.agent.chat(
    'I need help with my billing statement.',
    { conversationId: 'support-template', userId: 'user-789' }
  );
  console.log('Support Agent:', supportResponse);

  // Sales Assistant Template
  console.log('\n--- Sales Assistant Template ---');
  const salesAgent = AgentTemplates.salesAssistant().addSalesTools();
  const salesResult = await salesAgent.build(llmRouter.router);

  const salesResponse = await salesResult.agent.chat(
    'I\'m looking for a good laptop for programming.',
    { conversationId: 'sales-template', userId: 'user-789' }
  );
  console.log('Sales Agent:', salesResponse);

  // Technical Support Template
  console.log('\n--- Technical Support Template ---');
  const techAgent = AgentTemplates.technicalSupport();
  const techResult = await techAgent.build(llmRouter.router);

  const techResponse = await techResult.agent.chat(
    'My application keeps crashing when I try to export data.',
    { conversationId: 'tech-template', userId: 'user-789' }
  );
  console.log('Tech Support:', techResponse);

  // Clean up
  await supportResult.agent.destroy();
  await salesResult.agent.destroy();
  await techResult.agent.destroy();
  await llmRouter.router.destroy();
}

// ============================================================================
// Example 5: Advanced Agent with Complex Memory and Context
// ============================================================================

async function advancedAgentWithMemory() {
  console.log('🧠 Creating advanced agent with complex memory...');

  const llmRouter = await createMultiProviderBuilder({
    openai: process.env.OPENAI_API_KEY,
  }).build();

  // Advanced memory tools
  const rememberFact: ToolImplementation = async (params, context) => {
    await context.memory.store(`fact_${params.key}`, params.value, 'long_term');
    return { stored: true, key: params.key };
  };

  const recallFact: ToolImplementation = async (params, context) => {
    const value = await context.memory.retrieve(`fact_${params.key}`, 'long_term');
    return { key: params.key, value: value || 'Not found' };
  };

  const analyzeContext: ToolImplementation = async (params, context) => {
    const conversationHistory = await context.memory.getConversationHistory(context.conversationId || 'default');
    return {
      messageCount: conversationHistory.length,
      topics: ['memory', 'learning', 'AI'], // Mock topic extraction
      sentiment: 'positive',
      complexity: 'medium',
    };
  };

  // Create advanced agent
  const agent = await createAgent({
    name: 'Advanced Learning Assistant',
    defaultLLMProvider: 'openai-primary',
  })
    .setType('conversational')
    .setSystemPrompt(`You are an advanced AI assistant with sophisticated memory capabilities. You can:
- Remember facts and information long-term
- Recall previous conversations and context
- Learn about user preferences over time
- Adapt your responses based on accumulated knowledge

You have tools to store and retrieve information, and analyze conversation context. Use these capabilities to provide increasingly personalized and helpful responses.`)
    .addTool(
      'remember_fact',
      'Store a fact or piece of information for long-term memory',
      {
        type: 'object',
        properties: {
          key: { type: 'string', description: 'Unique identifier for the fact' },
          value: { type: 'string', description: 'The information to remember' },
        },
        required: ['key', 'value'],
      },
      rememberFact
    )
    .addTool(
      'recall_fact',
      'Retrieve a previously stored fact from memory',
      {
        type: 'object',
        properties: {
          key: { type: 'string', description: 'The identifier of the fact to recall' },
        },
        required: ['key'],
      },
      recallFact
    )
    .addTool(
      'analyze_context',
      'Analyze the current conversation context and history',
      {
        type: 'object',
        properties: {
          depth: { type: 'string', enum: ['shallow', 'deep'], description: 'Level of analysis' },
        },
      },
      analyzeContext
    )
    .enableMemory({
      type: 'both',
      maxSize: 2000,
      persistent: true,
      retention: {
        shortTermTTL: 3600, // 1 hour
        longTermTTL: 86400 * 30, // 30 days
      },
    })
    .setConversationConfig({
      maxTurns: 50,
      temperature: 0.7,
      maxTokens: 1000,
      enableSummarization: true,
      summarizationThreshold: 10,
    })
    .build(llmRouter.router);

  // Extended conversation with memory
  console.log('\n--- Advanced Memory Conversation ---');

  const memoryMessages = [
    'Hi, I\'m Sarah and I\'m a software engineer working on machine learning projects.',
    'I prefer Python for ML work and I\'m particularly interested in computer vision.',
    'Can you remember that I work at TechCorp and my current project is about object detection?',
    'What do you remember about me so far?',
    'What would you recommend for my next learning goal?',
  ];

  for (const message of memoryMessages) {
    console.log('\nSarah:', message);
    const response = await agent.agent.chat(message, {
      conversationId: 'advanced-memory-session',
      userId: 'sarah-engineer',
      session: {
        userProfile: {
          profession: 'software_engineer',
          experience: 'senior',
          interests: ['ML', 'AI', 'computer_vision'],
        },
      },
    });
    console.log('Assistant:', response);
  }

  // Check memory statistics
  const memoryStats = agent.agent.memory.getStats();
  console.log('\n📊 Memory Statistics:', memoryStats);

  await agent.agent.destroy();
  await llmRouter.router.destroy();
}

// ============================================================================
// Example 6: Multi-Agent Coordination
// ============================================================================

async function multiAgentCoordination() {
  console.log('🤝 Creating multi-agent coordination example...');

  const llmRouter = await createMultiProviderBuilder({
    openai: process.env.OPENAI_API_KEY,
  }).build();

  // Create specialized agents

  // Research Agent
  const researchAgent = await createAgent({
    name: 'Research Specialist',
  })
    .setType('conversational')
    .setSystemPrompt('You are a research specialist. Your job is to gather and analyze information on given topics. Provide thorough, well-researched responses with factual information.')
    .addTool(
      'research_topic',
      'Research a specific topic',
      {
        type: 'object',
        properties: {
          topic: { type: 'string', description: 'Topic to research' },
          depth: { type: 'string', enum: ['basic', 'detailed'], description: 'Research depth' },
        },
        required: ['topic'],
      },
      async (params, context) => {
        console.log('🔬 Researching:', params.topic);
        return {
          topic: params.topic,
          findings: [
            'Key finding 1 about the topic',
            'Important data point 2',
            'Relevant trend 3',
          ],
          sources: ['Source A', 'Source B', 'Source C'],
          confidence: 0.85,
        };
      }
    )
    .build(llmRouter.router);

  // Analysis Agent
  const analysisAgent = await createAgent({
    name: 'Data Analyst',
  })
    .setType('conversational')
    .setSystemPrompt('You are a data analyst. You take research findings and create structured analysis with insights and recommendations.')
    .addTool(
      'analyze_data',
      'Analyze research data and findings',
      {
        type: 'object',
        properties: {
          data: { type: 'string', description: 'Data to analyze' },
          analysisType: { type: 'string', description: 'Type of analysis needed' },
        },
        required: ['data'],
      },
      async (params, context) => {
        console.log('📊 Analyzing data for:', params.analysisType);
        return {
          insights: [
            'Trend analysis shows growth',
            'Key correlation identified',
            'Risk factors present',
          ],
          recommendations: [
            'Recommendation 1',
            'Recommendation 2',
          ],
          confidence: 0.9,
        };
      }
    )
    .build(llmRouter.router);

  // Coordinator Agent
  const coordinatorAgent = await createAgent({
    name: 'Project Coordinator',
  })
    .setType('conversational')
    .setSystemPrompt('You are a project coordinator. You manage workflows between different specialists and ensure tasks are completed efficiently.')
    .addTool(
      'delegate_task',
      'Delegate a task to a specialist agent',
      {
        type: 'object',
        properties: {
          agent: { type: 'string', description: 'Target agent for the task' },
          task: { type: 'string', description: 'Task description' },
          priority: { type: 'string', enum: ['low', 'medium', 'high'], description: 'Task priority' },
        },
        required: ['agent', 'task'],
      },
      async (params, context) => {
        console.log(`📋 Delegating to ${params.agent}:`, params.task);

        // Simulate delegation to appropriate agent
        if (params.agent === 'research') {
          const researchResult = await researchAgent.agent.chat(params.task, {
            conversationId: 'research-delegation',
            userId: 'coordinator',
          });
          return { agent: 'research', result: researchResult };
        } else if (params.agent === 'analysis') {
          const analysisResult = await analysisAgent.agent.chat(params.task, {
            conversationId: 'analysis-delegation',
            userId: 'coordinator',
          });
          return { agent: 'analysis', result: analysisResult };
        }

        return { agent: params.agent, result: 'Task delegated successfully' };
      }
    )
    .build(llmRouter.router);

  // Multi-agent workflow
  console.log('\n--- Multi-Agent Coordination Workflow ---');

  const userRequest = 'I need a comprehensive analysis of the electric vehicle market trends for 2024';

  console.log('User Request:', userRequest);

  // Coordinator handles the request
  const coordinatorResponse = await coordinatorAgent.agent.chat(
    `Please coordinate a comprehensive analysis project: "${userRequest}". Break this down into research and analysis tasks.`,
    {
      conversationId: 'coordination-session',
      userId: 'project-manager',
    }
  );

  console.log('\nCoordinator Response:', coordinatorResponse);

  // Clean up
  await researchAgent.agent.destroy();
  await analysisAgent.agent.destroy();
  await coordinatorAgent.agent.destroy();
  await llmRouter.router.destroy();
}

// ============================================================================
// Main Function - Run All Examples
// ============================================================================

async function main() {
  console.log('🚀 Circuit Breaker Agent Examples\n');

  const examples = [
    { name: 'Basic Conversational Agent', fn: basicConversationalAgent },
    { name: 'Customer Support Agent with Tools', fn: customerSupportAgent },
    { name: 'Order Processing State Machine', fn: orderProcessingStateMachine },
    { name: 'Pre-built Agent Templates', fn: agentTemplates },
    { name: 'Advanced Agent with Memory', fn: advancedAgentWithMemory },
    { name: 'Multi-Agent Coordination', fn: multiAgentCoordination },
  ];

  // Check environment variables
  const missingEnvVars = [];
  if (!process.env.OPENAI_API_KEY) missingEnvVars.push('OPENAI_API_KEY');

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

  console.log('\n🎉 All agent examples completed!');
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
  basicConversationalAgent,
  customerSupportAgent,
  orderProcessingStateMachine,
  agentTemplates,
  advancedAgentWithMemory,
  multiAgentCoordination,
};
