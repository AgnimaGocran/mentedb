use crate::entity::mtdb_meta;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, Set,
};

pub async fn upsert_meta(db: &DatabaseConnection, key: &str, value: &[u8]) -> Result<(), DbErr> {
    let existing = mtdb_meta::Entity::find()
        .filter(mtdb_meta::Column::Key.eq(key))
        .one(db)
        .await?;
    match existing {
        Some(model) => {
            let mut active: mtdb_meta::ActiveModel = model.into();
            active.value = Set(value.to_vec());
            active.update(db).await?;
        }
        None => {
            let model = mtdb_meta::ActiveModel {
                key: Set(key.to_string()),
                value: Set(value.to_vec()),
            };
            model.insert(db).await?;
        }
    }
    Ok(())
}

pub async fn load_meta(db: &DatabaseConnection, key: &str) -> Result<Option<Vec<u8>>, DbErr> {
    let result = mtdb_meta::Entity::find()
        .filter(mtdb_meta::Column::Key.eq(key))
        .one(db)
        .await?;
    Ok(result.map(|m| m.value))
}

pub fn upsert_meta_sync(db: &DatabaseConnection, key: &str, value: &[u8]) -> Result<(), DbErr> {
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(upsert_meta(db, key, value))
    })
}

pub fn load_meta_sync(db: &DatabaseConnection, key: &str) -> Result<Option<Vec<u8>>, DbErr> {
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(load_meta(db, key)))
}
