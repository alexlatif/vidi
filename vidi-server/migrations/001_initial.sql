-- Initial schema for Vidi dashboard storage

CREATE TABLE IF NOT EXISTS dashboards (
    id TEXT PRIMARY KEY,
    xp_name TEXT,
    user TEXT,
    tags TEXT NOT NULL DEFAULT '[]',
    permanent INTEGER NOT NULL DEFAULT 0,
    ttl INTEGER,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_accessed_at TEXT NOT NULL,
    dashboard_json TEXT NOT NULL
);

-- Indices for common queries
CREATE INDEX IF NOT EXISTS idx_dashboards_xp_name ON dashboards(xp_name);
CREATE INDEX IF NOT EXISTS idx_dashboards_user ON dashboards(user);
CREATE INDEX IF NOT EXISTS idx_dashboards_permanent ON dashboards(permanent);
CREATE INDEX IF NOT EXISTS idx_dashboards_updated_at ON dashboards(updated_at);
CREATE INDEX IF NOT EXISTS idx_dashboards_created_at ON dashboards(created_at);
CREATE INDEX IF NOT EXISTS idx_dashboards_last_accessed_at ON dashboards(last_accessed_at);
