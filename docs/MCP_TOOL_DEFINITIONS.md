# MCP Tool Definitions and API Integration Patterns

## Overview

This document defines the specific MCP (Model Context Protocol) tools provided by Circuit Breaker's secure MCP server, including tool schemas, usage patterns, and integration examples for external APIs like GitLab, GitHub, and other services.

## Core MCP Tools

### 1. Workflow Management Tools

#### create_workflow

Creates a new workflow definition in the Circuit Breaker engine.

```json
{
  "name": "create_workflow",
  "description": "Create a new workflow with places, transitions, and agents",
  "inputSchema": {
    "type": "object",
    "properties": {
      "name": {
        "type": "string",
        "description": "Human-readable name for the workflow"
      },
      "description": {
        "type": "string",
        "description": "Optional description of the workflow purpose"
      },
      "places": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "id": {"type": "string"},
            "name": {"type": "string"},
            "description": {"type": "string"}
          },
          "required": ["id", "name"]
        },
        "description": "List of workflow states/places"
      },
      "transitions": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "id": {"type": "string"},
            "from_places": {"type": "array", "items": {"type": "string"}},
            "to_place": {"type": "string"},
            "conditions": {"type": "array", "items": {"type": "string"}},
            "agent_execution": {
              "type": "object",
              "properties": {
                "agent_id": {"type": "string"},
                "input_mapping": {"type": "object"},
                "output_mapping": {"type": "object"}
              }
            }
          },
          "required": ["id", "from_places", "to_place"]
        }
      },
      "initial_place": {
        "type": "string",
        "description": "Starting place for new workflow instances"
      },
      "tags": {
        "type": "array",
        "items": {"type": "string"},
        "description": "Tags for workflow categorization"
      }
    },
    "required": ["name", "places", "transitions", "initial_place"]
  }
}
```

#### execute_workflow

Creates and executes a workflow instance with initial data.

```json
{
  "name": "execute_workflow",
  "description": "Create and execute a workflow instance",
  "inputSchema": {
    "type": "object",
    "properties": {
      "workflow_id": {
        "type": "string",
        "description": "ID of the workflow to execute"
      },
      "initial_data": {
        "type": "object",
        "description": "Initial data for the workflow token"
      },
      "metadata": {
        "type": "object",
        "description": "Additional metadata for the workflow instance"
      },
      "priority": {
        "type": "string",
        "enum": ["low", "normal", "high", "urgent"],
        "default": "normal"
      }
    },
    "required": ["workflow_id"]
  }
}
```

#### get_workflow_status

Retrieves the current status of a workflow instance.

```json
{
  "name": "get_workflow_status",
  "description": "Get current status and state of a workflow instance",
  "inputSchema": {
    "type": "object",
    "properties": {
      "instance_id": {
        "type": "string",
        "description": "Workflow instance ID"
      },
      "include_history": {
        "type": "boolean",
        "default": false,
        "description": "Include transition history in response"
      }
    },
    "required": ["instance_id"]
  }
}
```

### 2. Agent Execution Tools

#### execute_agent

Executes an AI agent with specified input data.

```json
{
  "name": "execute_agent",
  "description": "Execute an AI agent with input data and return results",
  "inputSchema": {
    "type": "object",
    "properties": {
      "agent_id": {
        "type": "string",
        "description": "ID of the agent to execute"
      },
      "input_data": {
        "type": "object",
        "description": "Input data for the agent"
      },
      "provider_override": {
        "type": "string",
        "enum": ["openai", "anthropic", "google", "ollama"],
        "description": "Override default LLM provider for this execution"
      },
      "model_override": {
        "type": "string",
        "description": "Override default model for this execution"
      },
      "temperature": {
        "type": "number",
        "minimum": 0,
        "maximum": 2,
        "description": "Temperature for LLM generation"
      },
      "max_tokens": {
        "type": "integer",
        "minimum": 1,
        "description": "Maximum tokens to generate"
      },
      "stream": {
        "type": "boolean",
        "default": false,
        "description": "Enable streaming response"
      }
    },
    "required": ["agent_id", "input_data"]
  }
}
```

#### list_agents

Lists available agents with their capabilities and configurations.

