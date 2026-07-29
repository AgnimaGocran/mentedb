//! MenteDB Storage Engine.
//!
//! Two backends available via feature flags:
//! - **default** (no features): legacy file-based storage with WAL
//! - **sql**: SQL-backed storage via SeaORM (SQLite or PostgreSQL)

#[cfg(not(feature = "sql"))]
pub mod backup;
#[cfg(not(feature = "sql"))]
pub mod buffer;
#[cfg(not(feature = "sql"))]
pub mod engine;
#[cfg(not(feature = "sql"))]
pub mod page;
#[cfg(not(feature = "sql"))]
pub mod wal;

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

#[cfg(not(feature = "sql"))]
pub use buffer::BufferPool;
#[cfg(not(feature = "sql"))]
pub use engine::StorageEngine;
#[cfg(not(feature = "sql"))]
pub use page::{PAGE_DATA_SIZE, PAGE_SIZE, Page, PageHeader, PageId, PageType};
#[cfg(not(feature = "sql"))]
pub use wal::{Lsn, Wal, WalEntry, WalEntryType};

#[cfg(feature = "sql")]
pub use meta_store::{load_meta, load_meta_sync, upsert_meta, upsert_meta_sync};
#[cfg(feature = "sql")]
pub use schema::ensure_schema_sync;
#[cfg(feature = "sql")]
pub use sqlite_engine::SqliteStorageEngine;
#[cfg(feature = "sql")]
pub use sqlite_engine::{PageId, SqlStorageEngine};
