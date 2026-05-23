#[cfg(feature = "sqlite")]
use sea_orm::entity::prelude::*;

#[cfg(feature = "sqlite")]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "mtdb_meta")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub key: String,
    pub value: Vec<u8>,
}

#[cfg(feature = "sqlite")]
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

#[cfg(feature = "sqlite")]
impl ActiveModelBehavior for ActiveModel {}