```json
{
  "name": "list_agents",
  "description": "List available AI agents and their capabilities",
  "inputSchema": {
    "type": "object",
    "properties": {
      "filter_by_capability": {
        "type": "array",
        "items": {"type": "string"},
        "description": "Filter agents by specific capabilities"
      },
      "include_configuration": {
        "type": "boolean",
        "default": false,
        "description": "Include agent configuration details"
      }
    }
  }
}
```

### 3. Project Context Management Tools

#### create_project_context

Creates a new project context for scoped operations within specific repositories or projects.

```json
{
  "name": "create_project_context",
  "description": "Create a project context for scoped AI operations",
  "inputSchema": {
    "type": "object",
    "properties": {
      "name": {
        "type": "string",
        "description": "Human-readable name for the project context"
      },
      "description": {
        "type": "string",
        "description": "Optional description of the project context"
      },
      "context_type": {
        "type": "object",
        "oneOf": [
          {
            "properties": {
              "GitLab": {
                "type": "object",
                "properties": {
                  "project_id": {"type": "integer"},
                  "namespace": {"type": "string"}
                },
                "required": ["project_id", "namespace"]
              }
            }
          },
          {
            "properties": {
              "GitHub": {
                "type": "object",
                "properties": {
                  "owner": {"type": "string"},
                  "repo": {"type": "string"}
                },
                "required": ["owner", "repo"]
              }
            }
          },
          {
            "properties": {
              "Combined": {
                "type": "object",
                "properties": {
                  "contexts": {
                    "type": "array",
                    "items": {"type": "string"}
                  }
                },
                "required": ["contexts"]
              }
            }
          }
        ]
      },
      "configuration": {
        "type": "object",
        "properties": {
          "default_branch": {"type": "string", "default": "main"},
          "include_patterns": {
            "type": "array",
            "items": {"type": "string"},
            "description": "File patterns to include (e.g., 'src/**/*.ts')"
          },
          "exclude_patterns": {
            "type": "array",
            "items": {"type": "string"},
            "description": "File patterns to exclude (e.g., 'node_modules/**')"
          },
          "max_depth": {"type": "integer", "minimum": 1, "default": 10},
          "cache_duration_hours": {"type": "integer", "minimum": 1, "default": 24}
        }
      }
    },
    "required": ["name", "context_type"]
  }
}
```

#### search_in_context

Searches within a specific project context for code, issues, or other content.

```json
{
  "name": "search_in_context",
  "description": "Search within a project context for relevant information",
  "inputSchema": {
    "type": "object",
    "properties": {
      "context_id": {
        "type": "string",
        "description": "Project context ID to search within"
      },
      "query": {
        "type": "string",
        "description": "Search query string"
      },
      "search_type": {
        "type": "string",
        "enum": ["code", "issues", "merge_requests", "pull_requests", "all"],
        "default": "all",
        "description": "Type of content to search"
      },
      "filters": {
        "type": "object",
        "properties": {
          "file_extension": {"type": "string"},
          "path": {"type": "string"},
          "author": {"type": "string"},
          "date_range": {
            "type": "object",
            "properties": {
              "from": {"type": "string", "format": "date"},
              "to": {"type": "string", "format": "date"}
            }
          }
        }
      },
      "limit": {
        "type": "integer",
        "minimum": 1,
        "maximum": 100,
        "default": 20
      }
    },
    "required": ["context_id", "query"]
  }
}
```

#### get_context_file

Retrieves the content of a specific file within a project context.

```json
{
  "name": "get_context_file",
  "description": "Get file content from within a project context",
  "inputSchema": {
    "type": "object",
    "properties": {
      "context_id": {
        "type": "string",
        "description": "Project context ID"
      },
      "file_path": {
        "type": "string",
        "description": "Path to the file within the project"
      },
      "ref_name": {
        "type": "string",
        "description": "Branch, tag, or commit reference (defaults to default branch)"
      }
    },
    "required": ["context_id", "file_path"]
  }
}
```

#### list_context_files

Lists files within a project context based on patterns and filters.

