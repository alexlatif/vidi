//! SQLite storage backend

use async_trait::async_trait;
use chrono::Utc;
use sqlx::{Row, SqlitePool, sqlite::SqlitePoolOptions};
use uuid::Uuid;

use crate::error::{Result, ServerError};
use crate::models::{
    DashboardMeta, DashboardRecord, DashboardSummary, ListQuery, UpdateMetaRequest, WasmStatus,
};
use crate::storage::DashboardStore;

pub struct SqliteStore {
    pool: SqlitePool,
}

impl SqliteStore {
    pub async fn new(path: &str) -> anyhow::Result<Self> {
        let url = format!("sqlite:{}?mode=rwc", path);
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&url)
            .await?;

        Ok(Self { pool })
    }

    pub async fn run_migrations(&self) -> anyhow::Result<()> {
        sqlx::query(
            r#"
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
                dashboard_json TEXT NOT NULL,
                wasm_status TEXT NOT NULL DEFAULT 'pending',
                wasm_error TEXT
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create indices for common queries
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_dashboards_xp_name ON dashboards(xp_name)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_dashboards_user ON dashboards(user)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_dashboards_permanent ON dashboards(permanent)")
            .execute(&self.pool)
            .await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_dashboards_updated_at ON dashboards(updated_at)",
        )
        .execute(&self.pool)
        .await?;

        // Migration: Add wasm_status and wasm_error columns if they don't exist
        // Check if wasm_status column exists by trying to select it
        let has_wasm_status = sqlx::query("SELECT wasm_status FROM dashboards LIMIT 1")
            .fetch_optional(&self.pool)
            .await
            .is_ok();

        if !has_wasm_status {
            let _ = sqlx::query(
                "ALTER TABLE dashboards ADD COLUMN wasm_status TEXT NOT NULL DEFAULT 'pending'",
            )
            .execute(&self.pool)
            .await;
            let _ = sqlx::query("ALTER TABLE dashboards ADD COLUMN wasm_error TEXT")
                .execute(&self.pool)
                .await;
        }

        Ok(())
    }

    fn row_to_record(&self, row: sqlx::sqlite::SqliteRow) -> Result<DashboardRecord> {
        let id_str: String = row.get("id");
        let tags_json: String = row.get("tags");
        let dashboard_json: String = row.get("dashboard_json");
        let created_at_str: String = row.get("created_at");
        let updated_at_str: String = row.get("updated_at");
        let last_accessed_at_str: String = row.get("last_accessed_at");
        let wasm_status_str: String = row
            .try_get("wasm_status")
            .unwrap_or_else(|_| "pending".to_string());
        let wasm_error: Option<String> = row.try_get("wasm_error").ok().flatten();

        let meta = DashboardMeta {
            id: Uuid::parse_str(&id_str).map_err(|e| ServerError::Internal(e.to_string()))?,
            xp_name: row.get("xp_name"),
            user: row.get("user"),
            tags: serde_json::from_str(&tags_json)?,
            permanent: row.get::<i32, _>("permanent") != 0,
            ttl: row.get::<Option<i64>, _>("ttl").map(|t| t as u64),
            created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| ServerError::Internal(e.to_string()))?
                .with_timezone(&Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                .map_err(|e| ServerError::Internal(e.to_string()))?
                .with_timezone(&Utc),
            last_accessed_at: chrono::DateTime::parse_from_rfc3339(&last_accessed_at_str)
                .map_err(|e| ServerError::Internal(e.to_string()))?
                .with_timezone(&Utc),
            wasm_status: WasmStatus::from_str(&wasm_status_str),
            wasm_error,
        };

        let dashboard = serde_json::from_str(&dashboard_json)?;

        Ok(DashboardRecord { meta, dashboard })
    }
}

