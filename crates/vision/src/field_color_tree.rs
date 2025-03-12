use types::color::Intensity;

pub struct Features {
pub blue_luminance: u8,
pub green_luminance: u8,
pub red_luminance: u8,
pub luminance: u8,
pub red_difference: u8,
pub blue_difference: u8,
pub blue_chromaticity: f32,
pub green_chromaticity: f32,
pub red_chromaticity: f32,
pub intensity: u8,
pub hue: u16,
pub saturation: u8,
pub value: u8,
}

#[allow(clippy::collapsible_else_if, clippy::if_same_then_else)]
pub fn predict(features: &Features) -> Intensity {
if features.blue_difference <= 117 {
if features.green_chromaticity <= 0.431 {
if features.blue_difference <= 108 {
if features.green_chromaticity <= 0.421 {
if features.saturation <= 86 {
if features.blue_luminance <= 99 {
if features.blue_luminance <= 95 {
Intensity::Low
} else {
if features.blue_luminance <= 97 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.hue <= 45 {
Intensity::High
} else {
if features.red_chromaticity <= 0.308 {
Intensity::High
} else {
if features.red_chromaticity <= 0.316 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.320 {
if features.green_chromaticity <= 0.407 {
Intensity::High
} else {
if features.green_chromaticity <= 0.409 {
if features.value <= 154 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.409 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.intensity <= 111 {
if features.red_chromaticity <= 0.329 {
if features.red_chromaticity <= 0.327 {
if features.green_chromaticity <= 0.414 {
Intensity::High
} else {
if features.saturation <= 101 {
if features.luminance <= 119 {
if features.blue_luminance <= 81 {
if features.red_chromaticity <= 0.324 {
Intensity::High
} else {
if features.red_difference <= 117 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.263 {
if features.red_chromaticity <= 0.320 {
if features.saturation <= 97 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.saturation <= 95 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_luminance <= 137 {
Intensity::Low
} else {
if features.intensity <= 109 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
} else {
Intensity::High
}
}
} else {
if features.hue <= 46 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_difference <= 104 {
if features.red_difference <= 125 {
if features.intensity <= 103 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 133 {
if features.green_chromaticity <= 0.403 {
if features.red_luminance <= 109 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.405 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.250 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.253 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.blue_chromaticity <= 0.269 {
if features.blue_luminance <= 87 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.251 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.412 {
Intensity::High
} else {
if features.intensity <= 126 {
if features.blue_chromaticity <= 0.257 {
Intensity::High
} else {
if features.red_chromaticity <= 0.316 {
if features.red_luminance <= 114 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.318 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
Intensity::High
}
}
}
}
} else {
if features.red_chromaticity <= 0.308 {
Intensity::High
} else {
if features.green_chromaticity <= 0.417 {
if features.value <= 148 {
Intensity::High
} else {
if features.green_chromaticity <= 0.415 {
if features.saturation <= 88 {
if features.green_luminance <= 150 {
if features.green_chromaticity <= 0.411 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.269 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 151 {
if features.red_chromaticity <= 0.313 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.274 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.274 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
}
} else {
if features.red_chromaticity <= 0.308 {
if features.blue_luminance <= 96 {
Intensity::Low
} else {
if features.value <= 150 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.313 {
if features.green_chromaticity <= 0.418 {
if features.green_chromaticity <= 0.418 {
if features.hue <= 52 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 94 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_difference <= 112 {
if features.intensity <= 123 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.271 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
Intensity::Low
}
}
}
}
}
}
}
} else {
if features.blue_difference <= 106 {
if features.green_luminance <= 143 {
if features.green_chromaticity <= 0.430 {
if features.saturation <= 105 {
if features.red_difference <= 116 {
if features.red_luminance <= 103 {
if features.green_luminance <= 136 {
if features.red_difference <= 115 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.316 {
if features.saturation <= 102 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.428 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.319 {
if features.blue_chromaticity <= 0.258 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.427 {
if features.green_chromaticity <= 0.424 {
if features.green_chromaticity <= 0.422 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.425 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
}
}
} else {
if features.saturation <= 104 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.hue <= 43 {
if features.red_luminance <= 89 {
Intensity::High
} else {
if features.blue_difference <= 99 {
Intensity::High
} else {
if features.luminance <= 101 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.427 {
if features.blue_difference <= 104 {
if features.blue_difference <= 103 {
Intensity::High
} else {
if features.red_chromaticity <= 0.330 {
Intensity::High
} else {
if features.green_chromaticity <= 0.422 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
Intensity::High
}
} else {
Intensity::High
}
}
}
} else {
if features.blue_chromaticity <= 0.256 {
if features.blue_luminance <= 73 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.blue_difference <= 104 {
if features.value <= 157 {
if features.intensity <= 111 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.257 {
if features.hue <= 49 {
Intensity::High
} else {
if features.green_luminance <= 160 {
if features.luminance <= 138 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.313 {
if features.blue_chromaticity <= 0.262 {
if features.intensity <= 115 {
if features.luminance <= 127 {
Intensity::High
} else {
if features.red_chromaticity <= 0.313 {
if features.red_chromaticity <= 0.311 {
if features.green_chromaticity <= 0.429 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_difference <= 105 {
Intensity::High
} else {
Intensity::High
}
}
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.431 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 112 {
if features.green_chromaticity <= 0.430 {
if features.red_difference <= 111 {
if features.green_chromaticity <= 0.428 {
Intensity::High
} else {
if features.red_chromaticity <= 0.303 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 110 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.425 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.304 {
if features.red_luminance <= 109 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.264 {
if features.value <= 159 {
if features.value <= 158 {
if features.blue_chromaticity <= 0.263 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.312 {
Intensity::High
} else {
if features.intensity <= 124 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
}
} else {
if features.red_chromaticity <= 0.313 {
if features.green_luminance <= 152 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.314 {
if features.green_chromaticity <= 0.423 {
if features.blue_chromaticity <= 0.264 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.value <= 152 {
if features.red_chromaticity <= 0.317 {
if features.green_luminance <= 151 {
if features.blue_luminance <= 90 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.424 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.319 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.value <= 153 {
if features.hue <= 49 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 115 {
if features.green_chromaticity <= 0.425 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::High
}
}
}
}
}
}
}
}
} else {
if features.green_chromaticity <= 0.429 {
if features.blue_difference <= 107 {
if features.hue <= 51 {
if features.saturation <= 102 {
if features.intensity <= 109 {
if features.red_luminance <= 100 {
if features.red_luminance <= 99 {
if features.saturation <= 101 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.red_luminance <= 101 {
if features.green_luminance <= 136 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 100 {
if features.red_chromaticity <= 0.320 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
}
} else {
if features.value <= 144 {
if features.red_luminance <= 104 {
if features.green_chromaticity <= 0.426 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.262 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 106 {
if features.red_luminance <= 105 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 143 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.blue_luminance <= 94 {
if features.blue_chromaticity <= 0.265 {
if features.saturation <= 95 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 148 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
}
}
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.265 {
Intensity::High
} else {
if features.intensity <= 123 {
if features.green_luminance <= 155 {
if features.red_chromaticity <= 0.305 {
if features.green_chromaticity <= 0.429 {
if features.green_chromaticity <= 0.428 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 93 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.305 {
if features.green_chromaticity <= 0.427 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.305 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.luminance <= 136 {
Intensity::Low
} else {
if features.luminance <= 137 {
if features.red_chromaticity <= 0.305 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
}
} else {
Intensity::High
}
}
}
} else {
if features.saturation <= 92 {
if features.blue_chromaticity <= 0.276 {
if features.green_chromaticity <= 0.427 {
if features.blue_chromaticity <= 0.275 {
if features.green_chromaticity <= 0.424 {
if features.green_chromaticity <= 0.422 {
if features.green_chromaticity <= 0.421 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 149 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.425 {
if features.saturation <= 90 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 97 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.429 {
if features.red_chromaticity <= 0.297 {
if features.red_difference <= 108 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 154 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
}
} else {
if features.value <= 161 {
if features.green_luminance <= 157 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.306 {
if features.red_chromaticity <= 0.306 {
if features.red_chromaticity <= 0.302 {
if features.red_chromaticity <= 0.300 {
if features.blue_chromaticity <= 0.271 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 114 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.302 {
Intensity::High
} else {
if features.blue_luminance <= 93 {
if features.green_luminance <= 143 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
}
} else {
if features.intensity <= 109 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.428 {
if features.intensity <= 103 {
if features.saturation <= 107 {
if features.luminance <= 107 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.269 {
if features.blue_chromaticity <= 0.269 {
if features.red_luminance <= 102 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.263 {
if features.red_chromaticity <= 0.313 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.red_chromaticity <= 0.302 {
if features.hue <= 54 {
Intensity::High
} else {
if features.red_chromaticity <= 0.298 {
if features.green_chromaticity <= 0.431 {
if features.red_difference <= 108 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.431 {
if features.red_luminance <= 106 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
}
} else {
if features.saturation <= 110 {
if features.luminance <= 116 {
Intensity::High
} else {
if features.value <= 134 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.303 {
Intensity::Low
} else {
if features.red_difference <= 112 {
if features.intensity <= 113 {
if features.red_chromaticity <= 0.307 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.intensity <= 104 {
if features.luminance <= 117 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.value <= 139 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
}
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.green_chromaticity <= 0.417 {
if features.green_chromaticity <= 0.412 {
if features.blue_chromaticity <= 0.287 {
if features.red_luminance <= 141 {
if features.value <= 127 {
if features.red_luminance <= 98 {
if features.red_chromaticity <= 0.347 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.347 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.411 {
if features.intensity <= 96 {
if features.blue_difference <= 114 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 109 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.377 {
Intensity::Low
} else {
if features.blue_luminance <= 90 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
} else {
Intensity::High
}
}
} else {
if features.blue_luminance <= 84 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.279 {
if features.red_luminance <= 99 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.279 {
if features.green_chromaticity <= 0.402 {
if features.green_luminance <= 144 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_difference <= 112 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 110 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.303 {
if features.red_luminance <= 106 {
if features.blue_difference <= 113 {
if features.value <= 140 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.411 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 123 {
if features.intensity <= 119 {
if features.blue_luminance <= 94 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 121 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.339 {
if features.saturation <= 79 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_difference <= 111 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
}
}
}
} else {
if features.saturation <= 63 {
Intensity::Low
} else {
if features.intensity <= 144 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.saturation <= 75 {
if features.green_chromaticity <= 0.402 {
if features.blue_difference <= 110 {
if features.red_chromaticity <= 0.332 {
Intensity::Low
} else {
if features.intensity <= 164 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.402 {
Intensity::High
} else {
if features.red_difference <= 114 {
if features.red_chromaticity <= 0.299 {
if features.red_chromaticity <= 0.299 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.299 {
Intensity::Low
} else {
if features.green_luminance <= 150 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 95 {
if features.blue_luminance <= 80 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 109 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.blue_chromaticity <= 0.287 {
Intensity::Low
} else {
if features.green_luminance <= 137 {
if features.green_chromaticity <= 0.411 {
if features.blue_difference <= 115 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.302 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.304 {
if features.hue <= 56 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
}
}
}
}
} else {
if features.blue_chromaticity <= 0.284 {
if features.red_chromaticity <= 0.299 {
Intensity::High
} else {
if features.value <= 160 {
if features.intensity <= 99 {
if features.red_luminance <= 51 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 80 {
Intensity::High
} else {
if features.luminance <= 113 {
if features.red_luminance <= 94 {
if features.red_luminance <= 91 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.279 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.412 {
Intensity::High
} else {
if features.red_chromaticity <= 0.306 {
if features.blue_difference <= 109 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.282 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
}
}
} else {
Intensity::High
}
}
} else {
if features.value <= 145 {
if features.blue_difference <= 114 {
if features.red_chromaticity <= 0.292 {
Intensity::High
} else {
if features.green_chromaticity <= 0.415 {
if features.red_chromaticity <= 0.301 {
if features.red_luminance <= 102 {
Intensity::High
} else {
if features.green_chromaticity <= 0.413 {
Intensity::Low
} else {
if features.red_difference <= 111 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.285 {
if features.green_luminance <= 141 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 120 {
Intensity::Low
} else {
if features.hue <= 56 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_chromaticity <= 0.417 {
if features.blue_chromaticity <= 0.285 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.296 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_chromaticity <= 0.415 {
if features.green_chromaticity <= 0.414 {
if features.red_chromaticity <= 0.292 {
if features.saturation <= 75 {
Intensity::High
} else {
if features.value <= 132 {
Intensity::Low
} else {
if features.value <= 133 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.saturation <= 75 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.289 {
if features.hue <= 57 {
if features.blue_chromaticity <= 0.285 {
if features.red_luminance <= 110 {
if features.red_difference <= 111 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.301 {
if features.saturation <= 80 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.288 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.416 {
if features.value <= 151 {
if features.green_luminance <= 149 {
if features.value <= 147 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 132 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 109 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
}
}
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_chromaticity <= 0.280 {
if features.green_luminance <= 122 {
if features.red_luminance <= 54 {
if features.red_luminance <= 52 {
if features.green_chromaticity <= 0.423 {
Intensity::High
} else {
if features.value <= 57 {
if features.red_difference <= 127 {
if features.blue_chromaticity <= 0.211 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.344 {
if features.red_chromaticity <= 0.337 {
if features.red_chromaticity <= 0.327 {
if features.green_chromaticity <= 0.431 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.luminance <= 56 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.426 {
Intensity::High
} else {
Intensity::High
}
}
} else {
Intensity::Low
}
}
}
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.280 {
if features.red_difference <= 114 {
if features.green_chromaticity <= 0.427 {
Intensity::High
} else {
if features.green_chromaticity <= 0.430 {
if features.green_chromaticity <= 0.429 {
if features.blue_luminance <= 74 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.425 {
if features.green_luminance <= 116 {
Intensity::Low
} else {
if features.green_luminance <= 117 {
if features.blue_luminance <= 74 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.425 {
Intensity::High
} else {
if features.intensity <= 63 {
if features.value <= 76 {
if features.red_chromaticity <= 0.324 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 104 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.luminance <= 105 {
if features.blue_luminance <= 57 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 121 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
}
}
} else {
Intensity::High
}
}
} else {
if features.blue_luminance <= 87 {
if features.blue_chromaticity <= 0.265 {
Intensity::High
} else {
if features.red_chromaticity <= 0.305 {
if features.green_chromaticity <= 0.427 {
if features.green_chromaticity <= 0.424 {
if features.green_chromaticity <= 0.423 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.430 {
if features.red_luminance <= 89 {
if features.red_chromaticity <= 0.302 {
if features.luminance <= 109 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 83 {
Intensity::High
} else {
if features.red_chromaticity <= 0.295 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.blue_luminance <= 84 {
if features.red_luminance <= 84 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.431 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_luminance <= 80 {
Intensity::High
} else {
if features.green_chromaticity <= 0.418 {
Intensity::High
} else {
if features.saturation <= 95 {
if features.blue_chromaticity <= 0.266 {
Intensity::High
} else {
if features.green_chromaticity <= 0.421 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.266 {
if features.blue_chromaticity <= 0.266 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
}
}
}
}
} else {
if features.green_chromaticity <= 0.427 {
if features.red_chromaticity <= 0.301 {
if features.saturation <= 90 {
if features.green_luminance <= 142 {
if features.red_chromaticity <= 0.300 {
if features.green_luminance <= 138 {
if features.blue_luminance <= 89 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 98 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.intensity <= 117 {
if features.intensity <= 112 {
Intensity::High
} else {
if features.blue_luminance <= 97 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.279 {
if features.blue_chromaticity <= 0.277 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.424 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_chromaticity <= 0.300 {
if features.green_luminance <= 144 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.blue_difference <= 109 {
if features.red_chromaticity <= 0.301 {
if features.value <= 149 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.268 {
if features.red_luminance <= 103 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 157 {
if features.red_luminance <= 106 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.426 {
if features.red_luminance <= 100 {
if features.red_chromaticity <= 0.302 {
if features.blue_difference <= 111 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 88 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.279 {
if features.blue_luminance <= 91 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.291 {
Intensity::High
} else {
if features.intensity <= 107 {
if features.green_chromaticity <= 0.430 {
if features.red_luminance <= 95 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 107 {
if features.green_luminance <= 152 {
if features.red_chromaticity <= 0.293 {
if features.red_difference <= 108 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_luminance <= 143 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.430 {
if features.saturation <= 90 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.291 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
Intensity::High
}
}
}
}
}
}
} else {
if features.blue_luminance <= 97 {
if features.blue_difference <= 114 {
if features.red_chromaticity <= 0.291 {
if features.green_chromaticity <= 0.430 {
if features.green_chromaticity <= 0.426 {
if features.green_chromaticity <= 0.425 {
Intensity::High
} else {
if features.intensity <= 108 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.287 {
if features.saturation <= 86 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.288 {
if features.blue_luminance <= 90 {
Intensity::High
} else {
if features.blue_difference <= 112 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 95 {
if features.red_luminance <= 94 {
if features.red_chromaticity <= 0.299 {
if features.green_chromaticity <= 0.422 {
if features.red_chromaticity <= 0.292 {
Intensity::High
} else {
if features.green_chromaticity <= 0.418 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 87 {
if features.red_luminance <= 91 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.292 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.intensity <= 101 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.424 {
if features.green_chromaticity <= 0.422 {
Intensity::High
} else {
if features.blue_luminance <= 92 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.302 {
if features.green_chromaticity <= 0.424 {
if features.red_chromaticity <= 0.298 {
if features.blue_chromaticity <= 0.284 {
if features.green_chromaticity <= 0.423 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.285 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.value <= 134 {
Intensity::Low
} else {
if features.blue_luminance <= 91 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.295 {
if features.red_chromaticity <= 0.291 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.425 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
}
} else {
Intensity::High
}
}
}
} else {
if features.red_luminance <= 92 {
if features.green_chromaticity <= 0.431 {
if features.green_luminance <= 132 {
if features.green_chromaticity <= 0.418 {
if features.red_chromaticity <= 0.289 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.saturation <= 90 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.430 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.294 {
Intensity::High
} else {
if features.saturation <= 84 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.288 {
if features.red_chromaticity <= 0.285 {
Intensity::High
} else {
if features.red_chromaticity <= 0.294 {
if features.red_chromaticity <= 0.294 {
if features.saturation <= 85 {
if features.blue_chromaticity <= 0.283 {
Intensity::High
} else {
if features.red_chromaticity <= 0.288 {
if features.blue_luminance <= 102 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.292 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.saturation <= 86 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.281 {
if features.green_luminance <= 150 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 87 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
Intensity::High
}
} else {
if features.saturation <= 85 {
if features.green_chromaticity <= 0.419 {
if features.green_chromaticity <= 0.419 {
if features.red_luminance <= 112 {
if features.blue_chromaticity <= 0.287 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.286 {
if features.blue_chromaticity <= 0.283 {
if features.green_luminance <= 157 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.421 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.280 {
Intensity::Low
} else {
if features.luminance <= 134 {
if features.green_chromaticity <= 0.425 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
}
}
}
}
} else {
if features.red_chromaticity <= 0.289 {
if features.red_difference <= 106 {
if features.green_chromaticity <= 0.431 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 85 {
if features.blue_chromaticity <= 0.299 {
if features.red_chromaticity <= 0.286 {
if features.value <= 145 {
Intensity::Low
} else {
if features.green_luminance <= 146 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 101 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.289 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
Intensity::High
}
} else {
if features.green_luminance <= 145 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.292 {
if features.luminance <= 133 {
Intensity::Low
} else {
if features.value <= 155 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 102 {
Intensity::Low
} else {
if features.intensity <= 121 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
}
}
}
}
} else {
if features.green_chromaticity <= 0.451 {
if features.blue_difference <= 108 {
if features.value <= 110 {
if features.green_chromaticity <= 0.447 {
if features.red_luminance <= 82 {
if features.value <= 93 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.211 {
Intensity::High
} else {
if features.intensity <= 71 {
Intensity::High
} else {
if features.green_chromaticity <= 0.442 {
if features.red_difference <= 117 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.232 {
if features.luminance <= 86 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
}
}
}
} else {
Intensity::High
}
} else {
if features.green_luminance <= 106 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 96 {
if features.green_luminance <= 152 {
if features.red_chromaticity <= 0.295 {
if features.hue <= 56 {
if features.green_chromaticity <= 0.434 {
Intensity::High
} else {
if features.value <= 148 {
if features.green_luminance <= 147 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.293 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.value <= 151 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 94 {
Intensity::High
} else {
if features.green_luminance <= 147 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.value <= 160 {
if features.red_luminance <= 99 {
Intensity::High
} else {
if features.red_luminance <= 107 {
if features.luminance <= 131 {
if features.value <= 153 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 101 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.275 {
if features.green_chromaticity <= 0.435 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.value <= 156 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.luminance <= 137 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.hue <= 57 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.blue_chromaticity <= 0.257 {
if features.value <= 144 {
if features.intensity <= 102 {
if features.red_chromaticity <= 0.295 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.446 {
if features.green_chromaticity <= 0.446 {
if features.value <= 132 {
if features.blue_luminance <= 62 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.intensity <= 101 {
Intensity::High
} else {
Intensity::High
}
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.450 {
if features.green_chromaticity <= 0.450 {
Intensity::High
} else {
if features.blue_luminance <= 73 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_difference <= 104 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.green_chromaticity <= 0.441 {
if features.red_chromaticity <= 0.323 {
if features.blue_luminance <= 78 {
if features.green_chromaticity <= 0.437 {
if features.saturation <= 108 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.438 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.439 {
if features.red_chromaticity <= 0.309 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.saturation <= 112 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.256 {
if features.red_chromaticity <= 0.298 {
if features.saturation <= 111 {
if features.green_chromaticity <= 0.449 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.255 {
if features.red_chromaticity <= 0.305 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.327 {
if features.blue_luminance <= 75 {
if features.saturation <= 123 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.255 {
if features.red_chromaticity <= 0.301 {
if features.red_chromaticity <= 0.301 {
if features.red_chromaticity <= 0.300 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.251 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.443 {
if features.green_chromaticity <= 0.440 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.256 {
if features.red_luminance <= 101 {
Intensity::High
} else {
if features.red_luminance <= 106 {
Intensity::High
} else {
Intensity::High
}
}
} else {
Intensity::High
}
}
}
} else {
if features.blue_luminance <= 81 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.436 {
if features.red_luminance <= 105 {
if features.green_chromaticity <= 0.432 {
if features.red_chromaticity <= 0.301 {
Intensity::High
} else {
if features.saturation <= 102 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_difference <= 107 {
if features.blue_difference <= 106 {
if features.red_chromaticity <= 0.303 {
if features.blue_chromaticity <= 0.263 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 129 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.intensity <= 110 {
if features.blue_chromaticity <= 0.260 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 150 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.300 {
Intensity::High
} else {
if features.blue_luminance <= 80 {
Intensity::High
} else {
if features.red_chromaticity <= 0.304 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.green_chromaticity <= 0.434 {
if features.intensity <= 121 {
if features.red_chromaticity <= 0.300 {
if features.saturation <= 98 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.305 {
if features.green_chromaticity <= 0.432 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.306 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.257 {
Intensity::Low
} else {
if features.red_luminance <= 88 {
Intensity::High
} else {
if features.green_chromaticity <= 0.446 {
if features.red_luminance <= 89 {
if features.saturation <= 104 {
if features.hue <= 55 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.luminance <= 134 {
if features.blue_luminance <= 96 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.intensity <= 115 {
if features.intensity <= 105 {
if features.luminance <= 118 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.449 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.luminance <= 134 {
if features.blue_chromaticity <= 0.262 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
}
}
}
}
}
}
}
} else {
if features.red_luminance <= 77 {
if features.green_chromaticity <= 0.440 {
if features.green_chromaticity <= 0.434 {
if features.green_chromaticity <= 0.433 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_luminance <= 76 {
if features.green_chromaticity <= 0.436 {
if features.green_chromaticity <= 0.435 {
if features.green_chromaticity <= 0.435 {
if features.red_luminance <= 63 {
if features.green_chromaticity <= 0.435 {
Intensity::Low
} else {
if features.blue_luminance <= 43 {
Intensity::High
} else {
Intensity::High
}
}
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.436 {
Intensity::High
} else {
if features.red_luminance <= 74 {
if features.green_luminance <= 58 {
Intensity::High
} else {
if features.green_luminance <= 71 {
Intensity::Low
} else {
if features.blue_luminance <= 46 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.saturation <= 94 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.267 {
Intensity::Low
} else {
if features.value <= 116 {
if features.blue_luminance <= 27 {
Intensity::High
} else {
if features.red_chromaticity <= 0.328 {
if features.hue <= 47 {
Intensity::High
} else {
if features.red_luminance <= 57 {
if features.luminance <= 56 {
if features.blue_chromaticity <= 0.239 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.445 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.279 {
if features.red_chromaticity <= 0.298 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 97 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.value <= 82 {
if features.green_chromaticity <= 0.450 {
if features.saturation <= 127 {
Intensity::Low
} else {
if features.blue_difference <= 113 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
Intensity::High
}
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.268 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.blue_luminance <= 95 {
if features.blue_luminance <= 85 {
if features.green_luminance <= 123 {
if features.green_chromaticity <= 0.440 {
if features.green_chromaticity <= 0.434 {
if features.blue_difference <= 110 {
Intensity::High
} else {
if features.saturation <= 91 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.299 {
Intensity::High
} else {
if features.intensity <= 86 {
Intensity::High
} else {
if features.green_chromaticity <= 0.438 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.259 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.blue_chromaticity <= 0.253 {
if features.luminance <= 100 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.green_luminance <= 134 {
if features.green_chromaticity <= 0.438 {
if features.blue_chromaticity <= 0.278 {
if features.green_chromaticity <= 0.437 {
if features.luminance <= 113 {
Intensity::High
} else {
if features.green_luminance <= 132 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.saturation <= 103 {
if features.green_luminance <= 131 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 82 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.278 {
if features.red_chromaticity <= 0.277 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.270 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.440 {
if features.intensity <= 103 {
if features.luminance <= 116 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.441 {
if features.blue_chromaticity <= 0.272 {
if features.value <= 138 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_difference <= 108 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.268 {
if features.green_luminance <= 137 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_difference <= 105 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
}
}
} else {
if features.blue_chromaticity <= 0.287 {
if features.blue_chromaticity <= 0.284 {
if features.value <= 135 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.280 {
if features.blue_chromaticity <= 0.274 {
if features.red_chromaticity <= 0.280 {
if features.value <= 144 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 99 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 89 {
Intensity::High
} else {
if features.red_chromaticity <= 0.288 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.red_difference <= 104 {
Intensity::High
} else {
if features.saturation <= 88 {
Intensity::High
} else {
if features.intensity <= 111 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
}
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 94 {
if features.luminance <= 110 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.290 {
if features.red_chromaticity <= 0.275 {
if features.red_luminance <= 88 {
if features.red_difference <= 105 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.saturation <= 98 {
if features.value <= 138 {
Intensity::High
} else {
if features.green_chromaticity <= 0.435 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
}
}
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.432 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.279 {
if features.green_chromaticity <= 0.436 {
if features.blue_chromaticity <= 0.277 {
if features.green_chromaticity <= 0.434 {
if features.red_chromaticity <= 0.290 {
Intensity::High
} else {
if features.green_chromaticity <= 0.433 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.435 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.luminance <= 133 {
if features.luminance <= 132 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 100 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
Intensity::High
}
} else {
if features.intensity <= 119 {
if features.green_chromaticity <= 0.446 {
if features.green_chromaticity <= 0.433 {
if features.blue_luminance <= 96 {
Intensity::High
} else {
if features.intensity <= 111 {
Intensity::High
} else {
if features.red_difference <= 105 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.436 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.281 {
if features.green_chromaticity <= 0.436 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
}
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.281 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
}
}
} else {
if features.green_chromaticity <= 0.457 {
if features.red_luminance <= 73 {
if features.blue_luminance <= 26 {
Intensity::High
} else {
if features.blue_luminance <= 32 {
if features.blue_chromaticity <= 0.221 {
if features.blue_luminance <= 30 {
if features.hue <= 46 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.454 {
if features.luminance <= 64 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.225 {
Intensity::High
} else {
if features.green_chromaticity <= 0.456 {
if features.green_chromaticity <= 0.451 {
if features.red_luminance <= 72 {
if features.blue_luminance <= 35 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
if features.saturation <= 106 {
if features.saturation <= 101 {
if features.red_difference <= 110 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.intensity <= 60 {
if features.red_luminance <= 49 {
if features.value <= 72 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.green_luminance <= 103 {
if features.hue <= 55 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.454 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
}
} else {
if features.red_luminance <= 64 {
if features.green_luminance <= 94 {
Intensity::High
} else {
if features.red_luminance <= 59 {
if features.saturation <= 107 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_difference <= 112 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
Intensity::High
}
}
}
}
}
} else {
if features.red_chromaticity <= 0.276 {
if features.red_chromaticity <= 0.275 {
if features.green_chromaticity <= 0.457 {
if features.red_luminance <= 85 {
if features.saturation <= 100 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 86 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.452 {
if features.value <= 147 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::High
}
}
}
} else {
if features.intensity <= 99 {
Intensity::Low
} else {
if features.red_luminance <= 85 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.455 {
if features.luminance <= 120 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::Low
}
}
} else {
if features.luminance <= 97 {
if features.blue_luminance <= 61 {
if features.hue <= 42 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.hue <= 52 {
if features.saturation <= 114 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_difference <= 108 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.luminance <= 116 {
if features.red_chromaticity <= 0.302 {
if features.blue_luminance <= 76 {
Intensity::High
} else {
if features.green_luminance <= 135 {
if features.red_difference <= 108 {
if features.red_chromaticity <= 0.282 {
if features.red_chromaticity <= 0.281 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.302 {
Intensity::Low
} else {
if features.saturation <= 118 {
if features.saturation <= 117 {
Intensity::High
} else {
if features.blue_luminance <= 69 {
Intensity::High
} else {
if features.blue_difference <= 104 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.305 {
if features.red_chromaticity <= 0.305 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_difference <= 111 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.luminance <= 123 {
if features.red_chromaticity <= 0.306 {
if features.saturation <= 112 {
if features.red_chromaticity <= 0.294 {
if features.red_chromaticity <= 0.281 {
Intensity::High
} else {
if features.red_chromaticity <= 0.283 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.294 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.455 {
if features.luminance <= 122 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.248 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.455 {
if features.saturation <= 119 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 92 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.blue_difference <= 101 {
if features.green_chromaticity <= 0.456 {
if features.red_chromaticity <= 0.315 {
if features.hue <= 49 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.453 {
Intensity::High
} else {
Intensity::High
}
}
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.235 {
if features.blue_chromaticity <= 0.234 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.451 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.237 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.blue_luminance <= 88 {
Intensity::High
} else {
if features.green_chromaticity <= 0.451 {
if features.blue_luminance <= 89 {
if features.intensity <= 113 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
Intensity::High
}
}
}
}
}
}
}
} else {
if features.saturation <= 121 {
if features.red_difference <= 110 {
if features.blue_luminance <= 91 {
if features.blue_chromaticity <= 0.261 {
if features.green_chromaticity <= 0.468 {
if features.saturation <= 114 {
if features.red_luminance <= 85 {
Intensity::High
} else {
if features.red_luminance <= 87 {
if features.green_luminance <= 141 {
if features.red_luminance <= 86 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.278 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_difference <= 102 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.460 {
if features.green_chromaticity <= 0.459 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.246 {
if features.intensity <= 101 {
if features.red_difference <= 107 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.luminance <= 119 {
if features.blue_luminance <= 67 {
if features.red_chromaticity <= 0.269 {
Intensity::High
} else {
if features.green_chromaticity <= 0.476 {
Intensity::High
} else {
Intensity::High
}
}
} else {
Intensity::High
}
} else {
if features.value <= 146 {
if features.red_difference <= 103 {
Intensity::High
} else {
if features.red_chromaticity <= 0.279 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
}
}
}
} else {
if features.blue_chromaticity <= 0.261 {
if features.value <= 123 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.261 {
if features.green_luminance <= 127 {
if features.red_luminance <= 70 {
if features.green_chromaticity <= 0.461 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.465 {
if features.saturation <= 110 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.green_luminance <= 128 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.value <= 135 {
Intensity::High
} else {
if features.red_chromaticity <= 0.268 {
if features.red_chromaticity <= 0.268 {
if features.green_luminance <= 137 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
}
}
}
} else {
if features.luminance <= 126 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 48 {
if features.red_chromaticity <= 0.285 {
Intensity::High
} else {
if features.red_chromaticity <= 0.286 {
if features.saturation <= 119 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 71 {
if features.green_chromaticity <= 0.466 {
if features.green_chromaticity <= 0.459 {
Intensity::High
} else {
if features.green_luminance <= 101 {
if features.luminance <= 77 {
if features.green_chromaticity <= 0.460 {
Intensity::High
} else {
if features.intensity <= 60 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.244 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.248 {
if features.red_chromaticity <= 0.290 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.286 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.green_chromaticity <= 0.471 {
if features.blue_luminance <= 49 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.265 {
if features.saturation <= 120 {
Intensity::High
} else {
if features.red_difference <= 111 {
if features.luminance <= 81 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.saturation <= 114 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
} else {
Intensity::High
}
}
}
} else {
if features.blue_chromaticity <= 0.280 {
if features.hue <= 40 {
if features.red_chromaticity <= 0.363 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.381 {
if features.blue_difference <= 113 {
if features.blue_difference <= 112 {
if features.value <= 50 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 47 {
if features.blue_chromaticity <= 0.111 {
Intensity::Low
} else {
if features.blue_luminance <= 10 {
Intensity::High
} else {
Intensity::High
}
}
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.intensity <= 16 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.503 {
if features.blue_difference <= 106 {
if features.blue_difference <= 104 {
if features.red_chromaticity <= 0.272 {
if features.red_chromaticity <= 0.272 {
Intensity::High
} else {
if features.blue_luminance <= 64 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.470 {
if features.green_chromaticity <= 0.470 {
if features.intensity <= 95 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_difference <= 102 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.hue <= 41 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_difference <= 108 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.243 {
if features.luminance <= 90 {
Intensity::High
} else {
if features.blue_luminance <= 50 {
Intensity::High
} else {
Intensity::High
}
}
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.468 {
if features.blue_chromaticity <= 0.244 {
if features.red_chromaticity <= 0.306 {
Intensity::High
} else {
if features.red_chromaticity <= 0.307 {
if features.luminance <= 81 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.458 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
Intensity::Low
}
} else {
if features.red_difference <= 112 {
if features.red_chromaticity <= 0.278 {
if features.blue_difference <= 116 {
if features.blue_chromaticity <= 0.245 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.486 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.blue_difference <= 108 {
if features.red_chromaticity <= 0.278 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.288 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_difference <= 121 {
if features.green_chromaticity <= 0.487 {
if features.green_chromaticity <= 0.487 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.saturation <= 156 {
if features.blue_chromaticity <= 0.187 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.491 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
}
} else {
if features.red_difference <= 121 {
if features.blue_chromaticity <= 0.204 {
if features.saturation <= 155 {
if features.value <= 56 {
if features.green_chromaticity <= 0.507 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.value <= 121 {
Intensity::High
} else {
if features.luminance <= 93 {
if features.green_chromaticity <= 0.553 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
}
} else {
if features.blue_chromaticity <= 0.204 {
if features.green_chromaticity <= 0.557 {
if features.green_chromaticity <= 0.531 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.intensity <= 29 {
if features.blue_luminance <= 18 {
Intensity::High
} else {
if features.saturation <= 188 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 50 {
if features.blue_difference <= 113 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 160 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.red_chromaticity <= 0.322 {
if features.red_chromaticity <= 0.321 {
if features.saturation <= 204 {
Intensity::High
} else {
if features.saturation <= 207 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.luminance <= 30 {
Intensity::High
} else {
Intensity::High
}
}
} else {
Intensity::High
}
}
}
}
} else {
if features.red_difference <= 107 {
if features.green_chromaticity <= 0.463 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::Low
}
}
}
}
}
}
} else {
if features.blue_difference <= 119 {
if features.green_chromaticity <= 0.448 {
if features.red_luminance <= 38 {
if features.hue <= 45 {
if features.intensity <= 34 {
if features.intensity <= 29 {
if features.red_difference <= 127 {
if features.red_chromaticity <= 0.362 {
Intensity::High
} else {
if features.intensity <= 28 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.213 {
if features.saturation <= 127 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 20 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.blue_chromaticity <= 0.217 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.435 {
if features.red_luminance <= 53 {
if features.red_chromaticity <= 0.361 {
if features.hue <= 39 {
Intensity::High
} else {
if features.green_chromaticity <= 0.422 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.423 {
Intensity::High
} else {
if features.red_difference <= 119 {
Intensity::Low
} else {
if features.hue <= 53 {
if features.value <= 57 {
if features.red_chromaticity <= 0.324 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.426 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
}
}
}
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.415 {
if features.red_chromaticity <= 0.336 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.336 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.337 {
if features.blue_chromaticity <= 0.303 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.415 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.287 {
if features.blue_chromaticity <= 0.286 {
Intensity::Low
} else {
if features.blue_difference <= 118 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 81 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.418 {
Intensity::Low
} else {
if features.blue_luminance <= 92 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
}
}
}
} else {
if features.red_chromaticity <= 0.296 {
if features.saturation <= 97 {
Intensity::High
} else {
if features.red_difference <= 106 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.265 {
Intensity::Low
} else {
if features.saturation <= 99 {
if features.red_chromaticity <= 0.277 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
}
}
} else {
Intensity::High
}
}
}
} else {
if features.red_luminance <= 46 {
if features.red_chromaticity <= 0.355 {
if features.saturation <= 117 {
if features.blue_difference <= 118 {
if features.red_luminance <= 38 {
if features.value <= 68 {
if features.blue_chromaticity <= 0.251 {
Intensity::Low
} else {
if features.blue_luminance <= 36 {
Intensity::High
} else {
if features.saturation <= 115 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
Intensity::High
}
} else {
if features.saturation <= 107 {
if features.value <= 73 {
if features.blue_chromaticity <= 0.263 {
Intensity::High
} else {
if features.green_chromaticity <= 0.455 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 32 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.blue_chromaticity <= 0.249 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.449 {
if features.red_difference <= 124 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.348 {
Intensity::High
} else {
if features.red_chromaticity <= 0.349 {
Intensity::Low
} else {
if features.intensity <= 18 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.red_chromaticity <= 0.387 {
if features.red_chromaticity <= 0.356 {
if features.red_chromaticity <= 0.355 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.466 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.intensity <= 56 {
Intensity::High
} else {
if features.intensity <= 74 {
if features.blue_luminance <= 52 {
if features.red_difference <= 113 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
}
}
} else {
if features.blue_difference <= 121 {
if features.green_chromaticity <= 0.437 {
if features.green_chromaticity <= 0.424 {
if features.red_luminance <= 40 {
if features.red_chromaticity <= 0.319 {
Intensity::High
} else {
if features.hue <= 37 {
Intensity::Low
} else {
if features.value <= 35 {
Intensity::High
} else {
if features.green_chromaticity <= 0.406 {
Intensity::High
} else {
if features.blue_luminance <= 30 {
if features.red_difference <= 125 {
Intensity::High
} else {
if features.red_luminance <= 32 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.green_chromaticity <= 0.420 {
if features.blue_chromaticity <= 0.306 {
if features.blue_chromaticity <= 0.306 {
if features.green_chromaticity <= 0.413 {
if features.red_chromaticity <= 0.298 {
if features.green_chromaticity <= 0.401 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.413 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 90 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.299 {
if features.green_chromaticity <= 0.422 {
if features.green_chromaticity <= 0.421 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
}
} else {
if features.red_luminance <= 32 {
Intensity::High
} else {
if features.blue_difference <= 120 {
if features.saturation <= 91 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.312 {
if features.green_chromaticity <= 0.433 {
if features.red_difference <= 111 {
Intensity::High
} else {
if features.hue <= 47 {
if features.green_luminance <= 42 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
}
}
} else {
if features.hue <= 39 {
Intensity::Low
} else {
if features.green_luminance <= 86 {
if features.value <= 29 {
if features.blue_chromaticity <= 0.185 {
if features.blue_luminance <= 2 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 19 {
if features.green_chromaticity <= 0.523 {
if features.value <= 27 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.192 {
if features.value <= 28 {
if features.hue <= 54 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.464 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.hue <= 65 {
if features.green_chromaticity <= 0.629 {
if features.green_chromaticity <= 0.458 {
if features.green_chromaticity <= 0.457 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.201 {
if features.luminance <= 26 {
Intensity::High
} else {
if features.value <= 34 {
Intensity::High
} else {
Intensity::High
}
}
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.631 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.233 {
if features.blue_chromaticity <= 0.261 {
if features.green_chromaticity <= 0.558 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.107 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.value <= 60 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
} else {
Intensity::Low
}
}
}
} else {
if features.blue_difference <= 123 {
if features.green_chromaticity <= 0.439 {
if features.green_chromaticity <= 0.417 {
if features.blue_luminance <= 61 {
if features.blue_luminance <= 60 {
if features.red_luminance <= 23 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.414 {
if features.green_luminance <= 76 {
if features.blue_chromaticity <= 0.296 {
if features.green_luminance <= 41 {
if features.red_luminance <= 36 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
if features.value <= 62 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.298 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.302 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.green_luminance <= 42 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.279 {
if features.red_chromaticity <= 0.267 {
Intensity::Low
} else {
if features.red_difference <= 118 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 24 {
if features.green_chromaticity <= 0.428 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.226 {
if features.red_difference <= 126 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.293 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.276 {
if features.intensity <= 26 {
if features.saturation <= 114 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 20 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.saturation <= 81 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
}
}
} else {
if features.green_luminance <= 24 {
if features.green_luminance <= 20 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.523 {
if features.intensity <= 14 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.215 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.hue <= 48 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.455 {
if features.blue_chromaticity <= 0.274 {
if features.green_chromaticity <= 0.453 {
if features.red_chromaticity <= 0.293 {
if features.saturation <= 101 {
Intensity::High
} else {
if features.red_luminance <= 24 {
Intensity::Low
} else {
if features.value <= 42 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.441 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.441 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.value <= 36 {
Intensity::Low
} else {
if features.hue <= 56 {
Intensity::High
} else {
if features.luminance <= 38 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
Intensity::High
}
} else {
if features.luminance <= 36 {
if features.red_chromaticity <= 0.305 {
if features.value <= 30 {
if features.luminance <= 22 {
Intensity::High
} else {
if features.hue <= 55 {
if features.luminance <= 23 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.243 {
if features.value <= 28 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
}
} else {
if features.red_difference <= 118 {
if features.red_chromaticity <= 0.211 {
if features.luminance <= 22 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.266 {
if features.red_chromaticity <= 0.227 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
Intensity::High
}
}
} else {
if features.red_difference <= 124 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.green_luminance <= 49 {
if features.hue <= 68 {
if features.value <= 45 {
if features.red_chromaticity <= 0.250 {
Intensity::High
} else {
if features.intensity <= 31 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 28 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
}
}
}
}
} else {
if features.red_difference <= 106 {
Intensity::High
} else {
if features.blue_difference <= 124 {
if features.red_chromaticity <= 0.284 {
if features.green_luminance <= 22 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.245 {
if features.green_luminance <= 27 {
if features.value <= 24 {
Intensity::High
} else {
if features.saturation <= 157 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 33 {
if features.blue_luminance <= 19 {
if features.blue_luminance <= 12 {
Intensity::High
} else {
if features.saturation <= 114 {
if features.blue_chromaticity <= 0.271 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.435 {
if features.green_chromaticity <= 0.433 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.408 {
if features.saturation <= 7 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 17 {
if features.saturation <= 107 {
if features.blue_chromaticity <= 0.260 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.296 {
if features.saturation <= 78 {
Intensity::High
} else {
if features.green_chromaticity <= 0.424 {
if features.red_luminance <= 26 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 20 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
Intensity::High
}
}
}
}
} else {
if features.luminance <= 22 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.479 {
if features.blue_chromaticity <= 0.302 {
if features.blue_chromaticity <= 0.299 {
if features.green_luminance <= 32 {
if features.blue_chromaticity <= 0.289 {
if features.green_chromaticity <= 0.411 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_difference <= 123 {
if features.green_chromaticity <= 0.456 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.value <= 34 {
if features.green_luminance <= 32 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 69 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.323 {
if features.red_chromaticity <= 0.217 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.322 {
if features.red_chromaticity <= 0.276 {
if features.blue_chromaticity <= 0.319 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.396 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.397 {
if features.blue_luminance <= 68 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.337 {
if features.blue_chromaticity <= 0.337 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.luminance <= 33 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
}
}
}
}
}
}
