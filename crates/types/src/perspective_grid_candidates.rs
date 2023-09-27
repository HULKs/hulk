use approx::{AbsDiffEq, RelativeEq};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::geometry::Circle;

#[derive(Default, Clone, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub struct PerspectiveGridCandidates {
    pub candidates: Vec<Circle>,
}

impl PartialEq for PerspectiveGridCandidates {
    fn eq(&self, other: &Self) -> bool {
        self.candidates.len() == other.candidates.len()
            && self
                .candidates
                .iter()
                .zip(other.candidates.iter())
                .all(|(own, other)| own == other)
    }
}

impl AbsDiffEq for PerspectiveGridCandidates {
    type Epsilon = f32;

    fn default_epsilon() -> Self::Epsilon {
        Self::Epsilon::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.candidates.len() == other.candidates.len()
            && self
                .candidates
                .iter()
                .zip(other.candidates.iter())
                .all(|(own, other)| own.abs_diff_eq(other, epsilon))
    }
}

impl RelativeEq for PerspectiveGridCandidates {
    fn default_max_relative() -> Self::Epsilon {
        Self::Epsilon::default_max_relative()
    }

    fn relative_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
        max_relative: Self::Epsilon,
    ) -> bool {
        self.candidates.len() == other.candidates.len()
            && self
                .candidates
                .iter()
                .zip(other.candidates.iter())
                .all(|(own, other)| own.relative_eq(other, epsilon, max_relative))
    }
}

#[cfg(test)]
mod tests {
    use approx::{assert_relative_eq, assert_relative_ne};
    use nalgebra::point;

    use super::*;

    #[test]
    fn candidates_cmp_same() {
        assert_relative_eq!(
            PerspectiveGridCandidates {
                candidates: vec![
                    Circle {
                        center: point![0.0, 0.0],
                        radius: 2.5
                    },
                    Circle {
                        center: point![0.0, 0.0],
                        radius: 2.5
                    },
                ],
            },
            PerspectiveGridCandidates {
                candidates: vec![
                    Circle {
                        center: point![0.0, 0.0],
                        radius: 2.5
                    },
                    Circle {
                        center: point![0.0, 0.0],
                        radius: 2.5
                    },
                ],
            }
        );
    }

    #[test]
    fn candidates_cmp_different_circles() {
        assert_relative_ne!(
            PerspectiveGridCandidates {
                candidates: vec![
                    Circle {
                        center: point![0.3, 0.0],
                        radius: 2.0
                    },
                    Circle {
                        center: point![0.0, 0.0],
                        radius: 2.5
                    },
                ],
            },
            PerspectiveGridCandidates {
                candidates: vec![
                    Circle {
                        center: point![0.0, 0.0],
                        radius: 2.5
                    },
                    Circle {
                        center: point![0.0, 0.0],
                        radius: 2.5
                    },
                ],
            }
        );
    }

    #[test]
    fn candidates_cmp_different_lengths() {
        assert_relative_ne!(
            PerspectiveGridCandidates {
                candidates: vec![
                    Circle {
                        center: point![0.0, 0.0],
                        radius: 2.5
                    },
                    Circle {
                        center: point![0.0, 0.0],
                        radius: 2.5
                    },
                ],
            },
            PerspectiveGridCandidates {
                candidates: vec![Circle {
                    center: point![0.0, 0.0],
                    radius: 2.5
                },],
            }
        );
    }
}
