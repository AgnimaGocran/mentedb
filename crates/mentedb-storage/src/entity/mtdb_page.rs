#[cfg(feature = "sqlite")]
use sea_orm::entity::prelude::*;

#[cfg(feature = "sqlite")]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "mtdb_pages")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub page_id: i64,
    pub memory_id: String,
    pub data: Vec<u8>,
    pub embedding: Option<Vec<u8>>,
    pub created_at: i64,
}

#[cfg(feature = "sqlite")]
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

#[cfg(feature = "sqlite")]
impl ActiveModelBehavior for ActiveModel {}
