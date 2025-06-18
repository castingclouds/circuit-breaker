// GraphQL server implementation for Circuit Breaker
// This creates a standalone GraphQL server with predefined workflows

use async_graphql::Schema;
use async_graphql_axum::{GraphQLRequest, GraphQLResponse, GraphQLSubscription};
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post},
    Router, Server,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tracing::{debug, info};

use crate::engine::{
    agents::{AgentEngine, AgentEngineConfig, AgentStorage, InMemoryAgentStorage},
    graphql::{
        create_schema_with_agents, create_schema_with_full_storage, create_schema_with_nats,
        create_schema_with_nats_and_agents, create_schema_with_storage, Mutation, Query,
        Subscription,
    },
    nats_storage::{NATSStorage, NATSStorageConfig, NATSStorageWrapper},
    rules::RulesEngine,
    storage::{InMemoryStorage, WorkflowStorage},
};
use crate::models::{ActivityDefinition, ActivityId, StateId, WorkflowDefinition};

pub type GraphQLSchema = Schema<Query, Mutation, Subscription>;

/// GraphQL server configuration
#[derive(Clone)]
pub struct GraphQLServerConfig {
    pub port: u16,
    pub cors_enabled: bool,
}

impl Default for GraphQLServerConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            cors_enabled: true,
        }
    }
}

/// GraphQL server
pub struct GraphQLServer {
    config: GraphQLServerConfig,
    storage: Box<dyn WorkflowStorage>,
    agent_storage: Option<std::sync::Arc<dyn AgentStorage>>,
    agent_engine: Option<AgentEngine>,
    nats_storage: Option<std::sync::Arc<NATSStorage>>,
    rule_storage: Option<std::sync::Arc<dyn crate::engine::rules::RuleStorage>>,
}

impl GraphQLServer {
    pub fn new() -> Self {
        Self {
            config: GraphQLServerConfig::default(),
            storage: Box::new(InMemoryStorage::default()),
            agent_storage: None,
            agent_engine: None,
            nats_storage: None,
            rule_storage: None,
        }
    }

