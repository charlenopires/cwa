# PROJECT.md - Hypothetical Use Case: TaskFlow

## Overview

This document demonstrates how to use **CWA (Claude Workflow Architect)** to build a hypothetical project management SaaS application called **TaskFlow**.

TaskFlow is a collaborative task management platform for remote teams, featuring real-time updates, time tracking, and team analytics.

---

## Phase 1: Project Initialization

### 1.1 Initialize the CWA Project

```bash
# Create project directory
mkdir taskflow
cd taskflow

# Initialize CWA
cwa init taskflow
```

This creates:
- `.cwa/cwa.db` - SQLite database for project metadata
- `.claude/` - Claude Code integration files
- `docs/constitution.md` - Project values and constraints
- `CLAUDE.md` - Auto-generated context file

### 1.2 Define Project Constitution

Edit `docs/constitution.md` to establish core values:

```markdown
# TaskFlow Constitution

## Core Values
1. **Simplicity** - Features must be intuitive
2. **Performance** - Sub-100ms response times
3. **Privacy** - User data belongs to users

## Technical Constraints
- Backend: Rust with Axum
- Frontend: React with TypeScript
- Database: PostgreSQL
- Real-time: WebSockets

## Non-Goals
- Mobile native apps (web-first approach)
- Offline mode (always-connected assumption)
```

---

## Phase 2: Spec Driven Development

### 2.1 Create Feature Specifications

```bash
# Core authentication spec
cwa spec new "User Authentication System"

# Main feature specs
cwa spec new "Task Management CRUD"
cwa spec new "Real-time Collaboration"
cwa spec new "Time Tracking Module"
cwa spec new "Team Analytics Dashboard"
```

### 2.2 Define Spec Details

Each spec should include acceptance criteria. Example for "User Authentication System":

**Title:** User Authentication System
**Priority:** Critical
**Status:** Draft

**Description:**
Implement secure user authentication with email/password and OAuth providers.

**Acceptance Criteria:**
- [ ] Users can register with email and password
- [ ] Users can login with Google OAuth
- [ ] Password reset via email link
- [ ] JWT tokens with 24h expiration
- [ ] Refresh token rotation
- [ ] Rate limiting: 5 attempts per minute

### 2.3 List and Review Specs

```bash
cwa spec list
```

Output:
```
ID  | Title                      | Status  | Priority
----|----------------------------|---------|----------
1   | User Authentication System | draft   | critical
2   | Task Management CRUD       | draft   | high
3   | Real-time Collaboration    | draft   | high
4   | Time Tracking Module       | draft   | medium
5   | Team Analytics Dashboard   | draft   | low
```

---

## Phase 3: Domain Driven Design

### 3.1 Identify Bounded Contexts

```bash
# Create bounded contexts
cwa domain context new "Identity"
cwa domain context new "TaskManagement"
cwa domain context new "Collaboration"
cwa domain context new "Analytics"
```

### 3.2 Model Domain Objects

For the **Identity** context:

| Type | Name | Description |
|------|------|-------------|
| Entity | User | Core user identity |
| Entity | Team | Group of users |
| Value Object | Email | Validated email address |
| Value Object | Password | Hashed password |
| Aggregate | UserAccount | User + credentials + preferences |

For the **TaskManagement** context:

| Type | Name | Description |
|------|------|-------------|
| Entity | Task | Work item with status |
| Entity | Project | Container for tasks |
| Value Object | TaskStatus | Enum: todo, in_progress, done |
| Value Object | Priority | Enum: low, medium, high, critical |
| Aggregate | ProjectBoard | Project + tasks + members |

### 3.3 Define Context Relationships

```
Identity ──────> TaskManagement (Customer/Supplier)
                      │
                      ▼
              Collaboration (Partnership)
                      │
                      ▼
                 Analytics (Downstream)
```

---

## Phase 4: Kanban Task Management

### 4.1 Break Specs into Tasks

For "User Authentication System" spec:

```bash
# Backend tasks
cwa task new "Setup user database schema" --spec 1 --priority high
cwa task new "Implement registration endpoint" --spec 1 --priority high
cwa task new "Implement login endpoint" --spec 1 --priority high
cwa task new "Add Google OAuth integration" --spec 1 --priority medium
cwa task new "Implement password reset flow" --spec 1 --priority medium
cwa task new "Add JWT middleware" --spec 1 --priority high
cwa task new "Implement rate limiting" --spec 1 --priority medium

# Frontend tasks
cwa task new "Create login form component" --spec 1 --priority high
cwa task new "Create registration form" --spec 1 --priority high
cwa task new "Add OAuth buttons" --spec 1 --priority medium
cwa task new "Build password reset page" --spec 1 --priority medium
```

### 4.2 View Kanban Board

```bash
cwa task board
```

