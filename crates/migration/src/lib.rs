pub use sea_orm_migration::prelude::*;

mod m20231206_080327_create_bots_table;
mod m20231206_091953_create_matches_table;
mod m20231206_095922_create_participations_table;
mod m20231207_020219_add_deleted_to_bots;
mod m20231207_031600_add_tag_to_matches;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20231206_080327_create_bots_table::Migration),
            Box::new(m20231206_091953_create_matches_table::Migration),
            Box::new(m20231206_095922_create_participations_table::Migration),
            Box::new(m20231207_020219_add_deleted_to_bots::Migration),
            Box::new(m20231207_031600_add_tag_to_matches::Migration),
        ]
    }
}