    pub fn with_config(mut self, config: GraphQLServerConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_storage(mut self, storage: Box<dyn WorkflowStorage>) -> Self {
        self.storage = storage;
        self
    }

    pub fn with_agents(mut self) -> Self {
        // Create a single shared agent storage instance
        let agent_storage = std::sync::Arc::new(InMemoryAgentStorage::default());
        let rules_engine = std::sync::Arc::new(RulesEngine::new());

        // Create agent engine with shared storage
        let agent_engine = AgentEngine::new(
            agent_storage.clone(),
            rules_engine,
            AgentEngineConfig::default(),
        );

        // Clone the shared Arc<dyn AgentStorage> and assign it to self.agent_storage
        // This ensures the storage is shared across the application
        let shared_storage = agent_storage.clone();

        self.agent_storage = Some(shared_storage as std::sync::Arc<dyn AgentStorage>);
        self.agent_engine = Some(agent_engine);
        self
    }

    pub fn with_nats_storage(mut self, nats_storage: std::sync::Arc<NATSStorage>) -> Self {
        self.nats_storage = Some(nats_storage);
        self
    }

    pub fn with_rule_storage(
        mut self,
        rule_storage: std::sync::Arc<dyn crate::engine::rules::RuleStorage>,
    ) -> Self {
        self.rule_storage = Some(rule_storage);
        self
    }

    pub async fn run(mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Add default workflows
        self.add_default_workflows().await?;

        let schema = match (
            self.nats_storage,
            self.agent_storage,
            self.agent_engine,
            self.rule_storage,
        ) {
            (Some(nats_storage), Some(agent_storage), Some(agent_engine), Some(rule_storage)) => {
                info!("🎯 Starting server with full storage support (NATS + Agents + Rules)");
                create_schema_with_full_storage(
                    nats_storage,
                    agent_storage,
                    std::sync::Arc::new(agent_engine),
                    rule_storage,
                )
            }
            (Some(nats_storage), Some(agent_storage), Some(agent_engine), None) => {
                info!("🤖 Starting server with NATS storage and AI agent support");
                create_schema_with_nats_and_agents(nats_storage, agent_storage, agent_engine)
            }
            (Some(nats_storage), _, _, _) => {
                info!("📡 Starting server with NATS storage support");
                create_schema_with_nats(nats_storage)
            }
            (None, Some(agent_storage), Some(agent_engine), _) => {
                info!("🤖 Starting server with AI agent support");
                create_schema_with_agents(self.storage, agent_storage, agent_engine)
            }
            (None, _, _, _) => {
                info!("📋 Starting server with basic workflow support");
                create_schema_with_storage(self.storage)
            }
        };

        let app_state = Arc::new(RwLock::new(schema.clone()));

        let subscription_service = GraphQLSubscription::new(schema);

        let mut app = Router::new()
            .route("/", get(graphiql).post(graphql_handler))
            .route("/graphql", post(graphql_handler))
            .route_service("/ws", subscription_service)
            .route("/health", get(health_check))
            .with_state(app_state);

        if self.config.cors_enabled {
            app = app.layer(CorsLayer::permissive());
        }

        let addr = format!("0.0.0.0:{}", self.config.port);

        info!(
            "🚀 GraphQL server running on http://localhost:{}",
            self.config.port
        );
        info!(
            "📊 GraphiQL interface: http://localhost:{}",
            self.config.port
        );
        info!(
            "🔗 GraphQL endpoint: http://localhost:{}/graphql",
            self.config.port
        );
        info!(
            "📡 GraphQL WebSocket: ws://localhost:{}/ws",
            self.config.port
        );

        // Use axum 0.6 syntax
        Server::bind(&addr.parse()?)
            .serve(app.into_make_service())
            .await?;
        Ok(())
    }

    async fn add_default_workflows(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Document Review Workflow
        let _document_workflow = WorkflowDefinition {
            id: "document_review".to_string(),
            name: "Document Review Process".to_string(),
            states: vec![
                StateId::from("draft"),
                StateId::from("pending_review"),
                StateId::from("reviewed"),
                StateId::from("approved"),
                StateId::from("rejected"),
            ],
            activities: vec![
                ActivityDefinition {
                    id: ActivityId::from("submit"),
                    from_states: vec![StateId::from("draft")],
                    to_state: StateId::from("pending_review"),
                    conditions: vec![],
                    rules: vec![],
                },
                ActivityDefinition {
                    id: ActivityId::from("review"),
                    from_states: vec![StateId::from("pending_review")],
                    to_state: StateId::from("reviewed"),
                    conditions: vec![],
                    rules: vec![],
                },
                ActivityDefinition {
                    id: ActivityId::from("approve"),
                    from_states: vec![StateId::from("reviewed")],
                    to_state: StateId::from("approved"),
                    conditions: vec![],
                    rules: vec![],
                },
                ActivityDefinition {
                    id: ActivityId::from("reject"),
                    from_states: vec![StateId::from("reviewed")],
                    to_state: StateId::from("rejected"),
                    conditions: vec![],
                    rules: vec![],
                },
                ActivityDefinition {
                    id: ActivityId::from("revise"),
                    from_states: vec![StateId::from("rejected")],
                    to_state: StateId::from("draft"),
                    conditions: vec![],
                    rules: vec![],
                },
            ],
            initial_state: StateId::from("draft"),
        };

        // Software Deployment Workflow
        let _deployment_workflow = WorkflowDefinition {
            id: "software_deployment".to_string(),
            name: "Software Deployment Pipeline".to_string(),
            states: vec![
                StateId::from("development"),
                StateId::from("staging"),
                StateId::from("production"),
                StateId::from("rollback"),
                StateId::from("hotfix"),
            ],
            activities: vec![
                ActivityDefinition {
                    id: ActivityId::from("deploy_to_staging"),
                    from_states: vec![StateId::from("development")],
                    to_state: StateId::from("staging"),
                    conditions: vec!["tests_passed".to_string()],
                    rules: vec![],
                },
                ActivityDefinition {
                    id: ActivityId::from("deploy_to_production"),
                    from_states: vec![StateId::from("staging")],
                    to_state: StateId::from("production"),
                    conditions: vec!["qa_approved".to_string()],
                    rules: vec![],
                },
                ActivityDefinition {
                    id: ActivityId::from("rollback_from_production"),
                    from_states: vec![StateId::from("production")],
                    to_state: StateId::from("rollback"),
                    conditions: vec![],
                    rules: vec![],
                },
                ActivityDefinition {
                    id: ActivityId::from("create_hotfix"),
                    from_states: vec![StateId::from("production")],
                    to_state: StateId::from("hotfix"),
                    conditions: vec!["critical_bug_detected".to_string()],
                    rules: vec![],
                },
                ActivityDefinition {
                    id: ActivityId::from("deploy_hotfix"),
                    from_states: vec![StateId::from("rollback")],
                    to_state: StateId::from("staging"),
                    conditions: vec![],
                    rules: vec![],
                },
                ActivityDefinition {
                    id: ActivityId::from("hotfix_to_staging"),
                    from_states: vec![StateId::from("hotfix")],
                    to_state: StateId::from("staging"),
                    conditions: vec!["hotfix_tested".to_string()],
                    rules: vec![],
                },
            ],
            initial_state: StateId::from("development"),
        };

        // Store workflows - we'll need to implement this in the storage trait
        debug!("✅ Added default workflows:");
        debug!("   📄 Document Review Process");
        debug!("   🚀 Software Deployment Pipeline");

        Ok(())
    }
}

impl Default for GraphQLServer {
    fn default() -> Self {
        Self::new()
    }
}

/// Legacy builder pattern for backwards compatibility
pub struct GraphQLServerBuilder {
    server: GraphQLServer,
}

impl GraphQLServerBuilder {
    pub fn new() -> Self {
        Self {
            server: GraphQLServer::new(),
        }
    }

