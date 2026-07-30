//! Durable storage for graph edges in the SQL backend.
//!
//! Edges are written as rows in `mtdb_edges` at the moment they are created, so
//! they survive a process that dies without a clean shutdown. The in memory CSR
//! graph is rebuilt from these rows when a database is opened.

use std::sync::Arc;

use mentedb_core::edge::{EdgeType, MemoryEdge};
use mentedb_core::error::{MenteError, MenteResult};
use mentedb_core::types::MemoryId;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, DbErr, EntityTrait,
    PaginatorTrait, QueryFilter, Set,
};

use crate::entity::mtdb_edge;

/// Canonical on disk name for an edge type.
fn edge_type_name(edge_type: EdgeType) -> &'static str {
    match edge_type {
        EdgeType::Caused => "caused",
        EdgeType::Before => "before",
        EdgeType::Related => "related",
        EdgeType::Contradicts => "contradicts",
        EdgeType::Supports => "supports",
        EdgeType::Supersedes => "supersedes",
        EdgeType::Derived => "derived",
        EdgeType::PartOf => "part_of",
    }
}

/// Parse an edge type written by [`edge_type_name`].
fn parse_edge_type(name: &str) -> Option<EdgeType> {
    match name {
        "caused" => Some(EdgeType::Caused),
        "before" => Some(EdgeType::Before),
        "related" => Some(EdgeType::Related),
        "contradicts" => Some(EdgeType::Contradicts),
        "supports" => Some(EdgeType::Supports),
        "supersedes" => Some(EdgeType::Supersedes),
        "derived" => Some(EdgeType::Derived),
        "part_of" => Some(EdgeType::PartOf),
        _ => None,
    }
}

fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(fut))
}

fn storage_err(e: DbErr) -> MenteError {
    MenteError::Storage(e.to_string())
}

/// Row oriented edge storage.
pub struct SqlEdgeStore {
    db: Arc<DatabaseConnection>,
}

impl SqlEdgeStore {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Persist an edge. Rewrites the row when the same triple already exists,
    /// so repeated inference of the same relationship does not duplicate rows.
    ///
    /// A unique index on the triple guards against two writers inserting the
    /// same relationship concurrently. If this call loses that race the insert
    /// is rejected and the existing row is updated instead.
    pub fn insert(&self, edge: &MemoryEdge) -> MenteResult<()> {
        block_on(async {
            if self.update_existing(edge).await? {
                return Ok(());
            }
            match self.insert_new(edge).await {
                Ok(()) => Ok(()),
                Err(_) if self.update_existing(edge).await? => Ok(()),
                Err(e) => Err(e),
            }
        })
        .map_err(storage_err)
    }

    /// Update the row for this triple. Returns `false` when no row exists yet.
    async fn update_existing(&self, edge: &MemoryEdge) -> Result<bool, DbErr> {
        let existing = mtdb_edge::Entity::find()
            .filter(mtdb_edge::Column::SourceId.eq(edge.source.to_string()))
            .filter(mtdb_edge::Column::TargetId.eq(edge.target.to_string()))
            .filter(mtdb_edge::Column::EdgeType.eq(edge_type_name(edge.edge_type)))
            .one(&*self.db)
            .await?;

        let Some(model) = existing else {
            return Ok(false);
        };
        let mut active: mtdb_edge::ActiveModel = model.into();
        active.weight = Set(edge.weight as f64);
        active.created_at = Set(edge.created_at as i64);
        active.valid_from = Set(edge.valid_from.map(|v| v as i64));
        active.valid_until = Set(edge.valid_until.map(|v| v as i64));
        active.label = Set(edge.label.clone());
        active.update(&*self.db).await?;
        Ok(true)
    }

    async fn insert_new(&self, edge: &MemoryEdge) -> Result<(), DbErr> {
        let model = mtdb_edge::ActiveModel {
            source_id: Set(edge.source.to_string()),
            target_id: Set(edge.target.to_string()),
            edge_type: Set(edge_type_name(edge.edge_type).to_string()),
            weight: Set(edge.weight as f64),
            created_at: Set(edge.created_at as i64),
            valid_from: Set(edge.valid_from.map(|v| v as i64)),
            valid_until: Set(edge.valid_until.map(|v| v as i64)),
            label: Set(edge.label.clone()),
            ..Default::default()
        };
        model.insert(&*self.db).await?;
        Ok(())
    }

    /// Load every stored edge. Rows with an unknown edge type are skipped.
    pub fn load_all(&self) -> MenteResult<Vec<MemoryEdge>> {
        let models = block_on(async { mtdb_edge::Entity::find().all(&*self.db).await })
            .map_err(storage_err)?;

        let mut edges = Vec::with_capacity(models.len());
        for m in models {
            let (Ok(source), Ok(target), Some(edge_type)) = (
                m.source_id.parse::<MemoryId>(),
                m.target_id.parse::<MemoryId>(),
                parse_edge_type(&m.edge_type),
            ) else {
                continue;
            };
            edges.push(MemoryEdge {
                source,
                target,
                edge_type,
                weight: m.weight as f32,
                created_at: m.created_at as u64,
                valid_from: m.valid_from.map(|v| v as u64),
                valid_until: m.valid_until.map(|v| v as u64),
                label: m.label,
            });
        }
        Ok(edges)
    }

    /// Delete every edge touching the given memory, in either direction.
    pub fn delete_for_memory(&self, id: MemoryId) -> MenteResult<()> {
        let id = id.to_string();
        block_on(async {
            mtdb_edge::Entity::delete_many()
                .filter(
                    Condition::any()
                        .add(mtdb_edge::Column::SourceId.eq(&id))
                        .add(mtdb_edge::Column::TargetId.eq(&id)),
                )
                .exec(&*self.db)
                .await
        })
        .map_err(storage_err)?;
        Ok(())
    }

    /// Number of stored edges.
    pub fn count(&self) -> MenteResult<u64> {
        block_on(async { mtdb_edge::Entity::find().count(&*self.db).await }).map_err(storage_err)
    }
}