```json
{
  "name": "list_context_files",
  "description": "List files within a project context",
  "inputSchema": {
    "type": "object",
    "properties": {
      "context_id": {
        "type": "string",
        "description": "Project context ID"
      },
      "path": {
        "type": "string",
        "description": "Directory path to list (defaults to root)"
      },
      "recursive": {
        "type": "boolean",
        "default": true,
        "description": "Whether to list files recursively"
      },
      "include_patterns": {
        "type": "array",
        "items": {"type": "string"},
        "description": "File patterns to include"
      },
      "exclude_patterns": {
        "type": "array",
        "items": {"type": "string"},
        "description": "File patterns to exclude"
      },
      "max_depth": {
        "type": "integer",
        "minimum": 1,
        "description": "Maximum directory depth to traverse"
      }
    },
    "required": ["context_id"]
  }
}
```

### 4. External API Integration Tools

#### call_external_api

Makes authenticated calls to external APIs with proper scoping and rate limiting.

```json
{
  "name": "call_external_api",
  "description": "Make authenticated calls to external APIs",
  "inputSchema": {
    "type": "object",
    "properties": {
      "service": {
        "type": "string",
        "enum": ["gitlab", "github", "stripe", "slack", "discord", "custom"],
        "description": "External service to call"
      },
      "endpoint": {
        "type": "string",
        "description": "API endpoint path (without base URL)"
      },
      "method": {
        "type": "string",
        "enum": ["GET", "POST", "PUT", "PATCH", "DELETE"],
        "default": "GET"
      },
      "headers": {
        "type": "object",
        "description": "Additional headers to include"
      },
      "query_params": {
        "type": "object",
        "description": "Query parameters"
      },
      "body": {
        "type": "object",
        "description": "Request body for POST/PUT/PATCH requests"
      },
      "timeout_seconds": {
        "type": "integer",
        "minimum": 1,
        "maximum": 300,
        "default": 30
      }
    },
    "required": ["service", "endpoint"]
  }
}
```

## GitLab-Specific Tools

### gitlab_list_projects

Lists GitLab projects accessible to the authenticated user.

```json
{
  "name": "gitlab_list_projects",
  "description": "List GitLab projects with optional filtering",
  "inputSchema": {
    "type": "object",
    "properties": {
      "visibility": {
        "type": "string",
        "enum": ["private", "internal", "public"],
        "description": "Filter by project visibility"
      },
      "owned": {
        "type": "boolean",
        "description": "Only show owned projects"
      },
      "starred": {
        "type": "boolean",
        "description": "Only show starred projects"
      },
      "search": {
        "type": "string",
        "description": "Search term for project names"
      },
      "order_by": {
        "type": "string",
        "enum": ["id", "name", "path", "created_at", "updated_at", "last_activity_at"],
        "default": "created_at"
      },
      "sort": {
        "type": "string",
        "enum": ["asc", "desc"],
        "default": "desc"
      },
      "per_page": {
        "type": "integer",
        "minimum": 1,
        "maximum": 100,
        "default": 20
      }
    }
  }
}
```

### gitlab_create_issue

Creates a new issue in a GitLab project.

```json
{
  "name": "gitlab_create_issue",
  "description": "Create a new issue in a GitLab project",
  "inputSchema": {
    "type": "object",
    "properties": {
      "project_id": {
        "type": "integer",
        "description": "GitLab project ID"
      },
      "title": {
        "type": "string",
        "description": "Issue title"
      },
      "description": {
        "type": "string",
        "description": "Issue description (supports Markdown)"
      },
      "labels": {
        "type": "array",
        "items": {"type": "string"},
        "description": "Labels to apply to the issue"
      },
      "assignee_ids": {
        "type": "array",
        "items": {"type": "integer"},
        "description": "User IDs to assign the issue to"
      },
      "milestone_id": {
        "type": "integer",
        "description": "Milestone ID to associate with the issue"
      },
      "due_date": {
        "type": "string",
        "format": "date",
        "description": "Due date for the issue (YYYY-MM-DD)"
      },
      "weight": {
        "type": "integer",
        "minimum": 0,
        "description": "Issue weight (if enabled)"
      },
      "confidential": {
        "type": "boolean",
        "default": false,
        "description": "Mark issue as confidential"
      }
    },
    "required": ["project_id", "title"]
  }
}
```

### gitlab_create_merge_request

Creates a new merge request in a GitLab project.

