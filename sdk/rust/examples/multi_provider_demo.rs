//! Multi-Provider LLM Demo - Rust Edition
//!
//! This demo showcases the Circuit Breaker LLM Router's multi-provider capabilities,
//! demonstrating OpenAI, Anthropic, and Google Gemini integration with cost tracking,
//! streaming, and smart routing features.
//!
//! Architecture Note:
//! The Circuit Breaker server normalizes all provider responses to OpenAI-compatible
//! format, enabling a single client interface regardless of the underlying provider.
//! This works seamlessly with virtual models that can route to any provider.
//!
//! Prerequisites:
//! 1. Circuit Breaker server running on ports 8081 (OpenAI API) and 4000 (GraphQL)
//! 2. API keys configured in server's .env file
//! 3. Run with: cargo run --example multi_provider_demo

use circuit_breaker_sdk::{
    create_chat,
    types::{ChatMessage, ChatRole, LLMRequest, LLMResponse},
    Client, Result, COMMON_MODELS,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::io::{self, Write};
use std::time::Instant;

#[derive(Debug, Deserialize)]
struct LLMProvider {
    id: String,
    #[serde(rename = "providerType")]
    provider_type: String,
    name: String,
    #[serde(rename = "baseUrl")]
    base_url: String,
    models: Vec<LLMModel>,
    #[serde(rename = "healthStatus")]
    health_status: HealthStatus,
}

#[derive(Debug, Deserialize)]
struct LLMModel {
    id: String,
    name: String,
    #[serde(rename = "maxTokens")]
    max_tokens: Option<u32>,
    #[serde(rename = "contextWindow")]
    context_window: Option<u32>,
    #[serde(rename = "costPerInputToken")]
    cost_per_input_token: f64,
    #[serde(rename = "costPerOutputToken")]
    cost_per_output_token: f64,
    #[serde(rename = "supportsStreaming")]
    supports_streaming: bool,
    #[serde(rename = "supportsFunctionCalling")]
    supports_function_calling: bool,
    capabilities: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct HealthStatus {
    #[serde(rename = "isHealthy")]
    is_healthy: bool,
    #[serde(rename = "errorRate")]
    error_rate: f64,
    #[serde(rename = "averageLatencyMs")]
    average_latency_ms: f64,
    #[serde(rename = "lastCheck")]
    last_check: Option<String>,
}

#[derive(Debug)]
struct ProviderComparison {
    provider: String,
    model: String,
    response: String,
    tokens: u32,
    estimated_cost: f64,
    latency_ms: u128,
    success: bool,
}

struct MultiProviderDemo {
    client: Client,
    openai_api_url: String,
}

impl MultiProviderDemo {
    fn new(client: Client) -> Self {
        Self {
            client,
            openai_api_url: "http://localhost:8081/v1/chat/completions".to_string(),
        }
    }

    async fn run(&self) -> Result<()> {
        println!("🤖 Circuit Breaker Multi-Provider LLM Demo - Rust Edition");
        println!("===========================================================");
        println!();

        println!("🔑 Multi-Provider Architecture:");
        println!("   📊 OpenAI: GPT-4, GPT-3.5, o4 models");
        println!("   🧠 Anthropic: Claude 3 Haiku, Sonnet, Opus");
        println!("   🔍 Google: Gemini Pro, Flash, Vision models");
        println!("   🎯 Smart Routing: Auto-select optimal provider");
        println!();

        // Test server connectivity
        self.test_server_connectivity().await?;
        self.wait_for_enter("Ready to explore multi-provider capabilities?")
            .await;

        // 1. List and analyze all providers
        self.list_providers().await?;
        self.wait_for_enter("Providers analyzed! Ready to test provider-specific models?")
            .await;

        // 2. Test each provider individually
        self.test_individual_providers().await?;
        self.wait_for_enter("Individual tests complete! Ready for cost comparison?")
            .await;

        // 3. Cost comparison across providers
        self.compare_costs().await?;
        self.wait_for_enter("Cost analysis done! Ready to test smart routing?")
            .await;

        // 4. Smart routing demonstration
        self.demonstrate_smart_routing().await?;
        self.wait_for_enter("Smart routing demo complete! Ready for advanced features?")
            .await;

        // 5. Advanced features
        self.test_advanced_features().await?;

        // 6. Final summary
        self.print_summary().await;

        Ok(())
    }

    async fn test_server_connectivity(&self) -> Result<()> {
        println!("🔗 Testing server connectivity...");

        // Test Circuit Breaker connection
        match self.client.ping().await {
            Ok(response) => {
                println!("✅ Circuit Breaker server v{} connected", response.version);
            }
            Err(e) => {
                println!("❌ Circuit Breaker server not responding: {}", e);
                println!("   Please start the server: cargo run --bin server");
                return Err(e);
            }
        }

        // Test OpenAI API endpoint availability
        let http_client = reqwest::Client::new();
        match http_client
            .get("http://localhost:8081/v1/models")
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => {
                println!("✅ OpenAI API endpoint accessible");
            }
            _ => {
                println!(
                    "⚠️  OpenAI API endpoint not accessible (this is expected if not configured)"
                );
            }
        }

        Ok(())
    }

    async fn list_providers(&self) -> Result<Vec<LLMProvider>> {
        println!("\n📊 1. Provider Discovery & Analysis");
        println!("===================================");

        let query = r#"
            query {
                llmProviders {
                    id
                    providerType
                    name
                    baseUrl
                    models {
                        id
                        name
                        maxTokens
                        contextWindow
                        costPerInputToken
                        costPerOutputToken
                        supportsStreaming
                        supportsFunctionCalling
                        capabilities
                    }
                    healthStatus {
                        isHealthy
                        errorRate
                        averageLatencyMs
                        lastCheck
                    }
                }
            }
        "#;

        #[derive(Deserialize)]
        struct Response {
            #[serde(rename = "llmProviders")]
            llm_providers: Vec<LLMProvider>,
        }

        let response: Response = self.client.graphql(query, ()).await?;
        let providers = response.llm_providers;

        println!("✅ Found {} providers configured:", providers.len());
        println!();

        for provider in &providers {
            let status = if provider.health_status.is_healthy {
                "🟢 Healthy"
            } else {
                "🔴 Unhealthy"
            };

            println!(
                "🏢 {} ({})",
                provider.name,
                provider.provider_type.to_uppercase()
            );
            println!("   Status: {}", status);
            println!("   Base URL: {}", provider.base_url);
            println!("   Models: {}", provider.models.len());

            // Show top 3 models with cost info
            let top_models: Vec<_> = provider.models.iter().take(3).collect();
            for model in top_models {
                let input_cost = model.cost_per_input_token * 1000.0;
                let output_cost = model.cost_per_output_token * 1000.0;
                println!(
                    "     • {}: ${:.4}/${:.4} per 1K tokens",
                    model.name, input_cost, output_cost
                );
            }

            if provider.models.len() > 3 {
                println!("     ... and {} more models", provider.models.len() - 3);
            }
            println!();
        }

        Ok(providers)
    }

    async fn test_individual_providers(&self) -> Result<()> {
        println!("\n🧪 2. Individual Provider Testing");
        println!("=================================");

        let test_cases = vec![
            (
                "OpenAI",
                COMMON_MODELS::GPT_4O_MINI,
                "Explain what makes you unique as an AI assistant in one sentence.",
            ),
            (
                "Anthropic",
                COMMON_MODELS::CLAUDE_3_HAIKU,
                "Explain what makes you unique as an AI assistant in one sentence.",
            ),
            (
                "Google",
                COMMON_MODELS::GEMINI_FLASH,
                "Say hello and introduce yourself briefly.",
            ),
        ];

        for (provider, model, prompt) in test_cases {
            println!("\n🔧 Testing {} ({}):", provider, model);

            let start_time = Instant::now();
            match self.client.llm().chat(model, prompt).await {
                Ok(response) => {
                    let latency = start_time.elapsed().as_millis();
                    println!("   ✅ Success ({}ms)", latency);
                    println!(
                        "   💬 Response: \"{}...\"",
                        &response[..80.min(response.len())]
                    );
                }
                Err(e) => {
                    println!("   ❌ Failed: {}", e);
                }
            }
        }

        Ok(())
    }

    async fn compare_costs(&self) -> Result<()> {
        println!("\n💰 3. Cost Comparison Analysis");
        println!("==============================");

        let models = vec![
            (
                COMMON_MODELS::GPT_4O_MINI,
                "Write a haiku about artificial intelligence.",
            ),
            (
                COMMON_MODELS::CLAUDE_3_HAIKU,
                "Write a haiku about artificial intelligence.",
            ),
            (COMMON_MODELS::GEMINI_FLASH, "Write a short haiku."),
        ];

        let mut results = Vec::new();

        for (model, prompt) in models {
            println!("\n💸 Testing cost for {}:", model);

            let start_time = Instant::now();
            match self.client.llm().chat(model, prompt).await {
                Ok(response) => {
                    let latency = start_time.elapsed().as_millis();
                    let estimated_cost = self.estimate_cost(model, &response);

                    results.push(ProviderComparison {
                        provider: self.get_provider_from_model(model),
                        model: model.to_string(),
                        response: response.clone(),
                        tokens: response.len() as u32, // Simplified token count
                        estimated_cost,
                        latency_ms: latency,
                        success: true,
                    });

                    println!("   ✅ Success");
                    println!("   📝 Response: \"{}\"", response);
                    println!("   💰 Estimated cost: ${:.6}", estimated_cost);
                    println!("   ⏱️  Latency: {}ms", latency);
                }
                Err(_) => {
                    results.push(ProviderComparison {
                        provider: self.get_provider_from_model(model),
                        model: model.to_string(),
                        response: "Failed".to_string(),
                        tokens: 0,
                        estimated_cost: 0.0,
                        latency_ms: 0,
                        success: false,
                    });
                    println!("   ❌ Failed");
                }
            }
        }

        // Print cost comparison table
        println!("\n📈 Cost Comparison Summary:");
        println!("┌─────────────┬──────────────────┬─────────────┬───────────┬───────────┐");
        println!("│ Provider    │ Model            │ Cost ($)    │ Tokens    │ Latency   │");
        println!("├─────────────┼──────────────────┼─────────────┼───────────┼───────────┤");

        for result in results.iter().filter(|r| r.success) {
            println!(
                "│ {:<11} │ {:<16} │ {:>11.6} │ {:>9} │ {:>7}ms │",
                result.provider,
                &result.model[..16.min(result.model.len())],
                result.estimated_cost,
                result.tokens,
                result.latency_ms
            );
        }
        println!("└─────────────┴──────────────────┴─────────────┴───────────┴───────────┘");

        // Find most cost-effective
        if let Some(cheapest) = results
            .iter()
            .filter(|r| r.success)
            .min_by(|a, b| a.estimated_cost.partial_cmp(&b.estimated_cost).unwrap())
        {
            println!(
                "\n🏆 Most Cost-Effective: {} ({}) - ${:.6}",
                cheapest.model, cheapest.provider, cheapest.estimated_cost
            );
        }

        Ok(())
    }

    async fn demonstrate_smart_routing(&self) -> Result<()> {
        println!("\n🧠 4. Smart Routing Demonstration");
        println!("=================================");

        // Test virtual models
        println!("\n🎯 Testing Virtual Models:");
        let virtual_models = vec![
            ("Auto-Route", "auto"),
            ("Cost-Optimal", "cb:cost-optimal"),
            ("Fastest", "cb:fastest"),
        ];

        for (name, model) in virtual_models {
            println!("\n   Testing virtual model: {}", model);
            match self
                .client
                .llm()
                .chat(model, "Hello! What provider are you?")
                .await
            {
                Ok(response) => {
                    println!(
                        "   ✅ {} → Response: \"{}...\"",
                        name,
                        &response[..60.min(response.len())]
                    );
                }
                Err(e) => {
                    println!("   ❌ {} failed: {}", name, e);
                }
            }
        }

        Ok(())
    }

    async fn test_advanced_features(&self) -> Result<()> {
        println!("\n🚀 5. Advanced Features");
        println!("=======================");

        // Test with different temperatures
        println!("\n🌡️  Temperature Variation Test:");
        let temperatures = vec![0.0, 0.5, 1.0];
        let creativity_prompt =
            "Complete this story starter: 'The last person on Earth sat alone in a room...'";

        for temp in temperatures {
            println!("\n   Testing temperature {}:", temp);

            let chat_request = create_chat(COMMON_MODELS::GPT_4O_MINI)
                .add_user_message(creativity_prompt)
                .set_temperature(temp)
                .set_max_tokens(100)
                .build();

            match self.client.llm().chat_completion(chat_request).await {
                Ok(response) => {
                    if let Some(choice) = response.choices.first() {
                        println!(
                            "   ✅ Temperature {}: \"{}...\"",
                            temp,
                            &choice.message.content[..60.min(choice.message.content.len())]
                        );
                    }
                }
                Err(e) => {
                    println!("   ❌ Temperature {} failed: {}", temp, e);
                }
            }
        }

        // Test Circuit Breaker SDK LLM client integration
        println!("\n🔧 Circuit Breaker SDK Integration:");
        match self
            .client
            .llm()
            .chat(
                COMMON_MODELS::GPT_4O_MINI,
                "Explain the benefits of multi-provider LLM routing in 2 sentences.",
            )
            .await
        {
            Ok(response) => {
                println!("   ✅ SDK LLM response: {}", response);
            }
            Err(e) => {
                println!("   ⚠️  SDK LLM integration skipped: {}", e);
            }
        }

        Ok(())
    }

    async fn print_summary(&self) {
        println!("\n📋 6. Demo Summary");
        println!("==================");

        println!("✅ Multi-Provider Integration Completed:");
        println!("   🏢 Provider Discovery: All configured providers detected");
        println!("   🧪 Individual Testing: Provider-specific model validation");
        println!("   💰 Cost Analysis: Comparative pricing across providers");
        println!("   🧠 Smart Routing: Virtual models and strategy-based routing");
        println!("   🚀 Advanced Features: Temperature testing and SDK integration");
        println!();

        println!("🎯 Key Benefits Demonstrated:");
        println!("   • Unified API across multiple LLM providers");
        println!("   • Automatic cost optimization and provider selection");
        println!("   • Smart routing based on task requirements");
        println!("   • Transparent cost tracking and comparison");
        println!();

        println!("🛠️  Next Steps:");
        println!("   • Integrate Circuit Breaker into your Rust application");
        println!("   • Configure provider preferences and cost limits");
        println!("   • Set up monitoring and analytics");
        println!("   • Explore async streaming capabilities");
        println!();

        println!("🌐 Resources:");
        println!("   • GraphQL Interface: http://localhost:4000");
        println!("   • OpenAI API Endpoint: http://localhost:8081");
        println!("   • Rust SDK Documentation: Check the docs/ directory");
        println!();

        println!("🎉 Multi-Provider Demo Complete!");
        println!("   Thank you for exploring Circuit Breaker's multi-provider capabilities!");
    }

    // Helper methods
    async fn wait_for_enter(&self, message: &str) {
        print!("\n🎤 {}\n   Press Enter to continue...", message);
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
    }

    fn estimate_cost(&self, model: &str, response: &str) -> f64 {
        // Simplified cost estimation based on known rates
        let rates = HashMap::from([
            ("o4-mini-2025-04-16", (0.003, 0.012)),
            ("gpt-4o-mini", (0.00015, 0.0006)),
            ("gpt-4", (0.03, 0.06)),
            ("claude-3-haiku-20240307", (0.00025, 0.00125)),
            ("claude-3-sonnet-20240229", (0.003, 0.015)),
            ("gemini-1.5-flash", (0.000075, 0.0003)),
            ("gemini-1.5-pro", (0.00125, 0.005)),
        ]);

        let (input_rate, output_rate) = rates.get(model).unwrap_or(&(0.001, 0.002));
        let estimated_input_tokens = 10.0; // Simplified
        let estimated_output_tokens = response.len() as f64 / 4.0; // Rough estimate

        (estimated_input_tokens * input_rate + estimated_output_tokens * output_rate) / 1000.0
    }

    fn get_provider_from_model(&self, model: &str) -> String {
        if model.starts_with("gpt-") || model.starts_with("o4-") || model.contains("o4-") {
            "OpenAI".to_string()
        } else if model.starts_with("claude-") {
            "Anthropic".to_string()
        } else if model.starts_with("gemini-") {
            "Google".to_string()
        } else {
            "Unknown".to_string()
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create client
    let mut client_builder = Client::builder()
        .base_url(
            &std::env::var("CIRCUIT_BREAKER_URL")
                .unwrap_or_else(|_| "http://localhost:4000".to_string()),
        )?
        .timeout(30000);

    if let Ok(api_key) = std::env::var("CIRCUIT_BREAKER_API_KEY") {
        client_builder = client_builder.api_key(api_key);
    }

    let client = client_builder.build()?;

    // Run the demo
    let demo = MultiProviderDemo::new(client);
    demo.run().await
}
