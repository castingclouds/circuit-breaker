//! Circuit Breaker Admin CLI
//!
//! Administrative CLI tool for managing Circuit Breaker data stored in NATS.
//! This tool provides cleanup, maintenance, and debugging capabilities.

use anyhow::Result;
use async_nats::jetstream::{self};
use circuit_breaker::engine::nats_storage::{NATSStorage, NATSStorageConfig};
use circuit_breaker::engine::rules::{NATSRuleStorage, RuleStorage};
use circuit_breaker::WorkflowStorage;
use clap::{Parser, Subcommand};
use futures::StreamExt;
use tokio;
use tracing::{error, info, warn};

#[derive(Parser)]
#[command(name = "circuit-breaker-admin")]
#[command(about = "Circuit Breaker Admin CLI - Manage NATS data and system maintenance")]
#[command(version = "1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// NATS server URL
    #[arg(long, env = "NATS_URL", default_value = "nats://localhost:4222")]
    nats_url: String,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Clean up all data
    Cleanup {
        /// Confirm the cleanup operation
        #[arg(long)]
        confirm: bool,

        /// Clean up workflows
        #[arg(long)]
        workflows: bool,

        /// Clean up resources
        #[arg(long)]
        resources: bool,

        /// Clean up rules
        #[arg(long)]
        rules: bool,

        /// Clean up everything (workflows, resources, rules)
        #[arg(long)]
        all: bool,
    },

    /// List data statistics
    Stats,

    /// List all workflows
    ListWorkflows,

    /// List all resources
    ListResources {
        /// Workflow ID to filter by
        #[arg(long)]
        workflow_id: Option<String>,
    },

    /// List all rules
    ListRules,

    /// Delete specific workflow
    DeleteWorkflow {
        /// Workflow ID to delete
        workflow_id: String,

        /// Confirm the deletion
        #[arg(long)]
        confirm: bool,
    },

    /// Delete specific resource
    DeleteResource {
        /// Resource ID to delete
        resource_id: String,

        /// Confirm the deletion
        #[arg(long)]
        confirm: bool,
    },

    /// Delete specific rule
    DeleteRule {
        /// Rule ID to delete
        rule_id: String,

        /// Confirm the deletion
        #[arg(long)]
        confirm: bool,
    },

    /// Delete all rules
    DeleteAllRules {
        /// Confirm the deletion
        #[arg(long)]
        confirm: bool,
    },

    /// NATS stream management
    Stream {
        #[command(subcommand)]
        action: StreamCommands,
    },
}

#[derive(Subcommand)]
enum StreamCommands {
    /// List all NATS streams
    List,

    /// Delete all Circuit Breaker streams
    DeleteAll {
        /// Confirm the deletion
        #[arg(long)]
        confirm: bool,
    },