```json
{
  "name": "gitlab_create_merge_request",
  "description": "Create a new merge request in a GitLab project",
  "inputSchema": {
    "type": "object",
    "properties": {
      "project_id": {
        "type": "integer",
        "description": "GitLab project ID"
      },
      "source_branch": {
        "type": "string",
        "description": "Source branch for the merge request"
      },
      "target_branch": {
        "type": "string",
        "description": "Target branch for the merge request"
      },
      "title": {
        "type": "string",
        "description": "Merge request title"
      },
      "description": {
        "type": "string",
        "description": "Merge request description (supports Markdown)"
      },
      "assignee_ids": {
        "type": "array",
        "items": {"type": "integer"},
        "description": "User IDs to assign the merge request to"
      },
      "reviewer_ids": {
        "type": "array",
        "items": {"type": "integer"},
        "description": "User IDs to request review from"
      },
      "labels": {
        "type": "array",
        "items": {"type": "string"},
        "description": "Labels to apply to the merge request"
      },
      "milestone_id": {
        "type": "integer",
        "description": "Milestone ID to associate with the merge request"
      },
      "remove_source_branch": {
        "type": "boolean",
        "default": false,
        "description": "Remove source branch when merge request is accepted"
      },
      "squash": {
        "type": "boolean",
        "default": false,
        "description": "Squash commits when merging"
      }
    },
    "required": ["project_id", "source_branch", "target_branch", "title"]
  }
}
```

### gitlab_add_comment

Adds a comment to a GitLab issue or merge request.

```json
{
  "name": "gitlab_add_comment",
  "description": "Add a comment to a GitLab issue or merge request",
  "inputSchema": {
    "type": "object",
    "properties": {
      "project_id": {
        "type": "integer",
        "description": "GitLab project ID"
      },
      "issue_iid": {
        "type": "integer",
        "description": "Issue internal ID (for issues)"
      },
      "merge_request_iid": {
        "type": "integer",
        "description": "Merge request internal ID (for merge requests)"
      },
      "body": {
        "type": "string",
        "description": "Comment body (supports Markdown)"
      },
      "confidential": {
        "type": "boolean",
        "default": false,
        "description": "Mark comment as confidential (issues only)"
      }
    },
    "required": ["project_id", "body"],
    "oneOf": [
      {"required": ["issue_iid"]},
      {"required": ["merge_request_iid"]}
    ]
  }
}
```

## GitHub-Specific Tools

### github_list_repositories

Lists GitHub repositories accessible to the authenticated user.

```json
{
  "name": "github_list_repositories",
  "description": "List GitHub repositories with optional filtering",
  "inputSchema": {
    "type": "object",
    "properties": {
      "visibility": {
        "type": "string",
        "enum": ["all", "public", "private"],
        "default": "all"
      },
      "affiliation": {
        "type": "string",
        "enum": ["owner", "collaborator", "organization_member"],
        "description": "Filter by user's affiliation"
      },
      "type": {
        "type": "string",
        "enum": ["all", "owner", "public", "private", "member"],
        "default": "all"
      },
      "sort": {
        "type": "string",
        "enum": ["created", "updated", "pushed", "full_name"],
        "default": "full_name"
      },
      "direction": {
        "type": "string",
        "enum": ["asc", "desc"],
        "default": "asc"
      },
      "per_page": {
        "type": "integer",
        "minimum": 1,
        "maximum": 100,
        "default": 30
      }
    }
  }
}
```

### github_create_issue

Creates a new issue in a GitHub repository.

```json
{
  "name": "github_create_issue",
  "description": "Create a new issue in a GitHub repository",
  "inputSchema": {
    "type": "object",
    "properties": {
      "owner": {
        "type": "string",
        "description": "Repository owner username or organization"
      },
      "repo": {
        "type": "string",
        "description": "Repository name"
      },
      "title": {
        "type": "string",
        "description": "Issue title"
      },
      "body": {
        "type": "string",
        "description": "Issue body (supports Markdown)"
      },
      "assignees": {
        "type": "array",
        "items": {"type": "string"},
        "description": "Usernames to assign the issue to"
      },
      "labels": {
        "type": "array",
        "items": {"type": "string"},
        "description": "Labels to apply to the issue"
      },
      "milestone": {
        "type": "integer",
        "description": "Milestone number to associate with the issue"
      }
    },
    "required": ["owner", "repo", "title"]
  }
}
```

