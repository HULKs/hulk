use serde::{Deserialize, Serialize};

use crate::std_msgs::header::Header;

#[repr(C)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct JointState {
    pub header: Header,
    pub name: Vec<String>,
    pub position: Vec<f64>,
    pub velocity: Vec<f64>,
    pub effort: Vec<f64>,
}
