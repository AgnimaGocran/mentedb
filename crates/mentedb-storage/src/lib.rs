//! MenteDB Storage Engine.
//!
//! Two backends are available:
//! - file based storage with WAL, always compiled
//! - SQL storage via SeaORM (SQLite or PostgreSQL), behind the `sql` feature
//!
//! The `sql` feature is additive: it adds the SQL backend without removing the
//! file backend. Use [`Storage`] to hold whichever backend was opened.

pub mod backend;
pub mod backup;
pub mod buffer;
pub mod engine;
pub mod page;
pub mod wal;

#[cfg(feature = "sql")]
pub mod edge_store;
#[cfg(feature = "sql")]
pub mod entity;
#[cfg(feature = "sql")]
pub mod meta_store;
#[cfg(feature = "sql")]
pub mod page_store;
#[cfg(feature = "sql")]
pub mod schema;
#[cfg(feature = "sql")]
pub mod serde_compat;
#[cfg(feature = "sql")]
pub mod sqlite_engine;

pub use backend::{PageRef, Storage};
pub use buffer::BufferPool;
pub use engine::StorageEngine;
pub use page::{PAGE_DATA_SIZE, PAGE_SIZE, Page, PageHeader, PageId, PageType};
pub use wal::{Lsn, Wal, WalEntry, WalEntryType};

#[cfg(feature = "sql")]
pub use meta_store::{load_meta, load_meta_sync, upsert_meta, upsert_meta_sync};
#[cfg(feature = "sql")]
pub use schema::ensure_schema_sync;
#[cfg(feature = "sql")]
pub use sqlite_engine::SqlStorageEngine;
#[cfg(feature = "sql")]
pub use sqlite_engine::SqliteStorageEngine;
