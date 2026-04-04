//! Environment and variable management.
//!
//! Supports a hierarchy: Global → Collection → Folder → Request,
//! where each level can override variables from parent levels.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A named environment containing a set of variables.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    /// Unique identifier.
    pub id: Uuid,
    /// Human-readable name (e.g., "Production", "Staging", "Local").
    pub name: String,
    /// Variable definitions.
    pub variables: Vec<Variable>,
}

impl Environment {
    /// Create a new environment with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            variables: Vec::new(),
        }
    }

    /// Add a variable to this environment.
    pub fn add_variable(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.variables.push(Variable {
            key: key.into(),
            value: VariableValue::Plain(value.into()),
            enabled: true,
        });
    }

    /// Resolve all enabled variables into a flat key→value map.
    pub fn resolve(&self) -> HashMap<String, String> {
        self.variables
            .iter()
            .filter(|v| v.enabled)
            .map(|v| {
                let value = match &v.value {
                    VariableValue::Plain(s) => s.clone(),
                    VariableValue::Secret(_) => v.value.reveal().to_string(),
                };
                (v.key.clone(), value)
            })
            .collect()
    }
}

/// A single variable definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    /// Variable name (used as `{{key}}` in templates).
    pub key: String,
    /// Variable value.
    pub value: VariableValue,
    /// Whether this variable is active.
    pub enabled: bool,
}

/// The value of a variable, which may be a plain string or a secret.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum VariableValue {
    /// A plain-text value.
    Plain(String),
    /// A secret value (should be masked in UI, never exported in plain text).
    Secret(String),
}

impl VariableValue {
    /// Get the underlying value regardless of type.
    pub fn reveal(&self) -> &str {
        match self {
            Self::Plain(s) | Self::Secret(s) => s,
        }
    }
}

/// Resolves variables from multiple environment layers.
///
/// Layers are applied in order — later layers override earlier ones.
/// This implements the hierarchy: Global → Collection → Folder → Request.
pub fn resolve_layers(layers: &[&Environment]) -> HashMap<String, String> {
    let mut resolved = HashMap::new();
    for env in layers {
        for (key, value) in env.resolve() {
            resolved.insert(key, value);
        }
    }
    resolved
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_resolve() {
        let mut env = Environment::new("Test");
        env.add_variable("host", "localhost");
        env.add_variable("port", "8080");

        let resolved = env.resolve();
        assert_eq!(resolved.get("host").unwrap(), "localhost");
        assert_eq!(resolved.get("port").unwrap(), "8080");
    }

    #[test]
    fn test_layer_override() {
        let mut global = Environment::new("Global");
        global.add_variable("host", "production.api.com");
        global.add_variable("timeout", "5000");

        let mut local = Environment::new("Local");
        local.add_variable("host", "localhost");

        let resolved = resolve_layers(&[&global, &local]);
        assert_eq!(resolved.get("host").unwrap(), "localhost");
        assert_eq!(resolved.get("timeout").unwrap(), "5000");
    }

    #[test]
    fn test_disabled_variable_excluded() {
        let mut env = Environment::new("Test");
        env.variables.push(Variable {
            key: "active".into(),
            value: VariableValue::Plain("yes".into()),
            enabled: true,
        });
        env.variables.push(Variable {
            key: "inactive".into(),
            value: VariableValue::Plain("no".into()),
            enabled: false,
        });

        let resolved = env.resolve();
        assert!(resolved.contains_key("active"));
        assert!(!resolved.contains_key("inactive"));
    }
}
