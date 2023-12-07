use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "matches")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub seed: i32,
    pub status: MatchStatus,
    pub created_at: DateTimeUtc,
    pub tag: Option<String>,
}

#[derive(Clone, Copy, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::participation::Entity")]
    Participation,
}

impl Related<super::participation::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Participation.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(EnumIter, DeriveActiveEnum, Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum MatchStatus {
    #[sea_orm(num_value = 0)]
    Pending,
    #[sea_orm(num_value = 1)]
    Finished,
}
