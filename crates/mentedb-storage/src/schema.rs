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

    Ok(())
}

pub fn ensure_schema_sync(db: &DatabaseConnection) -> Result<(), DbErr> {
    block_in_place(|| Handle::current().block_on(ensure_schema(db)))
}