### github_create_pull_request

Creates a new pull request in a GitHub repository.

```json
{
  "name": "github_create_pull_request",
  "description": "Create a new pull request in a GitHub repository",
  "inputSchema": {
    "type": "object",
    "properties": {
      "owner": {
        "type": "string",
        "description": "Repository owner username or organization"
      },
      "repo": {
        "type": "string",
        "description": "Repository name"
      },
      "title": {
        "type": "string",
        "description": "Pull request title"
      },
      "head": {
        "type": "string",
        "description": "Branch containing changes (source branch)"
      },
      "base": {
        "type": "string",
        "description": "Branch to merge changes into (target branch)"
      },
      "body": {
        "type": "string",
        "description": "Pull request body (supports Markdown)"
      },
      "draft": {
        "type": "boolean",
        "default": false,
        "description": "Create as draft pull request"
      },
      "maintainer_can_modify": {
        "type": "boolean",
        "default": true,
        "description": "Allow maintainers to modify the pull request"
      }
    },
    "required": ["owner", "repo", "title", "head", "base"]
  }
}
```

## Function Execution Tools

### execute_function

Executes a containerized function with specified input data.

```json
{
  "name": "execute_function",
  "description": "Execute a containerized function",
  "inputSchema": {
    "type": "object",
    "properties": {
      "function_id": {
        "type": "string",
        "description": "ID of the function to execute"
      },
      "input_data": {
        "type": "object",
        "description": "Input data for the function"
      },
      "environment_vars": {
        "type": "object",
        "description": "Additional environment variables"
      },
      "timeout_seconds": {
        "type": "integer",
        "minimum": 1,
        "maximum": 3600,
        "default": 300,
        "description": "Function execution timeout"
      },
      "memory_limit_mb": {
        "type": "integer",
        "minimum": 64,
        "maximum": 4096,
        "default": 512,
        "description": "Memory limit for function execution"
      }
    },
    "required": ["function_id", "input_data"]
  }
}
```

## Usage Examples

### Creating a Project Context for Focused Analysis

```json
{
  "tool": "create_project_context",
  "arguments": {
    "name": "MyOrg Main Project",
    "description": "Primary application repository with focused AI analysis scope",
    "context_type": {
      "GitLab": {
        "project_id": 123,
        "namespace": "myorg"
      }
    },
    "configuration": {
      "default_branch": "main",
      "include_patterns": [
        "src/**/*.ts",
        "src/**/*.js",
        "docs/**/*.md",
        "*.json",
        "*.yml"
      ],
      "exclude_patterns": [
        "node_modules/**",
        "dist/**",
        "*.log",
        "coverage/**"
      ],
      "max_depth": 15,
      "cache_duration_hours": 12
    }
  }
}
```

### Creating a GitLab Issue Analysis Workflow with Project Context

```json
{
  "tool": "create_workflow",
  "arguments": {
    "name": "gitlab_issue_analyzer_with_context",
    "description": "Analyzes GitLab issues with full project context awareness",
    "places": [
      {"id": "received", "name": "Issue Received"},
      {"id": "context_gathering", "name": "Gathering Project Context"},
      {"id": "analyzing", "name": "AI Analysis with Context"},
      {"id": "categorized", "name": "Categorized"},
      {"id": "responded", "name": "Response Added"}
    ],
    "transitions": [
      {
        "id": "gather_context",
        "from_places": ["received"],
        "to_place": "context_gathering",
        "agent_execution": {
          "agent_id": "context_gatherer",
          "input_mapping": {
            "issue_title": "data.title",
            "issue_description": "data.description",
            "project_context_id": "data.context_id"
          }
        }
      },
      {
        "id": "start_analysis",
        "from_places": ["context_gathering"],
        "to_place": "analyzing",
        "agent_execution": {
          "agent_id": "issue_classifier_with_context",
          "input_mapping": {
            "issue_title": "data.title",
            "issue_description": "data.description",
            "issue_labels": "data.labels",
            "project_context": "data.project_context",
            "related_files": "data.related_files",
            "similar_issues": "data.similar_issues"
          },
          "output_mapping": {
            "data.category": "category",
            "data.priority": "priority",
            "data.ai_summary": "summary",
            "data.affected_components": "affected_components",
            "data.suggested_assignees": "suggested_assignees"
          }
        }
      },
      {
        "id": "categorize",
        "from_places": ["analyzing"],
        "to_place": "categorized",
        "conditions": ["ai_analysis_complete"]
      },
      {
        "id": "add_response",
        "from_places": ["categorized"],
        "to_place": "responded",
        "agent_execution": {
          "agent_id": "response_generator_with_context",
          "input_mapping": {
            "analysis": "data.ai_summary",
            "category": "data.category",
            "priority": "data.priority",
            "affected_components": "data.affected_components",
            "project_context": "data.project_context"
          }
        }
      }
    ],
    "initial_place": "received",
    "tags": ["gitlab", "issue-processing", "ai-analysis", "context-aware"]
  }
}
```

