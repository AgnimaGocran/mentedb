//! Test suite for the SQL storage backend.
//!
//! Every test runs against a file backed SQLite database, and additionally
//! against PostgreSQL when `MENTEDB_TEST_POSTGRES` points at a reachable
//! server. Start one with:
//!
//! ```text
//! docker compose --profile test up -d postgres
//! export MENTEDB_TEST_POSTGRES=postgres://mentedb:mentedb@127.0.0.1:55432/mentedb
//! ```
//!
//! Each PostgreSQL case gets its own schema so cases stay independent.

#![cfg(feature = "sql")]

use std::sync::Arc;

use sea_orm::{ConnectionTrait, Database, DatabaseConnection};

use mentedb::MenteDb;
use mentedb::core::space::TenantContext;
use mentedb::prelude::*;

/// Environment variable holding the PostgreSQL connection URL.
const PG_ENV: &str = "MENTEDB_TEST_POSTGRES";

/// A prepared database that can be connected to repeatedly, so tests can
/// close a `MenteDb` and reopen it against the same data.
struct Backend {
    name: String,
    url: String,
    /// Kept alive so the SQLite file is not removed mid test.
    _dir: Option<tempfile::TempDir>,
}

impl Backend {
    async fn connect(&self) -> Arc<DatabaseConnection> {
        Arc::new(
            Database::connect(&self.url)
                .await
                .unwrap_or_else(|e| panic!("[{}] connect failed: {e}", self.name)),
        )
    }

    async fn open(&self) -> MenteDb {
        MenteDb::open_sql(self.connect().await)
            .unwrap_or_else(|e| panic!("[{}] open_sql failed: {e}", self.name))
    }

    async fn open_for(&self, tenant: TenantContext) -> MenteDb {
        MenteDb::open_sql_with_tenant(self.connect().await, Some(tenant))
            .unwrap_or_else(|e| panic!("[{}] open_sql_with_tenant failed: {e}", self.name))
    }
}

/// Build every backend under test for the given case name.
async fn backends(case: &str) -> Vec<Backend> {
    let mut out = Vec::new();

    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("mentedb.sqlite");
    out.push(Backend {
        name: "sqlite".to_string(),
        url: format!("sqlite://{}?mode=rwc", path.display()),
        _dir: Some(dir),
    });

    if let Ok(base) = std::env::var(PG_ENV) {
        let schema = format!("mt_{case}");
        let admin = Database::connect(&base)
            .await
            .unwrap_or_else(|e| panic!("connect to {PG_ENV} failed: {e}"));
        for sql in [
            format!("DROP SCHEMA IF EXISTS {schema} CASCADE"),
            format!("CREATE SCHEMA {schema}"),
        ] {
            admin
                .execute_unprepared(&sql)
                .await
                .expect("prepare test schema");
        }
        let sep = if base.contains('?') { '&' } else { '?' };
        out.push(Backend {
            name: "postgres".to_string(),
            url: format!("{base}{sep}options=-c%20search_path%3D{schema}"),
            _dir: None,
        });
    }

    out
}

/// Number of rows physically present in `mtdb_pages`.
async fn page_row_count(b: &Backend) -> u64 {
    use mentedb::storage::entity::mtdb_page;
    use sea_orm::{EntityTrait, PaginatorTrait};

    let conn = b.connect().await;
    mtdb_page::Entity::find()
        .count(&*conn)
        .await
        .unwrap_or_else(|e| panic!("[{}] count rows: {e}", b.name))
}

