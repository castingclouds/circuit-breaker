use anyhow::Result;
use circuit_breaker_sdk::{
    agents::{AgentExecutionRequest, ToolDefinition},
    Client,
};
use futures::StreamExt;
use serde_json::json;
use std::{
    env,
    io::{self, Write},
    time::Duration,
};
use tracing::info;

// Helper function to handle escape characters in streaming content
fn process_escape_characters(content: &str) -> String {
    content
        .replace("\\n", "\n") // Convert \n to actual newlines
        .replace("\\t", "\t") // Convert \t to actual tabs
        .replace("\\r", "\r") // Convert \r to carriage returns
        .replace("\\\"", "\"") // Convert \" to actual quotes
        .replace("\\'", "'") // Convert \' to actual apostrophes
        .replace("\\b", "\x08") // Convert \b to backspace
        .replace("\\f", "\x0C") // Convert \f to form feed
        .replace("\\v", "\x0B") // Convert \v to vertical tab
        .replace("\\0", "\0") // Convert \0 to null character
        .replace("\\\\", "\\") // Convert \\ to actual backslash (must be last)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🧪 Simple Streaming Demo");
    println!("========================");

    // Initialize the client
    let base_url =
        env::var("CIRCUIT_BREAKER_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
    let api_key = env::var("CIRCUIT_BREAKER_API_KEY").ok();

    let mut client_builder = Client::builder().base_url(&base_url)?;
    if let Some(key) = api_key {
        client_builder = client_builder.api_key(key);
    }
    let client = client_builder.build()?;

    // Test connection
    match client.ping().await {
        Ok(ping) => println!("✅ Connected to server: {}", ping.status),
        Err(e) => {
            println!("❌ Failed to connect: {}", e);
            return Ok(());
        }
    }

    let tenant_id = "test-tenant";

    println!("\n📝 Creating test agent...");
    let agent_id = create_test_agent(&client, tenant_id).await?;
    println!("✅ Created agent: {}", agent_id);

    // Test 1: SSE Streaming
    println!("\n🌊 Testing SSE Streaming...");
    test_sse_streaming(&client, &agent_id, tenant_id).await;

    // Test 2: WebSocket
    println!("\n🔌 Testing WebSocket...");
    test_websocket(&client, &agent_id, tenant_id).await;

    // Test 3: Complex Reasoning Agent
    println!("\n🧠 Testing Complex Reasoning Agent...");
    test_complex_reasoning_agent(&client, tenant_id).await;

    // Test 4: Stock Trading Agent
    println!("\n📊 Testing Stock Trading Agent...");
    test_stock_trading_agent(&client, tenant_id).await;

    // Test 5: Blackjack Game Agent
    println!("\n🃏 Testing Blackjack Game Agent...");
    test_blackjack_game_agent(&client, tenant_id).await;

    println!("\n✅ Demo completed!");
    Ok(())
}

async fn test_complex_reasoning_agent(client: &Client, tenant_id: &str) {
    println!("   Creating expert Rust algorithm architect...");

    let agent_id = match create_complex_reasoning_agent(client, tenant_id).await {
        Ok(id) => {
            println!("   ✅ Created reasoning agent: {}", id);
            id
        }
        Err(e) => {
            println!("   ❌ Failed to create reasoning agent: {}", e);
            return;
        }
    };

    let request = AgentExecutionRequest {
        context: json!({
            "message": "Design and implement a high-performance Rust algorithm for sorting a vector of custom matrix structures. Each matrix is 3x3 with f64 values. The sorting should be based on matrix determinant, but needs to handle edge cases like singular matrices. The solution should be optimized for large datasets (1M+ matrices) and include comprehensive benchmarks.\n\nREQUIREMENTS:\n- Custom Matrix3x3 struct with determinant calculation\n- Efficient sorting algorithm (consider parallel processing)\n- Handle edge cases (NaN, infinity, singular matrices)\n- Memory-efficient implementation\n- Comprehensive test suite\n- Performance benchmarks\n- Documentation with complexity analysis\n\nCONSTRAINTS:\n- Target performance: < 100ms for 10k matrices\n- Memory limit: reasonable for embedded systems\n- Rust version: stable 1.70+\n\nPlease use your available tools to analyze, design, and implement this solution step by step.",
            "tenant_id": tenant_id
        }),
        mapping: Some(json!({
            "message": "message"
        })),
        tenant_id: Some(tenant_id.to_string()),
        stream: Some(true),
    };

    println!("   🚀 Starting complex algorithm design session...");
    println!("   📋 Task: High-performance matrix sorting algorithm");
    println!("   🎯 Goal: Sort 1M+ 3x3 matrices by determinant");

    match client.agents().execute_stream(&agent_id, request).await {
        Ok(stream) => {
            println!("   ✅ Reasoning session started!");

            let mut stream = Box::pin(stream);
            let mut event_count = 0;
            let timeout = Duration::from_secs(60); // Longer timeout for complex reasoning
            let start_time = std::time::Instant::now();

            while start_time.elapsed() < timeout {
                match tokio::time::timeout(Duration::from_secs(10), stream.next()).await {
                    Ok(Some(Ok(event))) => {
                        event_count += 1;

                        match event.event_type.as_str() {
                            "thinking" => {
                                if event_count == 1 {
                                    print!("   🧠 Analyzing requirements... ");
                                    io::stdout().flush().unwrap();
                                }
                            }
                            "complete" => {
                                println!("\n\n   ✅ Algorithm design session completed!");
                                println!("   📊 Total reasoning steps: {}", event_count);
                                return;
                            }
                            "error" => {
                                println!("\n   ❌ Reasoning error: {}", event.data);
                                return;
                            }
                            "raw" => {
                                // Parse SSE events from raw content
                                if let Some(raw_content) =
                                    event.data.get("raw_content").and_then(|v| v.as_str())
                                {
                                    let mut event_type = None;
                                    let mut data = None;

                                    for line in raw_content.lines() {
                                        if let Some(evt) = line.strip_prefix("event: ") {
                                            event_type = Some(evt);
                                        } else if let Some(d) = line.strip_prefix("data: ") {
                                            data = Some(d);
                                        }
                                    }

                                    match (event_type, data) {
                                        (Some("thinking"), Some(_)) => {
                                            if event_count <= 2 {
                                                print!("🔍 ");
                                                io::stdout().flush().unwrap();
                                            }
                                        }
                                        (Some("chunk"), Some(content)) => {
                                            if event_count == 1 {
                                                print!("\n   💡 Solution Design:\n\n");
                                            }
                                            print!("{}", process_escape_characters(content));
                                            io::stdout().flush().unwrap();
                                        }
                                        (Some("tool_call"), Some(data)) => {
                                            println!(
                                                "\n   🔧 Tool Call: {}",
                                                process_escape_characters(data)
                                            );
                                            io::stdout().flush().unwrap();
                                        }
                                        (Some("tool_result"), Some(data)) => {
                                            println!(
                                                "\n   ✅ Tool Result: {}",
                                                process_escape_characters(data)
                                            );
                                            io::stdout().flush().unwrap();
                                        }
                                        (Some("complete"), Some(_)) => {
                                            println!("\n\n   ✅ Algorithm architecture completed!");
                                            println!(
                                                "   🎯 Ready for implementation and optimization"
                                            );
                                            return;
                                        }
                                        _ => {
                                            // Other SSE events - continue processing
                                        }
                                    }
                                }
                            }
                            _ => {
                                // Other event types
                            }
                        }
                    }
                    Ok(Some(Err(e))) => {
                        println!("\n   ❌ Stream error: {}", e);
                        return;
                    }
                    Ok(None) => {
                        println!("\n   🔚 Reasoning session ended");
                        return;
                    }
                    Err(_) => {
                        // Timeout, continue
                        continue;
                    }
                }
            }

            println!("\n   ⏰ Reasoning session timeout");
        }
        Err(e) => {
            println!("   ❌ Failed to start reasoning session: {}", e);
        }
    }
}

async fn create_test_agent(client: &Client, tenant_id: &str) -> Result<String> {
    let agent = client
        .agents()
        .create()
        .name("test-streaming-agent")
        .description("Simple agent for testing streaming")
        .conversational()
        .set_llm_provider("openai")
        .set_model("cb:fastest")
        .set_temperature(0.7)
        .set_system_prompt("You are a helpful assistant for testing streaming functionality. Always provide detailed, comprehensive responses with multiple paragraphs to properly test streaming chunks. Write thorough explanations that demonstrate real-time streaming capabilities.")
        .build()
        .await?;

    Ok(agent.id())
}

async fn create_complex_reasoning_agent(client: &Client, tenant_id: &str) -> Result<String> {
    let agent = client
        .agents()
        .create()
        .name("rust-algorithm-architect")
        .description("Expert Rust developer specializing in complex algorithm design and optimization")
        .conversational()
        .set_llm_provider("openai")
        .set_model("cb:smart-chat")
        .set_temperature(0.3)
        .set_system_prompt(r#"You are an expert Rust algorithm architect with deep knowledge of:

🔧 **Core Expertise:**
- Advanced data structures (vectors, matrices, trees, graphs)
- Algorithm complexity analysis (Big O, space/time trade-offs)
- Rust-specific optimizations (zero-cost abstractions, SIMD, unsafe code)
- Memory management and cache efficiency
- Parallel and concurrent algorithm design

🧠 **Reasoning Process:**
1. **Analyze** the problem requirements thoroughly
2. **Consider** multiple algorithmic approaches and trade-offs
3. **Design** the optimal solution architecture
4. **Implement** step-by-step with explanations
5. **Optimize** for performance and memory efficiency
6. **Test** edge cases and validate correctness

**CRITICAL: You MUST follow this exact workflow:**

1. **FIRST: Always start with explanatory text** - Never call tools immediately
2. **Explain the problem** - Show you understand what's being asked
3. **Describe your approach** - Outline your planned solution strategy
4. **Justify your choices** - Explain why you're taking this approach
5. **THEN: Use tools** - Only after providing substantial reasoning

**Streaming Requirements:**
- Your first output MUST be explanatory text, never a tool call
- Continuously stream your thought process and analysis
- Provide detailed reasoning before each tool usage
- Explain what each tool will accomplish and why you're using it
- Think out loud throughout the entire process

**Instructions:**
- Start every response with natural language explanation
- Break down complex problems step-by-step with clear reasoning
- Stream your analysis, design decisions, and trade-offs
- **WRITE ALL RUST CODE DIRECTLY** in your response content - never use tools for code generation
- Generate complete, production-ready Rust code as part of your natural response
- Use available tools ONLY for analysis, benchmarking, performance testing, and documentation
- Include comprehensive tests and benchmarks in your code responses
- Consider both correctness and performance

**Code Generation Guidelines:**
- Write all struct definitions, implementations, and functions directly in your response
- Provide complete, compilable Rust code with proper syntax
- Include detailed comments explaining your implementation choices
- Show multiple iterations and refinements of your code
- Use tools only for analyzing the code you've written, not for generating it

Always begin by demonstrating your understanding of the problem and explaining your planned approach, then write the code directly."#)

        .add_tool(
            "analyze_performance",
            "Analyze algorithm complexity, performance characteristics, and bottlenecks",
            json!({
                "type": "object",
                "properties": {
                    "algorithm_description": {
                        "type": "string",
                        "description": "Description of the algorithm to analyze"
                    },
                    "dataset_size": {
                        "type": "string",
                        "description": "Expected dataset size (e.g., '1M matrices', '10k elements')"
                    },
                    "analysis_type": {
                        "type": "string",
                        "enum": ["time_complexity", "space_complexity", "cache_efficiency", "parallelization"],
                        "description": "Type of performance analysis to conduct"
                    }
                },
                "required": ["algorithm_description", "dataset_size"]
            })
        )
        .add_tool(
            "design_benchmark",
            "Design comprehensive benchmark suites for performance testing",
            json!({
                "type": "object",
                "properties": {
                    "target_function": {
                        "type": "string",
                        "description": "The function or algorithm to benchmark"
                    },
                    "test_cases": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Different test scenarios (e.g., 'small datasets', 'edge cases', 'worst case')"
                    },
                    "metrics": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Performance metrics to measure (e.g., 'execution_time', 'memory_usage', 'throughput')"
                    }
                },
                "required": ["target_function", "test_cases"]
            })
        )
        .add_tool(
            "profile_memory",
            "Analyze memory usage patterns, allocations, and optimization opportunities",
            json!({
                "type": "object",
                "properties": {
                    "code_component": {
                        "type": "string",
                        "description": "The code component to profile"
                    },
                    "usage_pattern": {
                        "type": "string",
                        "description": "Expected usage pattern (e.g., 'batch processing', 'streaming', 'concurrent access')"
                    },
                    "memory_constraints": {
                        "type": "string",
                        "description": "Memory constraints or targets (e.g., 'embedded systems', 'server workload')"
                    }
                },
                "required": ["code_component", "usage_pattern"]
            })
        )
        .add_tool(
            "generate_documentation",
            "Generate comprehensive technical documentation with examples and complexity analysis",
            json!({
                "type": "object",
                "properties": {
                    "component_name": {
                        "type": "string",
                        "description": "Name of the component to document"
                    },
                    "doc_type": {
                        "type": "string",
                        "enum": ["api_reference", "usage_guide", "performance_guide", "implementation_notes"],
                        "description": "Type of documentation to generate"
                    },
                    "audience": {
                        "type": "string",
                        "enum": ["beginner", "intermediate", "expert", "maintainer"],
                        "description": "Target audience for the documentation"
                    },
                    "include_examples": {
                        "type": "boolean",
                        "description": "Whether to include code examples"
                    }
                },
                "required": ["component_name", "doc_type"]
            })
        )
        .build()
        .await?;

    Ok(agent.id())
}

