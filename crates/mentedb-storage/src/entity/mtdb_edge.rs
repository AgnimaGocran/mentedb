#[cfg(feature = "sqlite")]
use sea_orm::entity::prelude::*;

#[cfg(feature = "sqlite")]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "mtdb_edges")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub source_id: String,
    pub target_id: String,
    pub edge_type: String,
    pub weight: f64,
    pub created_at: i64,
    pub valid_from: Option<i64>,
    pub valid_until: Option<i64>,
    pub label: Option<String>,
}

#[cfg(feature = "sqlite")]
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

#[cfg(feature = "sqlite")]
impl ActiveModelBehavior for ActiveModel {}