    /// Purge all messages from streams (keeps stream structure)
    Purge {
        /// Confirm the purge
        #[arg(long)]
        confirm: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt().with_env_filter(log_level).init();

    // Initialize NATS storage
    let config = NATSStorageConfig {
        nats_urls: vec![cli.nats_url.clone()],
        ..Default::default()
    };

    let storage = NATSStorage::new(config).await?;

    match cli.command {
        Commands::Cleanup {
            confirm,
            workflows,
            resources,
            rules,
            all,
        } => {
            if !confirm {
                error!("❌ Cleanup operation requires --confirm flag for safety");
                return Ok(());
            }

            if all || workflows || resources || rules {
                cleanup_data(&storage, all, workflows, resources, rules).await?;
            } else {
                error!(
                    "❌ Please specify what to clean: --workflows, --resources, --rules, or --all"
                );
            }
        }

        Commands::Stats => {
            show_stats(&storage).await?;
        }

        Commands::ListWorkflows => {
            list_workflows(&storage).await?;
        }

        Commands::ListResources { workflow_id } => {
            list_resources(&storage, workflow_id).await?;
        }

        Commands::ListRules => {
            list_rules(&cli.nats_url).await?;
        }

        Commands::DeleteWorkflow {
            workflow_id,
            confirm,
        } => {
            if !confirm {
                error!("❌ Delete operation requires --confirm flag for safety");
                return Ok(());
            }
            delete_workflow(&storage, &workflow_id).await?;
        }

        Commands::DeleteResource {
            resource_id,
            confirm,
        } => {
            if !confirm {
                error!("❌ Delete operation requires --confirm flag for safety");
                return Ok(());
            }
            delete_resource(&storage, &resource_id).await?;
        }

        Commands::DeleteRule { rule_id, confirm } => {
            if !confirm {
                error!("❌ Delete operation requires --confirm flag for safety");
                return Ok(());
            }
            delete_rule(&cli.nats_url, &rule_id).await?;
        }

        Commands::DeleteAllRules { confirm } => {
            if !confirm {
                error!("❌ Delete all rules operation requires --confirm flag for safety");
                return Ok(());
            }
            delete_all_rules(&cli.nats_url).await?;
        }

        Commands::Stream { action } => {
            handle_stream_commands(&cli.nats_url, action).await?;
        }
    }

    Ok(())
}

async fn cleanup_data(
    storage: &NATSStorage,
    all: bool,
    workflows: bool,
    resources: bool,
    rules: bool,
) -> Result<()> {
    info!("🧹 Starting cleanup operation...");

    if all || workflows {
        info!("🔄 Cleaning up workflows...");
        let workflows = storage.list_workflows().await?;
        info!("Found {} workflows to clean", workflows.len());

        for workflow in workflows {
            info!("  Deleting workflow: {}", workflow.id);
            // Note: This would need a delete_workflow method in storage
            // For now, we'll clean via stream purging
        }
    }

    if all || resources {
        info!("🔄 Cleaning up resources...");
        let resources = storage.list_resources(None).await?;
        info!("Found {} resources to clean", resources.len());

        for resource in resources {
            info!("  Deleting resource: {}", resource.id);
            // Note: This would need a delete_resource method in storage
        }
    }

    if all || rules {
        info!("🔄 Cleaning up rules...");
        // Rules cleanup will be handled separately
        warn!("Rules cleanup not yet implemented - use stream purge instead");
    }

    // For now, the most effective cleanup is to purge the streams
    info!("🔄 Purging NATS streams for complete cleanup...");
    purge_streams(storage).await?;

    info!("✅ Cleanup completed successfully");
    Ok(())
}

async fn show_stats(storage: &NATSStorage) -> Result<()> {
    info!("📊 Gathering Circuit Breaker statistics...");

    let workflows = storage.list_workflows().await?;
    let resources = storage.list_resources(None).await?;

    println!("\n📈 Circuit Breaker Data Statistics");
    println!("==================================");
    println!("Workflows: {}", workflows.len());
    println!("Resources: {}", resources.len());

    // Group resources by workflow
    let mut workflow_resource_counts = std::collections::HashMap::new();
    for resource in &resources {
        *workflow_resource_counts
            .entry(resource.workflow_id.clone())
            .or_insert(0) += 1;
    }

    if !workflow_resource_counts.is_empty() {
        println!("\nResources per workflow:");
        for (workflow_id, count) in workflow_resource_counts {
            println!("  {}: {} resources", workflow_id, count);
        }
    }

    Ok(())
}

async fn list_workflows(storage: &NATSStorage) -> Result<()> {
    let workflows = storage.list_workflows().await?;

    println!("\n📋 Workflows ({})", workflows.len());
    println!("=====================================");

    if workflows.is_empty() {
        println!("No workflows found.");
        return Ok(());
    }

    for workflow in workflows {
        println!("🔧 ID: {}", workflow.id);
        println!("   Name: {}", workflow.name);
        println!("   States: {}", workflow.states.len());
        println!("   Activities: {}", workflow.activities.len());
        println!();
    }

    Ok(())
}

async fn list_resources(storage: &NATSStorage, workflow_id: Option<String>) -> Result<()> {
    let all_resources = storage.list_resources(None).await?;

    let resources: Vec<_> = if let Some(wf_id) = workflow_id {
        all_resources
            .into_iter()
            .filter(|r| r.workflow_id == wf_id)
            .collect()
    } else {
        all_resources
    };

    println!("\n📦 Resources ({})", resources.len());
    println!("=====================================");

    if resources.is_empty() {
        println!("No resources found.");
        return Ok(());
    }

    for resource in resources {
        println!("📦 ID: {}", resource.id);
        println!("   Workflow: {}", resource.workflow_id);
        println!("   State: {}", resource.state);
        println!("   Created: {}", resource.created_at);
        println!("   Updated: {}", resource.updated_at);
        if let Some(seq) = resource.nats_sequence {
            println!("   NATS Sequence: {}", seq);
        }
        println!();
    }

    Ok(())
}

async fn list_rules(nats_url: &str) -> Result<()> {
    info!("🔍 Connecting to NATS for rule listing...");

    // Create rule storage
    let client = async_nats::connect(nats_url).await?;
    let rule_storage = NATSRuleStorage::new(client).await?;

    let rules = rule_storage.list_rules(None).await?;

    println!("\n📏 Rules ({})", rules.len());
    println!("=====================================");

    if rules.is_empty() {
        println!("No rules found.");
        return Ok(());
    }

    for rule in rules {
        println!("📏 ID: {}", rule.id);
        println!("   Name: {}", rule.name);
        println!("   Description: {}", rule.description);
        println!("   Tags: {:?}", rule.tags);
        println!("   Created: {}", rule.created_at);
        println!();
    }

    Ok(())
}

async fn delete_workflow(storage: &NATSStorage, workflow_id: &str) -> Result<()> {
    info!("🗑️  Attempting to delete workflow: {}", workflow_id);

    // First check if workflow exists
    match storage.get_workflow(workflow_id).await? {
        Some(workflow) => {
            info!("Found workflow: {}", workflow.name);

            // Get associated resources
            let resources = storage.list_resources(None).await?;
            let workflow_resources: Vec<_> = resources
                .into_iter()
                .filter(|r| r.workflow_id == workflow_id)
                .collect();

            if !workflow_resources.is_empty() {
                warn!(
                    "⚠️  Workflow has {} associated resources",
                    workflow_resources.len()
                );
                for resource in workflow_resources {
                    info!("  Resource: {} (state: {})", resource.id, resource.state);
                }
                warn!("⚠️  Note: Direct workflow deletion not implemented yet");
                warn!("💡 Use 'stream delete-all --confirm' for complete cleanup");
            } else {
                warn!("💡 Direct workflow deletion not implemented yet");
                warn!("💡 Use 'stream delete-all --confirm' for complete cleanup");
            }
        }
        None => {
            error!("❌ Workflow not found: {}", workflow_id);
        }
    }

    Ok(())
}

async fn delete_resource(storage: &NATSStorage, resource_id: &str) -> Result<()> {
    info!("🗑️  Attempting to delete resource: {}", resource_id);

    let resource_uuid = resource_id
        .parse::<uuid::Uuid>()
        .map_err(|_| anyhow::anyhow!("Invalid resource ID format"))?;

    match storage.get_resource(&resource_uuid).await? {
        Some(resource) => {
            info!("Found resource in workflow: {}", resource.workflow_id);
            info!("Current state: {}", resource.state);
            warn!("💡 Direct resource deletion not implemented yet");
            warn!("💡 Use 'stream delete-all --confirm' for complete cleanup");
        }
        None => {
            error!("❌ Resource not found: {}", resource_id);
        }
    }

    Ok(())
}

async fn delete_rule(nats_url: &str, rule_id: &str) -> Result<()> {
    info!("🗑️  Attempting to delete rule: {}", rule_id);

    let client = async_nats::connect(nats_url).await?;
    let rule_storage = NATSRuleStorage::new(client).await?;

    match rule_storage.delete_rule(rule_id).await? {
        true => {
            info!("✅ Successfully deleted rule: {}", rule_id);
        }
        false => {
            warn!("⚠️  Rule not found or already deleted: {}", rule_id);
        }
    }

    Ok(())
}

async fn handle_stream_commands(nats_url: &str, action: StreamCommands) -> Result<()> {
    let client = async_nats::connect(nats_url).await?;
    let jetstream = jetstream::new(client);

    match action {
        StreamCommands::List => {
            list_streams(&jetstream).await?;
        }
        StreamCommands::DeleteAll { confirm } => {
            if !confirm {
                error!("❌ Stream deletion requires --confirm flag for safety");
                return Ok(());
            }
            delete_all_streams(&jetstream).await?;
        }
        StreamCommands::Purge { confirm } => {
            if !confirm {
                error!("❌ Stream purge requires --confirm flag for safety");
                return Ok(());
            }
            purge_all_streams(&jetstream).await?;
        }
    }

    Ok(())
}

async fn list_streams(jetstream: &jetstream::Context) -> Result<()> {
    info!("🔍 Listing NATS streams...");

    let mut streams = jetstream.streams();
    let mut stream_count = 0;

    println!("\n🌊 NATS Streams");
    println!("===============");

    while let Some(stream_result) = streams.next().await {
        match stream_result {
            Ok(info) => {
                stream_count += 1;

                println!("🌊 Stream: {}", info.config.name);
                println!("   Subjects: {:?}", info.config.subjects);
                println!("   Messages: {}", info.state.messages);
                println!("   Bytes: {}", info.state.bytes);
                println!("   Storage: {:?}", info.config.storage);
                println!();
            }
            Err(e) => {
                error!("Failed to get stream info: {}", e);
            }
        }
    }

    if stream_count == 0 {
        println!("No streams found.");
    } else {
        println!("Total streams: {}", stream_count);
    }

    Ok(())
}

async fn delete_all_streams(jetstream: &jetstream::Context) -> Result<()> {
    info!("🗑️  Deleting all Circuit Breaker streams...");

    let stream_names = vec![
        "CIRCUIT_BREAKER_GLOBAL",
        "CIRCUIT_BREAKER_WORKFLOWS",
        "CIRCUIT_BREAKER_RESOURCES",
        "CIRCUIT_BREAKER_RULES",
    ];

    for stream_name in stream_names {
        match jetstream.delete_stream(stream_name).await {
            Ok(_) => {
                info!("✅ Deleted stream: {}", stream_name);
            }
            Err(e) => {
                // Stream might not exist, which is fine
                warn!("⚠️  Could not delete stream {}: {}", stream_name, e);
            }
        }
    }

    info!("✅ Stream deletion completed");
    Ok(())
}

async fn purge_all_streams(jetstream: &jetstream::Context) -> Result<()> {
    info!("🧹 Purging all Circuit Breaker streams...");

    let stream_names = vec![
        "CIRCUIT_BREAKER_GLOBAL",
        "CIRCUIT_BREAKER_WORKFLOWS",
        "CIRCUIT_BREAKER_RESOURCES",
        "CIRCUIT_BREAKER_RULES",
    ];

    for stream_name in stream_names {
        match jetstream.get_stream(stream_name).await {
            Ok(stream) => match stream.purge().await {
                Ok(purge_response) => {
                    info!(
                        "✅ Purged stream {}: {} messages removed",
                        stream_name, purge_response.purged
                    );
                }
                Err(e) => {
                    error!("❌ Failed to purge stream {}: {}", stream_name, e);
                }
            },
            Err(_) => {
                // Stream doesn't exist, which is fine
                info!("ℹ️  Stream {} does not exist (skipping)", stream_name);
            }
        }
    }

    info!("✅ Stream purging completed");
    Ok(())
}

async fn delete_all_rules(nats_url: &str) -> Result<()> {
    info!("🗑️  Deleting all rules...");

    let client = async_nats::connect(nats_url).await?;
    let rule_storage = NATSRuleStorage::new(client).await?;

    // Get all rules first
    let rules = rule_storage.list_rules(None).await?;
    let total_rules = rules.len();

    if total_rules == 0 {
        info!("ℹ️  No rules found to delete");
        return Ok(());
    }

    info!("🔍 Found {} rules to delete", total_rules);

    let mut deleted_count = 0;
    let mut failed_count = 0;

    for rule in rules {
        match rule_storage.delete_rule(&rule.id).await {
            Ok(true) => {
                deleted_count += 1;
                info!("✅ Deleted rule: {} ({})", rule.id, rule.name);
            }
            Ok(false) => {
                failed_count += 1;
                warn!("⚠️  Rule not found: {} ({})", rule.id, rule.name);
            }
            Err(e) => {
                failed_count += 1;
                error!("❌ Failed to delete rule {}: {}", rule.id, e);
            }
        }
    }

    info!("📊 Delete Summary:");
    info!("   ✅ Successfully deleted: {}", deleted_count);
    if failed_count > 0 {
        warn!("   ❌ Failed to delete: {}", failed_count);
    }

    if deleted_count > 0 {
        info!("🎉 All rules cleanup completed!");
    }

    Ok(())
}

async fn purge_streams(_storage: &NATSStorage) -> Result<()> {
    info!("🧹 Performing complete NATS stream purge...");

    // This is a more thorough cleanup that purges the main global stream
    let client = async_nats::connect("nats://localhost:4222").await?;
    let jetstream = jetstream::new(client);

    purge_all_streams(&jetstream).await?;

    Ok(())
}
