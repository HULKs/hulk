use thiserror::Error;

#[derive(Debug, Error)]
pub enum TopicError {
    #[error("Topic name cannot be empty")]
    EmptyName,
}

pub enum Scope {
    Global,
    Local,
    Private,
}
pub struct Topic {
    pub scope: Scope,
    pub name: String,
}

impl Topic {
    // TODO: validate namespace and node name
    pub fn qualify(&self, namespace: &str, node: &str) -> String {
        let name = &self.name;
        match self.scope {
            Scope::Global => name.to_string(),
            Scope::Local => format!("{namespace}/{name}"),
            Scope::Private => format!("{namespace}/{node}/{name}"),
        }
    }
}

impl TryFrom<&str> for Topic {
    type Error = TopicError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let (scope, name) = if let Some(stripped) = value.strip_prefix("/") {
            (Scope::Global, stripped)
        } else if let Some(stripped) = value.strip_prefix("~/") {
            (Scope::Private, stripped)
        } else {
            (Scope::Local, value)
        };

        if name.is_empty() {
            return Err(TopicError::EmptyName);
        }

        Ok(Topic {
            scope,
            name: name.to_string(),
        })
    }
}
