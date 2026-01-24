-- Design systems table for storing extracted design tokens
CREATE TABLE design_systems (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    source_url TEXT NOT NULL,
    colors_json TEXT,
    typography_json TEXT,
    spacing_json TEXT,
    border_radius_json TEXT,
    shadows_json TEXT,
    breakpoints_json TEXT,
    components_json TEXT,
    raw_analysis TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_design_systems_project ON design_systems(project_id);
