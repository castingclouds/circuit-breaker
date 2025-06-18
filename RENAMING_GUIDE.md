# Circuit Breaker Renaming Guide

## Overview

This guide provides a comprehensive plan for refactoring Circuit Breaker from Petri Net terminology to intuitive workflow terminology based on user feedback and testing.

## Terminology Changes

### Core Concept Mapping

| **Old Term (Petri Net)** | **New Term (Workflow)** | **Rationale** |
|--------------------------|-------------------------|---------------|
| Places | States | Users understand "states" as current status/condition |
| Tokens | Resources | Resources are the things being worked on (documents, issues, etc.) |
| Transitions | Activities | Activities are the actions that move resources between states |

### Why This Change?

User testing revealed that Petri Net terminology doesn't resonate with business users:
- ❌ "Places" - Too abstract
- ❌ "Tokens" - Mathematical concept, not business concept  
- ❌ "Transitions" - Technical term

The new terminology is intuitive:
- ✅ **Resources** flow through **States** via **Activities**
- ✅ Activities have **Rules** (conditions) and **Functions** (work to perform)
- ✅ Aligns with business process modeling that users already understand

## File-by-File Refactoring Plan

### 1. Core Model Files

#### `src/models/place.rs` → `src/models/state.rs`
- [ ] Rename file
- [ ] `struct PlaceId` → `struct StateId`
- [ ] `impl PlaceId` → `impl StateId`
- [ ] Update documentation and comments
- [ ] Update module exports

#### `src/models/token.rs` → `src/models/resource.rs`
- [ ] Rename file
- [ ] `struct Token` → `struct Resource`
- [ ] `impl Token` → `impl Resource`
- [ ] `TokenMetadata` → `ResourceMetadata`
- [ ] `TransitionRecord` → `ActivityRecord`
- [ ] Update all method names and documentation

#### `src/models/transition.rs` → `src/models/activity.rs`
- [ ] Rename file
- [ ] `struct TransitionDefinition` → `struct ActivityDefinition`
- [ ] `struct TransitionId` → `struct ActivityId`
- [ ] `struct TransitionRuleEvaluation` → `struct ActivityRuleEvaluation`
- [ ] `impl TransitionDefinition` → `impl ActivityDefinition`
- [ ] Update all documentation

#### `src/models/workflow.rs`
- [ ] `places: Vec<PlaceId>` → `states: Vec<StateId>`
- [ ] `transitions: Vec<TransitionDefinition>` → `activities: Vec<ActivityDefinition>`
- [ ] `initial_place: PlaceId` → `initial_state: StateId`
- [ ] Update all method signatures and logic
- [ ] Update documentation

### 2. Engine Files

#### `src/engine/rules.rs`
- [ ] `available_transitions()` → `available_activities()`
- [ ] `can_transition()` → `can_execute_activity()`
- [ ] `evaluate_transition_rules()` → `evaluate_activity_rules()`
- [ ] Update all parameter types and return types
- [ ] Update documentation and examples

#### `src/engine/storage.rs` and `src/engine/nats_storage.rs`
- [ ] `get_tokens_in_place()` → `get_resources_in_state()`
- [ ] `store_token()` → `store_resource()`
- [ ] `transition_token()` → `execute_activity()`
- [ ] `get_token_from_workflow()` → `get_resource_from_workflow()`
- [ ] Update all NATS subject patterns
- [ ] Update all method signatures

#### `src/engine/graphql.rs`
- [ ] `PlaceAgentConfigGQL` → `StateAgentConfigGQL`
- [ ] `TransitionGQL` → `ActivityGQL`
- [ ] `TransitionRecordGQL` → `ActivityRecordGQL`
- [ ] `TransitionDefinitionInput` → `ActivityDefinitionInput`
- [ ] `TransitionFireInput` → `ActivityExecuteInput`
- [ ] Update all GraphQL schema definitions
- [ ] Update resolver method names

### 3. API Files

#### `src/api/mcp_server.rs`
- [ ] Update all GraphQL mutations and queries
- [ ] `transitionToken` → `executeActivity`
- [ ] `tokensInPlace` → `resourcesInState`
- [ ] `triggerPlaceAgents` → `triggerStateAgents`
- [ ] Update all endpoint documentation

### 4. Agent and Configuration Files

#### `src/models/agent.rs`
- [ ] `PlaceAgentConfig` → `StateAgentConfig`
- [ ] `PlaceAgentSchedule` → `StateAgentSchedule`
- [ ] `AgentTransitionConfig` → `AgentActivityConfig`
- [ ] `place_id: PlaceId` → `state_id: StateId`
- [ ] `auto_transition: Option<TransitionId>` → `auto_activity: Option<ActivityId>`

### 5. Example Files

#### All files in `examples/rust/`
- [ ] Update variable names: `token` → `resource`, `place` → `state`, `transition` → `activity`
- [ ] Update documentation and comments
- [ ] Update GraphQL queries and mutations
- [ ] Update printed output messages

### 6. Error Types