### Processing a GitLab Issue with Project Context and AI Analysis

```json
{
  "tool": "execute_workflow",
  "arguments": {
    "workflow_id": "gitlab_issue_analyzer_with_context",
    "initial_data": {
      "project_id": 123,
      "issue_iid": 456,
      "context_id": "myorg_main_project",
      "title": "Performance issue in dashboard loading",
      "description": "The main dashboard takes over 30 seconds to load, affecting user experience. Users report timeout errors and slow API responses.",
      "labels": ["performance", "frontend"],
      "reporter": "user@example.com"
    },
    "metadata": {
      "source": "gitlab_webhook",
      "webhook_id": "wh_12345",
      "created_by": "gitlab_integration"
    },
    "priority": "high"
  }
}
```

### Searching for Related Code in Project Context

```json
{
  "tool": "search_in_context",
  "arguments": {
    "context_id": "myorg_main_project",
    "query": "dashboard performance loading timeout",
    "search_type": "code",
    "filters": {
      "file_extension": "ts",
      "path": "src/"
    },
    "limit": 15
  }
}
```

### Analyzing Specific Files from Context

```json
{
  "tool": "get_context_file",
  "arguments": {
    "context_id": "myorg_main_project",
    "file_path": "src/components/Dashboard.tsx",
    "ref_name": "main"
  }
}
```

### Adding AI-Generated Comment to GitLab Issue

```json
{
  "tool": "gitlab_add_comment",
  "arguments": {
    "project_id": 123,
    "issue_iid": 456,
    "body": "## AI Analysis Results\n\n**Category:** Performance - Frontend\n**Priority:** High\n**Summary:** This issue appears to be related to dashboard loading performance. \n\n**Recommended Actions:**\n1. Profile frontend bundle size\n2. Analyze database queries for dashboard data\n3. Consider implementing lazy loading\n4. Review caching strategies\n\n**Estimated Effort:** 2-3 days\n**Suggested Assignee:** @frontend-team"
  }
}
```

### Creating GitHub Issue from Analysis

```json
{
  "tool": "github_create_issue",
  "arguments": {
    "owner": "myorg",
    "repo": "myproject",
    "title": "Dashboard Performance Optimization",
    "body": "Based on user feedback and AI analysis, we need to optimize dashboard loading performance.\n\n## Problem\nDashboard takes 30+ seconds to load, significantly impacting user experience.\n\n## Proposed Solution\n- Analyze and optimize frontend bundle\n- Review database query performance\n- Implement lazy loading for non-critical components\n- Enhance caching strategies\n\n## Acceptance Criteria\n- [ ] Dashboard loads in under 5 seconds\n- [ ] Bundle size reduced by at least 20%\n- [ ] Database queries optimized\n- [ ] Lazy loading implemented",
    "labels": ["performance", "frontend", "high-priority"],
    "assignees": ["frontend-lead"]
  }
}
```

### Executing AI Agent for Context-Aware Code Review

