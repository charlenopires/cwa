-- Enhanced memories table with embedding support.
-- Separate from the existing `memory` table used by CLI.

CREATE TABLE memories (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    entry_type TEXT NOT NULL CHECK(entry_type IN ('preference', 'decision', 'fact', 'pattern')),
    context TEXT,
    confidence REAL DEFAULT 0.5,
    embedding_id TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    last_used_at TEXT
);

CREATE INDEX idx_memories_project ON memories(project_id);
CREATE INDEX idx_memories_type ON memories(entry_type);
CREATE INDEX idx_memories_confidence ON memories(confidence);

-- Sync state tracking for SQLite → Neo4j synchronization.

CREATE TABLE sync_state (
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    last_synced_at TEXT,
    sync_version INTEGER DEFAULT 0,
    PRIMARY KEY (entity_type, entity_id)
);
