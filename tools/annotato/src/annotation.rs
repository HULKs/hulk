use serde::{Deserialize, Serialize};

use crate::classes::Class;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AnnotationFormat {
    pub points: [[f32; 2]; 2],
    pub class: Class,
}