#### `src/lib.rs`
- [ ] `InvalidTransition` → `InvalidActivity`
- [ ] `TokenNotFound` → `ResourceNotFound`
- [ ] `PlaceNotFound` → `StateNotFound`
- [ ] Update error messages to use new terminology

### 7. Documentation Files

#### Root Documentation
- [ ] `README.md` - Update all examples and explanations
- [ ] `IMPLEMENTATION_PLAN.md` - Update terminology throughout
- [ ] All `docs/*.md` files - Update terminology and examples

## Implementation Strategy

### Phase 1: Core Models (Week 1)
1. Rename and refactor core model files (`place.rs`, `token.rs`, `transition.rs`)
2. Update `workflow.rs` to use new types
3. Ensure all tests pass with new terminology

### Phase 2: Engine Layer (Week 2)
1. Update `rules.rs` with new method names and types
2. Refactor storage layers (`storage.rs`, `nats_storage.rs`)
3. Update GraphQL schema and resolvers
4. Test engine functionality

### Phase 3: API Layer (Week 3)
1. Update MCP server endpoints
2. Update GraphQL mutations and queries
3. Update agent configurations
4. Test all API endpoints

### Phase 4: Examples and Documentation (Week 4)
1. Update all example files
2. Update documentation and README
3. Update error messages and types
4. Final testing and validation

## Testing Strategy

### Unit Tests
- [ ] Update all test function names
- [ ] Update test data structures
- [ ] Update assertions and expectations
- [ ] Ensure all tests pass

### Integration Tests
- [ ] Test GraphQL API with new terminology
- [ ] Test NATS integration with new subjects
- [ ] Test agent configurations
- [ ] Test workflow execution end-to-end

### Example Validation
- [ ] Run all example files
- [ ] Validate GraphQL playground
- [ ] Test client integrations
- [ ] Validate documentation examples

## NATS Subject Pattern Changes

### Current Patterns
```
workflows.{workflow_id}.places.{place_id}.tokens
workflows.{workflow_id}.transitions.{transition_id}.events
```

### New Patterns
```
workflows.{workflow_id}.states.{state_id}.resources
workflows.{workflow_id}.activities.{activity_id}.events
```

## GraphQL Schema Changes

### Current Schema
```graphql
type Place {
  id: String!
  name: String
}

type Token {
  id: String!
  workflowId: String!
  place: String!
  data: JSON!
}

type Transition {
  id: String!
  fromPlaces: [String!]!
  toPlace: String!
}
```

### New Schema
```graphql
type State {
  id: String!
  name: String
}

type Resource {
  id: String!
  workflowId: String!
  state: String!
  data: JSON!
}

type Activity {
  id: String!
  fromStates: [String!]!
  toState: String!
}
```

## Migration Utilities

### Automated Refactoring Script
```bash
#!/bin/bash
# rename_petri_to_workflow.sh

# Rename files
mv src/models/place.rs src/models/state.rs
mv src/models/token.rs src/models/resource.rs  
mv src/models/transition.rs src/models/activity.rs

# Update imports in all files
find . -name "*.rs" -exec sed -i 's/use.*place::/use crate::models::state::/g' {} +
find . -name "*.rs" -exec sed -i 's/use.*token::/use crate::models::resource::/g' {} +
find . -name "*.rs" -exec sed -i 's/use.*transition::/use crate::models::activity::/g' {} +

# Update struct names
find . -name "*.rs" -exec sed -i 's/struct PlaceId/struct StateId/g' {} +
find . -name "*.rs" -exec sed -i 's/struct Token/struct Resource/g' {} +
find . -name "*.rs" -exec sed -i 's/struct TransitionDefinition/struct ActivityDefinition/g' {} +
```

## Validation Checklist

- [ ] All files compile without errors
- [ ] All tests pass
- [ ] GraphQL schema validates
- [ ] Examples run successfully
- [ ] Documentation is consistent
- [ ] NATS subjects are updated
- [ ] Error messages use new terminology
- [ ] Client libraries remain compatible

## Risk Mitigation

### Backward Compatibility
- Consider maintaining alias types during transition period
- Update client libraries simultaneously
- Provide migration guide for external users

### Testing Coverage
- Maintain 100% test coverage during refactoring
- Add integration tests for terminology consistency
- Validate all examples work with new terminology

### Documentation
- Update all documentation simultaneously
- Provide clear migration guide
- Update README with new terminology explanations

## Communication Plan

### Internal Team
- [ ] Share this guide with development team
- [ ] Schedule refactoring sprint planning
- [ ] Assign ownership for each phase

### External Users
- [ ] Announce terminology changes
- [ ] Provide migration timeline
- [ ] Offer support during transition

## Success Metrics

- [ ] 100% compilation success
- [ ] 100% test pass rate
- [ ] All examples working
- [ ] Documentation consistency
- [ ] User feedback improvement
- [ ] No breaking changes for existing workflows

---

This renaming will make Circuit Breaker much more accessible to business users while maintaining all the mathematical rigor and power of the underlying Petri Net implementation.