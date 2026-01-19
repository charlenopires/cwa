-- CWA Database Schema
-- Version: 1.0.0

-- Projects table (root entity)
CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    constitution_path TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Specifications table (SDD)
CREATE TABLE IF NOT EXISTS specs (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'draft',
    priority TEXT DEFAULT 'medium',
    acceptance_criteria TEXT,
    dependencies TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    archived_at TEXT
);

-- Bounded contexts table (DDD)
CREATE TABLE IF NOT EXISTS bounded_contexts (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    responsibilities TEXT,
    upstream_contexts TEXT,
    downstream_contexts TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Domain objects table (DDD)
CREATE TABLE IF NOT EXISTS domain_objects (
    id TEXT PRIMARY KEY,
    context_id TEXT NOT NULL REFERENCES bounded_contexts(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    object_type TEXT NOT NULL,
    description TEXT,
    properties TEXT,
    behaviors TEXT,
    invariants TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Tasks table (Kanban)
CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    spec_id TEXT REFERENCES specs(id) ON DELETE SET NULL,
    title TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'backlog',
    priority TEXT DEFAULT 'medium',
    assignee TEXT,
    labels TEXT,
    estimated_effort TEXT,
    actual_effort TEXT,
    blocked_by TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    started_at TEXT,
    completed_at TEXT
);

-- Kanban configuration table
CREATE TABLE IF NOT EXISTS kanban_config (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    column_name TEXT NOT NULL,
    column_order INTEGER NOT NULL,
    wip_limit INTEGER,
    UNIQUE(project_id, column_name)
);

-- Decisions table (ADR)
CREATE TABLE IF NOT EXISTS decisions (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'proposed',
    context TEXT NOT NULL,
    decision TEXT NOT NULL,
    consequences TEXT,
    alternatives TEXT,
    related_specs TEXT,
    superseded_by TEXT REFERENCES decisions(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Memory table (context management)
CREATE TABLE IF NOT EXISTS memory (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    session_id TEXT,
    entry_type TEXT NOT NULL,
    content TEXT NOT NULL,
    importance TEXT DEFAULT 'normal',
    tags TEXT,
    related_entity_type TEXT,
    related_entity_id TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT
);

-- Sessions table
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    ended_at TEXT,
    summary TEXT,
    goals TEXT,
    accomplishments TEXT
);

-- Analyses table
CREATE TABLE IF NOT EXISTS analyses (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    analysis_type TEXT NOT NULL,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    sources TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Glossary terms table
CREATE TABLE IF NOT EXISTS glossary (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    context_id TEXT REFERENCES bounded_contexts(id) ON DELETE SET NULL,
    term TEXT NOT NULL,
    definition TEXT NOT NULL,
    aliases TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(project_id, term)
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_specs_project ON specs(project_id);
CREATE INDEX IF NOT EXISTS idx_specs_status ON specs(status);
CREATE INDEX IF NOT EXISTS idx_tasks_project ON tasks(project_id);
CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
CREATE INDEX IF NOT EXISTS idx_tasks_spec ON tasks(spec_id);
CREATE INDEX IF NOT EXISTS idx_memory_project ON memory(project_id);
CREATE INDEX IF NOT EXISTS idx_memory_session ON memory(session_id);
CREATE INDEX IF NOT EXISTS idx_domain_objects_context ON domain_objects(context_id);
CREATE INDEX IF NOT EXISTS idx_bounded_contexts_project ON bounded_contexts(project_id);
CREATE INDEX IF NOT EXISTS idx_decisions_project ON decisions(project_id);

-- Default Kanban columns (inserted via trigger or initialization)