Output:
```
┌─────────────┬─────────────┬─────────────┬─────────────┬─────────────┐
│   BACKLOG   │    TODO     │ IN_PROGRESS │   REVIEW    │    DONE     │
│             │   (0/5)     │    (0/1)    │   (0/2)     │             │
├─────────────┼─────────────┼─────────────┼─────────────┼─────────────┤
│ #1 Setup    │             │             │             │             │
│ #2 Register │             │             │             │             │
│ #3 Login    │             │             │             │             │
│ #4 OAuth    │             │             │             │             │
│ #5 Reset    │             │             │             │             │
│ #6 JWT      │             │             │             │             │
│ ...         │             │             │             │             │
└─────────────┴─────────────┴─────────────┴─────────────┴─────────────┘
```

### 4.3 Work Through Tasks

```bash
# Move task to todo
cwa task move 1 todo

# Start working on it
cwa task move 1 in_progress

# Check WIP limits
cwa task wip
# Output: in_progress: 1/1 (at limit)

# Complete and move to review
cwa task move 1 review

# After review, mark as done
cwa task move 1 done
```

---

## Phase 5: Recording Architectural Decisions

### 5.1 Document Key Decisions

```bash
cwa decision add "Use JWT for authentication" \
  --context "Need stateless auth for horizontal scaling" \
  --consequences "Must handle token refresh, no server-side session invalidation"

cwa decision add "PostgreSQL over MongoDB" \
  --context "Strong relational data model, ACID compliance needed" \
  --consequences "Schema migrations required, but better data integrity"

cwa decision add "WebSocket for real-time updates" \
  --context "Need low-latency bidirectional communication" \
  --consequences "Must handle reconnection logic, more complex than polling"
```

---

## Phase 6: Claude Code Integration

### 6.1 MCP Server Usage

Start the MCP server for Claude Code integration:

```bash
cwa mcp stdio
```

Claude Code can now use these tools:
- `cwa_get_current_task` - Know what to work on
- `cwa_get_spec` - Understand feature requirements
- `cwa_get_context_summary` - Quick project overview
- `cwa_update_task_status` - Move tasks when done

### 6.2 Example Claude Code Session

```
User: What should I work on next?

Claude: [calls cwa_get_current_task]
You have task #3 "Implement login endpoint" in progress.

The spec requires:
- Email/password authentication
- JWT token generation
- Rate limiting (5 attempts/min)

Let me implement this for you...
```

### 6.3 Web Dashboard

Start the web server for visual management:

```bash
cwa serve
# Server running at http://localhost:3000
```

Features:
- Visual Kanban board with drag-and-drop
- Spec viewer with acceptance criteria
- Domain model visualization
- Decision log timeline
- Real-time updates via WebSocket

---

## Phase 7: Development Workflow Summary

```
┌──────────────────────────────────────────────────────────────────┐
│                     CWA Development Workflow                      │
├──────────────────────────────────────────────────────────────────┤
│                                                                   │
│  1. SPEC ─────> 2. DOMAIN ─────> 3. TASKS ─────> 4. BUILD        │
│     │              │                 │               │            │
│     │              │                 │               │            │
│  Define         Model            Break down      Implement        │
│  features       contexts         into work       with Claude      │
│  & criteria     & objects        items           Code             │
│                                                                   │
│                         ▼                                         │
│                                                                   │
│              5. REVIEW ─────> 6. DOCUMENT ─────> 7. ITERATE      │
│                  │                 │                 │            │
│                  │                 │                 │            │
│              Code review       Record ADRs       Refine specs     │
│              & testing         & learnings       & continue       │
│                                                                   │
└──────────────────────────────────────────────────────────────────┘
```

---

## Appendix: Quick Reference Commands

```bash
# Project
cwa init <name>              # Initialize new project
cwa context status           # Show current focus

# Specifications
cwa spec new <title>         # Create spec
cwa spec list                # List all specs
cwa spec show <id>           # Show spec details

# Domain
cwa domain context new <n>   # Create bounded context
cwa domain model             # Show domain model

# Tasks
cwa task new <title>         # Create task
cwa task board               # Show Kanban board
cwa task move <id> <status>  # Move task
cwa task wip                 # Check WIP limits

# Servers
cwa serve                    # Web dashboard on :3000
cwa mcp stdio                # MCP server for Claude
```

---

## Conclusion

CWA provides a structured approach to software development by combining:

1. **Spec Driven Development** - Clear feature definitions with acceptance criteria
2. **Domain Driven Design** - Proper modeling of business domains
3. **Kanban** - Controlled work flow with WIP limits
4. **Claude Integration** - AI-assisted development with full context

This combination ensures that Claude Code always has the necessary context to make informed decisions while maintaining development discipline through the Kanban methodology.
