use crate::camera_intrinsics::CameraIntrinsics as CI;
use factrs::{
    assign_symbols,
    variables::{SE3, SE23},
};

// Careful: factrs resolves symbols by first character in the symbol name
// There must be no ambiguity in the first character of the symbol name
assign_symbols!(State: SE23);
assign_symbols!(Extrinsics: SE3);
assign_symbols!(CameraIntrinsics: CI);
