-- Migration 006: Expand memories entry_type constraint
-- Adds 'design_system' and 'observation' to allowed types

-- SQLite doesn't support ALTER CONSTRAINT, so we recreate the table
CREATE TABLE memories_new (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    entry_type TEXT NOT NULL CHECK(entry_type IN ('preference', 'decision', 'fact', 'pattern', 'design_system', 'observation')),
    context TEXT,
    confidence REAL DEFAULT 0.5,
    embedding_id TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Copy existing data
INSERT INTO memories_new SELECT * FROM memories;

-- Drop old table
DROP TABLE memories;

-- Rename new table
ALTER TABLE memories_new RENAME TO memories;

-- Recreate index
CREATE INDEX IF NOT EXISTS idx_memories_project ON memories(project_id);
CREATE INDEX IF NOT EXISTS idx_memories_type ON memories(entry_type);