```json
{
  "tool": "execute_agent",
  "arguments": {
    "agent_id": "context_aware_code_reviewer",
    "input_data": {
      "context_id": "myorg_main_project",
      "pull_request_id": 789,
      "files_changed": [
        "src/components/Dashboard.tsx",
        "src/api/dashboard.ts",
        "src/utils/cache.ts"
      ],
      "diff_content": "... diff content here ...",
      "language": "typescript",
      "project_context": {
        "related_files": ["src/hooks/useDashboard.ts", "src/types/dashboard.ts"],
        "architectural_patterns": ["React hooks", "TypeScript", "REST API"],
        "performance_requirements": "< 3s load time",
        "recent_issues": ["Performance degradation", "Memory leaks"]
      }
    },
    "provider_override": "anthropic",
    "model_override": "claude-3-sonnet",
    "temperature": 0.2,
    "stream": true
  }
}
```

### Multi-Context Agent Coordination

```json
{
  "tool": "execute_agent",
  "arguments": {
    "agent_id": "cross_project_analyzer",
    "input_data": {
      "primary_context_id": "myorg_main_project",
      "secondary_contexts": ["myorg_api_service", "myorg_shared_components"],
      "analysis_type": "dependency_impact",
      "change_description": "Updating authentication middleware",
      "affected_files": [
        "src/middleware/auth.ts",
        "src/types/user.ts"
      ]
    },
    "provider_override": "anthropic",
    "model_override": "claude-3-opus",
    "temperature": 0.1
  }
}
```

## Tool Security and Permissions

### Permission Scoping Example

```json
{
  "installation_permissions": {
    "workflows": {
      "create": true,
      "read": ["gitlab_*", "github_*"],
      "update": ["gitlab_*"],
      "delete": false
    },
    "agents": {
      "execute": ["issue_classifier", "response_generator", "code_reviewer"],
      "configure": false
    },
    "external_apis": {
      "gitlab": {
        "scopes": ["api", "read_user"],
        "endpoints": [
          "/api/v4/projects",
          "/api/v4/projects/*/issues",
          "/api/v4/projects/*/merge_requests",
          "/api/v4/projects/*/issues/*/notes",
          "/api/v4/projects/*/repository/files/*",
          "/api/v4/projects/*/search"
        ],
        "rate_limit": {
          "requests_per_minute": 300,
          "burst_allowance": 50
        }
      },
      "github": {
        "scopes": ["repo", "issues", "pull_requests"],
        "endpoints": [
          "/user/repos",
          "/repos/*/*/issues",
          "/repos/*/*/pulls",
          "/repos/*/*/contents/*",
          "/search/code"
        ],
        "rate_limit": {
          "requests_per_minute": 200,
          "burst_allowance": 30
        }
      }
    },
    "project_contexts": [
      {
        "context_id": "myorg_main_project",
        "context_type": "GitLab",
        "permissions": {
          "read": true,
          "write": false,
          "admin": false,
          "allowed_operations": ["search", "get_file", "list_files"]
        },
        "resource_limits": {
          "max_file_size_mb": 10,
          "max_search_results": 50,
          "rate_limit_per_hour": 1000
        }
      },
      {
        "context_id": "combined_microservices",
        "context_type": "Combined",
        "permissions": {
          "read": true,
          "write": false,
          "admin": false,
          "allowed_operations": ["search", "get_file"]
        },
        "resource_limits": {
          "max_search_results": 100,
          "rate_limit_per_hour": 500
        }
      }
    ]
  }
}
```

### Rate Limiting Configuration

```json
{
  "rate_limits": {
    "global": {
      "requests_per_minute": 1000,
      "burst_allowance": 100
    },
    "per_tool": {
      "execute_agent": {
        "requests_per_minute": 60,
        "burst_allowance": 10
      },
      "call_external_api": {
        "requests_per_minute": 300,
        "burst_allowance": 50
      },
      "execute_function": {
        "requests_per_minute": 120,
        "burst_allowance": 20
      },
      "search_in_context": {
        "requests_per_minute": 100,
        "burst_allowance": 20
      },
      "get_context_file": {
        "requests_per_minute": 200,
        "burst_allowance": 40
      }
    },
    "per_service": {
      "gitlab": {
        "requests_per_minute": 300,
        "burst_allowance": 50
      },
      "github": {
        "requests_per_minute": 200,
        "burst_allowance": 30
      }
    }
  }
}
```

## Error Handling and Response Formats

