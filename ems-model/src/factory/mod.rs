pub mod line;
pub mod machine;
pub mod worker;

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, TS)]
#[ts(export, export_to = "./factory.ts")]
pub struct Factory {
    /// The name of the factory.
    pub name: String,
    /// The id of the factory.
    pub id: String,
    /// The location of the factory.
    pub location: String,
    /// The lines of the factory.
    pub lines: Vec<String>,
}
