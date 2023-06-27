use convert_case::{Case, Casing};

use crate::cyclers::InstanceName;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Path {
    pub segments: Vec<PathSegment>,
}

impl Path {
    pub fn try_new(path: &str, allow_optionals: bool) -> Result<Self, String> {
        let segments: Vec<_> = path.split('.').map(PathSegment::from).collect();
        if !allow_optionals && segments.iter().any(|segment| segment.is_optional) {
            return Err("no optional values allowed in this field type".to_string());
        }
        if segments
            .iter()
            .filter(|segment| segment.is_variable)
            .count()
            > 1
        {
            return Err("only one variable segment allowed per path".to_string());
        }
        if let Some(segment) = segments
            .iter()
            .find(|segment| segment.is_variable && segment.name != "cycler_instance")
        {
            return Err(format!(
                "invalid variable name `${}`, did you mean `$cycler_instance`?",
                segment.name
            ));
        }
        Ok(Self { segments })
    }
}

impl Path {
    pub fn contains_variable(&self) -> bool {
        self.segments.iter().any(|segment| segment.is_variable)
    }

    pub fn contains_optional(&self) -> bool {
        self.segments.iter().any(|segment| segment.is_optional)
    }

    pub fn expand_variables(&self, instances: &[InstanceName]) -> Vec<Path> {
        if !self.contains_variable() {
            return vec![self.clone()];
        }

        instances
            .iter()
            .map(|instance| {
                let segments = self
                    .segments
                    .iter()
                    .map(|segment| {
                        if segment.is_variable {
                            PathSegment {
                                name: instance.to_case(Case::Snake),
                                is_optional: segment.is_optional,
                                is_variable: false,
                            }
                        } else {
                            segment.clone()
                        }
                    })
                    .collect();
                Path { segments }
            })
            .collect()
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PathSegment {
    pub name: String,
    pub is_optional: bool,
    pub is_variable: bool,
}

impl From<&str> for PathSegment {
    fn from(segment: &str) -> Self {
        let (is_variable, start_index) = match segment.starts_with('$') {
            true => (true, 1),
            false => (false, 0),
        };
        let (is_optional, end_index) = match segment.ends_with('?') {
            true => (true, segment.chars().count() - 1),
            false => (false, segment.chars().count()),
        };

        Self {
            name: segment[start_index..end_index].to_string(),
            is_optional,
            is_variable,
        }
    }
}
