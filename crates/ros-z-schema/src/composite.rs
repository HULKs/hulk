use serde::{Deserialize, Serialize};

use crate::{SchemaError, TypeName};

/// Canonical service identity and its producer-owned protocol artifacts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceDef {
    /// The service type name.
    pub type_name: TypeName,
    /// The request message type.
    pub request: TypeName,
    /// The response message type.
    pub response: TypeName,
}

impl ServiceDef {
    /// Creates a service definition.
    pub fn new(
        type_name: impl Into<String>,
        request: impl Into<String>,
        response: impl Into<String>,
    ) -> Result<Self, SchemaError> {
        Ok(Self {
            type_name: TypeName::new(type_name)?,
            request: TypeName::new(request)?,
            response: TypeName::new(response)?,
        })
    }
}

/// Canonical action identity inputs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionDef {
    /// The action type name.
    pub type_name: TypeName,
    /// The goal message type.
    pub goal: TypeName,
    /// The result message type.
    pub result: TypeName,
    /// The feedback message type.
    pub feedback: TypeName,
}

/// Borrowed semantic identity inputs for an action descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionSemanticIdentity<'a> {
    /// The goal message type.
    pub goal: &'a TypeName,
    /// The result message type.
    pub result: &'a TypeName,
    /// The feedback message type.
    pub feedback: &'a TypeName,
}

impl ActionDef {
    /// Creates an action definition.
    pub fn new(
        type_name: impl Into<String>,
        goal: impl Into<String>,
        result: impl Into<String>,
        feedback: impl Into<String>,
    ) -> Result<Self, SchemaError> {
        Ok(Self {
            type_name: TypeName::new(type_name)?,
            goal: TypeName::new(goal)?,
            result: TypeName::new(result)?,
            feedback: TypeName::new(feedback)?,
        })
    }

    /// Returns the goal message type.
    pub fn goal(&self) -> &TypeName {
        &self.goal
    }

    /// Returns the semantic identity inputs for this action.
    pub fn semantic_identity(&self) -> ActionSemanticIdentity<'_> {
        ActionSemanticIdentity {
            goal: &self.goal,
            result: &self.result,
            feedback: &self.feedback,
        }
    }
}