fn memory(content: &str, embedding: Vec<f32>) -> MemoryNode {
    MemoryNode::new(
        AgentId::new(),
        MemoryType::Episodic,
        content.to_string(),
        embedding,
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn store_and_get() {
    for b in backends("store_and_get").await {
        let db = b.open().await;
        let node = memory("test content", vec![1.0, 0.0]);
        let id = node.id;
        db.store(node).expect("store");

        let got = db.get_memory(id).expect("get");
        assert_eq!(got.content, "test content", "[{}] content", b.name);
        assert_eq!(got.id, id, "[{}] id", b.name);
        assert_eq!(db.memory_count(), 1, "[{}] count", b.name);
        db.close().expect("close");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn store_batch_and_scan() {
    for b in backends("store_batch_and_scan").await {
        let db = b.open().await;
        let nodes = vec![
            memory("first", vec![1.0, 0.0]),
            memory("second", vec![0.0, 1.0]),
            memory("third", vec![0.5, 0.5]),
        ];
        let ids = db.store_batch(nodes).expect("store_batch");
        assert_eq!(ids.len(), 3, "[{}] returned ids", b.name);
        assert_eq!(db.memory_count(), 3, "[{}] count", b.name);
        for id in ids {
            assert!(db.get_memory(id).is_ok(), "[{}] get {id}", b.name);
        }
        db.close().expect("close");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn recall_similar_finds_nearest() {
    for b in backends("recall_similar").await {
        let db = b.open().await;
        db.store(memory("dark mode preference", vec![1.0, 0.0, 0.0]))
            .expect("store");
        db.store(memory("monday meeting", vec![0.0, 1.0, 0.0]))
            .expect("store");
        db.store(memory("api key rotation", vec![0.0, 0.0, 1.0]))
            .expect("store");

        let hits = db.recall_similar(&[1.0, 0.0, 0.0], 2).expect("recall");
        assert!(!hits.is_empty(), "[{}] expected hits", b.name);
        assert!(hits.len() <= 2, "[{}] respects k", b.name);

        let top = db.get_memory(hits[0].0).expect("get top");
        assert_eq!(
            top.content, "dark mode preference",
            "[{}] nearest neighbour",
            b.name
        );
        db.close().expect("close");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn recall_mql_returns_memories() {
    for b in backends("recall_mql").await {
        let db = b.open().await;
        for i in 0..5 {
            db.store(memory(&format!("fact {i}"), vec![1.0, 0.0]))
                .expect("store");
        }

        let window = db
            .recall("RECALL memories NEAR [1.0, 0.0] LIMIT 3")
            .expect("recall");
        let count: usize = window.blocks.iter().map(|blk| blk.memories.len()).sum();
        assert!(count > 0, "[{}] expected memories in window", b.name);
        db.close().expect("close");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn relate_and_traverse() {
    for b in backends("relate_and_traverse").await {
        let db = b.open().await;
        let a = memory("cause", vec![1.0, 0.0]);
        let c = memory("effect", vec![0.9, 0.1]);
        let (a_id, c_id) = (a.id, c.id);
        db.store(a).expect("store a");
        db.store(c).expect("store c");

        let now = 1_000_000u64;
        db.relate(MemoryEdge {
            source: a_id,
            target: c_id,
            edge_type: EdgeType::Caused,
            weight: 0.9,
            created_at: now,
            valid_from: None,
            valid_until: None,
            label: None,
        })
        .expect("relate");

        let window = db.recall(&format!("TRAVERSE {a_id} DEPTH 2")).expect("mql");
        let ids: Vec<MemoryId> = window
            .blocks
            .iter()
            .flat_map(|blk| blk.memories.iter().map(|m| m.memory.id))
            .collect();
        assert!(ids.contains(&c_id), "[{}] traversal reaches target", b.name);
        db.close().expect("close");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn forget_removes_from_reads() {
    for b in backends("forget").await {
        let db = b.open().await;
        let node = memory("temporary thought", vec![0.5, 0.5]);
        let id = node.id;
        db.store(node).expect("store");

        db.forget(id).expect("forget");

        assert!(db.get_memory(id).is_err(), "[{}] get after forget", b.name);
        let hits = db.recall_similar(&[0.5, 0.5], 5).expect("recall");
        assert!(
            !hits.iter().any(|(hit, _)| *hit == id),
            "[{}] forgotten memory still in results",
            b.name
        );
        db.close().expect("close");
    }
}

/// Known defect: `forget` drops the memory from the indexes and the page map
/// but never deletes the row, so the content stays in the database forever.
/// Un-ignore once `forget` deletes through to storage.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "known defect: forget does not delete the stored row"]
async fn forget_deletes_the_row() {
    for b in backends("forget_deletes_row").await {
        let db = b.open().await;
        let node = memory("temporary thought", vec![0.5, 0.5]);
        let id = node.id;
        db.store(node).expect("store");
        assert_eq!(page_row_count(&b).await, 1, "[{}] stored", b.name);

        db.forget(id).expect("forget");
        db.close().expect("close");

        assert_eq!(
            page_row_count(&b).await,
            0,
            "[{}] forget left the row in storage",
            b.name
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn invalidated_memory_excluded_from_current_recall() {
    for b in backends("invalidate").await {
        let db = b.open().await;
        let node = memory("alice works at acme", vec![1.0, 0.0]);
        let id = node.id;
        let created = node.created_at;
        db.store(node).expect("store");

        db.invalidate_memory(id, created + 1_000)
            .expect("invalidate");

        let before = db
            .recall_similar_at(&[1.0, 0.0], 5, created)
            .expect("recall before");
        assert!(
            before.iter().any(|(hit, _)| *hit == id),
            "[{}] valid before invalidation",
            b.name
        );

        let after = db
            .recall_similar_at(&[1.0, 0.0], 5, created + 2_000)
            .expect("recall after");
        assert!(
            !after.iter().any(|(hit, _)| *hit == id),
            "[{}] excluded after invalidation",
            b.name
        );
        db.close().expect("close");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn memories_survive_reopen() {
    for b in backends("survive_reopen").await {
        let db = b.open().await;
        let node = memory("durable fact", vec![1.0, 0.0]);
        let id = node.id;
        db.store(node).expect("store");
        db.close().expect("close");

        let reopened = b.open().await;
        let got = reopened
            .get_memory(id)
            .unwrap_or_else(|e| panic!("[{}] get after reopen: {e}", b.name));
        assert_eq!(got.content, "durable fact", "[{}] content", b.name);
        assert_eq!(reopened.memory_count(), 1, "[{}] count", b.name);
        reopened.close().expect("close");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn index_survives_reopen() {
    for b in backends("index_reopen").await {
        let db = b.open().await;
        let node = memory("searchable fact", vec![1.0, 0.0, 0.0]);
        let id = node.id;
        db.store(node).expect("store");
        db.close().expect("close");

        let reopened = b.open().await;
        let hits = reopened
            .recall_similar(&[1.0, 0.0, 0.0], 5)
            .expect("recall after reopen");
        assert!(
            hits.iter().any(|(hit, _)| *hit == id),
            "[{}] vector index lost across reopen",
            b.name
        );
        reopened.close().expect("close");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn edges_survive_reopen() {
    for b in backends("edges_reopen").await {
        let db = b.open().await;
        let a = memory("cause", vec![1.0, 0.0]);
        let c = memory("effect", vec![0.9, 0.1]);
        let (a_id, c_id) = (a.id, c.id);
        db.store(a).expect("store a");
        db.store(c).expect("store c");
        db.relate(MemoryEdge {
            source: a_id,
            target: c_id,
            edge_type: EdgeType::Caused,
            weight: 0.9,
            created_at: 1_000_000,
            valid_from: None,
            valid_until: None,
            label: None,
        })
        .expect("relate");
        db.close().expect("close");

        let reopened = b.open().await;
        let window = reopened
            .recall(&format!("TRAVERSE {a_id} DEPTH 2"))
            .expect("mql");
        let ids: Vec<MemoryId> = window
            .blocks
            .iter()
            .flat_map(|blk| blk.memories.iter().map(|m| m.memory.id))
            .collect();
        assert!(ids.contains(&c_id), "[{}] edge lost across reopen", b.name);
        reopened.close().expect("close");
    }
}

/// Edges must be durable the moment they are created, not only after a clean
/// shutdown. This drops the database handle without calling `close`, which is
/// what happens on OOM, `docker stop` past its timeout, or a panic.
#[tokio::test(flavor = "multi_thread")]
async fn edges_survive_without_clean_shutdown() {
    for b in backends("edges_crash").await {
        let (a_id, c_id) = {
            let db = b.open().await;
            let a = memory("cause", vec![1.0, 0.0]);
            let c = memory("effect", vec![0.9, 0.1]);
            let (a_id, c_id) = (a.id, c.id);
            db.store(a).expect("store a");
            db.store(c).expect("store c");
            db.relate(MemoryEdge {
                source: a_id,
                target: c_id,
                edge_type: EdgeType::Caused,
                weight: 0.9,
                created_at: 1_000_000,
                valid_from: None,
                valid_until: None,
                label: None,
            })
            .expect("relate");
            // No close, no flush: simulate the process dying here.
            (a_id, c_id)
        };

        let reopened = b.open().await;
        let window = reopened
            .recall(&format!("TRAVERSE {a_id} DEPTH 2"))
            .expect("mql");
        let ids: Vec<MemoryId> = window
            .blocks
            .iter()
            .flat_map(|blk| blk.memories.iter().map(|m| m.memory.id))
            .collect();
        assert!(
            ids.contains(&c_id),
            "[{}] edge lost after unclean shutdown",
            b.name
        );
        reopened.close().expect("close");
    }
}

/// Edges inferred internally (write inference, consolidation) must be persisted
/// the same way as edges created through `relate`.
#[tokio::test(flavor = "multi_thread")]
async fn inferred_edges_are_persisted() {
    for b in backends("inferred_edges").await {
        let stored = {
            let db = b.open().await;
            // Near identical content triggers write inference to relate them.
            db.store(memory("the user prefers dark mode", vec![1.0, 0.0]))
                .expect("store");
            db.store(memory("the user prefers dark themes", vec![1.0, 0.0]))
                .expect("store");
            db.edge_count().expect("edge count")
        };

        assert!(
            stored > 0,
            "[{}] write inference produced no persisted edges",
            b.name
        );

        let reopened = b.open().await;
        assert_eq!(
            reopened.edge_count().expect("edge count"),
            stored,
            "[{}] inferred edges lost across reopen",
            b.name
        );
        reopened.close().expect("close");
    }
}

/// Relating the same pair twice must not accumulate rows.
#[tokio::test(flavor = "multi_thread")]
async fn repeated_relate_is_idempotent() {
    for b in backends("relate_idempotent").await {
        let db = b.open().await;
        let a = memory("cause", vec![1.0, 0.0]);
        let c = memory("effect", vec![0.0, 1.0]);
        let (a_id, c_id) = (a.id, c.id);
        db.store(a).expect("store a");
        db.store(c).expect("store c");

        let edge = MemoryEdge {
            source: a_id,
            target: c_id,
            edge_type: EdgeType::Caused,
            weight: 0.5,
            created_at: 1_000_000,
            valid_from: None,
            valid_until: None,
            label: None,
        };
        db.relate(edge.clone()).expect("relate once");
        let after_first = db.edge_count().expect("count");

        db.relate(MemoryEdge {
            weight: 0.9,
            ..edge
        })
        .expect("relate twice");

        assert_eq!(
            db.edge_count().expect("count"),
            after_first,
            "[{}] duplicate edge row created",
            b.name
        );
        db.close().expect("close");
    }
}

/// Forgetting a memory must drop its edges from storage too, otherwise they are
/// restored on the next open and point at a memory that no longer exists.
#[tokio::test(flavor = "multi_thread")]
async fn forget_removes_edges_from_storage() {
    for b in backends("forget_edges").await {
        let db = b.open().await;
        let a = memory("cause", vec![1.0, 0.0]);
        let c = memory("effect", vec![0.9, 0.1]);
        let (a_id, c_id) = (a.id, c.id);
        db.store(a).expect("store a");
        db.store(c).expect("store c");
        db.relate(MemoryEdge {
            source: a_id,
            target: c_id,
            edge_type: EdgeType::Caused,
            weight: 0.9,
            created_at: 1_000_000,
            valid_from: None,
            valid_until: None,
            label: None,
        })
        .expect("relate");
        assert!(db.edge_count().expect("count") > 0, "[{}] stored", b.name);

        db.forget(c_id).expect("forget");
        db.close().expect("close");

        let reopened = b.open().await;
        let window = reopened
            .recall(&format!("TRAVERSE {a_id} DEPTH 2"))
            .expect("mql");
        let ids: Vec<MemoryId> = window
            .blocks
            .iter()
            .flat_map(|blk| blk.memories.iter().map(|m| m.memory.id))
            .collect();
        assert!(
            !ids.contains(&c_id),
            "[{}] edge to forgotten memory came back",
            b.name
        );
        reopened.close().expect("close");
    }
}

/// Canary for version selection on reopen. An update writes a new row rather
/// than replacing the old one, and the page map is rebuilt by scanning rows
/// without an explicit ORDER BY, so which version wins depends on the order
/// the server happens to return. This test documents the expected behaviour;
/// it is not proof of determinism.
#[tokio::test(flavor = "multi_thread")]
async fn latest_version_wins_after_reopen() {
    for b in backends("latest_version").await {
        let db = b.open().await;
        let node = memory("versioned fact", vec![1.0, 0.0]);
        let id = node.id;
        let created = node.created_at;
        db.store(node).expect("store");
        // Writes a second row for the same memory id.
        db.invalidate_memory(id, created + 1_000)
            .expect("invalidate");
        db.close().expect("close");

        let reopened = b.open().await;
        let got = reopened.get_memory(id).expect("get after reopen");
        assert!(
            got.is_invalidated(),
            "[{}] reopen resurrected a stale version",
            b.name
        );
        reopened.close().expect("close");
    }
}

/// Memories are isolated by space and agent. A tenant-scoped database can only
/// read memories that match its context, both through direct lookups and MQL.
#[tokio::test(flavor = "multi_thread")]
async fn tenant_isolation_filters_reads() {
    for b in backends("tenant_isolation").await {
        let space_a = SpaceId::new();
        let agent_a = AgentId::new();
        let space_b = SpaceId::new();
        let agent_b = AgentId::new();

        let db = b.open().await;
        let mut node_a = memory("alpha fact", vec![1.0, 0.0]);
        node_a.space_id = space_a;
        node_a.agent_id = agent_a;
        let id_a = node_a.id;

        let mut node_b = memory("beta fact", vec![0.0, 1.0]);
        node_b.space_id = space_b;
        node_b.agent_id = agent_b;
        let id_b = node_b.id;

        db.store(node_a).expect("store a");
        db.store(node_b).expect("store b");
        db.close().expect("close");

        // Tenant A only sees its own memory.
        let db_a = b.open_for(TenantContext::new(space_a, agent_a)).await;
        assert_eq!(db_a.memory_count(), 1, "[{}] tenant A count", b.name);
        assert!(db_a.get_memory(id_a).is_ok(), "[{}] tenant A get a", b.name);
        assert!(
            db_a.get_memory(id_b).is_err(),
            "[{}] tenant A cannot get b",
            b.name
        );

        let hits_a = db_a.recall_similar(&[1.0, 0.0], 2).expect("recall a");
        assert_eq!(hits_a.len(), 1, "[{}] tenant A recall count", b.name);
        assert_eq!(hits_a[0].0, id_a, "[{}] tenant A recall id", b.name);

        let window_a = db_a
            .recall(&format!(
                "RECALL memories NEAR [1.0, 0.0] WHERE space = {space_a} LIMIT 5"
            ))
            .expect("recall mql a");
        let count_a: usize = window_a.blocks.iter().map(|b| b.memories.len()).sum();
        assert_eq!(count_a, 1, "[{}] tenant A mql count", b.name);

        // Tenant B only sees its own memory.
        let db_b = b.open_for(TenantContext::new(space_b, agent_b)).await;
        assert_eq!(db_b.memory_count(), 1, "[{}] tenant B count", b.name);
        assert!(db_b.get_memory(id_b).is_ok(), "[{}] tenant B get b", b.name);
        assert!(
            db_b.get_memory(id_a).is_err(),
            "[{}] tenant B cannot get a",
            b.name
        );

        db_a.close().expect("close a");
        db_b.close().expect("close b");
    }
}