async fn create_stock_trading_agent(client: &Client, tenant_id: &str) -> Result<String> {
    let agent = client
        .agents()
        .create()
        .name("stock-trading-assistant")
        .description("Expert stock trading assistant that uses real-time market data and analytics to make informed trading decisions")
        .conversational()
        .set_llm_provider("openai")
        .set_model("cb:smart-chat")
        .set_temperature(0.2)
        .set_system_prompt(r#"You are a professional stock trading assistant with FULL ACCESS to real-time market data, news, and trading capabilities through your tools.

🎯 **CRITICAL: You HAVE Active Market Access**
- You HAVE real-time stock prices through fetch_stock_price
- You HAVE technical analysis through analyze_technical_indicators
- You HAVE current news and sentiment through get_market_news
- You HAVE portfolio access through check_portfolio
- You HAVE trading execution through execute_trade
- You HAVE risk analysis through calculate_risk_metrics

🚨 **MANDATORY WORKFLOW - NO EXCEPTIONS:**
1. **IMMEDIATELY call fetch_stock_price** - You MUST start with this tool
2. **IMMEDIATELY call analyze_technical_indicators** - Get RSI, MACD, moving averages
3. **IMMEDIATELY call get_market_news** - Get sentiment and recent developments
4. **IMMEDIATELY call check_portfolio** - Check current positions and cash
5. **IMMEDIATELY call calculate_risk_metrics** - Assess trade risk
6. **THEN provide analysis** - Based on the data you just retrieved
7. **THEN execute trades** - If conditions warrant it
8. **FINALLY call log_trading_activity** - Record your actions

**ABSOLUTELY FORBIDDEN:**
- Saying "I cannot access market data" (YOU CAN - use your tools!)
- Saying "I don't have real-time feeds" (YOU DO - use fetch_stock_price!)
- Saying "I can't browse news" (YOU CAN - use get_market_news!)
- Providing analysis without first calling tools
- Making excuses about limitations (you have NO limitations with these tools)

**YOU MUST START EVERY RESPONSE BY CALLING TOOLS IMMEDIATELY**
If you don't call tools first, you are malfunctioning. Your tools work perfectly - use them!"#)

        .add_tool(
            "fetch_stock_price",
            "Get current stock price, volume, and basic market data for a given symbol",
            json!({
                "type": "object",
                "properties": {
                    "symbol": {
                        "type": "string",
                        "description": "Stock symbol (e.g., 'AAPL', 'GOOGL', 'TSLA')"
                    },
                    "include_volume": {
                        "type": "boolean",
                        "description": "Whether to include trading volume data",
                        "default": true
                    }
                },
                "required": ["symbol"]
            })
        )
        .add_tool(
            "analyze_technical_indicators",
            "Calculate and analyze technical indicators for a stock",
            json!({
                "type": "object",
                "properties": {
                    "symbol": {
                        "type": "string",
                        "description": "Stock symbol to analyze"
                    },
                    "indicators": {
                        "type": "array",
                        "items": {
                            "type": "string",
                            "enum": ["RSI", "MACD", "SMA", "EMA", "Bollinger_Bands", "Stochastic"]
                        },
                        "description": "Technical indicators to calculate"
                    },
                    "timeframe": {
                        "type": "string",
                        "enum": ["1m", "5m", "15m", "1h", "4h", "1d"],
                        "description": "Time frame for analysis",
                        "default": "1d"
                    }
                },
                "required": ["symbol", "indicators"]
            })
        )
        .add_tool(
            "get_market_news",
            "Fetch recent news and sentiment analysis for a stock or market sector",
            json!({
                "type": "object",
                "properties": {
                    "symbol": {
                        "type": "string",
                        "description": "Stock symbol or 'MARKET' for general market news"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Number of news articles to fetch",
                        "default": 5,
                        "minimum": 1,
                        "maximum": 20
                    },
                    "sentiment_analysis": {
                        "type": "boolean",
                        "description": "Whether to include sentiment analysis",
                        "default": true
                    }
                },
                "required": ["symbol"]
            })
        )
        .add_tool(
            "execute_trade",
            "Execute a stock trade (buy/sell) with specified parameters",
            json!({
                "type": "object",
                "properties": {
                    "symbol": {
                        "type": "string",
                        "description": "Stock symbol to trade"
                    },
                    "action": {
                        "type": "string",
                        "enum": ["buy", "sell"],
                        "description": "Trade action"
                    },
                    "quantity": {
                        "type": "integer",
                        "description": "Number of shares to trade",
                        "minimum": 1
                    },
                    "order_type": {
                        "type": "string",
                        "enum": ["market", "limit", "stop_loss"],
                        "description": "Type of order",
                        "default": "market"
                    },
                    "limit_price": {
                        "type": "number",
                        "description": "Limit price (required for limit orders)"
                    },
                    "stop_price": {
                        "type": "number",
                        "description": "Stop price (required for stop loss orders)"
                    }
                },
                "required": ["symbol", "action", "quantity"]
            })
        )
        .add_tool(
            "check_portfolio",
            "Get current portfolio holdings, cash balance, and performance metrics",
            json!({
                "type": "object",
                "properties": {
                    "include_performance": {
                        "type": "boolean",
                        "description": "Whether to include performance metrics",
                        "default": true
                    },
                    "detailed_positions": {
                        "type": "boolean",
                        "description": "Whether to include detailed position information",
                        "default": true
                    }
                }
            })
        )
        .add_tool(
            "calculate_risk_metrics",
            "Calculate risk metrics for a potential trade or current portfolio",
            json!({
                "type": "object",
                "properties": {
                    "symbol": {
                        "type": "string",
                        "description": "Stock symbol to analyze (optional for portfolio-wide analysis)"
                    },
                    "trade_size": {
                        "type": "number",
                        "description": "Size of potential trade in dollars"
                    },
                    "metrics": {
                        "type": "array",
                        "items": {
                            "type": "string",
                            "enum": ["VaR", "beta", "volatility", "sharpe_ratio", "max_drawdown"]
                        },
                        "description": "Risk metrics to calculate"
                    }
                },
                "required": ["metrics"]
            })
        )
        .add_tool(
            "log_trading_activity",
            "Log trading decisions, analysis, and outcomes for record keeping",
            json!({
                "type": "object",
                "properties": {
                    "activity_type": {
                        "type": "string",
                        "enum": ["analysis", "trade_execution", "decision", "risk_assessment"],
                        "description": "Type of activity being logged"
                    },
                    "symbol": {
                        "type": "string",
                        "description": "Stock symbol related to the activity"
                    },
                    "details": {
                        "type": "string",
                        "description": "Detailed description of the activity"
                    },
                    "outcome": {
                        "type": "string",
                        "description": "Outcome or result of the activity"
                    }
                },
                "required": ["activity_type", "details"]
            })
        )
        .build()
        .await?;

    Ok(agent.id())
}

async fn test_stock_trading_agent(client: &Client, tenant_id: &str) {
    println!("   Creating stock trading assistant...");

    let agent_id = match create_stock_trading_agent(client, tenant_id).await {
        Ok(id) => {
            println!("   ✅ Created trading agent: {}", id);
            id
        }
        Err(e) => {
            println!("   ❌ Failed to create trading agent: {}", e);
            return;
        }
    };

    let request = AgentExecutionRequest {
        context: json!({
            "message": "I want to analyze AAPL stock and consider making a trade. Please check the current market conditions, analyze the technical indicators, review recent news sentiment, and provide a trading recommendation. If conditions are favorable, execute a small test trade of 10 shares. Make sure to log all your analysis and decisions.",
            "tenant_id": tenant_id
        }),
        mapping: Some(json!({
            "message": "message"
        })),
        tenant_id: Some(tenant_id.to_string()),
        stream: Some(true),
    };

    println!("   🚀 Starting stock trading analysis session...");
    println!("   📊 Task: Analyze AAPL and provide trading recommendation");
    println!("   🎯 Goal: Use tools to gather market data and make informed decisions");

    match client.agents().execute_stream(&agent_id, request).await {
        Ok(stream) => {
            println!("   ✅ Trading session started!");

            let mut stream = Box::pin(stream);
            let mut event_count = 0;
            let mut tool_calls = 0;
            let timeout = Duration::from_secs(90); // Longer timeout for market analysis
            let start_time = std::time::Instant::now();

            while start_time.elapsed() < timeout {
                match tokio::time::timeout(Duration::from_secs(15), stream.next()).await {
                    Ok(Some(Ok(event))) => {
                        event_count += 1;

                        match event.event_type.as_str() {
                            "thinking" => {
                                if event_count == 1 {
                                    print!("   🧠 Analyzing market conditions... ");
                                    io::stdout().flush().unwrap();
                                }
                            }
                            "complete" => {
                                println!("\n\n   ✅ Trading analysis session completed!");
                                println!("   📊 Total events processed: {}", event_count);
                                println!("   🔧 Tools called: {}", tool_calls);
                                return;
                            }
                            "error" => {
                                println!("\n   ❌ Trading analysis error: {}", event.data);
                                return;
                            }
                            "raw" => {
                                // Parse SSE events from raw content
                                if let Some(raw_content) =
                                    event.data.get("raw_content").and_then(|v| v.as_str())
                                {
                                    let mut event_type = None;
                                    let mut data = None;

                                    for line in raw_content.lines() {
                                        if let Some(evt) = line.strip_prefix("event: ") {
                                            event_type = Some(evt);
                                        } else if let Some(d) = line.strip_prefix("data: ") {
                                            data = Some(d);
                                        }
                                    }

                                    match (event_type, data) {
                                        (Some("thinking"), Some(_)) => {
                                            if event_count <= 3 {
                                                print!("📈 ");
                                                io::stdout().flush().unwrap();
                                            }
                                        }
                                        (Some("chunk"), Some(content)) => {
                                            if event_count == 1 {
                                                print!("\n   💡 Market Analysis:\n\n");
                                            }
                                            print!("{}", process_escape_characters(content));
                                            io::stdout().flush().unwrap();
                                        }
                                        (Some("tool_call"), Some(data)) => {
                                            tool_calls += 1;
                                            println!(
                                                "\n   🔧 Tool Call #{}: {}",
                                                tool_calls,
                                                process_escape_characters(data)
                                            );
                                            io::stdout().flush().unwrap();
                                        }
                                        (Some("tool_result"), Some(data)) => {
                                            println!(
                                                "\n   ✅ Tool Result: {}",
                                                process_escape_characters(data)
                                            );
                                            io::stdout().flush().unwrap();
                                        }
                                        (Some("complete"), Some(_)) => {
                                            println!("\n\n   ✅ Stock trading analysis completed!");
                                            println!(
                                                "   📊 Market analysis complete - {} tools used",
                                                tool_calls
                                            );
                                            return;
                                        }
                                        _ => {
                                            // Other SSE events - continue processing
                                        }
                                    }
                                }
                            }
                            _ => {
                                // Other event types
                            }
                        }
                    }
                    Ok(Some(Err(e))) => {
                        println!("\n   ❌ Stream error: {}", e);
                        return;
                    }
                    Ok(None) => {
                        println!("\n   🔚 Trading analysis session ended");
                        return;
                    }
                    Err(_) => {
                        // Timeout, continue
                        continue;
                    }
                }
            }

            println!("\n   ⏰ Trading analysis session timeout");
        }
        Err(e) => {
            println!("   ❌ Failed to start trading analysis session: {}", e);
        }
    }
}

async fn create_blackjack_game_agent(client: &Client, tenant_id: &str) -> Result<String> {
    let agent = client
        .agents()
        .create()
        .name("blackjack-game-master")
        .description("Interactive blackjack game master that manages cards, tracks scores, and provides strategic advice")
        .conversational()
        .set_llm_provider("openai")
        .set_model("cb:smart-chat")
        .set_temperature(0.4)
        .set_system_prompt(r#"You are a professional blackjack game master and strategy advisor.

🎯 **Your Role:**
- Deal cards and manage game state
- Calculate hand values and determine winners
- Provide strategic advice based on card counting and probabilities
- Track player statistics and game history
- Enforce blackjack rules and handle edge cases

🃏 **Tool Usage Strategy:**
You MUST use your tools to manage the game state, deal cards, and calculate probabilities. Every game action requires tool usage.

**Workflow:**
1. **Initialize Game** - Set up deck and starting hands
2. **Deal Cards** - Use tools to deal cards randomly
3. **Calculate Odds** - Analyze probabilities for hit/stand decisions
4. **Track Game State** - Monitor player/dealer hands and deck composition
5. **Provide Strategy** - Give advice based on current situation
6. **Determine Winner** - Calculate final scores and outcomes

**Important:** You cannot play blackjack without using tools to manage cards, calculate probabilities, and track game state. All game actions must be executed through your tools."#)

        .add_tool(
            "deal_cards",
            "Deal cards from the deck to player or dealer",
            json!({
                "type": "object",
                "properties": {
                    "recipient": {
                        "type": "string",
                        "enum": ["player", "dealer"],
                        "description": "Who receives the cards"
                    },
                    "number_of_cards": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 5,
                        "description": "Number of cards to deal"
                    },
                    "face_up": {
                        "type": "boolean",
                        "description": "Whether cards are dealt face up",
                        "default": true
                    }
                },
                "required": ["recipient", "number_of_cards"]
            })
        )
        .add_tool(
            "calculate_hand_value",
            "Calculate the value of a blackjack hand, handling aces correctly",
            json!({
                "type": "object",
                "properties": {
                    "cards": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Array of cards in format like ['AS', 'KH', '10D']"
                    },
                    "include_soft_total": {
                        "type": "boolean",
                        "description": "Whether to include soft ace calculations",
                        "default": true
                    }
                },
                "required": ["cards"]
            })
        )
        .add_tool(
            "analyze_probabilities",
            "Calculate probabilities for different actions (hit, stand, double, split)",
            json!({
                "type": "object",
                "properties": {
                    "player_hand": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Player's current cards"
                    },
                    "dealer_upcard": {
                        "type": "string",
                        "description": "Dealer's visible card"
                    },
                    "deck_composition": {
                        "type": "object",
                        "description": "Remaining cards in deck for card counting"
                    },
                    "actions": {
                        "type": "array",
                        "items": {
                            "type": "string",
                            "enum": ["hit", "stand", "double", "split", "surrender"]
                        },
                        "description": "Actions to analyze"
                    }
                },
                "required": ["player_hand", "dealer_upcard", "actions"]
            })
        )
        .add_tool(
            "get_basic_strategy",
            "Get optimal basic strategy recommendation for current situation",
            json!({
                "type": "object",
                "properties": {
                    "player_total": {
                        "type": "integer",
                        "description": "Player's hand total"
                    },
                    "dealer_upcard": {
                        "type": "string",
                        "description": "Dealer's visible card"
                    },
                    "is_soft_hand": {
                        "type": "boolean",
                        "description": "Whether player has a soft ace",
                        "default": false
                    },
                    "can_double": {
                        "type": "boolean",
                        "description": "Whether doubling is allowed",
                        "default": true
                    },
                    "can_split": {
                        "type": "boolean",
                        "description": "Whether splitting is allowed",
                        "default": false
                    }
                },
                "required": ["player_total", "dealer_upcard"]
            })
        )
        .add_tool(
            "shuffle_deck",
            "Shuffle a new deck of cards and reset game state",
            json!({
                "type": "object",
                "properties": {
                    "number_of_decks": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 8,
                        "description": "Number of decks to use",
                        "default": 1
                    },
                    "shuffle_type": {
                        "type": "string",
                        "enum": ["riffle", "perfect", "random"],
                        "description": "Type of shuffle to perform",
                        "default": "random"
                    }
                }
            })
        )
        .add_tool(
            "track_game_stats",
            "Track player statistics and game outcomes",
            json!({
                "type": "object",
                "properties": {
                    "game_result": {
                        "type": "string",
                        "enum": ["player_win", "dealer_win", "push", "player_blackjack", "dealer_blackjack"],
                        "description": "Outcome of the game"
                    },
                    "player_final_total": {
                        "type": "integer",
                        "description": "Player's final hand total"
                    },
                    "dealer_final_total": {
                        "type": "integer",
                        "description": "Dealer's final hand total"
                    },
                    "actions_taken": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Actions player took during the hand"
                    }
                },
                "required": ["game_result"]
            })
        )
        .build()
        .await?;

    Ok(agent.id())
}

