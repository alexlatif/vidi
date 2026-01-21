//! In-memory storage backend for testing

#![allow(dead_code)]

use async_trait::async_trait;
use chrono::Utc;
use dashmap::DashMap;
use uuid::Uuid;

use crate::error::{Result, ServerError};
use crate::models::{DashboardRecord, DashboardSummary, ListQuery, UpdateMetaRequest, WasmStatus};
use crate::storage::DashboardStore;

/// In-memory dashboard store for testing
pub struct MemoryStore {
    dashboards: DashMap<Uuid, DashboardRecord>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self {
            dashboards: DashMap::new(),
        }
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DashboardStore for MemoryStore {
    async fn create(&self, record: DashboardRecord) -> Result<DashboardRecord> {
        let id = record.meta.id;
        self.dashboards.insert(id, record.clone());
        Ok(record)
    }

    async fn get(&self, id: Uuid) -> Result<Option<DashboardRecord>> {
        Ok(self.dashboards.get(&id).map(|r| r.clone()))
    }

    async fn list(&self, query: &ListQuery) -> Result<Vec<DashboardSummary>> {
        let mut results: Vec<DashboardSummary> = self
            .dashboards
            .iter()
            .filter(|entry| {
                let record = entry.value();
                let meta = &record.meta;

                // Apply filters
                if let Some(ref xp_name) = query.xp_name
                    && meta.xp_name.as_ref() != Some(xp_name)
                {
                    return false;
                }
                if let Some(ref user) = query.user
                    && meta.user.as_ref() != Some(user)
                {
                    return false;
                }
                if let Some(ref tag) = query.tag
                    && !meta.tags.contains(tag)
                {
                    return false;
                }
                if let Some(permanent) = query.permanent
                    && meta.permanent != permanent
                {
                    return false;
                }
                true
            })
            .map(|entry| DashboardSummary::from(entry.value()))
            .collect();

        // Sort
        match query.sort.as_str() {
            "created_at" => {
                results.sort_by(|a, b| {
                    if query.order == "asc" {
                        a.created_at.cmp(&b.created_at)
                    } else {
                        b.created_at.cmp(&a.created_at)
                    }
                });
            }
            _ => {
                results.sort_by(|a, b| {
                    if query.order == "asc" {
                        a.updated_at.cmp(&b.updated_at)
                    } else {
                        b.updated_at.cmp(&a.updated_at)
                    }
                });
            }
        }

        // Pagination
        let start = query.offset as usize;
        let end = start + query.limit as usize;
        let len = results.len();

        Ok(results.into_iter().skip(start).take(end.min(len)).collect())
    }

    async fn replace(&self, id: Uuid, mut record: DashboardRecord) -> Result<DashboardRecord> {
        if !self.dashboards.contains_key(&id) {
            return Err(ServerError::NotFound(id.to_string()));
        }
        record.meta.id = id;
        record.meta.updated_at = Utc::now();
        self.dashboards.insert(id, record.clone());
        Ok(record)
    }

    async fn update_meta(&self, id: Uuid, update: UpdateMetaRequest) -> Result<DashboardRecord> {
        let mut entry = self
            .dashboards
            .get_mut(&id)
            .ok_or_else(|| ServerError::NotFound(id.to_string()))?;

        let record = entry.value_mut();

        if let Some(xp_name) = update.xp_name {
            record.meta.xp_name = Some(xp_name);
        }
        if let Some(user) = update.user {
            record.meta.user = Some(user);
        }
        if let Some(tags) = update.tags {
            record.meta.tags = tags;
        }
        if let Some(permanent) = update.permanent {
            record.meta.permanent = permanent;
            if permanent {
                record.meta.ttl = None;
            }
        }
        if let Some(ttl) = update.ttl {
            record.meta.ttl = Some(ttl);
        }
        record.meta.updated_at = Utc::now();

        Ok(record.clone())
    }

    async fn delete(&self, id: Uuid) -> Result<bool> {
        Ok(self.dashboards.remove(&id).is_some())
    }

    async fn touch(&self, id: Uuid) -> Result<()> {
        if let Some(mut entry) = self.dashboards.get_mut(&id) {
            entry.value_mut().meta.last_accessed_at = Utc::now();
        }
        Ok(())
    }

    async fn cleanup_expired(&self, active_ids: &[Uuid]) -> Result<u64> {
        let mut deleted = 0u64;

        let expired_ids: Vec<Uuid> = self
            .dashboards
            .iter()
            .filter(|entry| {
                let meta = &entry.value().meta;
                if meta.permanent {
                    return false;
                }
                if active_ids.contains(&meta.id) {
                    return false;
                }
                meta.is_expired()
            })
            .map(|entry| *entry.key())
            .collect();

        for id in expired_ids {
            if self.dashboards.remove(&id).is_some() {
                deleted += 1;
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
        if let Some(mut entry) = self.dashboards.get_mut(&id) {
            entry.value_mut().meta.wasm_status = status;
            entry.value_mut().meta.wasm_error = error;
        }
        Ok(())
    }

    async fn get_dashboard_json(&self, id: Uuid) -> Result<Option<String>> {
        if let Some(entry) = self.dashboards.get(&id) {
            let json = serde_json::to_string(&entry.value().dashboard)?;
            Ok(Some(json))
        } else {
            Ok(None)
        }
    }
}
