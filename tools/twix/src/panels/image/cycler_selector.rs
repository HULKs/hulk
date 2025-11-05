// impl VisionCycler {
//     pub fn as_path(&self) -> Path {
//         match self {
//             VisionCycler::Top => "Vision".to_string(),
//         }
//     }

//     pub fn as_snake_case_path(&self) -> String {
//         match self {
//             VisionCycler::Top => "vision".to_string(),
//         }
//     }
// }

// impl TryFrom<&str> for VisionCycler {
//     type Error = &'static str;

//     fn try_from(value: &str) -> Result<Self, Self::Error> {
//         match value {
//             "VisionTop" => Ok(VisionCycler::Top),
//             _ => Err("Invalid vision cycler"),
//         }
//     }
// }
