use convert_case::{Case, Casing};

use crate::cycler::Cycler;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Path {
    pub segments: Vec<PathSegment>,
}

impl From<&str> for Path {
    fn from(path: &str) -> Self {
        let segments = path.split('.').map(PathSegment::from).collect();
        Self { segments }
    }
}

impl Path {
    pub fn contains_variable(&self) -> bool {
        self.segments.iter().any(|segment| segment.is_variable)
    }

    pub fn contains_optional(&self) -> bool {
        self.segments.iter().any(|segment| segment.is_optional)
    }

    pub fn expand_variables(&self, cycler: &Cycler) -> Vec<Path> {
        if !self.contains_variable() {
            return vec![self.clone()];
        }
        cycler
            .instances
            .iter()
            .map(|instance| {
                let segments = self
                    .segments
                    .iter()
                    .map(|segment| {
                        if segment.is_variable {
                            PathSegment {
                                name: instance.name.to_case(Case::Snake),
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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
