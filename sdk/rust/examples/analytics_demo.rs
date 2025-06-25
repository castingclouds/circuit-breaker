//! Analytics and Budget Management Demo
//!
//! This example demonstrates the Circuit Breaker SDK's analytics and budget management
//! capabilities, showing how to track costs, set budgets, and monitor spending across
//! different users and projects.

use circuit_breaker_sdk::{Client, Result};
use serde_json::json;
use std::env;
use tokio;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Circuit Breaker Analytics & Budget Management Demo");
    println!("====================================================");

    // Initialize the client
    let base_url =
        env::var("CIRCUIT_BREAKER_URL").unwrap_or_else(|_| "http://localhost:4000".to_string());
    let api_key = env::var("CIRCUIT_BREAKER_API_KEY").ok();

    let mut client_builder = Client::builder().base_url(&base_url)?;
    if let Some(key) = api_key {
        client_builder = client_builder.api_key(key);
    }
    let client = client_builder.build()?;

    // Test connection
    match client.ping().await {
        Ok(ping) => println!("✅ Connected to Circuit Breaker server: {}", ping.message),
        Err(e) => {
            println!("❌ Failed to connect to server: {}", e);
            println!(
                "   Make sure the Circuit Breaker server is running at {}",
                base_url
            );
            return Ok(());
        }
    }

    println!("\n📊 Analytics & Budget Management Features:");
    println!("==========================================");

    // 1. Budget Management Demo
    println!("\n1. 💰 Budget Management");
    println!("   ---------------------");

    let user_id = "demo_user_123";
    let project_id = "demo_project_456";

    // Set a monthly budget for a user
    println!("   Setting monthly budget for user: {}", user_id);
    match client
        .analytics()
        .set_budget()
        .user_id(user_id)
        .limit(100.0)
        .period("monthly")
        .warning_threshold(0.8)
        .execute()
        .await
    {
        Ok(budget) => {
            println!("   ✅ Budget set successfully:");
            println!("      • Budget ID: {}", budget.budget_id);
            println!("      • Limit: ${:.2}", budget.limit);
            println!(
                "      • Used: ${:.2} ({:.1}%)",
                budget.used, budget.percentage_used
            );
            println!("      • Remaining: ${:.2}", budget.remaining);
            println!("      • Status: {}", budget.message);

            if budget.is_warning {
                println!("      ⚠️  Warning: Budget is in warning state!");
            }
            if budget.is_exhausted {
                println!("      🚨 Alert: Budget is exhausted!");
            }
        }
        Err(e) => {
            println!(
                "   ⚠️  Could not set budget (server may not be running): {}",
                e
            );
        }
    }

    // Set a project budget
    println!("\n   Setting project budget for: {}", project_id);
    match client
        .analytics()
        .set_budget()
        .project_id(project_id)
        .limit(500.0)
        .period("monthly")
        .warning_threshold(0.75)
        .execute()
        .await
    {
        Ok(budget) => {
            println!("   ✅ Project budget set:");
            println!("      • Limit: ${:.2}", budget.limit);
            println!("      • Warning at: {:.0}%", 0.75 * 100.0);
        }
        Err(e) => {
            println!("   ⚠️  Could not set project budget: {}", e);
        }
    }

    // 2. Budget Status Monitoring
    println!("\n2. 📈 Budget Status Monitoring");
    println!("   ----------------------------");

    // Check user budget status
    println!("   Checking budget status for user: {}", user_id);
    match client
        .analytics()
        .budget_status()
        .user_id(user_id)
        .get()
        .await
    {
        Ok(status) => {
            println!("   ✅ User budget status:");
            display_budget_status(&status);
        }
        Err(e) => {
            println!("   ⚠️  Could not get user budget status: {}", e);
        }
    }

    // Check project budget status
    println!("\n   Checking budget status for project: {}", project_id);
    match client
        .analytics()
        .budget_status()
        .project_id(project_id)
        .get()
        .await
    {
        Ok(status) => {
            println!("   ✅ Project budget status:");
            display_budget_status(&status);
        }
        Err(e) => {
            println!("   ⚠️  Could not get project budget status: {}", e);
        }
    }

    // 3. Cost Analytics
    println!("\n3. 📊 Cost Analytics");
    println!("   -----------------");

    let start_date = "2024-01-01";
    let end_date = "2024-01-31";

    println!(
        "   Getting cost analytics for user: {} ({} to {})",
        user_id, start_date, end_date
    );
    match client
        .analytics()
        .cost_analytics()
        .user_id(user_id)
        .date_range(start_date, end_date)
        .get()
        .await
    {
        Ok(analytics) => {
            println!("   ✅ Cost analytics retrieved:");
            display_cost_analytics(&analytics);
        }
        Err(e) => {
            println!("   ⚠️  Could not get cost analytics: {}", e);
        }
    }

    // Get project analytics
    println!("\n   Getting cost analytics for project: {}", project_id);
    match client
        .analytics()
        .cost_analytics()
        .project_id(project_id)
        .date_range(start_date, end_date)
        .get()
        .await
    {
        Ok(analytics) => {
            println!("   ✅ Project cost analytics:");
            display_cost_analytics(&analytics);
        }
        Err(e) => {
            println!("   ⚠️  Could not get project analytics: {}", e);
        }
    }

    // 4. Convenience Functions Demo
    println!("\n4. 🛠️  Convenience Functions");
    println!("   -------------------------");

    // Using convenience functions
    println!("   Using convenience functions for common operations:");

    // Budget status convenience function
    match circuit_breaker_sdk::budget_status(&client)
        .user_id(user_id)
        .get()
        .await
    {
        Ok(status) => {
            println!(
                "   ✅ Convenience budget status: ${:.2} used of ${:.2}",
                status.used, status.limit
            );
        }
        Err(e) => {
            println!("   ⚠️  Convenience budget status failed: {}", e);
        }
    }

    // Cost analytics convenience function
    match circuit_breaker_sdk::cost_analytics(&client, start_date, end_date)
        .user_id(user_id)
        .get()
        .await
    {
        Ok(analytics) => {
            println!(
                "   ✅ Convenience analytics: ${:.2} total cost",
                analytics.total_cost
            );
        }
        Err(e) => {
            println!("   ⚠️  Convenience analytics failed: {}", e);
        }
    }

    // Set budget convenience function
    match circuit_breaker_sdk::set_budget(&client, 200.0, "monthly")
        .user_id("convenience_user")
        .execute()
        .await
    {
        Ok(budget) => {
            println!("   ✅ Convenience budget set: ${:.2} limit", budget.limit);
        }
        Err(e) => {
            println!("   ⚠️  Convenience budget set failed: {}", e);
        }
    }

    // 5. Real-time Cost Monitoring (Future Feature)
    println!("\n5. ⏰ Real-time Cost Monitoring");
    println!("   -----------------------------");
    println!(
        "   Real-time cost monitoring via subscriptions will be available in a future release."
    );
    println!("   This will allow you to:");
    println!("   • Subscribe to cost updates as they happen");
    println!("   • Get real-time alerts when budgets are exceeded");
    println!("   • Monitor spending patterns in real-time");

    // Demonstrate that subscription isn't implemented yet
    match client
        .analytics()
        .subscribe_cost_updates(Some(user_id))
        .await
    {
        Ok(_) => {
            println!("   ✅ Subscribed to cost updates");
        }
        Err(e) => {
            println!("   ⚠️  Cost update subscriptions: {}", e);
        }
    }

    // 6. Advanced Analytics Scenarios
    println!("\n6. 🔬 Advanced Analytics Scenarios");
    println!("   --------------------------------");

    // Multi-date range analysis
    let date_ranges = vec![
        ("2024-01-01", "2024-01-31", "January"),
        ("2024-02-01", "2024-02-29", "February"),
        ("2024-03-01", "2024-03-31", "March"),
    ];

    println!("   Analyzing costs across multiple months:");
    for (start, end, month) in date_ranges {
        match client
            .analytics()
            .cost_analytics()
            .user_id(user_id)
            .date_range(start, end)
            .get()
            .await
        {
            Ok(analytics) => {
                println!(
                    "   • {}: ${:.2} total, {} tokens used",
                    month, analytics.total_cost, analytics.total_tokens
                );
            }
            Err(_) => {
                println!("   • {}: No data available", month);
            }
        }
    }

    // Budget health check
    println!("\n   Budget Health Check:");
    match client
        .analytics()
        .budget_status()
        .user_id(user_id)
        .get()
        .await
    {
        Ok(status) => {
            let health = if status.is_exhausted {
                "🚨 CRITICAL"
            } else if status.is_warning {
                "⚠️  WARNING"
            } else if status.percentage_used > 50.0 {
                "🟡 MODERATE"
            } else {
                "✅ HEALTHY"
            };

            println!(
                "   Budget Health: {} ({:.1}% used)",
                health, status.percentage_used
            );

            // Recommendations
            if status.is_exhausted {
                println!("   💡 Recommendation: Increase budget limit or optimize usage");
            } else if status.is_warning {
                println!("   💡 Recommendation: Monitor usage closely, consider optimizations");
            } else if status.percentage_used > 50.0 {
                println!(
                    "   💡 Recommendation: Review usage patterns for optimization opportunities"
                );
            } else {
                println!("   💡 Budget is healthy - continue current usage patterns");
            }
        }
        Err(e) => {
            println!("   ⚠️  Could not perform health check: {}", e);
        }
    }

    println!("\n🎉 Analytics Demo Complete!");
    println!("============================");
    println!("This demo showcased:");
    println!("• Budget management for users and projects");
    println!("• Real-time budget status monitoring");
    println!("• Comprehensive cost analytics");
    println!("• Convenience functions for common operations");
    println!("• Advanced analytics scenarios and health checks");
    println!("\nThe Analytics client provides powerful tools for:");
    println!("• Cost control and budget management");
    println!("• Usage optimization and monitoring");
    println!("• Financial planning and reporting");
    println!("• Multi-tenant cost tracking");

    Ok(())
}

