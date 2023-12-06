use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("participations"))
                    .if_not_exists()
                    .col(ColumnDef::new(Alias::new("match_id")).text().not_null())
                    .col(ColumnDef::new(Alias::new("bot_id")).string().not_null())
                    .col(ColumnDef::new(Alias::new("index")).integer().not_null())
                    .col(ColumnDef::new(Alias::new("score")).integer())
                    .primary_key(
                        Index::create()
                            .col(Alias::new("match_id"))
                            .col(Alias::new("bot_id"))
                            .col(Alias::new("index"))
                            .primary(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("participations"), Alias::new("match_id"))
                            .to(Alias::new("matches"), Alias::new("id")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Alias::new("participations"), Alias::new("bot_id"))
                            .to(Alias::new("bots"), Alias::new("id")),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Alias::new("participations")).to_owned())
            .await
    }
}
