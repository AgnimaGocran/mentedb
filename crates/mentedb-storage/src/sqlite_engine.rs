use std::sync::Arc;

use mentedb_core::MemoryNode;
use mentedb_core::error::MenteResult;
use mentedb_core::types::MemoryId;
use sea_orm::DatabaseConnection;
use tracing::info;

use crate::page_store::SqlPageStore;

pub type PageId = i64;

pub struct SqlStorageEngine {
    store: SqlPageStore,
}

/// Backwards-compatible alias for `SqlStorageEngine`.
pub type SqliteStorageEngine = SqlStorageEngine;

impl SqlStorageEngine {
    pub fn open(db: &Arc<DatabaseConnection>) -> MenteResult<Self> {
        let store = SqlPageStore::new(Arc::clone(db));
        info!("SqlStorageEngine opened");
        Ok(Self { store })
    }

    pub fn store_memory(&self, node: &MemoryNode) -> MenteResult<PageId> {
        let page_id = self.store.insert(node)?;
        info!(page_id, "stored memory node");
        Ok(page_id)
    }

    pub fn store_memory_batch(&self, nodes: &[MemoryNode]) -> MenteResult<Vec<PageId>> {
        let page_ids = self.store.insert_batch(nodes)?;
        info!(count = page_ids.len(), "stored memory batch");
        Ok(page_ids)
    }

    pub fn load_memory(&self, page_id: PageId) -> MenteResult<MemoryNode> {
        self.store.read_by_page_id(page_id)
    }

    pub fn load_memory_by_uuid(&self, memory_id: MemoryId) -> MenteResult<Option<MemoryNode>> {
        self.store.read_by_memory_id(memory_id)
    }

    pub fn scan_all_memories(&self) -> Vec<(MemoryId, PageId)> {
        self.store.scan_all().unwrap_or_default()
    }

    pub fn checkpoint(&self) -> MenteResult<()> {
        Ok(())
    }

    pub fn close(&self) -> MenteResult<()> {
        info!("SqlStorageEngine closed");
        Ok(())
    }

    pub fn db(&self) -> &DatabaseConnection {
        &self.store.db
    }

    pub fn db_arc(&self) -> Arc<DatabaseConnection> {
        Arc::clone(&self.store.db)
    }
}
