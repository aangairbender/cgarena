use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "matches")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub job_id: Uuid,
    pub seed: i32,
}

#[derive(Clone, Copy, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::participation::Entity")]
    Participation,
    #[sea_orm(
        belongs_to = "super::job::Entity",
        from = "Column::JobId",
        to = "super::job::Column::Id"
    )]
    Job,
}

impl Related<super::participation::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Participation.def()
    }
}

impl Related<super::job::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Job.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
