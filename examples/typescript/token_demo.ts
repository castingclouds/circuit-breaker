#!/usr/bin/env npx tsx
// Token operations demonstration - TypeScript GraphQL Client
// Shows detailed token lifecycle operations using GraphQL API
// Run with: npx tsx examples/typescript/token_demo.ts

import { CircuitBreakerClient, type TokenGQL } from './basic_workflow.js';

function logSuccess(message: string) {
  console.log(`✅ ${message}`);
}

function logInfo(message: string) {
  console.log(`ℹ️  ${message}`);
}

function logError(message: string) {
  console.log(`❌ ${message}`);
}

function logWarning(message: string) {
  console.log(`⚠️  ${message}`);
}

async function main() {
  console.log('🚀 Circuit Breaker Token Operations Demo - TypeScript Client');
  console.log('============================================================');
  console.log();

  const client = new CircuitBreakerClient();

  try {
    // Create AI content creation workflow
    logInfo('Creating AI Content Creation Workflow...');
    
    const workflowInput = {
      name: 'AI-Powered Content Creation',
      places: ['ideation', 'drafting', 'review', 'revision', 'approval', 'published', 'archived'],
      transitions: [
        {
          id: 'start_drafting',
          fromPlaces: ['ideation'],
          toPlace: 'drafting',
          conditions: []
        },
        {
          id: 'submit_for_review',
          fromPlaces: ['drafting'],
          toPlace: 'review',
          conditions: []
        },
        {
          id: 'request_revision',
          fromPlaces: ['review'],
          toPlace: 'revision',
          conditions: []
        },
        {
          id: 'back_to_drafting',
          fromPlaces: ['revision'],
          toPlace: 'drafting',
          conditions: []
        },
        {
          id: 'approve_content',
          fromPlaces: ['review'],
          toPlace: 'approval',
          conditions: []
        },
        {
          id: 'publish_content',
          fromPlaces: ['approval'],
          toPlace: 'published',
          conditions: []
        },
        {
          id: 'archive_content',
          fromPlaces: ['published'],
          toPlace: 'archived',
          conditions: []
        },
        {
          id: 'back_to_ideation',
          fromPlaces: ['archived'],
          toPlace: 'ideation',
          conditions: []
        }
      ],
      initialPlace: 'ideation'
    };

    const workflowResult = await client.createWorkflow(workflowInput);
    
    if (workflowResult.errors) {
      logError(`Failed to create workflow: ${workflowResult.errors.map(e => e.message).join(', ')}`);
      return;
    }

    const workflow = workflowResult.data!.createWorkflow;
    logSuccess(`Created workflow: ${workflow.name} (${workflow.id})`);
    console.log();

    // Create multiple content tokens
    const contentTopics = [
      {
        title: 'Introduction to Rust Programming',
        type: 'tutorial',
        targetAudience: 'beginners',
        estimatedReadTime: 15
      },
      {
        title: 'Advanced TypeScript Patterns',
        type: 'guide',
        targetAudience: 'intermediate',
        estimatedReadTime: 25
      },
      {
        title: 'Building Scalable APIs with GraphQL',
        type: 'article',
        targetAudience: 'advanced',
        estimatedReadTime: 20
      }
    ];

    const tokens: TokenGQL[] = [];
    
    for (const [index, topic] of contentTopics.entries()) {
      logInfo(`Creating content token ${index + 1}/3: ${topic.title}`);
      
      const tokenInput = {
        workflowId: workflow.id,
        initialPlace: 'ideation',
        data: {
          title: topic.title,
          type: topic.type,
          targetAudience: topic.targetAudience,
          estimatedReadTime: topic.estimatedReadTime,
          keywords: topic.title.toLowerCase().split(' ').slice(0, 3),
          status: 'planning',
          wordCount: 0,
          authorId: 'ai-assistant',
          createdAt: new Date().toISOString()
        },
        metadata: {
          priority: index === 1 ? 'high' : 'medium',
          contentType: topic.type,
          seoOptimized: false,
          socialMediaReady: false,
          version: '1.0'
        }
      };

      const tokenResult = await client.createToken(tokenInput);
      
      if (tokenResult.errors) {
        logError(`Failed to create token: ${tokenResult.errors.map(e => e.message).join(', ')}`);
        continue;
      }

      const token = tokenResult.data!.createToken;
      tokens.push(token);
      logSuccess(`Created token: ${token.data.title} (${token.id})`);
    }

    console.log();
    logInfo(`Created ${tokens.length} content tokens`);
    
    // Demonstrate different token lifecycles
    for (const [index, token] of tokens.entries()) {
      console.log();
      logInfo(`Processing content ${index + 1}: ${token.data.title}`);
      
      let currentToken = token;
      
      // Start drafting
      logInfo('Starting drafting phase...');
      const draftResult = await client.fireTransition({
        tokenId: currentToken.id,
        transitionId: 'start_drafting',
        data: {
          action: 'start_drafting',
          timestamp: new Date().toISOString(),
          notes: 'AI content generation initiated'
        }
      });
      
      if (draftResult.errors) {
        logError(`Failed to start drafting: ${draftResult.errors.map(e => e.message).join(', ')}`);
        continue;
      }
      
      currentToken = draftResult.data!.fireTransition;
      
      // Update token data to simulate content creation
      currentToken.data.wordCount = Math.floor(Math.random() * 1000) + 500;
      currentToken.data.status = 'draft_complete';
      
      // Submit for review
      logInfo('Submitting for review...');
      const reviewResult = await client.fireTransition({
        tokenId: currentToken.id,
        transitionId: 'submit_for_review',
        data: {
          action: 'submit_for_review',
          timestamp: new Date().toISOString(),
          wordCount: currentToken.data.wordCount,
          readabilityScore: Math.floor(Math.random() * 40) + 60
        }
      });
      
      if (reviewResult.errors) {
        logError(`Failed to submit for review: ${reviewResult.errors.map(e => e.message).join(', ')}`);
        continue;
      }
      
      currentToken = reviewResult.data!.fireTransition;
      
      // Simulate different review outcomes
      const reviewOutcome = index === 1 ? 'revision' : 'approval';
      
      if (reviewOutcome === 'revision') {
        logWarning('Content needs revision...');
        
        const revisionResult = await client.fireTransition({
          tokenId: currentToken.id,
          transitionId: 'request_revision',
          data: {
            action: 'request_revision',
            timestamp: new Date().toISOString(),
            feedback: 'Needs more technical examples and clearer explanations'
          }
        });
        
        if (revisionResult.errors) {
          logError(`Failed to request revision: ${revisionResult.errors.map(e => e.message).join(', ')}`);
          continue;
        }
        
        currentToken = revisionResult.data!.fireTransition;
        
        // Back to drafting
        logInfo('Returning to drafting for revisions...');
        const backToDraftResult = await client.fireTransition({
          tokenId: currentToken.id,
          transitionId: 'back_to_drafting',
          data: {
            action: 'back_to_drafting',
            timestamp: new Date().toISOString(),
            revisionNotes: 'Incorporating reviewer feedback'
          }
        });
        
        if (backToDraftResult.errors) {
          logError(`Failed to return to drafting: ${backToDraftResult.errors.map(e => e.message).join(', ')}`);
          continue;
        }
        
        currentToken = backToDraftResult.data!.fireTransition;
        
        // Resubmit
        logInfo('Resubmitting revised content...');
        const resubmitResult = await client.fireTransition({
          tokenId: currentToken.id,
          transitionId: 'submit_for_review',
          data: {
            action: 'resubmit_for_review',
            timestamp: new Date().toISOString(),
            revisionComplete: true,
            wordCount: currentToken.data.wordCount + 200
          }
        });
        
        if (resubmitResult.errors) {
          logError(`Failed to resubmit: ${resubmitResult.errors.map(e => e.message).join(', ')}`);
          continue;
        }
        
        currentToken = resubmitResult.data!.fireTransition;
      }
      
      // Approve content
      logInfo('Approving content...');
      const approveResult = await client.fireTransition({
        tokenId: currentToken.id,
        transitionId: 'approve_content',
        data: {
          action: 'approve_content',
          timestamp: new Date().toISOString(),
          approvedBy: 'content-manager',
          qualityScore: Math.floor(Math.random() * 20) + 80
        }
      });
      
      if (approveResult.errors) {
        logError(`Failed to approve content: ${approveResult.errors.map(e => e.message).join(', ')}`);
        continue;
      }
      
      currentToken = approveResult.data!.fireTransition;
      
      // Publish content
      logInfo('Publishing content...');
      const publishResult = await client.fireTransition({
        tokenId: currentToken.id,
        transitionId: 'publish_content',
        data: {
          action: 'publish_content',
          timestamp: new Date().toISOString(),
          publishUrl: `https://blog.example.com/${currentToken.data.title.toLowerCase().replace(/\s+/g, '-')}`,
          seoOptimized: true
        }
      });
      
      if (publishResult.errors) {
        logError(`Failed to publish content: ${publishResult.errors.map(e => e.message).join(', ')}`);
        continue;
      }
      
      currentToken = publishResult.data!.fireTransition;
      logSuccess(`Content published: ${currentToken.data.title}`);
      
      // Show token history
      logInfo('Token lifecycle history:');
      currentToken.history.forEach((event, histIndex) => {
        const timestamp = new Date(event.timestamp).toLocaleTimeString();
        console.log(`  ${histIndex + 1}. ${event.fromPlace} → ${event.toPlace} via ${event.transition} (${timestamp})`);
      });
    }

    console.log();
    logInfo('Token Demo Summary:');
    console.log('  • Created multiple content tokens with different data');
    console.log('  • Demonstrated complex workflow with revision cycles');
    console.log('  • Showed token state transitions and data updates');
    console.log('  • Complete audit trail for each token lifecycle');
    console.log('  • TypeScript integration with GraphQL API');

  } catch (error) {
    logError(`Demo failed: ${error}`);
    process.exit(1);
  }
}

if (require.main === module) {
  main().catch(console.error);
}

export { main as tokenDemo }; 