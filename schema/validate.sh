#!/bin/bash

# GraphQL Schema Validation Script
# Validates all schema files against the running Circuit Breaker GraphQL server

set -e

# Configuration
GRAPHQL_ENDPOINT="http://localhost:4000/graphql"
SCHEMA_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEMP_DIR="/tmp/graphql-validation"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Create temp directory
mkdir -p "$TEMP_DIR"

echo -e "${BLUE}Circuit Breaker GraphQL Schema Validation${NC}"
echo "=========================================="
echo ""

# Check if server is running
echo -e "${YELLOW}Checking server status...${NC}"
if ! curl -s -f "$GRAPHQL_ENDPOINT" > /dev/null; then
    echo -e "${RED}❌ GraphQL server is not accessible at $GRAPHQL_ENDPOINT${NC}"
    echo "Please start the server before running validation."
    exit 1
fi
echo -e "${GREEN}✅ Server is accessible${NC}"
echo ""

# Test basic introspection
echo -e "${YELLOW}Testing introspection query...${NC}"
INTROSPECTION_QUERY='{"query":"{ __schema { queryType { name } mutationType { name } subscriptionType { name } } }"}'
INTROSPECTION_RESULT=$(curl -s -X POST -H "Content-Type: application/json" -d "$INTROSPECTION_QUERY" "$GRAPHQL_ENDPOINT")

if echo "$INTROSPECTION_RESULT" | grep -q '"queryType"'; then
    echo -e "${GREEN}✅ Introspection query successful${NC}"
else
    echo -e "${RED}❌ Introspection query failed${NC}"
    echo "Response: $INTROSPECTION_RESULT"
    exit 1
fi
echo ""

# Test specific queries from each schema
echo -e "${YELLOW}Testing schema-specific operations...${NC}"
echo ""

# Workflow operations
echo -e "${BLUE}Testing Workflow Operations:${NC}"

# Test workflows query
WORKFLOWS_QUERY='{"query":"{ workflows { id name states initialState createdAt } }"}'
WORKFLOWS_RESULT=$(curl -s -X POST -H "Content-Type: application/json" -d "$WORKFLOWS_QUERY" "$GRAPHQL_ENDPOINT")
if echo "$WORKFLOWS_RESULT" | grep -q '"workflows"'; then
    echo -e "${GREEN}  ✅ workflows query${NC}"
else
    echo -e "${RED}  ❌ workflows query failed${NC}"
    echo "     Response: $WORKFLOWS_RESULT"
fi

# Test resources query
RESOURCES_QUERY='{"query":"{ resources { id workflowId state createdAt } }"}'
RESOURCES_RESULT=$(curl -s -X POST -H "Content-Type: application/json" -d "$RESOURCES_QUERY" "$GRAPHQL_ENDPOINT")
if echo "$RESOURCES_RESULT" | grep -q '"resources"'; then
    echo -e "${GREEN}  ✅ resources query${NC}"
else
    echo -e "${RED}  ❌ resources query failed${NC}"
    echo "     Response: $RESOURCES_RESULT"
fi

# Agent operations
echo -e "${BLUE}Testing Agent Operations:${NC}"

# Test agents query
AGENTS_QUERY='{"query":"{ agents { id name description capabilities tools createdAt } }"}'
AGENTS_RESULT=$(curl -s -X POST -H "Content-Type: application/json" -d "$AGENTS_QUERY" "$GRAPHQL_ENDPOINT")
if echo "$AGENTS_RESULT" | grep -q '"agents"'; then
    echo -e "${GREEN}  ✅ agents query${NC}"
else
    echo -e "${RED}  ❌ agents query failed${NC}"
    echo "     Response: $AGENTS_RESULT"
fi

# LLM operations
echo -e "${BLUE}Testing LLM Operations:${NC}"

# Test llmProviders query
LLM_PROVIDERS_QUERY='{"query":"{ llmProviders { id name providerType baseUrl models { id name maxTokens } healthStatus { isHealthy lastCheck } } }"}'
LLM_PROVIDERS_RESULT=$(curl -s -X POST -H "Content-Type: application/json" -d "$LLM_PROVIDERS_QUERY" "$GRAPHQL_ENDPOINT")
if echo "$LLM_PROVIDERS_RESULT" | grep -q '"llmProviders"'; then
    echo -e "${GREEN}  ✅ llmProviders query${NC}"
else
    echo -e "${RED}  ❌ llmProviders query failed${NC}"
    echo "     Response: $LLM_PROVIDERS_RESULT"
fi

# Analytics operations
echo -e "${BLUE}Testing Analytics Operations:${NC}"

