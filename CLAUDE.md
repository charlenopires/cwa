# CWA

CWA v0.7.0 - Claude Workflow Architect development

## MCP Servers

- **cwa mcp stdio**: Full MCP server (34 tools, 11 resources)
- **cwa mcp planner**: Full MCP server + DDD/SDD planning (35 tools, 11 resources)
- **cwa mcp install**: Install MCP server to Claude Desktop, Claude Code, Gemini CLI, VSCode
- **cwa mcp uninstall**: Remove MCP server from targets

Both servers conform to MCP Protocol Version 2025-06-18.

## Workflow Guidelines

**IMPORTANT:** Always update task status on the Kanban board as you work:

1. **Before starting work:** Move task to `in_progress`
   ```
   cwa task move <task-id> in_progress
   ```
   Or via MCP: `cwa_update_task_status(task_id, "in_progress")`

2. **When ready for review:** Move task to `review`
   ```
   cwa task move <task-id> review
   ```

3. **When complete:** Move task to `done`
   ```
   cwa task move <task-id> done
   ```

**Live Board:** Run `cwa serve` and open http://127.0.0.1:3030 to see real-time updates.

## DDD/SDD Methodology

The planner uses Domain-Driven Design and Specification-Driven Development:

- **Strategic Design**: Bounded contexts, subdomains (Core/Supporting/Generic)
- **Ubiquitous Language**: Domain glossary with shared vocabulary
- **Architectural Decisions**: ADRs with rationale
- **Specifications**: Source of truth with acceptance criteria

## Current Work

- Live Reload Test Task [high]

## Recent Observations

- **[DISCOVERY]** Test observation for Qdrant fix

