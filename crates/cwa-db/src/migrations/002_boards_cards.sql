-- Kanban boards, columns, and cards for the web UI.
-- These are separate from the CLI `tasks` table.

CREATE TABLE boards (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX idx_boards_project ON boards(project_id);

CREATE TABLE columns (
    id TEXT PRIMARY KEY,
    board_id TEXT NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    position INTEGER NOT NULL,
    color TEXT,
    wip_limit INTEGER,
    UNIQUE(board_id, position)
);

CREATE INDEX idx_columns_board ON columns(board_id);

CREATE TABLE cards (
    id TEXT PRIMARY KEY,
    column_id TEXT NOT NULL REFERENCES columns(id),
    title TEXT NOT NULL,
    description TEXT,
    position INTEGER NOT NULL,
    priority TEXT CHECK(priority IN ('low', 'medium', 'high', 'critical')),
    due_date TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    completed_at TEXT
);

CREATE INDEX idx_cards_column ON cards(column_id);

CREATE TABLE labels (
    id TEXT PRIMARY KEY,
    board_id TEXT NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    color TEXT NOT NULL
);

CREATE TABLE card_labels (
    card_id TEXT REFERENCES cards(id) ON DELETE CASCADE,
    label_id TEXT REFERENCES labels(id) ON DELETE CASCADE,
    PRIMARY KEY (card_id, label_id)
);

CREATE TABLE card_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    card_id TEXT NOT NULL REFERENCES cards(id),
    action TEXT NOT NULL CHECK(action IN ('created', 'moved', 'updated', 'completed', 'archived')),
    from_column_id TEXT,
    to_column_id TEXT,
    timestamp TEXT DEFAULT (datetime('now'))
);

CREATE INDEX idx_card_history_card ON card_history(card_id);
