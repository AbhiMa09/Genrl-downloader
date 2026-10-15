use serde::{Deserialize, Serialize};

/// Primary domain entity for {{MODULE_NAME}}.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct {{ENTITY_NAME}} {
    pub id: i64,
    pub name: String,
    pub created_at: String,
}

/// Request parameters for {{MODULE_NAME}} operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct {{ENTITY_NAME}}Params {
    pub name: String,
}
