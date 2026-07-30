//! Backend neutral storage facade.
//!
//! MenteDB has two storage backends: the page based file engine with WAL and
//! the SQL engine backed by SeaORM. Both are compiled in whenever the `sql`
//! feature is enabled, and the backend is chosen at runtime by which
//! constructor was used. This keeps the `sql` feature additive: enabling it
//! adds the SQL backend without removing the file backend.

use mentedb_core::MemoryNode;
use mentedb_core::edge::MemoryEdge;
use mentedb_core::error::MenteResult;
use mentedb_core::types::MemoryId;

use crate::engine::StorageEngine;
use crate::page::PageId;
#[cfg(feature = "sql")]
use crate::sqlite_engine::SqlStorageEngine;

/// Backend neutral handle to a stored memory.
///
/// For the file backend this is the page number, for the SQL backend it is the
/// row id. Callers treat it as an opaque token obtained from a write and passed
/// back to a read.
pub type PageRef = u64;

/// The storage backend in use by a database instance.
pub enum Storage {
    /// Page based file storage with write ahead logging.
    File(StorageEngine),
    /// SQL storage via SeaORM (SQLite or PostgreSQL).
    #[cfg(feature = "sql")]
    Sql(SqlStorageEngine),
}

impl Storage {
    /// Returns `true` when this instance is backed by SQL.
    pub fn is_sql(&self) -> bool {
        #[cfg(feature = "sql")]
        {
            matches!(self, Storage::Sql(_))
        }
        #[cfg(not(feature = "sql"))]
        {
            false
        }
    }

    /// Persist a memory node and return its handle.
    pub fn store_memory(&self, node: &MemoryNode) -> MenteResult<PageRef> {
        match self {
            Storage::File(e) => e.store_memory(node).map(|p| p.0),
            #[cfg(feature = "sql")]
            Storage::Sql(e) => e.store_memory(node).map(|p| p as PageRef),
        }
    }

    /// Persist several memory nodes and return their handles in order.
    pub fn store_memory_batch(&self, nodes: &[MemoryNode]) -> MenteResult<Vec<PageRef>> {
        match self {
            Storage::File(e) => e
                .store_memory_batch(nodes)
                .map(|ids| ids.into_iter().map(|p| p.0).collect()),
            #[cfg(feature = "sql")]
            Storage::Sql(e) => e
                .store_memory_batch(nodes)
                .map(|ids| ids.into_iter().map(|p| p as PageRef).collect()),
        }
    }

    /// Load a memory node by its handle.
    pub fn load_memory(&self, page_ref: PageRef) -> MenteResult<MemoryNode> {
        match self {
            Storage::File(e) => e.load_memory(PageId(page_ref)),
            #[cfg(feature = "sql")]
            Storage::Sql(e) => e.load_memory(page_ref as i64),
        }
    }

    /// Enumerate every stored memory with its handle.
    pub fn scan_all_memories(&self) -> Vec<(MemoryId, PageRef)> {
        match self {
            Storage::File(e) => e
                .scan_all_memories()
                .into_iter()
                .map(|(id, p)| (id, p.0))
                .collect(),
            #[cfg(feature = "sql")]
            Storage::Sql(e) => e
                .scan_all_memories()
                .into_iter()
                .map(|(id, p)| (id, p as PageRef))
                .collect(),
        }
    }

    /// Whether this backend stores edges durably, row by row, as they are
    /// created. When `false` the caller is responsible for persisting the graph
    /// as a whole, which is what the file backend does with `graph.json`.
    pub fn edges_are_durable(&self) -> bool {
        #[cfg(feature = "sql")]
        {
            matches!(self, Storage::Sql(_))
        }
        #[cfg(not(feature = "sql"))]
        {
            false
        }
    }

    /// Persist a single edge. No op on backends without durable edges.
    pub fn store_edge(&self, edge: &MemoryEdge) -> MenteResult<()> {
        match self {
            Storage::File(_) => {
                let _ = edge;
                Ok(())
            }
            #[cfg(feature = "sql")]
            Storage::Sql(e) => e.store_edge(edge),
        }
    }

    /// Load every persisted edge. Empty on backends without durable edges.
    pub fn load_all_edges(&self) -> MenteResult<Vec<MemoryEdge>> {
        match self {
            Storage::File(_) => Ok(Vec::new()),
            #[cfg(feature = "sql")]
            Storage::Sql(e) => e.load_all_edges(),
        }
    }

    /// Delete every persisted edge touching the given memory.
    pub fn delete_edges_for_memory(&self, id: MemoryId) -> MenteResult<()> {
        match self {
            Storage::File(_) => {
                let _ = id;
                Ok(())
            }
            #[cfg(feature = "sql")]
            Storage::Sql(e) => e.delete_edges_for_memory(id),
        }
    }

    /// Number of persisted edges.
    pub fn edge_count(&self) -> MenteResult<u64> {
        match self {
            Storage::File(_) => Ok(0),
            #[cfg(feature = "sql")]
            Storage::Sql(e) => e.edge_count(),
        }
    }

    /// Flush pending writes to durable storage.
    pub fn checkpoint(&self) -> MenteResult<()> {
        match self {
            Storage::File(e) => e.checkpoint(),
            #[cfg(feature = "sql")]
            Storage::Sql(e) => e.checkpoint(),
        }
    }

    /// Close the backend.
    pub fn close(&self) -> MenteResult<()> {
        match self {
            Storage::File(e) => e.close(),
            #[cfg(feature = "sql")]
            Storage::Sql(e) => e.close(),
        }
    }
}
