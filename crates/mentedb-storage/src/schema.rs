use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, Schema};
use tokio::runtime::Handle;
use tokio::task::block_in_place;

use crate::entity::{mtdb_edge, mtdb_meta, mtdb_page};

pub async fn ensure_schema(db: &DatabaseConnection) -> Result<(), DbErr> {
    let backend = db.get_database_backend();
    let schema = Schema::new(backend);

    let mut page_table = schema.create_table_from_entity(mtdb_page::Entity);
    page_table.if_not_exists();
    db.execute(&page_table).await?;

    let mut meta_table = schema.create_table_from_entity(mtdb_meta::Entity);
    meta_table.if_not_exists();
    db.execute(&meta_table).await?;

    let mut edge_table = schema.create_table_from_entity(mtdb_edge::Entity);
    edge_table.if_not_exists();
    db.execute(&edge_table).await?;

    // `create_table_from_entity` does not emit secondary indexes. The unique
    // index makes an edge triple unique at the database level, so two writers
    // cannot insert the same relationship twice. The other two support lookups
    // by endpoint when deleting a memory's edges.
    for stmt in [
        "CREATE UNIQUE INDEX IF NOT EXISTS mtdb_edges_triple_uniq \
         ON mtdb_edges (source_id, target_id, edge_type)",
        "CREATE INDEX IF NOT EXISTS mtdb_edges_source_idx ON mtdb_edges (source_id)",
        "CREATE INDEX IF NOT EXISTS mtdb_edges_target_idx ON mtdb_edges (target_id)",
        "CREATE INDEX IF NOT EXISTS mtdb_pages_memory_idx ON mtdb_pages (memory_id)",
    ] {
        db.execute_unprepared(stmt).await?;
    }

    Ok(())
}

pub fn ensure_schema_sync(db: &DatabaseConnection) -> Result<(), DbErr> {
    block_in_place(|| Handle::current().block_on(ensure_schema(db)))
}
