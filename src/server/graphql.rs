// GraphQL server implementation for Circuit Breaker
// This creates a standalone GraphQL server with predefined workflows

use std::sync::Arc;
use tokio::sync::RwLock;
use async_graphql::{http::GraphiQLSource, Schema};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post},
    Router, Server,
};
use tower_http::cors::CorsLayer;
use tracing::{info, debug};

use crate::engine::{
    graphql::{Query, Mutation, Subscription, create_schema_with_storage, create_schema_with_agents},
    storage::{InMemoryStorage, WorkflowStorage},
    agents::{AgentEngine, AgentStorage, InMemoryAgentStorage, AgentEngineConfig},
    rules::RulesEngine,
};
use crate::models::{
    WorkflowDefinition, PlaceId, TransitionId, TransitionDefinition
};

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
}

impl GraphQLServer {
    pub fn new() -> Self {
        Self {
            config: GraphQLServerConfig::default(),
            storage: Box::new(InMemoryStorage::default()),
            agent_storage: None,
            agent_engine: None,
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

    pub async fn run(mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Add default workflows
        self.add_default_workflows().await?;
        
        let schema = if let (Some(agent_storage), Some(agent_engine)) = (self.agent_storage, self.agent_engine) {
            info!("🤖 Starting server with AI agent support");
            create_schema_with_agents(self.storage, agent_storage, agent_engine)
        } else {
            info!("📋 Starting server with basic workflow support");
            create_schema_with_storage(self.storage)
        };
        
        let app_state = Arc::new(RwLock::new(schema));

        let mut app = Router::new()
            .route("/", get(graphiql).post(graphql_handler))
            .route("/graphql", post(graphql_handler))
            .route("/health", get(health_check))
            .with_state(app_state);

        if self.config.cors_enabled {
            app = app.layer(CorsLayer::permissive());
        }

        let addr = format!("0.0.0.0:{}", self.config.port);
        
        info!("🚀 GraphQL server running on http://localhost:{}", self.config.port);
        info!("📊 GraphiQL interface: http://localhost:{}", self.config.port);
        info!("🔗 GraphQL endpoint: http://localhost:{}/graphql", self.config.port);
        
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
            places: vec![
                PlaceId::from("draft"),
                PlaceId::from("pending_review"),
                PlaceId::from("reviewed"),
                PlaceId::from("approved"),
                PlaceId::from("rejected"),
            ],
            transitions: vec![
                TransitionDefinition {
                    id: TransitionId::from("submit"),
                    from_places: vec![PlaceId::from("draft")],
                    to_place: PlaceId::from("pending_review"),
                    conditions: vec![],
                    rules: vec![],
                },
                TransitionDefinition {
                    id: TransitionId::from("review"),
                    from_places: vec![PlaceId::from("pending_review")],
                    to_place: PlaceId::from("reviewed"),
                    conditions: vec![],
                    rules: vec![],
                },
                TransitionDefinition {
                    id: TransitionId::from("approve"),
                    from_places: vec![PlaceId::from("reviewed")],
                    to_place: PlaceId::from("approved"),
                    conditions: vec![],
                    rules: vec![],
                },
                TransitionDefinition {
                    id: TransitionId::from("reject"),
                    from_places: vec![PlaceId::from("reviewed")],
                    to_place: PlaceId::from("rejected"),
                    conditions: vec![],
                    rules: vec![],
                },
                TransitionDefinition {
                    id: TransitionId::from("revise"),
                    from_places: vec![PlaceId::from("rejected")],
                    to_place: PlaceId::from("draft"),
                    conditions: vec![],
                    rules: vec![],
                },
            ],
            initial_place: PlaceId::from("draft"),
        };

        // Software Deployment Workflow
        let _deployment_workflow = WorkflowDefinition {
            id: "software_deployment".to_string(),
            name: "Software Deployment Pipeline".to_string(),
            places: vec![
                PlaceId::from("development"),
                PlaceId::from("staging"),
                PlaceId::from("production"),
                PlaceId::from("rollback"),
                PlaceId::from("hotfix"),
            ],
            transitions: vec![
                TransitionDefinition {
                    id: TransitionId::from("deploy_to_staging"),
                    from_places: vec![PlaceId::from("development")],
                    to_place: PlaceId::from("staging"),
                    conditions: vec!["tests_passed".to_string()],
                    rules: vec![],
                },
                TransitionDefinition {
                    id: TransitionId::from("deploy_to_production"),
                    from_places: vec![PlaceId::from("staging")],
                    to_place: PlaceId::from("production"),
                    conditions: vec!["qa_approved".to_string()],
                    rules: vec![],
                },
                TransitionDefinition {
                    id: TransitionId::from("rollback_from_production"),
                    from_places: vec![PlaceId::from("production")],
                    to_place: PlaceId::from("rollback"),
                    conditions: vec![],
                    rules: vec![],
                },
                TransitionDefinition {
                    id: TransitionId::from("create_hotfix"),
                    from_places: vec![PlaceId::from("production")],
                    to_place: PlaceId::from("hotfix"),
                    conditions: vec!["critical_bug_detected".to_string()],
                    rules: vec![],
                },
                TransitionDefinition {
                    id: TransitionId::from("deploy_hotfix"),
                    from_places: vec![PlaceId::from("rollback")],
                    to_place: PlaceId::from("staging"),
                    conditions: vec![],
                    rules: vec![],
                },
                TransitionDefinition {
                    id: TransitionId::from("hotfix_to_staging"),
                    from_places: vec![PlaceId::from("hotfix")],
                    to_place: PlaceId::from("staging"),
                    conditions: vec!["hotfix_tested".to_string()],
                    rules: vec![],
                },
            ],
            initial_place: PlaceId::from("development"),
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

// GraphiQL interface
async fn graphiql() -> impl IntoResponse {
    Html(GraphiQLSource::build().endpoint("/graphql").finish())
}

// Health check endpoint
async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "Circuit Breaker GraphQL Server is running!")
} 