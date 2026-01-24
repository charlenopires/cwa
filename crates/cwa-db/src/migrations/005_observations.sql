-- Observations: structured capture of development activity
CREATE TABLE IF NOT EXISTS observations (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    session_id TEXT REFERENCES sessions(id) ON DELETE SET NULL,
    obs_type TEXT NOT NULL CHECK(obs_type IN ('bugfix','feature','refactor','discovery','decision','change','insight')),
    title TEXT NOT NULL,
    narrative TEXT,
    facts TEXT,           -- JSON array of strings
    concepts TEXT,        -- JSON array of strings (how-it-works, why-it-exists, what-changed, problem-solution, gotcha, pattern, trade-off)
    files_modified TEXT,  -- JSON array of file paths
    files_read TEXT,      -- JSON array of file paths
    related_entity_type TEXT,
    related_entity_id TEXT,
    confidence REAL NOT NULL DEFAULT 0.8,
    embedding_id TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_observations_project ON observations(project_id);
CREATE INDEX IF NOT EXISTS idx_observations_type ON observations(obs_type);
CREATE INDEX IF NOT EXISTS idx_observations_confidence ON observations(confidence);
CREATE INDEX IF NOT EXISTS idx_observations_created ON observations(created_at);

-- Summaries: compressed session/time-range observations
CREATE TABLE IF NOT EXISTS summaries (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    session_id TEXT REFERENCES sessions(id) ON DELETE SET NULL,
    content TEXT NOT NULL,
    observations_count INTEGER DEFAULT 0,
    key_facts TEXT,       -- JSON array of strings
    time_range_start TEXT,
    time_range_end TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_summaries_project ON summaries(project_id);