    pub fn with_storage(mut self, storage: Box<dyn WorkflowStorage>) -> Self {
        self.server = self.server.with_storage(storage);
        self
    }

    pub fn with_port(mut self, port: u16) -> Self {
        let mut config = self.server.config.clone();
        config.port = port;
        self.server = self.server.with_config(config);
        self
    }

    pub fn with_agents(mut self) -> Self {
        self.server = self.server.with_agents();
        self
    }

    pub fn with_rule_storage(
        mut self,
        rule_storage: std::sync::Arc<dyn crate::engine::rules::RuleStorage>,
    ) -> Self {
        self.server = self.server.with_rule_storage(rule_storage);
        self
    }

    pub async fn with_nats(mut self, nats_url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let nats_config = NATSStorageConfig {
            nats_urls: vec![nats_url.to_string()],
            ..Default::default()
        };

        let nats_storage = std::sync::Arc::new(NATSStorage::new(nats_config).await?);
        let storage_wrapper = NATSStorageWrapper::new(nats_storage.clone());

        // Create NATS client for rule storage
        let nats_client = async_nats::connect(&nats_url).await?;
        let rule_storage =
            std::sync::Arc::new(crate::engine::rules::NATSRuleStorage::new(nats_client).await?);

        self.server = self.server.with_storage(Box::new(storage_wrapper));
        self.server = self.server.with_nats_storage(nats_storage);
        self.server = self.server.with_rule_storage(rule_storage);
        Ok(self)
    }

    pub async fn build_and_run(self) -> Result<(), Box<dyn std::error::Error>> {
        self.server.run().await
    }
}

impl Default for GraphQLServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// GraphQL handler
async fn graphql_handler(
    State(schema): State<Arc<RwLock<GraphQLSchema>>>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    let schema = schema.read().await;
    schema.execute(req.into_inner()).await.into()
}

// GraphiQL interface with WebSocket support
async fn graphiql() -> impl IntoResponse {
    Html(
        r#"
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="robots" content="noindex">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <meta name="referrer" content="origin">
    <title>GraphiQL IDE</title>
    <style>
      body {
        height: 100%;
        margin: 0;
        width: 100%;
        overflow: hidden;
      }
      #graphiql {
        height: 100vh;
      }
    </style>
    <script crossorigin src="https://unpkg.com/react@18/umd/react.development.js"></script>
    <script crossorigin src="https://unpkg.com/react-dom@18/umd/react-dom.development.js"></script>
    <link rel="icon" href="https://graphql.org/favicon.ico">
    <link rel="stylesheet" href="https://unpkg.com/graphiql@3/graphiql.min.css" />
  </head>
  <body>
    <div id="graphiql">Loading...</div>
    <script src="https://unpkg.com/graphiql@3/graphiql.min.js" type="application/javascript"></script>
    <script>
      const root = ReactDOM.createRoot(document.getElementById('graphiql'));

      const fetcher = GraphiQL.createFetcher({
        url: '/graphql',
        subscriptionUrl: 'ws://localhost:4000/ws',
      });

      root.render(React.createElement(GraphiQL, {
        fetcher: fetcher,
        defaultEditorToolsVisibility: true,
      }));
    </script>
  </body>
</html>
"#,
    )
}

// Health check endpoint
async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "Circuit Breaker GraphQL Server is running!")
}
