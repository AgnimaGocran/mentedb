#![cfg(feature = "sql")]

use std::sync::Arc;

use sea_orm::Database;

use mentedb::MenteDb;
use mentedb::prelude::*;

#[tokio::test(flavor = "multi_thread")]
async fn test_sql_backend_sqlite_in_memory() {
    let db = Database::connect("sqlite::memory:")
        .await
        .expect("connect to sqlite");
    let db = Arc::new(db);
    let mdb = MenteDb::open_sql(db).expect("open MenteDb");

    let node = MemoryNode::new(
        AgentId::nil(),
        MemoryType::Episodic,
        "test content".to_string(),
        vec![],
    );
    let id = node.id;
    mdb.store(node).expect("store memory");

    let mem = mdb.get_memory(id).expect("get memory");
    assert_eq!(mem.content, "test content");
}
