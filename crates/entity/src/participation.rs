use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "participations")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub match_id: i32,
    #[sea_orm(primary_key)]
    pub bot_id: i32,
    #[sea_orm(primary_key)]
    pub index: u8,
    pub score: Option<i32>,
}

#[derive(Clone, Copy, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::r#match::Entity",
        from = "Column::MatchId",
        to = "super::r#match::Column::Id"
    )]
    Match,
}

impl Related<super::r#match::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Match.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
