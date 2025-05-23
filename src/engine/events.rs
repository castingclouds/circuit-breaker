// Event system for triggering functions

//! # Event System
//! 
//! This module provides the event bus system that connects workflow operations
//! to function execution. It handles:
//! - Event emission from workflow operations
//! - Event subscription and routing
//! - Integration with function engine

use tokio::sync::broadcast;
use uuid::Uuid;

use crate::models::{Token, TriggerEvent, EventType, PlaceId, TransitionId, TokenMetadata};
use crate::{Result};

/// Event bus for publishing and subscribing to workflow events
pub struct EventBus {
    sender: broadcast::Sender<TriggerEvent>,
}

impl EventBus {
    /// Create a new event bus
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1000); // Buffer up to 1000 events
        
        Self {
            sender,
        }
    }

    /// Publish an event to all subscribers
    pub async fn publish(&self, event: TriggerEvent) -> Result<()> {
        // Send to broadcast channel (for any future subscribers)
        let _ = self.sender.send(event.clone());
        
        // TODO: Process event with function engine when implemented
        println!("Event published: {:?}", event.event_type);
        
        Ok(())
    }

    /// Subscribe to events (for future use)
    pub fn subscribe(&self) -> broadcast::Receiver<TriggerEvent> {
        self.sender.subscribe()
    }

    /// Emit a token created event
    pub async fn emit_token_created(&self, token: &Token) -> Result<()> {
        let event = TriggerEvent::token_created(
            &token.workflow_id,
            token.id,
            token.place.clone(),
            token.data.clone(),
            token.metadata.clone(),
        );
        
        self.publish(event).await
    }

    /// Emit a token transitioned event
    pub async fn emit_token_transitioned(
        &self, 
        token: &Token, 
        from_place: PlaceId, 
        transition: TransitionId
    ) -> Result<()> {
        let event = TriggerEvent::token_transitioned(
            &token.workflow_id,
            token.id,
            from_place,
            token.place.clone(),
            transition,
            token.data.clone(),
            token.metadata.clone(),
        );
        
        self.publish(event).await
    }

    /// Emit a token updated event
    pub async fn emit_token_updated(&self, token: &Token) -> Result<()> {
        let event = TriggerEvent {
            id: Uuid::new_v4(),
            event_type: EventType::TokenUpdated { place: Some(token.place.clone()) },
            workflow_id: token.workflow_id.clone(),
            token_id: Some(token.id),
            data: token.data.clone(),
            metadata: token.metadata.clone(),
            timestamp: chrono::Utc::now(),
        };
        
        self.publish(event).await
    }

    /// Emit a workflow created event
    pub async fn emit_workflow_created(&self, workflow_id: &str) -> Result<()> {
        let event = TriggerEvent {
            id: Uuid::new_v4(),
            event_type: EventType::WorkflowCreated,
            workflow_id: workflow_id.to_string(),
            token_id: None,
            data: serde_json::Value::Null,
            metadata: TokenMetadata::new(),
            timestamp: chrono::Utc::now(),
        };
        
        self.publish(event).await
    }

    /// Emit a custom event
    pub async fn emit_custom_event(
        &self,
        event_name: String,
        workflow_id: String,
        token_id: Option<Uuid>,
        data: serde_json::Value,
        metadata: TokenMetadata,
    ) -> Result<()> {
        let event = TriggerEvent {
            id: Uuid::new_v4(),
            event_type: EventType::Custom { event_name },
            workflow_id,
            token_id,
            data,
            metadata,
            timestamp: chrono::Utc::now(),
        };
        
        self.publish(event).await
    }
}

impl Clone for EventBus {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

/// Extension trait to add event emission to Token operations
pub trait TokenEvents {
    /// Create a token and emit creation event
    async fn new_with_events(
        workflow_id: &str, 
        initial_place: PlaceId, 
        event_bus: &EventBus
    ) -> Result<Token>;
    
    /// Transition to a new place and emit transition event
    async fn transition_to_with_events(
        &mut self, 
        new_place: PlaceId, 
        transition_id: TransitionId,
        event_bus: &EventBus
    ) -> Result<()>;
    
    /// Set metadata and emit update event
    async fn set_metadata_with_events<K: Into<String>>(
        &mut self, 
        key: K, 
        value: serde_json::Value,
        event_bus: &EventBus
    ) -> Result<()>;
}

impl TokenEvents for Token {
    async fn new_with_events(
        workflow_id: &str, 
        initial_place: PlaceId, 
        event_bus: &EventBus
    ) -> Result<Token> {
        let token = Token::new(workflow_id, initial_place);
        event_bus.emit_token_created(&token).await?;
        Ok(token)
    }
    
    async fn transition_to_with_events(
        &mut self, 
        new_place: PlaceId, 
        transition_id: TransitionId,
        event_bus: &EventBus
    ) -> Result<()> {
        let old_place = self.place.clone();
        self.transition_to(new_place, transition_id.clone());
        event_bus.emit_token_transitioned(self, old_place, transition_id).await?;
        Ok(())
    }
    
    async fn set_metadata_with_events<K: Into<String>>(
        &mut self, 
        key: K, 
        value: serde_json::Value,
        event_bus: &EventBus
    ) -> Result<()> {
        self.set_metadata(key, value);
        event_bus.emit_token_updated(self).await?;
        Ok(())
    }
} 