#[async_trait]
impl DashboardStore for SqliteStore {
    async fn create(&self, record: DashboardRecord) -> Result<DashboardRecord> {
        let tags_json = serde_json::to_string(&record.meta.tags)?;
        let dashboard_json = serde_json::to_string(&record.dashboard)?;

        sqlx::query(
            r#"
            INSERT INTO dashboards (
                id, xp_name, user, tags, permanent, ttl,
                created_at, updated_at, last_accessed_at, dashboard_json,
                wasm_status, wasm_error
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(record.meta.id.to_string())
        .bind(&record.meta.xp_name)
        .bind(&record.meta.user)
        .bind(&tags_json)
        .bind(record.meta.permanent as i32)
        .bind(record.meta.ttl.map(|t| t as i64))
        .bind(record.meta.created_at.to_rfc3339())
        .bind(record.meta.updated_at.to_rfc3339())
        .bind(record.meta.last_accessed_at.to_rfc3339())
        .bind(&dashboard_json)
        .bind(record.meta.wasm_status.as_str())
        .bind(&record.meta.wasm_error)
        .execute(&self.pool)
        .await?;

        Ok(record)
    }

    async fn get(&self, id: Uuid) -> Result<Option<DashboardRecord>> {
        let row = sqlx::query("SELECT * FROM dashboards WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(r) => Ok(Some(self.row_to_record(r)?)),
            None => Ok(None),
        }
    }

    async fn list(&self, query: &ListQuery) -> Result<Vec<DashboardSummary>> {
        let mut sql = String::from("SELECT * FROM dashboards WHERE 1=1");
        let mut params: Vec<String> = vec![];

        if let Some(xp_name) = &query.xp_name {
            sql.push_str(" AND xp_name = ?");
            params.push(xp_name.clone());
        }

        if let Some(user) = &query.user {
            sql.push_str(" AND user = ?");
            params.push(user.clone());
        }

        if let Some(tag) = &query.tag {
            sql.push_str(" AND tags LIKE ?");
            params.push(format!("%\"{}%", tag));
        }

        if let Some(permanent) = query.permanent {
            sql.push_str(" AND permanent = ?");
            params.push((permanent as i32).to_string());
        }

        // Sort
        let sort_col = match query.sort.as_str() {
            "created_at" => "created_at",
            "last_accessed_at" => "last_accessed_at",
            _ => "updated_at",
        };
        let sort_order = if query.order == "asc" { "ASC" } else { "DESC" };
        sql.push_str(&format!(" ORDER BY {} {}", sort_col, sort_order));

        // Pagination
        sql.push_str(&format!(" LIMIT {} OFFSET {}", query.limit, query.offset));

        let mut q = sqlx::query(&sql);
        for param in &params {
            q = q.bind(param);
        }

        let rows = q.fetch_all(&self.pool).await?;
        let mut results = vec![];

        for row in rows {
            let record = self.row_to_record(row)?;
            results.push(DashboardSummary::from(&record));
        }

        Ok(results)
    }

    async fn replace(&self, id: Uuid, record: DashboardRecord) -> Result<DashboardRecord> {
        let tags_json = serde_json::to_string(&record.meta.tags)?;
        let dashboard_json = serde_json::to_string(&record.dashboard)?;
        let now = Utc::now();

        let result = sqlx::query(
            r#"
            UPDATE dashboards SET
                xp_name = ?,
                user = ?,
                tags = ?,
                permanent = ?,
                ttl = ?,
                updated_at = ?,
                dashboard_json = ?
            WHERE id = ?
            "#,
        )
        .bind(&record.meta.xp_name)
        .bind(&record.meta.user)
        .bind(&tags_json)
        .bind(record.meta.permanent as i32)
        .bind(record.meta.ttl.map(|t| t as i64))
        .bind(now.to_rfc3339())
        .bind(&dashboard_json)
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(ServerError::NotFound(id.to_string()));
        }

        // Return updated record
        self.get(id)
            .await?
            .ok_or_else(|| ServerError::NotFound(id.to_string()))
    }

    async fn update_meta(&self, id: Uuid, update: UpdateMetaRequest) -> Result<DashboardRecord> {
        let existing = self
            .get(id)
            .await?
            .ok_or_else(|| ServerError::NotFound(id.to_string()))?;

        let new_xp_name = update.xp_name.or(existing.meta.xp_name);
        let new_user = update.user.or(existing.meta.user);
        let new_tags = update.tags.unwrap_or(existing.meta.tags);
        let new_permanent = update.permanent.unwrap_or(existing.meta.permanent);
        let new_ttl = if update.permanent == Some(true) {
            None
        } else {
            update.ttl.or(existing.meta.ttl)
        };

        let tags_json = serde_json::to_string(&new_tags)?;
        let now = Utc::now();

        sqlx::query(
            r#"
            UPDATE dashboards SET
                xp_name = ?,
                user = ?,
                tags = ?,
                permanent = ?,
                ttl = ?,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&new_xp_name)
        .bind(&new_user)
        .bind(&tags_json)
        .bind(new_permanent as i32)
        .bind(new_ttl.map(|t| t as i64))
        .bind(now.to_rfc3339())
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;

        self.get(id)
            .await?
            .ok_or_else(|| ServerError::NotFound(id.to_string()))
    }

    async fn delete(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM dashboards WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn touch(&self, id: Uuid) -> Result<()> {
        let now = Utc::now();
        sqlx::query("UPDATE dashboards SET last_accessed_at = ? WHERE id = ?")
            .bind(now.to_rfc3339())
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn cleanup_expired(&self, active_ids: &[Uuid]) -> Result<u64> {
        let now = Utc::now();

        // Build query to find expired dashboards not in active list
        let active_ids_str: Vec<String> = active_ids.iter().map(|id| id.to_string()).collect();

        // Get all temporary dashboards
        let rows = sqlx::query(
            "SELECT id, last_accessed_at, ttl FROM dashboards WHERE permanent = 0 AND ttl IS NOT NULL"
        )
        .fetch_all(&self.pool)
        .await?;

        let mut deleted = 0u64;

        for row in rows {
            let id_str: String = row.get("id");
            let last_accessed_str: String = row.get("last_accessed_at");
            let ttl: i64 = row.get("ttl");

            // Skip if in active connections
            if active_ids_str.contains(&id_str) {
                continue;
            }

            // Parse and check expiry
            if let Ok(last_accessed) = chrono::DateTime::parse_from_rfc3339(&last_accessed_str) {
                let expiry = last_accessed + chrono::Duration::seconds(ttl);
                if now > expiry.with_timezone(&Utc) {
                    sqlx::query("DELETE FROM dashboards WHERE id = ?")
                        .bind(&id_str)
                        .execute(&self.pool)
                        .await?;
                    deleted += 1;
                }
            }
        }

        Ok(deleted)
    }

    async fn update_wasm_status(
        &self,
        id: Uuid,
        status: WasmStatus,
        error: Option<String>,
    ) -> Result<()> {
        sqlx::query("UPDATE dashboards SET wasm_status = ?, wasm_error = ? WHERE id = ?")
            .bind(status.as_str())
            .bind(&error)
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn get_dashboard_json(&self, id: Uuid) -> Result<Option<String>> {
        let row = sqlx::query("SELECT dashboard_json FROM dashboards WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| r.get("dashboard_json")))
    }
}