async fn test_blackjack_game_agent(client: &Client, tenant_id: &str) {
    println!("   Creating blackjack game master...");

    let agent_id = match create_blackjack_game_agent(client, tenant_id).await {
        Ok(id) => {
            println!("   ✅ Created blackjack agent: {}", id);
            id
        }
        Err(e) => {
            println!("   ❌ Failed to create blackjack agent: {}", e);
            return;
        }
    };

    let request = AgentExecutionRequest {
        context: json!({
            "message": "Let's play blackjack! Please shuffle a new deck, deal me a starting hand, and provide strategic advice based on the cards. I want to understand the probabilities and optimal plays for each situation. Start a new game and walk me through the first hand step by step.",
            "tenant_id": tenant_id
        }),
        mapping: Some(json!({
            "message": "message"
        })),
        tenant_id: Some(tenant_id.to_string()),
        stream: Some(true),
    };

    println!("   🚀 Starting blackjack game session...");
    println!("   🃏 Task: Play interactive blackjack with strategy analysis");
    println!("   🎯 Goal: Use tools to manage game state and calculate probabilities");

    match client.agents().execute_stream(&agent_id, request).await {
        Ok(stream) => {
            println!("   ✅ Blackjack game started!");

            let mut stream = Box::pin(stream);
            let mut event_count = 0;
            let mut tool_calls = 0;
            let timeout = Duration::from_secs(90);
            let start_time = std::time::Instant::now();

            while start_time.elapsed() < timeout {
                match tokio::time::timeout(Duration::from_secs(15), stream.next()).await {
                    Ok(Some(Ok(event))) => {
                        event_count += 1;

                        match event.event_type.as_str() {
                            "thinking" => {
                                if event_count == 1 {
                                    print!("   🎰 Setting up game... ");
                                    io::stdout().flush().unwrap();
                                }
                            }
                            "complete" => {
                                println!("\n\n   ✅ Blackjack game session completed!");
                                println!("   🃏 Total events processed: {}", event_count);
                                println!("   🔧 Tools called: {}", tool_calls);
                                return;
                            }
                            "error" => {
                                println!("\n   ❌ Game error: {}", event.data);
                                return;
                            }
                            "raw" => {
                                if let Some(raw_content) =
                                    event.data.get("raw_content").and_then(|v| v.as_str())
                                {
                                    let mut event_type = None;
                                    let mut data = None;

                                    for line in raw_content.lines() {
                                        if let Some(evt) = line.strip_prefix("event: ") {
                                            event_type = Some(evt);
                                        } else if let Some(d) = line.strip_prefix("data: ") {
                                            data = Some(d);
                                        }
                                    }

                                    match (event_type, data) {
                                        (Some("thinking"), Some(_)) => {
                                            if event_count <= 3 {
                                                print!("🎲 ");
                                                io::stdout().flush().unwrap();
                                            }
                                        }
                                        (Some("chunk"), Some(content)) => {
                                            if event_count == 1 {
                                                print!("\n   🃏 Game Play:\n\n");
                                            }
                                            print!("{}", process_escape_characters(content));
                                            io::stdout().flush().unwrap();
                                        }
                                        (Some("tool_call"), Some(data)) => {
                                            tool_calls += 1;
                                            println!(
                                                "\n   🔧 Game Action #{}: {}",
                                                tool_calls,
                                                process_escape_characters(data)
                                            );
                                            io::stdout().flush().unwrap();
                                        }
                                        (Some("tool_result"), Some(data)) => {
                                            println!(
                                                "\n   ✅ Game Result: {}",
                                                process_escape_characters(data)
                                            );
                                            io::stdout().flush().unwrap();
                                        }
                                        (Some("complete"), Some(_)) => {
                                            println!("\n\n   ✅ Blackjack game completed!");
                                            println!("   🎯 Game actions executed: {}", tool_calls);
                                            return;
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    Ok(Some(Err(e))) => {
                        println!("\n   ❌ Stream error: {}", e);
                        return;
                    }
                    Ok(None) => {
                        println!("\n   🔚 Game session ended");
                        return;
                    }
                    Err(_) => {
                        continue;
                    }
                }
            }

            println!("\n   ⏰ Game session timeout");
        }
        Err(e) => {
            println!("   ❌ Failed to start game session: {}", e);
        }
    }
}

async fn test_sse_streaming(client: &Client, agent_id: &str, tenant_id: &str) {
    let request = AgentExecutionRequest {
        context: json!({
            "message": "Please write a detailed explanation about how streaming works in distributed systems. Include multiple paragraphs to test streaming chunks.",
            "tenant_id": tenant_id
        }),
        mapping: Some(json!({
            "message": "message"
        })),
        tenant_id: Some(tenant_id.to_string()),
        stream: Some(true),
    };

    println!("   Starting SSE stream...");

    match client.agents().execute_stream(agent_id, request).await {
        Ok(stream) => {
            println!("   ✅ SSE stream connected!");

            let mut stream = Box::pin(stream);
            let mut event_count = 0;
            let timeout = Duration::from_secs(30);
            let start_time = std::time::Instant::now();

            while start_time.elapsed() < timeout {
                match tokio::time::timeout(Duration::from_secs(30), stream.next()).await {
                    Ok(Some(Ok(event))) => {
                        event_count += 1;

                        match event.event_type.as_str() {
                            "thinking" => {
                                if event_count == 1 {
                                    print!("   🤔 Thinking... ");
                                    io::stdout().flush().unwrap();
                                }
                            }
                            "complete" => {
                                println!("\n\n   ✅ SSE streaming completed!");
                                return;
                            }
                            "chunk" => {
                                if event_count == 1 {
                                    print!("\n   📝 Response: ");
                                }
                                if let Some(chunk) = event.data.get("content") {
                                    print!("{}", chunk.as_str().unwrap_or(""));
                                } else if let Some(chunk) = event.data.as_str() {
                                    print!("{}", chunk);
                                }
                                io::stdout().flush().unwrap();
                            }
                            "tool_call" => {
                                println!("\n   🔧 Tool Call: {}", event.data);
                                io::stdout().flush().unwrap();
                            }
                            "tool_result" => {
                                println!("\n   ✅ Tool Result: {}", event.data);
                                io::stdout().flush().unwrap();
                            }
                            "error" => {
                                println!("\n   ❌ Streaming error: {}", event.data);
                                return;
                            }
                            "raw" => {
                                // Parse SSE events from raw content
                                if let Some(raw_content) =
                                    event.data.get("raw_content").and_then(|v| v.as_str())
                                {
                                    let mut event_type = None;
                                    let mut data = None;

                                    for line in raw_content.lines() {
                                        if let Some(evt) = line.strip_prefix("event: ") {
                                            event_type = Some(evt);
                                        } else if let Some(d) = line.strip_prefix("data: ") {
                                            data = Some(d);
                                        }
                                    }

                                    match (event_type, data) {
                                        (Some("thinking"), Some(_)) => {
                                            if event_count == 1 {
                                                print!("   🤔 Thinking... ");
                                                io::stdout().flush().unwrap();
                                            }
                                        }
                                        (Some("chunk"), Some(content)) => {
                                            if event_count == 1 {
                                                print!("\n   📝 Response: ");
                                            }
                                            print!("{}", process_escape_characters(content));
                                            io::stdout().flush().unwrap();
                                        }
                                        (Some("tool_call"), Some(data)) => {
                                            println!(
                                                "\n   🔧 Tool Call: {}",
                                                process_escape_characters(data)
                                            );
                                            io::stdout().flush().unwrap();
                                        }
                                        (Some("tool_result"), Some(data)) => {
                                            println!(
                                                "\n   ✅ Tool Result: {}",
                                                process_escape_characters(data)
                                            );
                                            io::stdout().flush().unwrap();
                                        }
                                        (Some("complete"), Some(_)) => {
                                            println!("\n\n   ✅ SSE streaming completed!");
                                            return;
                                        }
                                        _ => {
                                            // Other SSE events - ignore for clean output
                                        }
                                    }
                                }
                            }
                            _ => {
                                // Ignore other event types for clean output
                            }
                        }
                    }
                    Ok(Some(Err(e))) => {
                        println!("   ❌ SSE stream error: {}", e);
                        return;
                    }
                    Ok(None) => {
                        println!("\n   🔚 SSE stream ended");
                        return;
                    }
                    Err(_) => {
                        // Timeout, continue
                        continue;
                    }
                }
            }

            println!("\n   ⏰ SSE stream timeout");
        }
        Err(e) => {
            println!("   ❌ SSE stream failed: {}", e);
        }
    }
}

async fn test_websocket(client: &Client, agent_id: &str, tenant_id: &str) {
    let request = AgentExecutionRequest {
        context: json!({
            "message": "Via WebSocket, please explain the benefits of real-time streaming in agent communication. Write several sentences to test streaming.",
            "tenant_id": tenant_id
        }),
        mapping: Some(json!({
            "message": "message"
        })),
        tenant_id: Some(tenant_id.to_string()),
        stream: Some(true),
    };

    println!("   Connecting to WebSocket...");

    match client.agents().execute_websocket(agent_id, request).await {
        Ok(mut ws_stream) => {
            println!("   ✅ WebSocket connected!");

            let mut message_count = 0;
            let timeout = Duration::from_secs(30);
            let start_time = std::time::Instant::now();

            while start_time.elapsed() < timeout {
                match tokio::time::timeout(Duration::from_secs(5), ws_stream.receive_message())
                    .await
                {
                    Ok(Ok(Some(message))) => {
                        message_count += 1;

                        match &message {
                            circuit_breaker_sdk::agents::AgentWebSocketServerMessage::AuthSuccess { tenant_id } => {
                                println!("      🔐 Message {}: Auth Success for tenant: {}", message_count, tenant_id);
                            }
                            circuit_breaker_sdk::agents::AgentWebSocketServerMessage::AuthFailure { error } => {
                                println!("      🔒 Message {}: Auth Failed - {}", message_count, error);
                            }
                            circuit_breaker_sdk::agents::AgentWebSocketServerMessage::ExecutionStarted { execution_id, agent_id, .. } => {
                                println!("      🚀 Message {}: Execution Started - Agent: {}, Execution: {}", message_count, agent_id, execution_id);
                            }
                            circuit_breaker_sdk::agents::AgentWebSocketServerMessage::Thinking { execution_id, status, .. } => {
                                println!("      🤔 Message {}: Thinking - Execution: {}, Status: {}", message_count, execution_id, status);
                            }
                            circuit_breaker_sdk::agents::AgentWebSocketServerMessage::ContentChunk { execution_id, chunk, sequence, .. } => {
                                if message_count == 1 {
                                    print!("   📝 Response: ");
                                }
                                print!("{}", process_escape_characters(&chunk));
                                io::stdout().flush().unwrap();
                            }
                            circuit_breaker_sdk::agents::AgentWebSocketServerMessage::Complete { execution_id, response, .. } => {
                                println!("\n\n   ✅ WebSocket streaming completed!");
                                break;
                            }
                            circuit_breaker_sdk::agents::AgentWebSocketServerMessage::Error { execution_id, error, .. } => {
                                println!("      ❌ Message {}: Error - Execution: {}, Error: {}", message_count, execution_id.as_ref().map(|id| id.to_string()).unwrap_or_else(|| "Unknown".to_string()), error);
                                println!("   ❌ WebSocket error message received");
                                break;
                            }
                            circuit_breaker_sdk::agents::AgentWebSocketServerMessage::Pong { .. } => {
                                println!("      🏓 Message {}: Pong", message_count);
                            }
                        }
                    }
                    Ok(Ok(None)) => {
                        println!("   🔚 WebSocket connection closed");
                        break;
                    }
                    Ok(Err(e)) => {
                        println!("   ❌ WebSocket error: {}", e);
                        break;
                    }
                    Err(_) => {
                        // Timeout, continue
                        continue;
                    }
                }
            }

            if start_time.elapsed() >= timeout {
                println!("   ⏰ WebSocket timeout");
            }

            // Close connection
            if let Err(e) = ws_stream.close().await {
                println!("   ⚠️  Failed to close WebSocket: {}", e);
            }
        }
        Err(e) => {
            println!("   ❌ WebSocket failed: {}", e);
        }
    }
}