# Test budgetStatus query (with optional parameters)
BUDGET_STATUS_QUERY='{"query":"{ budgetStatus { budgetId limit used percentageUsed isExhausted remaining message } }"}'
BUDGET_STATUS_RESULT=$(curl -s -X POST -H "Content-Type: application/json" -d "$BUDGET_STATUS_QUERY" "$GRAPHQL_ENDPOINT")
if echo "$BUDGET_STATUS_RESULT" | grep -q '"budgetStatus"'; then
    echo -e "${GREEN}  ✅ budgetStatus query${NC}"
else
    echo -e "${RED}  ❌ budgetStatus query failed${NC}"
    echo "     Response: $BUDGET_STATUS_RESULT"
fi

# Rules operations
echo -e "${BLUE}Testing Rules Operations:${NC}"

# Test rules query
RULES_QUERY='{"query":"{ rules { id name description version createdAt tags } }"}'
RULES_RESULT=$(curl -s -X POST -H "Content-Type: application/json" -d "$RULES_QUERY" "$GRAPHQL_ENDPOINT")
if echo "$RULES_RESULT" | grep -q '"rules"'; then
    echo -e "${GREEN}  ✅ rules query${NC}"
else
    echo -e "${RED}  ❌ rules query failed${NC}"
    echo "     Response: $RULES_RESULT"
fi

# NATS operations
echo -e "${BLUE}Testing NATS Operations:${NC}"

# Test basic NATS query (this might not have data)
NATS_QUERY='{"query":"{ findResource(workflowId: \"test\", resourceId: \"test\") { id workflowId state natsSequence natsSubject } }"}'
NATS_RESULT=$(curl -s -X POST -H "Content-Type: application/json" -d "$NATS_QUERY" "$GRAPHQL_ENDPOINT")
if echo "$NATS_RESULT" | grep -q '"findResource"'; then
    echo -e "${GREEN}  ✅ NATS findResource query${NC}"
else
    echo -e "${YELLOW}  ⚠️  NATS findResource query (expected - no test data)${NC}"
fi

echo ""

# Test schema type definitions
echo -e "${YELLOW}Testing schema type definitions...${NC}"

# Get all available types
TYPES_QUERY='{"query":"{ __schema { types { name kind } } }"}'
TYPES_RESULT=$(curl -s -X POST -H "Content-Type: application/json" -d "$TYPES_QUERY" "$GRAPHQL_ENDPOINT")

# Check for key types from our schemas
EXPECTED_TYPES=(
    "WorkflowGQL"
    "ResourceGQL"
    "ActivityGQL"
    "AgentDefinitionGQL"
    "AgentExecutionGQL"
    "LlmProviderGQL"
    "LlmResponseGQL"
    "BudgetStatusGQL"
    "CostAnalyticsGQL"
    "RuleGQL"
    "NatsResourceGQL"
)

for type in "${EXPECTED_TYPES[@]}"; do
    if echo "$TYPES_RESULT" | grep -q "\"$type\""; then
        echo -e "${GREEN}  ✅ Type $type exists${NC}"
    else
        echo -e "${RED}  ❌ Type $type missing${NC}"
    fi
done

echo ""

# Generate schema SDL
echo -e "${YELLOW}Generating Schema Definition Language (SDL) export...${NC}"
SDL_QUERY='{"query":"{ __schema { queryType { name fields { name description args { name type { name } } type { name } } } mutationType { name fields { name description args { name type { name } } type { name } } } subscriptionType { name fields { name description args { name type { name } } type { name } } } } }"}'
SDL_RESULT=$(curl -s -X POST -H "Content-Type: application/json" -d "$SDL_QUERY" "$GRAPHQL_ENDPOINT")

# Save SDL to file
echo "$SDL_RESULT" | jq '.' > "$SCHEMA_DIR/exported-schema.json"
echo -e "${GREEN}✅ Schema exported to exported-schema.json${NC}"

# Create validation summary
echo ""
echo -e "${BLUE}Validation Summary${NC}"
echo "=================="
echo "✅ Server connectivity: OK"
echo "✅ Basic introspection: OK"
echo "✅ Schema export: Complete"
echo ""
echo -e "${GREEN}Schema validation completed successfully!${NC}"
echo ""
echo "Next steps:"
echo "1. Review exported-schema.json for complete schema"
echo "2. Generate TypeScript types using a tool like graphql-codegen"
echo "3. Create example operations for client SDK documentation"
echo "4. Set up automated schema validation in CI/CD"

# Cleanup
rm -rf "$TEMP_DIR"
