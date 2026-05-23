use std::sync::Arc;

use mentedb_core::MemoryNode;
use mentedb_core::error::{MenteError, MenteResult};
use mentedb_core::types::MemoryId;
use sea_orm::{DatabaseConnection, DbErr, EntityTrait, Set, ActiveModelTrait, QueryFilter, ColumnTrait, PaginatorTrait};
use crate::entity::mtdb_page;
use crate::serde_compat;

pub struct SqlitePageStore {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl SqlitePageStore {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub fn insert(
        &self,
        node: &MemoryNode,
    ) -> MenteResult<i64> {
        let (data, embedding) = serde_compat::serialize_node(node);
        let emb_opt = if embedding.is_empty() { None } else { Some(embedding) };

        let model = mtdb_page::ActiveModel {
            memory_id: Set(node.id.to_string()),
            data: Set(data),
            embedding: Set(emb_opt),
            created_at: Set(node.created_at as i64),
            ..Default::default()
        };

        let result: mtdb_page::Model = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                model.insert(&*self.db).await
            })
        })
        .map_err(|e: DbErr| MenteError::Storage(e.to_string()))?;

        Ok(result.page_id)
    }

    pub fn insert_batch(
        &self,
        nodes: &[MemoryNode],
    ) -> MenteResult<Vec<i64>> {
        let mut ids = Vec::with_capacity(nodes.len());
        for node in nodes {
            let page_id = self.insert(node)?;
            ids.push(page_id);
        }
        Ok(ids)
    }

    pub fn read_by_page_id(&self, page_id: i64) -> MenteResult<MemoryNode> {
        let model: Option<mtdb_page::Model> = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                mtdb_page::Entity::find_by_id(page_id)
                    .one(&*self.db)
                    .await
            })
        })
        .map_err(|e: DbErr| MenteError::Storage(e.to_string()))?;

        let model = model.ok_or_else(|| MenteError::Storage(format!("page {} not found", page_id)))?;
        let embedding = model.embedding.unwrap_or_default();
        Ok(serde_compat::deserialize_node(&model.data, &embedding))
    }

    pub fn read_by_memory_id(&self, memory_id: MemoryId) -> MenteResult<Option<MemoryNode>> {
        let mid_str = memory_id.to_string();
        let model: Option<mtdb_page::Model> = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                mtdb_page::Entity::find()
                    .filter(mtdb_page::Column::MemoryId.eq(&mid_str))
                    .one(&*self.db)
                    .await
            })
        })
        .map_err(|e: DbErr| MenteError::Storage(e.to_string()))?;

        match model {
            Some(m) => {
                let embedding = m.embedding.unwrap_or_default();
                Ok(Some(serde_compat::deserialize_node(&m.data, &embedding)))
            }
            None => Ok(None),
        }
    }

    pub fn update(&self, page_id: i64, node: &MemoryNode) -> MenteResult<()> {
        let (data, embedding) = serde_compat::serialize_node(node);
        let emb_opt = if embedding.is_empty() { None } else { Some(embedding) };

        let existing: Option<mtdb_page::Model> = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                mtdb_page::Entity::find_by_id(page_id)
                    .one(&*self.db)
                    .await
            })
        })
        .map_err(|e: DbErr| MenteError::Storage(e.to_string()))?;

        let existing = existing.ok_or_else(|| MenteError::Storage(format!("page {} not found", page_id)))?;

        let mut active: mtdb_page::ActiveModel = existing.into();
        active.data = Set(data);
        active.embedding = Set(emb_opt);
        active.created_at = Set(node.created_at as i64);

        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                active.update(&*self.db).await
            })
        })
        .map_err(|e: DbErr| MenteError::Storage(e.to_string()))?;

        Ok(())
    }

    pub fn delete(&self, page_id: i64) -> MenteResult<()> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                mtdb_page::Entity::delete_by_id(page_id)
                    .exec(&*self.db)
                    .await
            })
        })
        .map_err(|e: DbErr| MenteError::Storage(e.to_string()))?;
        Ok(())
    }

    pub fn scan_all(&self) -> MenteResult<Vec<(MemoryId, i64)>> {
        let models: Vec<mtdb_page::Model> = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                mtdb_page::Entity::find()
                    .all(&*self.db)
                    .await
            })
        })
        .map_err(|e: DbErr| MenteError::Storage(e.to_string()))?;

        let mut results = Vec::new();
        for m in models {
            if let Ok(mid) = m.memory_id.parse::<MemoryId>() {
                results.push((mid, m.page_id));
            }
        }
        Ok(results)
    }

    pub fn page_count(&self) -> MenteResult<u64> {
        let count: u64 = tokio::task::block_in_place(|| -> Result<u64, DbErr> {
            tokio::runtime::Handle::current().block_on(async {
                mtdb_page::Entity::find()
                    .count(&*self.db)
                    .await
            })
        })
        .map_err(|e: DbErr| MenteError::Storage(e.to_string()))?;
        Ok(count)
    }
}
