use super::types::{{{ENTITY_NAME}}, {{ENTITY_NAME}}Params};
use anyhow::Result;

/// Core business logic for {{MODULE_NAME}}.
pub fn validate_{{MODULE_NAME}}_params(params: &{{ENTITY_NAME}}Params) -> Result<()> {
    if params.name.trim().is_empty() {
        anyhow::bail!("Name cannot be empty");
    }
    Ok(())
}

/// Compute or transform entity properties.
pub fn process_{{MODULE_NAME}}(entity: &mut {{ENTITY_NAME}}) {
    entity.name = entity.name.trim().to_string();
}
