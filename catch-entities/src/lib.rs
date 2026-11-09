//! catch L0 core — the SeaORM entities (URDUME L0 "SeaORM-derives-N").
//!
//! These `DeriveEntityModel` entities are the single typed source from which
//! data-access, the shinka `DatabaseMigration` schema, and the serve DTOs
//! derive. Schema `catch`, **add-only forever** (never drop a column —
//! deprecate + error-on-use; SPEC.md data model + the add-only doctrine).
//!
//! M0 (DEPLOY.md): the running service serves from the in-memory `Board`
//! (`catch-core`); these entities are the durable destination the
//! `PgOutcomeChain` (the breathe-store leaf-swap) writes — the SeaORM SSoT is
//! the URDUME destination, the in-memory/SQLx tier is the shipped interim.

pub mod workspace {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

    /// A tenant. The multi-tenant axis (`ProductScoped`) runs through `id`.
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(schema_name = "catch", table_name = "workspace")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub name: String,
        pub created_at: ChronoDateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod potato {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

    /// A PR on the board. The current `state` is a projection of the
    /// OutcomeChain; this row is the queryable view.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(schema_name = "catch", table_name = "potato")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String, // "{workspace}/{repo}#{number}"
        pub workspace_id: String,
        pub repo: String,
        pub number: i64,
        pub state: String, // FuseState as snake_case
        pub holder: String,
        pub fuse_pct: f64,
        pub updated_at: ChronoDateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod outcome_chain {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

    /// The append-only, BLAKE3-chained OutcomeChain leaf (the system of record).
    /// `UNIQUE(stream, seq)` + the hash columns are enforced by the shinka
    /// migration; the `PgOutcomeChain` writer (breathe-store leaf-swap) appends
    /// state + row in one transaction. Positive-only ontology (SPEC.md §4.3).
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(schema_name = "catch", table_name = "outcome_chain")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub stream: String, // workspace or workspace/repo#number
        pub seq: i64,
        pub event_kind: String,
        pub occurred_at: ChronoDateTimeUtc,
        #[sea_orm(column_type = "VarBinary(StringLen::None)")]
        pub content_hash: Vec<u8>,
        #[sea_orm(column_type = "VarBinary(StringLen::None)")]
        pub prev_hash: Vec<u8>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub use outcome_chain::Entity as OutcomeChain;
pub use potato::Entity as Potato;
pub use workspace::Entity as Workspace;