### Standard Success Response

```json
{
  "jsonrpc": "2.0",
  "id": "req_12345",
  "result": {
    "success": true,
    "data": {
      "workflow_id": "wf_abc123",
      "instance_id": "inst_def456",
      "status": "created"
    },
    "metadata": {
      "execution_time_ms": 145,
      "tokens_used": 0,
      "api_calls_made": 1
    }
  }
}
```

### Standard Error Response

```json
{
  "jsonrpc": "2.0",
  "id": "req_12345",
  "error": {
    "code": -32001,
    "message": "Insufficient permissions",
    "data": {
      "error_type": "authorization_error",
      "required_permission": "workflows.create",
      "current_permissions": ["workflows.read"],
      "installation_id": "inst_789",
      "app_id": "app_456"
    }
  }
}
```

### Streaming Response Format

```json
{
  "jsonrpc": "2.0",
  "id": "req_12345",
  "result": {
    "stream_id": "stream_abc123",
    "stream_url": "wss://circuit-breaker.example.com/streams/stream_abc123",
    "initial_data": {
      "execution_id": "exec_def456",
      "status": "started"
    }
  }
}
```

## Integration Patterns

### Webhook-Triggered Workflow Pattern

1. **Webhook Reception**: External service sends webhook to Circuit Breaker
2. **Event Processing**: Webhook processor creates workflow instance
3. **AI Analysis**: Workflow executes AI agents for analysis
4. **Action Execution**: Based on analysis, take actions via MCP tools
5. **Response Generation**: Generate appropriate responses back to external service

### Multi-Service Coordination Pattern with Project Context

1. **GitLab Issue Created**: Webhook triggers workflow
2. **Context Gathering**: Search related code, issues, and documentation within project context
3. **AI Analysis**: Classify and prioritize issue with full project understanding
4. **Cross-Project Impact**: Analyze dependencies across multiple project contexts
5. **GitHub Issue Creation**: Create corresponding tracking issue with context-aware details
6. **Slack Notification**: Notify relevant team members with intelligent assignee suggestions
7. **Project Management**: Update project management tools with detailed analysis

### Context-Aware Agent Swarm Pattern

1. **Context Discovery**: Multiple agents scan different project contexts
2. **Information Gathering**: Agents collect relevant code, issues, and documentation
3. **Analysis Coordination**: Agents share findings and coordinate analysis
4. **Impact Assessment**: Cross-context agents analyze dependencies and relationships
5. **Solution Generation**: Agents collaborate on comprehensive solutions
6. **Implementation Planning**: Context-aware agents suggest implementation strategies

### Continuous Integration Pattern

1. **Code Push**: Git push triggers webhook
2. **Code Analysis**: AI agent reviews code changes
3. **Test Execution**: Run automated tests via function execution
4. **Quality Gates**: Apply quality rules and checks
5. **Deployment Decision**: Automatic or manual deployment based on results

This comprehensive tool definition system enables powerful automation workflows while maintaining security, auditability, and fine-grained permission controls across all external service integrations. The project context system ensures AI agents operate efficiently within defined boundaries while enabling sophisticated cross-project coordination and analysis capabilities.

## Project Context Benefits

### Focused AI Operations
- **Scoped Search**: AI agents search only within relevant project boundaries
- **Contextual Understanding**: Agents understand project structure, patterns, and conventions
- **Efficient Resource Usage**: Prevent unnecessary broad searches across all repositories
- **Intelligent Caching**: Project structure and content cached for rapid access

### Cross-Project Intelligence
- **Dependency Analysis**: Understand relationships between multiple projects
- **Impact Assessment**: Analyze how changes in one project affect others
- **Coordinated Workflows**: Agents work across related projects simultaneously
- **Unified Context**: Combined contexts provide holistic view of complex systems

### Agent Swarm Coordination
- **Bounded Operations**: Each agent works within defined project contexts
- **Efficient Collaboration**: Agents share context-specific insights
- **Scalable Architecture**: Handle multiple teams and projects simultaneously
- **Resource Optimization**: Context boundaries prevent resource waste and conflicts

This architecture enables truly intelligent AI agent swarms that understand project boundaries, relationships, and context while maintaining security and operational efficiency.