fn display_budget_status(status: &circuit_breaker_sdk::BudgetStatus) {
    println!("      • Budget ID: {}", status.budget_id);
    println!("      • Limit: ${:.2}", status.limit);
    println!("      • Used: ${:.2}", status.used);
    println!("      • Percentage Used: {:.1}%", status.percentage_used);
    println!("      • Remaining: ${:.2}", status.remaining);
    println!("      • Is Warning: {}", status.is_warning);
    println!("      • Is Exhausted: {}", status.is_exhausted);
    println!("      • Message: {}", status.message);
}

fn display_cost_analytics(analytics: &circuit_breaker_sdk::CostAnalytics) {
    println!(
        "      • Period: {} to {}",
        analytics.period_start, analytics.period_end
    );
    println!("      • Total Cost: ${:.2}", analytics.total_cost);
    println!("      • Total Tokens: {}", analytics.total_tokens);
    println!(
        "      • Avg Cost/Token: ${:.6}",
        analytics.average_cost_per_token
    );

    if !analytics.provider_breakdown.is_empty() {
        println!("      • Provider Breakdown:");
        for (provider, cost) in &analytics.provider_breakdown {
            println!("        - {}: ${:.2}", provider, cost);
        }
    }

    if !analytics.model_breakdown.is_empty() {
        println!("      • Model Breakdown:");
        for (model, cost) in &analytics.model_breakdown {
            println!("        - {}: ${:.2}", model, cost);
        }
    }

    if !analytics.daily_costs.is_empty() {
        println!("      • Daily Costs (last 5 days):");
        let mut daily_costs: Vec<_> = analytics.daily_costs.iter().collect();
        daily_costs.sort_by(|a, b| a.0.cmp(b.0));
        for (date, cost) in daily_costs.iter().rev().take(5) {
            println!("        - {}: ${:.2}", date, cost);
        }
    }
}
