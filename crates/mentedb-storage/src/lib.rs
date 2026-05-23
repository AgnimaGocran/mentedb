//! MenteDB Storage Engine.
//!
//! Two backends available via feature flags:
//! - **default** (no features): legacy file-based storage with WAL
//! - **sqlite**: SQLite-backed storage via SeaORM (mtdb_pages table)

#[cfg(not(feature = "sqlite"))]
pub mod backup;
#[cfg(not(feature = "sqlite"))]
pub mod buffer;
#[cfg(not(feature = "sqlite"))]
pub mod engine;
#[cfg(not(feature = "sqlite"))]
pub mod page;
#[cfg(not(feature = "sqlite"))]
pub mod wal;

#[cfg(feature = "sqlite")]
pub mod entity;
#[cfg(feature = "sqlite")]
pub mod meta_store;
#[cfg(feature = "sqlite")]
pub mod page_store;
#[cfg(feature = "sqlite")]
pub mod serde_compat;
#[cfg(feature = "sqlite")]
pub mod sqlite_engine;

#[cfg(not(feature = "sqlite"))]
pub use buffer::BufferPool;
#[cfg(not(feature = "sqlite"))]
pub use engine::StorageEngine;
#[cfg(not(feature = "sqlite"))]
pub use page::{PAGE_DATA_SIZE, PAGE_SIZE, Page, PageHeader, PageId, PageType};
#[cfg(not(feature = "sqlite"))]
pub use wal::{Lsn, Wal, WalEntry, WalEntryType};

#[cfg(feature = "sqlite")]
pub use sqlite_engine::{PageId, SqliteStorageEngine};
#[cfg(feature = "sqlite")]
pub use meta_store::{load_meta, load_meta_sync, upsert_meta, upsert_meta_sync};
