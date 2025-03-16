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

#[allow(clippy::collapsible_else_if)]
pub fn predict(features: &Features) -> Intensity {
if features.blue_difference <= 114 {
if features.green_chromaticity <= 0.407 {
if features.green_chromaticity <= 0.393 {
if features.green_chromaticity <= 0.386 {
if features.green_chromaticity <= 0.382 {
if features.red_difference <= 125 {
if features.blue_luminance <= 141 {
if features.green_chromaticity <= 0.377 {
if features.value <= 161 {
if features.red_chromaticity <= 0.341 {
if features.red_chromaticity <= 0.341 {
if features.green_chromaticity <= 0.366 {
Intensity::Low
} else {
if features.saturation <= 56 {
if features.green_chromaticity <= 0.367 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.340 {
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
if features.blue_luminance <= 119 {
if features.saturation <= 54 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.345 {
if features.value <= 155 {
if features.luminance <= 147 {
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
if features.green_chromaticity <= 0.374 {
if features.red_luminance <= 165 {
if features.red_chromaticity <= 0.331 {
if features.red_chromaticity <= 0.331 {
if features.blue_chromaticity <= 0.297 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 162 {
if features.blue_chromaticity <= 0.294 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.green_luminance <= 178 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.296 {
if features.blue_chromaticity <= 0.296 {
if features.green_luminance <= 173 {
if features.red_chromaticity <= 0.327 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 135 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.375 {
if features.green_chromaticity <= 0.374 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.375 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.325 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.297 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
}
} else {
if features.blue_luminance <= 107 {
if features.green_chromaticity <= 0.377 {
Intensity::Low
} else {
if features.blue_difference <= 111 {
if features.hue <= 40 {
if features.red_chromaticity <= 0.348 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.348 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.340 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.346 {
if features.blue_luminance <= 94 {
if features.green_chromaticity <= 0.378 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 119 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.346 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_chromaticity <= 0.382 {
if features.intensity <= 153 {
if features.green_chromaticity <= 0.380 {
if features.green_chromaticity <= 0.377 {
Intensity::High
} else {
if features.luminance <= 133 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.284 {
if features.red_chromaticity <= 0.335 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.331 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_difference <= 104 {
Intensity::High
} else {
if features.green_chromaticity <= 0.379 {
if features.red_luminance <= 154 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.hue <= 41 {
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
if features.blue_luminance <= 145 {
if features.intensity <= 168 {
if features.green_chromaticity <= 0.378 {
if features.intensity <= 166 {
if features.green_chromaticity <= 0.372 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.373 {
Intensity::Low
} else {
if features.saturation <= 52 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.hue <= 41 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.296 {
if features.red_luminance <= 159 {
if features.red_chromaticity <= 0.325 {
if features.intensity <= 160 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 142 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.value <= 181 {
if features.blue_luminance <= 142 {
if features.red_luminance <= 155 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.hue <= 53 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 143 {
if features.red_luminance <= 154 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 144 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.337 {
if features.red_luminance <= 156 {
if features.saturation <= 52 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.316 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.301 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.317 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.value <= 219 {
if features.red_chromaticity <= 0.331 {
if features.intensity <= 163 {
if features.green_chromaticity <= 0.377 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 170 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.331 {
Intensity::Low
} else {
if features.red_luminance <= 163 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.329 {
if features.red_difference <= 118 {
if features.red_chromaticity <= 0.319 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 45 {
Intensity::Low
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
if features.red_chromaticity <= 0.337 {
Intensity::High
} else {
if features.red_chromaticity <= 0.337 {
if features.blue_chromaticity <= 0.301 {
if features.red_difference <= 122 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 217 {
if features.blue_luminance <= 184 {
if features.blue_difference <= 111 {
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
}
}
}
}
} else {
if features.green_luminance <= 184 {
if features.red_difference <= 126 {
if features.blue_difference <= 111 {
if features.value <= 179 {
if features.blue_luminance <= 109 {
if features.red_chromaticity <= 0.349 {
if features.green_chromaticity <= 0.376 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.376 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.351 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.351 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_difference <= 105 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.345 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.value <= 105 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_difference <= 128 {
if features.blue_difference <= 107 {
if features.green_chromaticity <= 0.374 {
if features.green_chromaticity <= 0.374 {
if features.value <= 174 {
if features.value <= 161 {
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
} else {
if features.red_chromaticity <= 0.357 {
if features.saturation <= 77 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::Low
}
}
} else {
if features.red_difference <= 127 {
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
if features.red_luminance <= 178 {
if features.hue <= 36 {
if features.saturation <= 49 {
Intensity::Low
} else {
if features.blue_luminance <= 139 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.296 {
if features.blue_chromaticity <= 0.296 {
if features.red_chromaticity <= 0.349 {
if features.red_chromaticity <= 0.349 {
if features.blue_chromaticity <= 0.296 {
if features.green_chromaticity <= 0.362 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_difference <= 126 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
Intensity::Low
}
} else {
if features.saturation <= 44 {
if features.blue_chromaticity <= 0.293 {
Intensity::High
} else {
if features.blue_difference <= 111 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.336 {
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
Intensity::Low
}
}
}
}
} else {
if features.blue_luminance <= 141 {
if features.red_difference <= 122 {
if features.blue_luminance <= 135 {
if features.blue_difference <= 112 {
if features.green_chromaticity <= 0.384 {
if features.red_chromaticity <= 0.337 {
if features.red_chromaticity <= 0.327 {
if features.red_chromaticity <= 0.327 {
if features.blue_luminance <= 129 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.383 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.value <= 145 {
if features.blue_luminance <= 105 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.382 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_difference <= 109 {
if features.intensity <= 151 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 146 {
if features.red_chromaticity <= 0.337 {
if features.green_chromaticity <= 0.386 {
if features.blue_luminance <= 113 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.287 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 137 {
if features.green_chromaticity <= 0.384 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.287 {
if features.blue_chromaticity <= 0.287 {
if features.green_chromaticity <= 0.386 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.328 {
if features.red_chromaticity <= 0.327 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 155 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.red_chromaticity <= 0.324 {
if features.blue_chromaticity <= 0.295 {
if features.blue_chromaticity <= 0.295 {
if features.saturation <= 62 {
if features.red_chromaticity <= 0.323 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.290 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_difference <= 113 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.hue <= 53 {
if features.red_chromaticity <= 0.320 {
if features.blue_chromaticity <= 0.296 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.383 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 57 {
if features.green_chromaticity <= 0.385 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 167 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.value <= 125 {
Intensity::Low
} else {
if features.value <= 135 {
if features.blue_luminance <= 95 {
if features.blue_luminance <= 92 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.281 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.288 {
if features.red_chromaticity <= 0.331 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 62 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
}
} else {
if features.blue_luminance <= 139 {
if features.saturation <= 56 {
if features.red_chromaticity <= 0.315 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.383 {
if features.intensity <= 153 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 144 {
Intensity::Low
} else {
if features.hue <= 54 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_luminance <= 183 {
if features.red_chromaticity <= 0.317 {
if features.luminance <= 163 {
if features.green_chromaticity <= 0.385 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.289 {
if features.blue_chromaticity <= 0.287 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.385 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_luminance <= 185 {
if features.blue_luminance <= 137 {
if features.green_chromaticity <= 0.384 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_luminance <= 157 {
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
if features.green_chromaticity <= 0.384 {
if features.red_luminance <= 149 {
if features.luminance <= 165 {
if features.red_chromaticity <= 0.317 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.327 {
if features.red_luminance <= 157 {
if features.red_chromaticity <= 0.324 {
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
}
} else {
if features.red_luminance <= 149 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.385 {
if features.hue <= 50 {
Intensity::Low
} else {
if features.hue <= 52 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 58 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.294 {
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
} else {
if features.red_chromaticity <= 0.344 {
if features.blue_difference <= 111 {
if features.blue_luminance <= 97 {
if features.green_chromaticity <= 0.385 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.272 {
Intensity::High
} else {
if features.saturation <= 74 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.278 {
if features.red_chromaticity <= 0.341 {
if features.red_chromaticity <= 0.340 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.342 {
if features.red_chromaticity <= 0.341 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.343 {
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
} else {
if features.red_luminance <= 119 {
if features.red_chromaticity <= 0.336 {
if features.value <= 123 {
if features.intensity <= 106 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.336 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.385 {
if features.red_chromaticity <= 0.341 {
if features.green_chromaticity <= 0.384 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.341 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.386 {
if features.red_chromaticity <= 0.340 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
}
} else {
if features.blue_luminance <= 99 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.280 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_chromaticity <= 0.349 {
if features.blue_chromaticity <= 0.267 {
if features.blue_luminance <= 86 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.346 {
if features.green_luminance <= 106 {
if features.green_chromaticity <= 0.386 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.269 {
if features.intensity <= 103 {
if features.red_chromaticity <= 0.349 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_difference <= 110 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.270 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.270 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.red_chromaticity <= 0.349 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_luminance <= 144 {
if features.blue_luminance <= 143 {
if features.red_chromaticity <= 0.319 {
if features.red_chromaticity <= 0.317 {
if features.green_luminance <= 181 {
Intensity::Low
} else {
if features.luminance <= 168 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_difference <= 112 {
Intensity::Low
} else {
if features.red_difference <= 115 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.294 {
if features.green_chromaticity <= 0.384 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.288 {
Intensity::Low
} else {
if features.blue_difference <= 110 {
if features.red_chromaticity <= 0.325 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
}
} else {
if features.blue_luminance <= 142 {
Intensity::Low
} else {
if features.intensity <= 161 {
if features.saturation <= 57 {
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
} else {
if features.green_chromaticity <= 0.384 {
if features.luminance <= 171 {
if features.hue <= 54 {
if features.red_luminance <= 153 {
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
if features.value <= 190 {
if features.blue_chromaticity <= 0.296 {
Intensity::Low
} else {
if features.blue_difference <= 113 {
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
if features.intensity <= 162 {
if features.blue_chromaticity <= 0.299 {
Intensity::Low
} else {
if features.red_luminance <= 152 {
if features.red_chromaticity <= 0.313 {
Intensity::Low
} else {
if features.luminance <= 171 {
if features.hue <= 55 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.green_luminance <= 185 {
Intensity::Low
} else {
if features.blue_luminance <= 145 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.315 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.383 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
}
} else {
if features.red_difference <= 114 {
if features.green_chromaticity <= 0.385 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.385 {
if features.red_chromaticity <= 0.310 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 176 {
if features.green_luminance <= 191 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.318 {
if features.green_luminance <= 188 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 205 {
if features.blue_chromaticity <= 0.299 {
if features.green_chromaticity <= 0.385 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.296 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.382 {
Intensity::Low
} else {
if features.blue_difference <= 112 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.386 {
Intensity::Low
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
if features.intensity <= 154 {
if features.red_difference <= 121 {
if features.saturation <= 61 {
if features.value <= 176 {
if features.green_chromaticity <= 0.387 {
if features.red_chromaticity <= 0.319 {
if features.blue_luminance <= 123 {
Intensity::Low
} else {
if features.blue_luminance <= 133 {
if features.green_chromaticity <= 0.387 {
if features.blue_luminance <= 127 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_difference <= 113 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 59 {
if features.saturation <= 58 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_difference <= 112 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.hue <= 51 {
Intensity::Low
} else {
if features.saturation <= 60 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.299 {
if features.blue_chromaticity <= 0.298 {
if features.blue_difference <= 112 {
Intensity::Low
} else {
if features.red_luminance <= 129 {
if features.luminance <= 145 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 146 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.314 {
if features.green_luminance <= 168 {
if features.blue_luminance <= 127 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 169 {
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
if features.luminance <= 159 {
if features.luminance <= 158 {
if features.value <= 173 {
Intensity::Low
} else {
if features.red_difference <= 112 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.392 {
if features.green_chromaticity <= 0.390 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 58 {
if features.green_chromaticity <= 0.389 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.hue <= 57 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.blue_chromaticity <= 0.298 {
if features.hue <= 52 {
if features.red_chromaticity <= 0.319 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.297 {
if features.blue_chromaticity <= 0.295 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.387 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.296 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.297 {
Intensity::Low
} else {
if features.blue_luminance <= 135 {
Intensity::Low
} else {
if features.value <= 178 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.green_chromaticity <= 0.390 {
if features.red_chromaticity <= 0.311 {
if features.red_difference <= 113 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 153 {
if features.saturation <= 58 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.299 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.308 {
if features.red_difference <= 111 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 137 {
if features.red_chromaticity <= 0.309 {
if features.blue_luminance <= 136 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 179 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_difference <= 112 {
if features.green_chromaticity <= 0.391 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.hue <= 56 {
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
} else {
if features.blue_difference <= 112 {
if features.intensity <= 149 {
if features.green_chromaticity <= 0.390 {
if features.red_chromaticity <= 0.324 {
if features.green_chromaticity <= 0.388 {
if features.blue_chromaticity <= 0.294 {
if features.green_chromaticity <= 0.387 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.318 {
if features.green_chromaticity <= 0.390 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.290 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.388 {
if features.blue_difference <= 108 {
Intensity::Low
} else {
if features.blue_luminance <= 118 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 148 {
if features.blue_chromaticity <= 0.284 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 71 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.blue_difference <= 111 {
if features.blue_luminance <= 122 {
if features.red_luminance <= 136 {
if features.red_luminance <= 127 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 168 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.luminance <= 155 {
if features.green_chromaticity <= 0.392 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.282 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_luminance <= 139 {
if features.value <= 162 {
if features.red_chromaticity <= 0.317 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.317 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_difference <= 114 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.luminance <= 163 {
if features.green_chromaticity <= 0.391 {
if features.red_chromaticity <= 0.330 {
if features.intensity <= 150 {
if features.value <= 174 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.290 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.333 {
if features.green_luminance <= 175 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.hue <= 54 {
if features.red_luminance <= 144 {
if features.red_chromaticity <= 0.316 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 126 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.393 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.392 {
if features.intensity <= 151 {
Intensity::High
} else {
if features.green_luminance <= 180 {
if features.hue <= 47 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 71 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.293 {
if features.red_chromaticity <= 0.328 {
if features.luminance <= 166 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.393 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.green_luminance <= 180 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.315 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
}
} else {
if features.red_chromaticity <= 0.328 {
if features.green_chromaticity <= 0.390 {
if features.green_chromaticity <= 0.386 {
if features.hue <= 48 {
Intensity::Low
} else {
if features.intensity <= 130 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 125 {
if features.green_chromaticity <= 0.389 {
if features.red_luminance <= 117 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.318 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 64 {
if features.red_luminance <= 135 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 140 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_chromaticity <= 0.315 {
if features.value <= 159 {
if features.green_chromaticity <= 0.393 {
if features.green_chromaticity <= 0.392 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 64 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_luminance <= 177 {
if features.green_luminance <= 172 {
Intensity::Low
} else {
Intensity::Low
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
if features.blue_chromaticity <= 0.290 {
if features.red_chromaticity <= 0.322 {
if features.red_luminance <= 122 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 70 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.316 {
if features.green_luminance <= 159 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.318 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.value <= 124 {
if features.blue_luminance <= 88 {
if features.value <= 122 {
if features.saturation <= 73 {
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
if features.intensity <= 114 {
if features.green_chromaticity <= 0.390 {
if features.red_chromaticity <= 0.330 {
if features.hue <= 46 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.389 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 108 {
if features.value <= 127 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.391 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.328 {
if features.green_luminance <= 144 {
if features.blue_chromaticity <= 0.285 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.intensity <= 116 {
if features.red_luminance <= 114 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.387 {
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
} else {
if features.red_difference <= 123 {
if features.blue_difference <= 111 {
if features.blue_chromaticity <= 0.267 {
if features.green_chromaticity <= 0.390 {
Intensity::High
} else {
if features.red_chromaticity <= 0.344 {
if features.green_luminance <= 133 {
Intensity::Low
} else {
if features.blue_luminance <= 95 {
if features.green_chromaticity <= 0.391 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.341 {
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
} else {
if features.blue_difference <= 108 {
if features.blue_luminance <= 113 {
if features.red_chromaticity <= 0.343 {
Intensity::High
} else {
if features.red_chromaticity <= 0.343 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.389 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.386 {
Intensity::High
} else {
if features.luminance <= 153 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_luminance <= 123 {
Intensity::High
} else {
if features.green_chromaticity <= 0.386 {
Intensity::High
} else {
if features.red_chromaticity <= 0.339 {
if features.red_chromaticity <= 0.339 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.342 {
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
if features.value <= 122 {
if features.green_chromaticity <= 0.386 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.337 {
if features.blue_chromaticity <= 0.273 {
if features.intensity <= 100 {
if features.green_chromaticity <= 0.392 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.389 {
if features.green_chromaticity <= 0.389 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.276 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.391 {
if features.intensity <= 101 {
Intensity::Low
} else {
if features.value <= 120 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.391 {
if features.red_chromaticity <= 0.337 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.270 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.blue_difference <= 112 {
if features.green_chromaticity <= 0.393 {
if features.intensity <= 108 {
if features.green_chromaticity <= 0.388 {
Intensity::Low
} else {
if features.red_luminance <= 106 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.387 {
if features.green_chromaticity <= 0.386 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.390 {
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
if features.green_chromaticity <= 0.389 {
if features.green_chromaticity <= 0.388 {
if features.red_luminance <= 109 {
if features.green_luminance <= 123 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.387 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 105 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.335 {
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
} else {
if features.red_difference <= 125 {
if features.blue_luminance <= 89 {
if features.red_luminance <= 102 {
if features.green_chromaticity <= 0.390 {
if features.green_chromaticity <= 0.386 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.347 {
if features.blue_difference <= 112 {
if features.red_chromaticity <= 0.343 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 90 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.hue <= 39 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.272 {
if features.intensity <= 108 {
if features.saturation <= 84 {
if features.red_chromaticity <= 0.348 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.390 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.272 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.272 {
if features.blue_chromaticity <= 0.272 {
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
} else {
if features.hue <= 40 {
if features.value <= 134 {
if features.blue_chromaticity <= 0.263 {
if features.red_luminance <= 120 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.386 {
if features.green_luminance <= 141 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 101 {
if features.saturation <= 88 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 130 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_chromaticity <= 0.345 {
if features.blue_chromaticity <= 0.270 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.342 {
if features.value <= 130 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 132 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_luminance <= 123 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
} else {
if features.red_luminance <= 79 {
if features.saturation <= 91 {
if features.red_chromaticity <= 0.358 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_difference <= 111 {
if features.value <= 74 {
if features.saturation <= 129 {
if features.green_chromaticity <= 0.389 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.luminance <= 44 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.356 {
if features.red_chromaticity <= 0.356 {
if features.red_chromaticity <= 0.356 {
if features.blue_difference <= 109 {
if features.blue_difference <= 108 {
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
Intensity::High
}
} else {
if features.red_chromaticity <= 0.363 {
if features.red_chromaticity <= 0.362 {
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
if features.intensity <= 159 {
if features.blue_luminance <= 141 {
if features.intensity <= 157 {
if features.blue_chromaticity <= 0.298 {
if features.red_luminance <= 145 {
Intensity::Low
} else {
if features.green_luminance <= 180 {
if features.saturation <= 76 {
if features.blue_chromaticity <= 0.290 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.387 {
if features.red_luminance <= 152 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_difference <= 116 {
if features.blue_chromaticity <= 0.290 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_difference <= 117 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.red_chromaticity <= 0.309 {
if features.value <= 183 {
if features.red_difference <= 111 {
Intensity::Low
} else {
if features.blue_difference <= 113 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.301 {
if features.green_chromaticity <= 0.388 {
if features.blue_chromaticity <= 0.299 {
Intensity::Low
} else {
if features.blue_difference <= 113 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_difference <= 112 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.298 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.saturation <= 56 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_chromaticity <= 0.392 {
if features.luminance <= 169 {
if features.value <= 184 {
if features.blue_chromaticity <= 0.291 {
Intensity::Low
} else {
if features.hue <= 53 {
if features.saturation <= 60 {
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
if features.red_chromaticity <= 0.328 {
if features.red_chromaticity <= 0.323 {
if features.green_chromaticity <= 0.389 {
if features.blue_chromaticity <= 0.292 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 137 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 68 {
if features.blue_chromaticity <= 0.286 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 155 {
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
} else {
if features.red_luminance <= 152 {
if features.blue_chromaticity <= 0.294 {
if features.blue_luminance <= 137 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.392 {
Intensity::Low
} else {
if features.saturation <= 65 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_luminance <= 187 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_luminance <= 186 {
Intensity::Low
} else {
if features.blue_difference <= 108 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.green_chromaticity <= 0.389 {
if features.red_chromaticity <= 0.309 {
if features.red_luminance <= 146 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.313 {
if features.blue_chromaticity <= 0.302 {
if features.value <= 182 {
Intensity::Low
} else {
if features.blue_difference <= 113 {
Intensity::Low
} else {
if features.green_luminance <= 183 {
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
if features.intensity <= 158 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.387 {
if features.red_chromaticity <= 0.316 {
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
} else {
if features.luminance <= 168 {
if features.value <= 184 {
if features.red_luminance <= 144 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.saturation <= 60 {
if features.green_luminance <= 185 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.392 {
if features.blue_luminance <= 142 {
if features.blue_chromaticity <= 0.298 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.308 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.307 {
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
} else {
if features.blue_luminance <= 142 {
if features.luminance <= 173 {
if features.saturation <= 72 {
if features.green_chromaticity <= 0.388 {
if features.blue_luminance <= 137 {
Intensity::Low
} else {
if features.saturation <= 61 {
Intensity::Low
} else {
if features.hue <= 50 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.saturation <= 62 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.295 {
if features.red_difference <= 116 {
if features.value <= 187 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.326 {
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
} else {
if features.blue_chromaticity <= 0.287 {
if features.green_chromaticity <= 0.393 {
if features.red_chromaticity <= 0.324 {
if features.green_chromaticity <= 0.390 {
Intensity::High
} else {
if features.red_chromaticity <= 0.322 {
Intensity::Low
} else {
if features.red_luminance <= 157 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_luminance <= 141 {
if features.green_luminance <= 189 {
if features.saturation <= 69 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.390 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
Intensity::High
}
} else {
if features.value <= 190 {
Intensity::Low
} else {
if features.blue_luminance <= 141 {
Intensity::Low
} else {
if features.blue_difference <= 109 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.luminance <= 173 {
if features.blue_chromaticity <= 0.300 {
if features.red_chromaticity <= 0.314 {
if features.red_luminance <= 149 {
if features.green_chromaticity <= 0.391 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.392 {
if features.blue_chromaticity <= 0.298 {
if features.red_difference <= 112 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.299 {
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
if features.intensity <= 161 {
if features.luminance <= 171 {
Intensity::Low
} else {
if features.hue <= 54 {
if features.blue_difference <= 111 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 154 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.303 {
if features.blue_chromaticity <= 0.302 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.387 {
if features.green_chromaticity <= 0.387 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.304 {
if features.red_chromaticity <= 0.309 {
if features.green_luminance <= 188 {
if features.red_chromaticity <= 0.307 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 189 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.hue <= 57 {
Intensity::Low
} else {
if features.saturation <= 55 {
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
} else {
if features.blue_luminance <= 153 {
if features.blue_chromaticity <= 0.305 {
if features.intensity <= 171 {
if features.red_luminance <= 150 {
if features.green_chromaticity <= 0.392 {
if features.saturation <= 58 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.305 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.391 {
if features.green_chromaticity <= 0.390 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.393 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.saturation <= 61 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.390 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.390 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_chromaticity <= 0.306 {
if features.value <= 195 {
if features.saturation <= 57 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.393 {
if features.value <= 195 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.295 {
if features.red_luminance <= 165 {
if features.red_luminance <= 164 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.saturation <= 52 {
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
} else {
if features.red_difference <= 121 {
if features.saturation <= 70 {
if features.blue_luminance <= 133 {
if features.blue_difference <= 112 {
if features.blue_luminance <= 129 {
if features.green_chromaticity <= 0.396 {
if features.blue_luminance <= 116 {
if features.red_luminance <= 128 {
if features.blue_chromaticity <= 0.285 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.396 {
if features.red_chromaticity <= 0.316 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.320 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.294 {
if features.red_chromaticity <= 0.314 {
if features.green_chromaticity <= 0.396 {
if features.luminance <= 151 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.hue <= 53 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.319 {
if features.saturation <= 69 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.hue <= 50 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.310 {
if features.value <= 171 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.saturation <= 64 {
Intensity::Low
} else {
if features.red_difference <= 113 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.red_difference <= 112 {
if features.blue_luminance <= 128 {
if features.blue_chromaticity <= 0.291 {
if features.green_chromaticity <= 0.400 {
if features.luminance <= 155 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.402 {
if features.red_chromaticity <= 0.308 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.295 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.hue <= 59 {
if features.green_luminance <= 175 {
if features.red_chromaticity <= 0.306 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_luminance <= 131 {
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
if features.green_chromaticity <= 0.396 {
if features.red_luminance <= 128 {
Intensity::High
} else {
if features.saturation <= 68 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.intensity <= 131 {
if features.red_chromaticity <= 0.314 {
if features.blue_chromaticity <= 0.289 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.intensity <= 129 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_difference <= 113 {
if features.green_chromaticity <= 0.398 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.398 {
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
if features.blue_luminance <= 132 {
if features.intensity <= 151 {
if features.blue_chromaticity <= 0.293 {
if features.green_chromaticity <= 0.402 {
if features.red_chromaticity <= 0.313 {
if features.green_chromaticity <= 0.397 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.394 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 148 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.saturation <= 69 {
if features.green_chromaticity <= 0.394 {
if features.red_chromaticity <= 0.311 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.406 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.295 {
if features.blue_luminance <= 130 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.296 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_luminance <= 142 {
Intensity::Low
} else {
if features.value <= 181 {
if features.green_chromaticity <= 0.395 {
Intensity::Low
} else {
if features.value <= 180 {
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
if features.red_luminance <= 143 {
if features.red_luminance <= 135 {
if features.red_difference <= 108 {
if features.green_chromaticity <= 0.405 {
Intensity::Low
} else {
if features.hue <= 60 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.401 {
if features.green_chromaticity <= 0.394 {
if features.red_luminance <= 141 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 136 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.403 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.saturation <= 68 {
Intensity::Low
} else {
if features.intensity <= 154 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.saturation <= 67 {
if features.saturation <= 65 {
if features.blue_difference <= 113 {
if features.green_chromaticity <= 0.394 {
if features.blue_luminance <= 131 {
if features.intensity <= 144 {
if features.red_chromaticity <= 0.313 {
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
if features.red_luminance <= 135 {
if features.blue_luminance <= 132 {
if features.green_chromaticity <= 0.397 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.396 {
if features.green_chromaticity <= 0.395 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.396 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_chromaticity <= 0.401 {
if features.red_luminance <= 122 {
if features.intensity <= 130 {
if features.green_luminance <= 148 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.306 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.305 {
if features.red_chromaticity <= 0.302 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.397 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_luminance <= 125 {
if features.red_chromaticity <= 0.300 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
}
} else {
if features.value <= 158 {
if features.green_chromaticity <= 0.401 {
if features.green_chromaticity <= 0.396 {
if features.blue_chromaticity <= 0.290 {
if features.blue_chromaticity <= 0.290 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_difference <= 115 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.399 {
if features.green_chromaticity <= 0.398 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 156 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.295 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.401 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.299 {
if features.green_chromaticity <= 0.400 {
if features.red_chromaticity <= 0.306 {
if features.red_chromaticity <= 0.305 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.397 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 128 {
if features.luminance <= 142 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.401 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_luminance <= 174 {
if features.green_chromaticity <= 0.402 {
if features.green_chromaticity <= 0.401 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 169 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.297 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.403 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
}
} else {
if features.red_difference <= 108 {
if features.blue_luminance <= 130 {
if features.blue_chromaticity <= 0.298 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.294 {
Intensity::Low
} else {
if features.luminance <= 154 {
if features.blue_chromaticity <= 0.301 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 175 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_chromaticity <= 0.296 {
if features.green_chromaticity <= 0.405 {
if features.green_chromaticity <= 0.403 {
Intensity::Low
} else {
if features.luminance <= 156 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.406 {
if features.luminance <= 159 {
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
}
} else {
if features.value <= 159 {
if features.red_luminance <= 111 {
if features.red_chromaticity <= 0.319 {
if features.green_chromaticity <= 0.396 {
if features.luminance <= 124 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.400 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.394 {
if features.green_luminance <= 134 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.287 {
if features.saturation <= 69 {
if features.green_chromaticity <= 0.394 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.395 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.299 {
if features.green_chromaticity <= 0.405 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.293 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_chromaticity <= 0.297 {
if features.red_chromaticity <= 0.302 {
if features.blue_chromaticity <= 0.295 {
if features.green_luminance <= 162 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.404 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.305 {
if features.green_chromaticity <= 0.402 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.402 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.luminance <= 150 {
if features.green_chromaticity <= 0.404 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.406 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 139 {
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
} else {
if features.blue_luminance <= 139 {
if features.red_chromaticity <= 0.299 {
if features.red_chromaticity <= 0.296 {
if features.intensity <= 149 {
if features.green_luminance <= 178 {
Intensity::Low
} else {
if features.value <= 180 {
if features.intensity <= 147 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.402 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.301 {
if features.green_luminance <= 181 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.296 {
if features.blue_luminance <= 138 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.403 {
Intensity::Low
} else {
if features.red_luminance <= 134 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_luminance <= 180 {
if features.red_luminance <= 134 {
if features.green_chromaticity <= 0.399 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.401 {
if features.green_chromaticity <= 0.399 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.299 {
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
if features.red_luminance <= 137 {
if features.red_difference <= 107 {
if features.blue_chromaticity <= 0.299 {
if features.hue <= 60 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.404 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.303 {
if features.green_chromaticity <= 0.402 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.299 {
if features.blue_luminance <= 138 {
if features.green_luminance <= 188 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_difference <= 111 {
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
} else {
if features.blue_luminance <= 135 {
if features.green_chromaticity <= 0.398 {
if features.blue_chromaticity <= 0.287 {
if features.luminance <= 168 {
if features.hue <= 51 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.319 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 136 {
if features.green_luminance <= 176 {
Intensity::Low
} else {
if features.green_luminance <= 177 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.397 {
if features.blue_chromaticity <= 0.293 {
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
if features.green_chromaticity <= 0.402 {
if features.green_chromaticity <= 0.400 {
if features.green_chromaticity <= 0.398 {
if features.blue_difference <= 111 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.294 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.401 {
if features.blue_difference <= 110 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.402 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.403 {
Intensity::Low
} else {
if features.red_difference <= 108 {
if features.blue_chromaticity <= 0.295 {
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
} else {
if features.luminance <= 169 {
if features.saturation <= 63 {
if features.blue_chromaticity <= 0.299 {
if features.green_chromaticity <= 0.393 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.296 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.302 {
if features.green_chromaticity <= 0.397 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.394 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_luminance <= 137 {
if features.red_luminance <= 145 {
if features.red_chromaticity <= 0.305 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.314 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.294 {
Intensity::Low
} else {
if features.green_luminance <= 188 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.hue <= 56 {
if features.luminance <= 171 {
if features.red_chromaticity <= 0.313 {
if features.value <= 189 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.316 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 137 {
Intensity::Low
} else {
if features.intensity <= 159 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.hue <= 57 {
if features.green_chromaticity <= 0.403 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.intensity <= 156 {
Intensity::Low
} else {
if features.value <= 191 {
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
} else {
if features.value <= 191 {
if features.blue_luminance <= 141 {
if features.red_chromaticity <= 0.299 {
if features.hue <= 59 {
Intensity::Low
} else {
if features.blue_difference <= 113 {
if features.green_luminance <= 188 {
if features.green_chromaticity <= 0.402 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.401 {
if features.value <= 184 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.402 {
if features.blue_chromaticity <= 0.292 {
if features.value <= 189 {
Intensity::Low
} else {
if features.green_luminance <= 190 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.399 {
if features.saturation <= 61 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.301 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.402 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_luminance <= 186 {
if features.blue_chromaticity <= 0.304 {
if features.saturation <= 59 {
if features.blue_chromaticity <= 0.303 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.hue <= 61 {
if features.green_chromaticity <= 0.395 {
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
if features.blue_chromaticity <= 0.301 {
if features.red_chromaticity <= 0.298 {
Intensity::Low
} else {
if features.value <= 189 {
if features.green_chromaticity <= 0.395 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_difference <= 111 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.intensity <= 155 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.300 {
if features.value <= 190 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.304 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
}
} else {
if features.value <= 213 {
if features.green_luminance <= 210 {
if features.blue_luminance <= 142 {
if features.luminance <= 172 {
if features.red_chromaticity <= 0.300 {
if features.green_chromaticity <= 0.404 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.303 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 69 {
if features.saturation <= 66 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 193 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_luminance <= 193 {
if features.saturation <= 64 {
if features.saturation <= 61 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.401 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 163 {
if features.saturation <= 64 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 59 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.saturation <= 66 {
if features.value <= 211 {
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
}
}
} else {
if features.intensity <= 149 {
if features.value <= 123 {
if features.green_luminance <= 118 {
if features.green_luminance <= 113 {
if features.blue_chromaticity <= 0.269 {
if features.intensity <= 91 {
if features.green_chromaticity <= 0.406 {
if features.green_chromaticity <= 0.404 {
if features.intensity <= 90 {
Intensity::Low
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
if features.red_difference <= 120 {
Intensity::Low
} else {
if features.luminance <= 96 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.406 {
if features.value <= 112 {
if features.saturation <= 84 {
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
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.406 {
if features.red_chromaticity <= 0.327 {
if features.red_luminance <= 90 {
if features.intensity <= 90 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.327 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.273 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_difference <= 119 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.402 {
if features.value <= 117 {
if features.blue_luminance <= 78 {
if features.blue_luminance <= 77 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 79 {
if features.intensity <= 96 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.277 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.saturation <= 81 {
if features.intensity <= 98 {
if features.green_chromaticity <= 0.400 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.329 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.270 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.saturation <= 89 {
if features.value <= 116 {
if features.blue_chromaticity <= 0.271 {
if features.blue_chromaticity <= 0.266 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 80 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.hue <= 50 {
if features.luminance <= 106 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.278 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.value <= 115 {
Intensity::High
} else {
if features.saturation <= 91 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.green_chromaticity <= 0.403 {
if features.saturation <= 85 {
if features.blue_chromaticity <= 0.281 {
if features.red_luminance <= 99 {
if features.blue_chromaticity <= 0.270 {
Intensity::High
} else {
if features.green_chromaticity <= 0.401 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 84 {
if features.intensity <= 101 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.275 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_luminance <= 86 {
if features.hue <= 50 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 100 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.276 {
if features.saturation <= 90 {
if features.green_chromaticity <= 0.406 {
if features.blue_chromaticity <= 0.265 {
if features.saturation <= 89 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.322 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.325 {
if features.blue_chromaticity <= 0.273 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.406 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.317 {
if features.blue_luminance <= 84 {
if features.red_difference <= 117 {
Intensity::Low
} else {
if features.red_luminance <= 93 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.luminance <= 110 {
if features.luminance <= 109 {
Intensity::High
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
} else {
if features.green_chromaticity <= 0.399 {
if features.red_luminance <= 124 {
if features.green_chromaticity <= 0.396 {
if features.green_chromaticity <= 0.395 {
if features.value <= 146 {
if features.red_chromaticity <= 0.334 {
if features.red_luminance <= 117 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.value <= 149 {
if features.blue_chromaticity <= 0.284 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 96 {
if features.red_luminance <= 107 {
if features.red_luminance <= 104 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 109 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 103 {
if features.green_chromaticity <= 0.395 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.395 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_chromaticity <= 0.287 {
if features.green_chromaticity <= 0.398 {
if features.blue_luminance <= 93 {
if features.red_chromaticity <= 0.332 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.saturation <= 80 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.blue_luminance <= 97 {
if features.green_luminance <= 136 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_difference <= 110 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.314 {
if features.green_chromaticity <= 0.399 {
Intensity::High
} else {
if features.hue <= 53 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.287 {
Intensity::Low
} else {
if features.red_difference <= 115 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.red_luminance <= 141 {
if features.blue_difference <= 108 {
if features.blue_chromaticity <= 0.280 {
if features.red_difference <= 120 {
if features.red_luminance <= 126 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.336 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.value <= 171 {
if features.intensity <= 141 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 140 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.396 {
if features.red_chromaticity <= 0.322 {
if features.luminance <= 154 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.intensity <= 138 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 139 {
if features.green_luminance <= 159 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.320 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
} else {
if features.luminance <= 162 {
if features.green_chromaticity <= 0.395 {
if features.blue_luminance <= 119 {
if features.red_luminance <= 142 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 144 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.330 {
if features.intensity <= 147 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.267 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.red_luminance <= 151 {
if features.red_difference <= 116 {
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
} else {
if features.blue_difference <= 111 {
if features.green_chromaticity <= 0.404 {
if features.blue_difference <= 109 {
if features.red_luminance <= 140 {
if features.green_chromaticity <= 0.401 {
if features.blue_luminance <= 100 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.337 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.323 {
if features.value <= 180 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 116 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.401 {
if features.red_chromaticity <= 0.329 {
if features.red_chromaticity <= 0.327 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.luminance <= 121 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.value <= 163 {
if features.red_luminance <= 110 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 123 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.green_luminance <= 162 {
if features.blue_difference <= 108 {
if features.blue_chromaticity <= 0.268 {
if features.hue <= 42 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.406 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.261 {
if features.blue_chromaticity <= 0.261 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.405 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.intensity <= 146 {
if features.blue_chromaticity <= 0.291 {
if features.red_chromaticity <= 0.314 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.value <= 176 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 137 {
if features.green_chromaticity <= 0.404 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.274 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.red_difference <= 109 {
if features.intensity <= 140 {
if features.blue_luminance <= 124 {
Intensity::Low
} else {
if features.blue_luminance <= 125 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.302 {
if features.hue <= 61 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.293 {
if features.blue_luminance <= 135 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
}
} else {
if features.saturation <= 73 {
if features.value <= 162 {
if features.blue_chromaticity <= 0.290 {
if features.green_chromaticity <= 0.403 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.407 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.green_luminance <= 165 {
if features.red_chromaticity <= 0.301 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 123 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.402 {
if features.blue_luminance <= 90 {
if features.green_luminance <= 125 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.319 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.306 {
if features.green_chromaticity <= 0.407 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.intensity <= 104 {
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
}
} else {
if features.blue_luminance <= 134 {
if features.red_chromaticity <= 0.315 {
if features.blue_luminance <= 132 {
if features.red_chromaticity <= 0.309 {
if features.blue_luminance <= 131 {
if features.red_luminance <= 137 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.288 {
if features.green_chromaticity <= 0.404 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_difference <= 109 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_luminance <= 186 {
if features.value <= 183 {
Intensity::Low
} else {
if features.intensity <= 151 {
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
if features.green_chromaticity <= 0.407 {
if features.blue_chromaticity <= 0.282 {
if features.green_chromaticity <= 0.405 {
if features.green_luminance <= 185 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.intensity <= 153 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 142 {
if features.blue_chromaticity <= 0.286 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 181 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.value <= 185 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.saturation <= 72 {
if features.blue_chromaticity <= 0.292 {
if features.value <= 186 {
if features.green_luminance <= 184 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.287 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.hue <= 54 {
if features.luminance <= 168 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.292 {
if features.luminance <= 165 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 133 {
if features.blue_difference <= 107 {
Intensity::High
} else {
if features.green_chromaticity <= 0.404 {
if features.green_chromaticity <= 0.403 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.287 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.313 {
if features.green_chromaticity <= 0.406 {
if features.green_luminance <= 188 {
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
} else {
if features.green_chromaticity <= 0.403 {
if features.red_luminance <= 146 {
if features.intensity <= 150 {
if features.blue_difference <= 108 {
if features.blue_difference <= 106 {
if features.saturation <= 80 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 180 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 71 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.285 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.saturation <= 72 {
if features.saturation <= 71 {
if features.blue_chromaticity <= 0.286 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 130 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 129 {
if features.green_luminance <= 183 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_difference <= 107 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.luminance <= 177 {
if features.intensity <= 154 {
if features.green_luminance <= 182 {
if features.red_chromaticity <= 0.339 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.324 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.399 {
if features.blue_luminance <= 130 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.321 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.red_difference <= 117 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_luminance <= 153 {
if features.blue_chromaticity <= 0.270 {
if features.green_chromaticity <= 0.404 {
Intensity::High
} else {
if features.green_luminance <= 187 {
if features.green_chromaticity <= 0.406 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.406 {
if features.green_chromaticity <= 0.404 {
if features.red_chromaticity <= 0.319 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.275 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.407 {
if features.red_chromaticity <= 0.322 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.intensity <= 152 {
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
}
} else {
if features.red_difference <= 108 {
if features.green_luminance <= 210 {
if features.blue_luminance <= 136 {
if features.blue_chromaticity <= 0.293 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 182 {
if features.blue_chromaticity <= 0.302 {
if features.blue_difference <= 112 {
if features.green_chromaticity <= 0.405 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 165 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 132 {
if features.blue_chromaticity <= 0.305 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 71 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_difference <= 102 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.405 {
if features.luminance <= 171 {
if features.blue_chromaticity <= 0.290 {
if features.blue_chromaticity <= 0.288 {
if features.value <= 189 {
if features.green_chromaticity <= 0.399 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 190 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.309 {
if features.green_chromaticity <= 0.402 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.310 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.292 {
if features.saturation <= 71 {
if features.blue_difference <= 109 {
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
}
} else {
if features.green_chromaticity <= 0.403 {
if features.saturation <= 73 {
if features.value <= 205 {
if features.value <= 188 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.red_difference <= 110 {
Intensity::High
} else {
if features.red_luminance <= 167 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.306 {
if features.red_luminance <= 145 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 142 {
if features.red_chromaticity <= 0.312 {
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
} else {
if features.green_chromaticity <= 0.405 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.286 {
Intensity::High
} else {
if features.red_luminance <= 143 {
if features.blue_chromaticity <= 0.289 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.luminance <= 173 {
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
if features.red_difference <= 123 {
if features.blue_difference <= 110 {
if features.green_chromaticity <= 0.393 {
Intensity::High
} else {
if features.red_difference <= 122 {
if features.intensity <= 94 {
if features.saturation <= 95 {
if features.blue_chromaticity <= 0.255 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.265 {
if features.red_chromaticity <= 0.342 {
if features.red_chromaticity <= 0.342 {
if features.blue_chromaticity <= 0.264 {
if features.saturation <= 94 {
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
} else {
if features.blue_chromaticity <= 0.253 {
if features.blue_chromaticity <= 0.252 {
if features.green_chromaticity <= 0.405 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.345 {
if features.green_chromaticity <= 0.399 {
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
if features.saturation <= 83 {
if features.value <= 130 {
Intensity::High
} else {
if features.green_chromaticity <= 0.394 {
if features.hue <= 42 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 118 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.intensity <= 107 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_chromaticity <= 0.345 {
if features.blue_luminance <= 76 {
if features.red_luminance <= 98 {
if features.blue_chromaticity <= 0.251 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.402 {
Intensity::Low
} else {
if features.luminance <= 103 {
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
if features.green_luminance <= 124 {
if features.saturation <= 86 {
Intensity::High
} else {
if features.red_chromaticity <= 0.344 {
if features.blue_luminance <= 78 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.luminance <= 120 {
if features.green_chromaticity <= 0.398 {
if features.luminance <= 118 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 116 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.value <= 132 {
if features.green_chromaticity <= 0.394 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 118 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.intensity <= 104 {
if features.green_chromaticity <= 0.404 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.249 {
if features.blue_chromaticity <= 0.246 {
Intensity::Low
} else {
if features.red_luminance <= 99 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.luminance <= 111 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.349 {
if features.red_chromaticity <= 0.348 {
if features.red_luminance <= 121 {
if features.red_chromaticity <= 0.346 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.257 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 139 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.253 {
if features.luminance <= 121 {
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
if features.blue_difference <= 111 {
if features.red_chromaticity <= 0.341 {
if features.green_luminance <= 125 {
if features.blue_luminance <= 71 {
if features.intensity <= 87 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.406 {
if features.red_luminance <= 91 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.404 {
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
if features.value <= 113 {
Intensity::Low
} else {
if features.intensity <= 94 {
Intensity::High
} else {
if features.green_chromaticity <= 0.397 {
if features.green_chromaticity <= 0.395 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 101 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.red_chromaticity <= 0.337 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.value <= 107 {
if features.value <= 103 {
if features.saturation <= 97 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.400 {
if features.saturation <= 87 {
if features.blue_luminance <= 76 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.saturation <= 90 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.257 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.blue_chromaticity <= 0.273 {
if features.red_luminance <= 102 {
if features.luminance <= 82 {
if features.green_chromaticity <= 0.406 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_difference <= 113 {
if features.red_chromaticity <= 0.340 {
if features.red_chromaticity <= 0.340 {
if features.green_chromaticity <= 0.406 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.340 {
if features.red_luminance <= 85 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 83 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.406 {
if features.red_difference <= 122 {
if features.saturation <= 88 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 94 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_luminance <= 83 {
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
if features.red_difference <= 125 {
if features.luminance <= 105 {
if features.blue_chromaticity <= 0.252 {
if features.green_chromaticity <= 0.396 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.251 {
if features.green_chromaticity <= 0.401 {
if features.green_chromaticity <= 0.399 {
if features.red_chromaticity <= 0.353 {
if features.red_luminance <= 80 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 66 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.hue <= 39 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.245 {
if features.red_chromaticity <= 0.356 {
if features.green_chromaticity <= 0.404 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.356 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.349 {
if features.green_chromaticity <= 0.405 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 95 {
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
} else {
if features.luminance <= 97 {
if features.red_luminance <= 78 {
if features.value <= 91 {
if features.red_chromaticity <= 0.349 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.350 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.396 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.401 {
if features.blue_chromaticity <= 0.258 {
Intensity::Low
} else {
if features.blue_luminance <= 70 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.259 {
Intensity::Low
} else {
if features.green_luminance <= 107 {
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
}
}
} else {
if features.red_luminance <= 117 {
if features.intensity <= 110 {
if features.green_luminance <= 132 {
if features.blue_chromaticity <= 0.248 {
if features.red_chromaticity <= 0.348 {
Intensity::High
} else {
if features.green_chromaticity <= 0.398 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.244 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_luminance <= 80 {
if features.red_chromaticity <= 0.352 {
if features.blue_luminance <= 74 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 103 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 113 {
if features.blue_difference <= 109 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 130 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.value <= 133 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.393 {
Intensity::High
} else {
if features.red_chromaticity <= 0.350 {
if features.red_chromaticity <= 0.350 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.hue <= 39 {
if features.green_chromaticity <= 0.396 {
if features.green_chromaticity <= 0.396 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
if features.blue_difference <= 105 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
}
} else {
if features.hue <= 36 {
if features.red_luminance <= 77 {
if features.value <= 81 {
if features.value <= 79 {
if features.red_chromaticity <= 0.374 {
if features.red_chromaticity <= 0.374 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.404 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.370 {
if features.luminance <= 77 {
Intensity::Low
} else {
if features.blue_difference <= 112 {
if features.blue_chromaticity <= 0.233 {
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
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.365 {
if features.luminance <= 108 {
if features.green_luminance <= 116 {
if features.intensity <= 92 {
if features.blue_luminance <= 49 {
if features.green_chromaticity <= 0.397 {
if features.green_luminance <= 79 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 83 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.354 {
if features.red_chromaticity <= 0.354 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 69 {
if features.green_luminance <= 112 {
if features.green_chromaticity <= 0.397 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 114 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 73 {
if features.blue_difference <= 107 {
Intensity::Low
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
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.365 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.406 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.229 {
Intensity::Low
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
if features.green_chromaticity <= 0.428 {
if features.green_luminance <= 115 {
if features.red_difference <= 119 {
if features.blue_difference <= 112 {
if features.green_luminance <= 111 {
if features.green_chromaticity <= 0.426 {
if features.red_difference <= 118 {
if features.blue_luminance <= 65 {
if features.saturation <= 99 {
Intensity::Low
} else {
if features.blue_difference <= 111 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.423 {
if features.blue_chromaticity <= 0.263 {
if features.intensity <= 85 {
Intensity::Low
} else {
if features.value <= 110 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.419 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.314 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.423 {
if features.red_luminance <= 81 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.424 {
if features.luminance <= 95 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.hue <= 49 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.red_chromaticity <= 0.324 {
if features.green_chromaticity <= 0.417 {
if features.green_luminance <= 109 {
Intensity::Low
} else {
if features.red_luminance <= 85 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.257 {
if features.red_luminance <= 80 {
if features.saturation <= 102 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.255 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.258 {
if features.blue_chromaticity <= 0.257 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 98 {
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
}
} else {
if features.blue_chromaticity <= 0.249 {
if features.intensity <= 84 {
Intensity::High
} else {
if features.red_chromaticity <= 0.327 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.427 {
if features.green_chromaticity <= 0.426 {
if features.red_chromaticity <= 0.320 {
if features.blue_luminance <= 64 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.saturation <= 103 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_luminance <= 98 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.257 {
if features.blue_chromaticity <= 0.257 {
if features.hue <= 48 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.308 {
Intensity::Low
} else {
if features.blue_luminance <= 65 {
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
} else {
if features.green_chromaticity <= 0.421 {
if features.red_chromaticity <= 0.322 {
if features.hue <= 49 {
if features.green_luminance <= 112 {
Intensity::Low
} else {
if features.value <= 114 {
if features.red_luminance <= 88 {
if features.red_chromaticity <= 0.320 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.saturation <= 92 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.266 {
if features.green_luminance <= 112 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.267 {
if features.green_luminance <= 112 {
Intensity::High
} else {
if features.value <= 114 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.saturation <= 91 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.hue <= 47 {
if features.red_luminance <= 88 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.261 {
Intensity::High
} else {
if features.blue_luminance <= 71 {
Intensity::High
} else {
if features.value <= 114 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.intensity <= 87 {
if features.red_difference <= 116 {
Intensity::Low
} else {
if features.hue <= 46 {
Intensity::High
} else {
if features.blue_luminance <= 66 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.328 {
if features.saturation <= 102 {
if features.red_chromaticity <= 0.311 {
if features.red_chromaticity <= 0.309 {
if features.blue_luminance <= 71 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.311 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.424 {
if features.red_chromaticity <= 0.325 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 85 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.value <= 114 {
if features.red_chromaticity <= 0.328 {
if features.red_chromaticity <= 0.318 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.saturation <= 105 {
Intensity::High
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
if features.red_difference <= 117 {
if features.green_luminance <= 112 {
if features.green_chromaticity <= 0.428 {
if features.green_chromaticity <= 0.418 {
if features.intensity <= 88 {
if features.saturation <= 88 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_difference <= 116 {
Intensity::Low
} else {
if features.green_luminance <= 111 {
Intensity::Low
} else {
if features.blue_difference <= 113 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_luminance <= 78 {
if features.green_chromaticity <= 0.421 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.422 {
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
if features.green_chromaticity <= 0.420 {
if features.green_luminance <= 107 {
Intensity::High
} else {
if features.saturation <= 92 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.424 {
if features.red_luminance <= 79 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.270 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.415 {
if features.saturation <= 84 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.275 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.270 {
if features.blue_chromaticity <= 0.270 {
Intensity::High
} else {
if features.red_difference <= 116 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.271 {
Intensity::High
} else {
if features.blue_luminance <= 76 {
if features.blue_chromaticity <= 0.277 {
if features.saturation <= 86 {
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
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.blue_difference <= 113 {
if features.red_chromaticity <= 0.319 {
if features.red_chromaticity <= 0.317 {
if features.luminance <= 96 {
if features.saturation <= 93 {
if features.saturation <= 92 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 78 {
if features.red_chromaticity <= 0.315 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 105 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.luminance <= 97 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.318 {
if features.red_chromaticity <= 0.318 {
if features.red_luminance <= 87 {
if features.saturation <= 89 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.318 {
if features.hue <= 49 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 94 {
Intensity::Low
} else {
if features.saturation <= 95 {
Intensity::Low
} else {
if features.saturation <= 96 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.saturation <= 86 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.413 {
if features.green_chromaticity <= 0.410 {
Intensity::Low
} else {
if features.saturation <= 88 {
Intensity::Low
} else {
if features.intensity <= 89 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.266 {
if features.value <= 107 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 84 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.green_chromaticity <= 0.420 {
if features.red_chromaticity <= 0.315 {
if features.green_chromaticity <= 0.419 {
if features.intensity <= 85 {
if features.saturation <= 90 {
if features.red_luminance <= 79 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 91 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.314 {
Intensity::Low
} else {
if features.red_luminance <= 85 {
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
if features.green_chromaticity <= 0.408 {
if features.green_luminance <= 112 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.409 {
if features.intensity <= 91 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 93 {
if features.red_chromaticity <= 0.317 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 94 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.red_chromaticity <= 0.312 {
if features.value <= 96 {
if features.red_luminance <= 69 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.312 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.312 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.315 {
if features.red_chromaticity <= 0.315 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 94 {
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
if features.red_difference <= 121 {
if features.blue_difference <= 111 {
if features.green_chromaticity <= 0.423 {
if features.value <= 113 {
if features.red_chromaticity <= 0.331 {
if features.red_chromaticity <= 0.331 {
if features.green_chromaticity <= 0.422 {
if features.saturation <= 95 {
Intensity::Low
} else {
if features.saturation <= 102 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.329 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.saturation <= 100 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.saturation <= 103 {
if features.red_chromaticity <= 0.332 {
if features.red_chromaticity <= 0.332 {
if features.green_chromaticity <= 0.415 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 88 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_luminance <= 112 {
if features.blue_chromaticity <= 0.249 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.257 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.421 {
if features.green_chromaticity <= 0.420 {
if features.red_chromaticity <= 0.337 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 97 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.241 {
if features.green_luminance <= 110 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.335 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.red_luminance <= 91 {
if features.saturation <= 100 {
if features.saturation <= 96 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 104 {
Intensity::Low
} else {
if features.green_luminance <= 114 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.hue <= 45 {
if features.saturation <= 107 {
if features.red_luminance <= 93 {
if features.blue_luminance <= 72 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.luminance <= 102 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.412 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_chromaticity <= 0.337 {
if features.blue_luminance <= 58 {
if features.blue_luminance <= 57 {
if features.green_chromaticity <= 0.428 {
if features.green_chromaticity <= 0.426 {
if features.value <= 95 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.intensity <= 76 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.427 {
if features.intensity <= 79 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.427 {
if features.green_chromaticity <= 0.427 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.425 {
if features.blue_chromaticity <= 0.241 {
if features.green_chromaticity <= 0.424 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.425 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 80 {
if features.green_chromaticity <= 0.424 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.332 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.425 {
Intensity::High
} else {
if features.value <= 112 {
if features.saturation <= 111 {
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
} else {
if features.red_luminance <= 91 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.intensity <= 72 {
if features.green_luminance <= 92 {
if features.red_chromaticity <= 0.328 {
if features.red_chromaticity <= 0.326 {
if features.green_chromaticity <= 0.427 {
if features.blue_luminance <= 49 {
Intensity::High
} else {
if features.red_chromaticity <= 0.321 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_difference <= 113 {
Intensity::Low
} else {
if features.hue <= 47 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_luminance <= 70 {
if features.blue_difference <= 113 {
if features.blue_difference <= 112 {
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
}
} else {
if features.green_chromaticity <= 0.426 {
if features.red_luminance <= 70 {
if features.saturation <= 105 {
Intensity::Low
} else {
if features.red_luminance <= 68 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_difference <= 112 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_difference <= 112 {
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
if features.red_luminance <= 86 {
if features.blue_chromaticity <= 0.264 {
if features.saturation <= 99 {
if features.red_luminance <= 85 {
if features.red_chromaticity <= 0.329 {
if features.red_chromaticity <= 0.329 {
Intensity::Low
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
} else {
if features.blue_luminance <= 68 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.418 {
if features.green_chromaticity <= 0.418 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.252 {
if features.blue_chromaticity <= 0.247 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 102 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.saturation <= 88 {
if features.blue_chromaticity <= 0.269 {
if features.intensity <= 83 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.326 {
if features.blue_chromaticity <= 0.266 {
if features.red_chromaticity <= 0.324 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 102 {
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
if features.red_luminance <= 89 {
if features.red_chromaticity <= 0.330 {
if features.green_chromaticity <= 0.408 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.409 {
if features.blue_chromaticity <= 0.264 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.408 {
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
if features.hue <= 37 {
if features.red_chromaticity <= 0.384 {
if features.blue_chromaticity <= 0.194 {
if features.blue_chromaticity <= 0.192 {
if features.intensity <= 73 {
if features.red_chromaticity <= 0.383 {
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
} else {
if features.red_chromaticity <= 0.384 {
if features.red_chromaticity <= 0.373 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.374 {
if features.value <= 74 {
if features.value <= 64 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.373 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_difference <= 110 {
if features.red_luminance <= 67 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 56 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 29 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.422 {
if features.blue_difference <= 111 {
if features.value <= 79 {
Intensity::High
} else {
if features.red_difference <= 123 {
if features.blue_difference <= 107 {
if features.red_chromaticity <= 0.348 {
if features.red_chromaticity <= 0.346 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_luminance <= 91 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.407 {
Intensity::High
} else {
if features.blue_difference <= 109 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.intensity <= 71 {
if features.value <= 89 {
if features.red_chromaticity <= 0.358 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.224 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.246 {
if features.hue <= 39 {
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
} else {
if features.green_chromaticity <= 0.415 {
if features.red_luminance <= 73 {
if features.red_chromaticity <= 0.360 {
if features.blue_chromaticity <= 0.251 {
if features.red_chromaticity <= 0.342 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 89 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 42 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.352 {
if features.green_chromaticity <= 0.409 {
if features.blue_luminance <= 58 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 81 {
Intensity::Low
} else {
Intensity::Low
}
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
if features.luminance <= 73 {
if features.blue_luminance <= 45 {
if features.blue_chromaticity <= 0.241 {
if features.red_luminance <= 64 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.hue <= 43 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.saturation <= 111 {
if features.green_chromaticity <= 0.416 {
if features.blue_chromaticity <= 0.238 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.saturation <= 109 {
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
if features.blue_luminance <= 61 {
if features.luminance <= 95 {
if features.green_chromaticity <= 0.428 {
if features.blue_difference <= 113 {
if features.red_luminance <= 57 {
if features.green_chromaticity <= 0.427 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.427 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.value <= 63 {
if features.intensity <= 47 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 107 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.luminance <= 78 {
Intensity::High
} else {
if features.green_chromaticity <= 0.428 {
if features.luminance <= 87 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::Low
}
}
}
} else {
if features.red_luminance <= 89 {
Intensity::High
} else {
if features.red_chromaticity <= 0.353 {
if features.hue <= 41 {
Intensity::High
} else {
if features.green_luminance <= 111 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_luminance <= 111 {
if features.blue_chromaticity <= 0.213 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.427 {
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
}
}
}
}
} else {
if features.red_difference <= 106 {
if features.blue_luminance <= 116 {
if features.red_difference <= 105 {
if features.green_luminance <= 161 {
if features.blue_chromaticity <= 0.293 {
if features.green_luminance <= 158 {
if features.saturation <= 87 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.282 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.280 {
if features.blue_chromaticity <= 0.296 {
if features.green_chromaticity <= 0.427 {
Intensity::High
} else {
if features.red_luminance <= 102 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.luminance <= 129 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.intensity <= 125 {
if features.red_chromaticity <= 0.281 {
if features.red_chromaticity <= 0.280 {
if features.saturation <= 87 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.value <= 159 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.value <= 160 {
if features.green_luminance <= 159 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_difference <= 112 {
Intensity::High
} else {
if features.saturation <= 85 {
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
if features.blue_luminance <= 115 {
if features.red_luminance <= 111 {
if features.hue <= 63 {
if features.blue_chromaticity <= 0.294 {
if features.red_chromaticity <= 0.284 {
if features.red_chromaticity <= 0.281 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.425 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.427 {
if features.blue_luminance <= 114 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 107 {
if features.green_chromaticity <= 0.428 {
if features.green_chromaticity <= 0.427 {
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
} else {
if features.blue_chromaticity <= 0.287 {
if features.green_luminance <= 167 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.289 {
if features.blue_luminance <= 113 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.luminance <= 149 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.value <= 169 {
if features.luminance <= 144 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.290 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_difference <= 104 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_difference <= 104 {
if features.green_chromaticity <= 0.426 {
if features.red_chromaticity <= 0.277 {
if features.intensity <= 129 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 143 {
if features.green_chromaticity <= 0.423 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 145 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.luminance <= 146 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 82 {
if features.green_chromaticity <= 0.423 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.295 {
if features.saturation <= 86 {
if features.green_luminance <= 169 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.blue_difference <= 113 {
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
if features.blue_luminance <= 114 {
if features.luminance <= 146 {
if features.intensity <= 123 {
if features.intensity <= 122 {
if features.blue_chromaticity <= 0.291 {
Intensity::High
} else {
if features.intensity <= 118 {
if features.blue_chromaticity <= 0.295 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 107 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.blue_difference <= 113 {
if features.blue_luminance <= 108 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.intensity <= 130 {
if features.blue_luminance <= 110 {
if features.blue_luminance <= 107 {
Intensity::High
} else {
if features.green_chromaticity <= 0.423 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.420 {
if features.green_chromaticity <= 0.420 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.295 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.value <= 168 {
if features.hue <= 60 {
if features.value <= 167 {
Intensity::High
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
}
} else {
if features.green_chromaticity <= 0.426 {
if features.intensity <= 133 {
if features.saturation <= 85 {
if features.intensity <= 132 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.428 {
if features.red_chromaticity <= 0.294 {
if features.saturation <= 88 {
if features.hue <= 57 {
Intensity::High
} else {
Intensity::High
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
}
} else {
if features.blue_chromaticity <= 0.296 {
if features.green_chromaticity <= 0.425 {
if features.green_chromaticity <= 0.419 {
Intensity::High
} else {
if features.green_chromaticity <= 0.421 {
if features.green_chromaticity <= 0.420 {
if features.blue_difference <= 112 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.hue <= 60 {
if features.luminance <= 149 {
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
if features.blue_chromaticity <= 0.280 {
if features.intensity <= 137 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.276 {
Intensity::High
} else {
if features.green_chromaticity <= 0.426 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.hue <= 57 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.luminance <= 140 {
Intensity::High
} else {
if features.green_luminance <= 163 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.blue_chromaticity <= 0.292 {
if features.red_difference <= 105 {
if features.red_difference <= 104 {
if features.blue_chromaticity <= 0.282 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.289 {
if features.blue_chromaticity <= 0.288 {
if features.blue_luminance <= 120 {
if features.green_chromaticity <= 0.427 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 189 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.288 {
if features.green_luminance <= 180 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.425 {
if features.blue_chromaticity <= 0.291 {
if features.green_chromaticity <= 0.418 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 121 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 122 {
if features.green_luminance <= 177 {
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
} else {
if features.red_chromaticity <= 0.290 {
if features.green_chromaticity <= 0.423 {
if features.blue_chromaticity <= 0.289 {
if features.green_chromaticity <= 0.423 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.289 {
Intensity::High
} else {
if features.hue <= 60 {
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
if features.blue_chromaticity <= 0.284 {
if features.luminance <= 165 {
if features.intensity <= 147 {
if features.green_chromaticity <= 0.424 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 125 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.hue <= 54 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.292 {
if features.red_chromaticity <= 0.291 {
if features.green_chromaticity <= 0.422 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.295 {
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
if features.blue_luminance <= 125 {
if features.red_chromaticity <= 0.296 {
if features.luminance <= 157 {
if features.blue_chromaticity <= 0.287 {
if features.red_chromaticity <= 0.295 {
if features.red_luminance <= 123 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
if features.intensity <= 142 {
if features.blue_chromaticity <= 0.290 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.417 {
Intensity::Low
} else {
if features.luminance <= 158 {
if features.green_chromaticity <= 0.419 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.295 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_chromaticity <= 0.273 {
if features.green_chromaticity <= 0.428 {
if features.green_chromaticity <= 0.426 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.272 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.value <= 182 {
if features.red_chromaticity <= 0.296 {
Intensity::High
} else {
if features.blue_luminance <= 117 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_difference <= 103 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.green_chromaticity <= 0.417 {
if features.value <= 191 {
if features.red_chromaticity <= 0.298 {
if features.blue_chromaticity <= 0.291 {
if features.intensity <= 153 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 128 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.291 {
if features.hue <= 56 {
if features.intensity <= 154 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 193 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.285 {
if features.green_chromaticity <= 0.419 {
if features.blue_chromaticity <= 0.284 {
if features.green_luminance <= 189 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.418 {
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
}
} else {
if features.red_difference <= 104 {
if features.blue_luminance <= 121 {
if features.red_difference <= 103 {
if features.blue_luminance <= 117 {
if features.red_difference <= 102 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.294 {
Intensity::Low
} else {
if features.blue_difference <= 112 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.423 {
if features.saturation <= 87 {
if features.blue_chromaticity <= 0.298 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.hue <= 65 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.281 {
if features.blue_chromaticity <= 0.299 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 173 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_chromaticity <= 0.422 {
if features.saturation <= 84 {
if features.value <= 169 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 119 {
if features.luminance <= 145 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.419 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.425 {
if features.green_chromaticity <= 0.422 {
if features.saturation <= 85 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 167 {
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
if features.red_luminance <= 143 {
if features.blue_luminance <= 128 {
if features.saturation <= 80 {
if features.green_chromaticity <= 0.419 {
if features.green_chromaticity <= 0.418 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.279 {
if features.luminance <= 152 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.280 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.295 {
if features.green_luminance <= 185 {
if features.saturation <= 80 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.294 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.417 {
if features.green_chromaticity <= 0.417 {
Intensity::Low
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
} else {
if features.green_chromaticity <= 0.414 {
if features.red_difference <= 105 {
if features.green_chromaticity <= 0.411 {
if features.red_chromaticity <= 0.290 {
if features.green_chromaticity <= 0.409 {
if features.saturation <= 75 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.410 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.295 {
if features.green_chromaticity <= 0.410 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.302 {
if features.saturation <= 73 {
if features.blue_luminance <= 136 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.413 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 120 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.300 {
if features.red_chromaticity <= 0.295 {
if features.blue_chromaticity <= 0.295 {
if features.blue_chromaticity <= 0.295 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.300 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.296 {
if features.saturation <= 73 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 193 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_luminance <= 123 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.301 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.303 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.red_difference <= 105 {
if features.blue_luminance <= 119 {
if features.green_chromaticity <= 0.420 {
if features.green_chromaticity <= 0.420 {
if features.green_chromaticity <= 0.418 {
Intensity::Low
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
} else {
if features.green_luminance <= 168 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.290 {
if features.blue_chromaticity <= 0.297 {
if features.blue_chromaticity <= 0.297 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.298 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.293 {
if features.red_chromaticity <= 0.292 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.418 {
if features.blue_chromaticity <= 0.297 {
if features.blue_chromaticity <= 0.293 {
if features.blue_chromaticity <= 0.293 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.295 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 119 {
if features.green_chromaticity <= 0.416 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.415 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.value <= 169 {
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
if features.green_chromaticity <= 0.415 {
if features.red_chromaticity <= 0.348 {
if features.saturation <= 76 {
if features.blue_chromaticity <= 0.295 {
if features.blue_luminance <= 126 {
if features.blue_difference <= 111 {
if features.red_difference <= 108 {
if features.green_luminance <= 171 {
if features.red_luminance <= 120 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.293 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.306 {
if features.blue_chromaticity <= 0.288 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.407 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.410 {
if features.hue <= 58 {
if features.red_chromaticity <= 0.303 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.296 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.value <= 140 {
if features.luminance <= 123 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.value <= 153 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.red_chromaticity <= 0.304 {
if features.blue_luminance <= 131 {
if features.green_chromaticity <= 0.408 {
if features.red_luminance <= 136 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.300 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.409 {
if features.green_chromaticity <= 0.409 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 134 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.408 {
if features.green_chromaticity <= 0.407 {
if features.green_chromaticity <= 0.407 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.hue <= 55 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_luminance <= 130 {
if features.blue_luminance <= 129 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
}
}
} else {
if features.red_difference <= 107 {
if features.green_chromaticity <= 0.410 {
if features.blue_chromaticity <= 0.298 {
if features.blue_difference <= 111 {
if features.green_chromaticity <= 0.407 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.297 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.300 {
if features.red_chromaticity <= 0.290 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.408 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.299 {
if features.green_chromaticity <= 0.412 {
if features.intensity <= 137 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.296 {
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
if features.green_chromaticity <= 0.408 {
if features.blue_difference <= 112 {
if features.red_chromaticity <= 0.296 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.297 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.408 {
if features.luminance <= 141 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 162 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.red_luminance <= 116 {
if features.green_chromaticity <= 0.409 {
Intensity::High
} else {
if features.red_chromaticity <= 0.290 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.294 {
if features.hue <= 61 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.intensity <= 133 {
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
if features.blue_difference <= 110 {
if features.blue_luminance <= 124 {
if features.green_chromaticity <= 0.411 {
if features.red_luminance <= 104 {
if features.red_chromaticity <= 0.335 {
if features.blue_chromaticity <= 0.267 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 103 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.green_luminance <= 162 {
if features.blue_chromaticity <= 0.271 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 131 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_luminance <= 118 {
if features.blue_luminance <= 74 {
if features.green_chromaticity <= 0.414 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_difference <= 108 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_difference <= 108 {
if features.green_chromaticity <= 0.415 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.hue <= 44 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.blue_luminance <= 129 {
if features.red_chromaticity <= 0.301 {
if features.red_chromaticity <= 0.301 {
if features.red_chromaticity <= 0.300 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 126 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.312 {
if features.green_chromaticity <= 0.411 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.314 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.blue_chromaticity <= 0.280 {
if features.green_chromaticity <= 0.408 {
if features.blue_luminance <= 131 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.407 {
Intensity::High
} else {
if features.red_chromaticity <= 0.307 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.green_luminance <= 120 {
if features.blue_chromaticity <= 0.272 {
if features.blue_luminance <= 74 {
Intensity::High
} else {
if features.value <= 118 {
if features.hue <= 50 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.413 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_luminance <= 117 {
if features.blue_chromaticity <= 0.279 {
if features.green_chromaticity <= 0.408 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.409 {
if features.value <= 118 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.luminance <= 106 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
} else {
if features.green_chromaticity <= 0.408 {
if features.blue_luminance <= 84 {
if features.luminance <= 110 {
if features.blue_difference <= 112 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.317 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.310 {
if features.blue_chromaticity <= 0.284 {
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
}
} else {
if features.red_difference <= 107 {
if features.red_chromaticity <= 0.289 {
if features.blue_luminance <= 116 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.value <= 125 {
if features.green_chromaticity <= 0.410 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 79 {
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
if features.red_chromaticity <= 0.353 {
if features.blue_chromaticity <= 0.243 {
if features.red_chromaticity <= 0.353 {
if features.intensity <= 95 {
if features.red_chromaticity <= 0.350 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.350 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.235 {
Intensity::High
} else {
if features.green_luminance <= 124 {
if features.red_chromaticity <= 0.350 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.352 {
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
if features.red_difference <= 122 {
if features.red_luminance <= 119 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.414 {
if features.red_chromaticity <= 0.356 {
if features.blue_chromaticity <= 0.233 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.414 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_chromaticity <= 0.353 {
if features.blue_difference <= 109 {
if features.green_chromaticity <= 0.421 {
if features.blue_chromaticity <= 0.285 {
if features.green_luminance <= 122 {
if features.red_difference <= 120 {
if features.blue_chromaticity <= 0.251 {
if features.red_luminance <= 93 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.luminance <= 104 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.339 {
if features.luminance <= 104 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.hue <= 40 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.344 {
if features.blue_difference <= 108 {
if features.blue_luminance <= 122 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.310 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.416 {
if features.green_luminance <= 136 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.419 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_luminance <= 178 {
if features.luminance <= 154 {
if features.value <= 175 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.value <= 177 {
if features.red_difference <= 107 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.intensity <= 145 {
if features.blue_luminance <= 123 {
Intensity::Low
} else {
if features.red_luminance <= 129 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 79 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.value <= 124 {
if features.blue_chromaticity <= 0.233 {
if features.green_luminance <= 120 {
if features.intensity <= 90 {
if features.hue <= 42 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.424 {
if features.intensity <= 96 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.luminance <= 108 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.green_luminance <= 119 {
if features.blue_chromaticity <= 0.250 {
if features.red_chromaticity <= 0.330 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.425 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.332 {
if features.red_chromaticity <= 0.330 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 120 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.blue_difference <= 108 {
if features.red_difference <= 107 {
if features.intensity <= 134 {
if features.blue_luminance <= 112 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 135 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.223 {
Intensity::Low
} else {
if features.value <= 157 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.423 {
if features.red_chromaticity <= 0.295 {
if features.green_chromaticity <= 0.422 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.296 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.hue <= 56 {
if features.saturation <= 87 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.295 {
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
if features.value <= 123 {
if features.green_chromaticity <= 0.423 {
if features.red_luminance <= 89 {
if features.red_luminance <= 85 {
if features.blue_chromaticity <= 0.273 {
if features.saturation <= 91 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.278 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 81 {
if features.saturation <= 86 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 82 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.419 {
if features.red_luminance <= 93 {
if features.red_chromaticity <= 0.323 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 122 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 92 {
if features.green_chromaticity <= 0.421 {
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
if features.blue_difference <= 113 {
if features.blue_luminance <= 71 {
if features.saturation <= 100 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.311 {
if features.saturation <= 91 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.263 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.blue_chromaticity <= 0.280 {
if features.green_chromaticity <= 0.424 {
if features.value <= 117 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.294 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.281 {
if features.red_luminance <= 82 {
Intensity::High
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
} else {
if features.green_chromaticity <= 0.420 {
if features.blue_difference <= 111 {
if features.green_chromaticity <= 0.418 {
if features.saturation <= 77 {
if features.green_chromaticity <= 0.416 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.264 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.276 {
if features.red_luminance <= 97 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.420 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.416 {
if features.blue_chromaticity <= 0.288 {
if features.luminance <= 112 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.295 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.289 {
if features.green_luminance <= 157 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.value <= 124 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.green_chromaticity <= 0.424 {
if features.red_luminance <= 93 {
if features.green_chromaticity <= 0.421 {
if features.intensity <= 100 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_difference <= 109 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.287 {
if features.green_chromaticity <= 0.423 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 103 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.luminance <= 114 {
if features.red_chromaticity <= 0.297 {
if features.blue_luminance <= 81 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 87 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_difference <= 110 {
if features.value <= 146 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.427 {
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
if features.red_difference <= 122 {
Intensity::High
} else {
if features.red_luminance <= 98 {
Intensity::High
} else {
if features.red_difference <= 123 {
if features.red_chromaticity <= 0.355 {
if features.value <= 125 {
if features.value <= 119 {
if features.blue_chromaticity <= 0.223 {
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
} else {
Intensity::High
}
} else {
if features.saturation <= 122 {
if features.red_chromaticity <= 0.369 {
if features.red_chromaticity <= 0.357 {
if features.green_chromaticity <= 0.417 {
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
} else {
if features.hue <= 29 {
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
if features.red_difference <= 120 {
if features.green_chromaticity <= 0.472 {
if features.blue_difference <= 110 {
if features.blue_luminance <= 110 {
if features.green_luminance <= 128 {
if features.green_chromaticity <= 0.453 {
if features.blue_difference <= 107 {
if features.red_difference <= 118 {
if features.blue_difference <= 105 {
if features.blue_chromaticity <= 0.240 {
if features.green_chromaticity <= 0.450 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 127 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.319 {
if features.value <= 125 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_difference <= 106 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_luminance <= 120 {
if features.green_chromaticity <= 0.449 {
if features.green_chromaticity <= 0.430 {
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
if features.red_difference <= 119 {
if features.green_luminance <= 124 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.432 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.red_difference <= 114 {
if features.luminance <= 107 {
if features.green_chromaticity <= 0.447 {
if features.green_luminance <= 117 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.285 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.saturation <= 101 {
if features.red_luminance <= 85 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.434 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_difference <= 118 {
if features.blue_difference <= 108 {
if features.red_chromaticity <= 0.312 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.322 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.244 {
if features.red_chromaticity <= 0.329 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 108 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.blue_difference <= 106 {
if features.blue_chromaticity <= 0.234 {
if features.red_difference <= 117 {
if features.green_chromaticity <= 0.458 {
if features.red_chromaticity <= 0.311 {
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
if features.saturation <= 139 {
if features.green_chromaticity <= 0.455 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.472 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.467 {
if features.red_chromaticity <= 0.298 {
if features.red_chromaticity <= 0.297 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.304 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.245 {
if features.red_chromaticity <= 0.294 {
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
if features.green_chromaticity <= 0.459 {
if features.blue_difference <= 109 {
if features.red_difference <= 117 {
if features.red_chromaticity <= 0.284 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.320 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.454 {
if features.green_chromaticity <= 0.454 {
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
}
} else {
if features.intensity <= 84 {
if features.luminance <= 82 {
if features.value <= 83 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.465 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.value <= 124 {
if features.green_luminance <= 122 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 71 {
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
if features.blue_difference <= 106 {
if features.blue_chromaticity <= 0.273 {
if features.green_chromaticity <= 0.434 {
if features.green_chromaticity <= 0.430 {
if features.green_chromaticity <= 0.429 {
if features.blue_difference <= 102 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.233 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 104 {
if features.blue_chromaticity <= 0.248 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 92 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_difference <= 99 {
if features.red_luminance <= 99 {
Intensity::High
} else {
if features.blue_difference <= 103 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.272 {
if features.blue_difference <= 104 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_difference <= 102 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
} else {
if features.red_difference <= 103 {
if features.green_luminance <= 170 {
if features.green_luminance <= 169 {
if features.luminance <= 143 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.value <= 173 {
if features.green_luminance <= 171 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_luminance <= 174 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.292 {
if features.red_chromaticity <= 0.287 {
if features.green_luminance <= 169 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.red_luminance <= 115 {
if features.hue <= 56 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_luminance <= 117 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.green_luminance <= 166 {
if features.green_chromaticity <= 0.438 {
if features.hue <= 59 {
if features.green_chromaticity <= 0.430 {
if features.red_chromaticity <= 0.307 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 88 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_luminance <= 165 {
if features.saturation <= 91 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.intensity <= 126 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.value <= 131 {
if features.blue_chromaticity <= 0.263 {
if features.green_chromaticity <= 0.464 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.448 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.450 {
if features.value <= 163 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 160 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.green_chromaticity <= 0.436 {
if features.green_luminance <= 169 {
if features.red_chromaticity <= 0.289 {
if features.blue_luminance <= 109 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.saturation <= 91 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.288 {
Intensity::High
} else {
if features.red_luminance <= 114 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.red_difference <= 100 {
if features.blue_luminance <= 106 {
if features.green_chromaticity <= 0.449 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_luminance <= 91 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.280 {
if features.intensity <= 128 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.284 {
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
} else {
if features.red_difference <= 103 {
if features.red_difference <= 102 {
if features.green_luminance <= 170 {
if features.blue_chromaticity <= 0.288 {
if features.red_chromaticity <= 0.278 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.289 {
if features.blue_chromaticity <= 0.288 {
Intensity::Low
} else {
if features.green_luminance <= 168 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 129 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_difference <= 103 {
Intensity::High
} else {
if features.red_difference <= 101 {
if features.blue_chromaticity <= 0.293 {
if features.red_chromaticity <= 0.286 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.434 {
if features.blue_luminance <= 123 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.saturation <= 90 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.red_chromaticity <= 0.288 {
if features.intensity <= 131 {
if features.blue_luminance <= 111 {
if features.saturation <= 89 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.431 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.288 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.430 {
if features.blue_chromaticity <= 0.283 {
Intensity::High
} else {
if features.green_chromaticity <= 0.429 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.430 {
Intensity::High
} else {
if features.green_luminance <= 172 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_chromaticity <= 0.291 {
Intensity::High
} else {
if features.blue_luminance <= 124 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
} else {
if features.red_chromaticity <= 0.291 {
if features.green_chromaticity <= 0.431 {
if features.green_chromaticity <= 0.430 {
if features.saturation <= 85 {
if features.blue_chromaticity <= 0.286 {
if features.intensity <= 135 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.428 {
if features.intensity <= 135 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 110 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.hue <= 60 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.435 {
if features.green_chromaticity <= 0.432 {
if features.saturation <= 89 {
if features.red_chromaticity <= 0.284 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 112 {
if features.green_chromaticity <= 0.433 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 136 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
Intensity::High
}
}
} else {
if features.blue_luminance <= 112 {
if features.green_chromaticity <= 0.431 {
if features.intensity <= 134 {
if features.hue <= 56 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 111 {
if features.blue_chromaticity <= 0.272 {
Intensity::High
} else {
if features.red_luminance <= 119 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.434 {
if features.green_chromaticity <= 0.432 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.301 {
if features.red_chromaticity <= 0.299 {
if features.blue_difference <= 104 {
Intensity::High
} else {
if features.saturation <= 93 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 95 {
if features.red_chromaticity <= 0.299 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.267 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_difference <= 107 {
Intensity::High
} else {
if features.luminance <= 162 {
Intensity::Low
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
if features.red_difference <= 113 {
if features.blue_luminance <= 111 {
if features.green_luminance <= 127 {
if features.green_chromaticity <= 0.458 {
if features.blue_difference <= 112 {
if features.blue_difference <= 111 {
if features.green_luminance <= 126 {
if features.red_chromaticity <= 0.281 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.271 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.287 {
if features.blue_luminance <= 70 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.275 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_difference <= 111 {
if features.blue_difference <= 113 {
if features.green_chromaticity <= 0.451 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 125 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.453 {
if features.blue_luminance <= 78 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.value <= 97 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.blue_difference <= 112 {
if features.value <= 122 {
if features.green_chromaticity <= 0.460 {
if features.luminance <= 85 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.264 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.hue <= 63 {
if features.red_chromaticity <= 0.263 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 125 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_difference <= 112 {
if features.green_chromaticity <= 0.459 {
if features.red_luminance <= 71 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 115 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.saturation <= 114 {
if features.green_chromaticity <= 0.462 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 52 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.green_chromaticity <= 0.448 {
if features.red_difference <= 102 {
if features.blue_luminance <= 108 {
if features.blue_chromaticity <= 0.289 {
if features.green_chromaticity <= 0.447 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.value <= 144 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 102 {
if features.green_luminance <= 161 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.green_luminance <= 130 {
if features.saturation <= 93 {
if features.luminance <= 110 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.447 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.intensity <= 106 {
if features.green_chromaticity <= 0.437 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.292 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.green_luminance <= 133 {
if features.blue_chromaticity <= 0.289 {
if features.red_chromaticity <= 0.264 {
if features.blue_chromaticity <= 0.282 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.265 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.value <= 129 {
if features.saturation <= 123 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.saturation <= 110 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_luminance <= 89 {
if features.blue_chromaticity <= 0.294 {
if features.green_chromaticity <= 0.456 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.294 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.intensity <= 114 {
Intensity::Low
} else {
if features.hue <= 65 {
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
if features.red_difference <= 101 {
if features.blue_chromaticity <= 0.293 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.296 {
if features.blue_chromaticity <= 0.296 {
if features.red_difference <= 100 {
if features.blue_luminance <= 121 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 94 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 116 {
if features.green_luminance <= 170 {
if features.blue_chromaticity <= 0.297 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 167 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_chromaticity <= 0.292 {
if features.green_chromaticity <= 0.429 {
if features.blue_luminance <= 113 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 166 {
if features.blue_chromaticity <= 0.291 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 167 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_luminance <= 105 {
if features.red_chromaticity <= 0.274 {
if features.green_chromaticity <= 0.431 {
if features.blue_chromaticity <= 0.297 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 103 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.luminance <= 146 {
if features.saturation <= 90 {
if features.red_difference <= 102 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 114 {
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
if features.green_chromaticity <= 0.449 {
if features.red_difference <= 118 {
if features.blue_difference <= 112 {
if features.red_chromaticity <= 0.303 {
if features.intensity <= 92 {
if features.saturation <= 104 {
if features.blue_chromaticity <= 0.260 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.445 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.saturation <= 95 {
Intensity::High
} else {
if features.green_chromaticity <= 0.430 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.blue_difference <= 111 {
if features.red_chromaticity <= 0.317 {
if features.saturation <= 100 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.intensity <= 80 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.439 {
if features.red_chromaticity <= 0.312 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 68 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_chromaticity <= 0.440 {
if features.red_difference <= 114 {
if features.blue_chromaticity <= 0.272 {
if features.green_chromaticity <= 0.437 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.intensity <= 83 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.256 {
if features.blue_luminance <= 56 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_difference <= 115 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_difference <= 113 {
if features.red_chromaticity <= 0.301 {
if features.saturation <= 102 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.441 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.446 {
if features.red_luminance <= 65 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.446 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.saturation <= 120 {
if features.blue_chromaticity <= 0.247 {
if features.luminance <= 66 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.321 {
if features.intensity <= 68 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.intensity <= 72 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.blue_difference <= 112 {
if features.blue_chromaticity <= 0.247 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.251 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.433 {
if features.green_chromaticity <= 0.431 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 66 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
} else {
if features.saturation <= 124 {
if features.green_chromaticity <= 0.444 {
if features.green_chromaticity <= 0.443 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 80 {
if features.intensity <= 58 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.intensity <= 62 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.saturation <= 125 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.red_difference <= 117 {
if features.saturation <= 116 {
if features.luminance <= 79 {
if features.green_chromaticity <= 0.462 {
if features.green_chromaticity <= 0.452 {
if features.red_chromaticity <= 0.294 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_difference <= 115 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.465 {
if features.blue_chromaticity <= 0.253 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.intensity <= 71 {
if features.hue <= 53 {
if features.red_chromaticity <= 0.297 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.289 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 64 {
if features.red_chromaticity <= 0.291 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.449 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.value <= 85 {
if features.green_chromaticity <= 0.466 {
if features.blue_chromaticity <= 0.238 {
if features.blue_chromaticity <= 0.235 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.292 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.295 {
if features.luminance <= 68 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.235 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.459 {
if features.red_chromaticity <= 0.307 {
if features.green_chromaticity <= 0.451 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.308 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_difference <= 111 {
if features.value <= 90 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.463 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.blue_difference <= 113 {
if features.green_chromaticity <= 0.462 {
if features.red_chromaticity <= 0.326 {
if features.saturation <= 123 {
if features.green_chromaticity <= 0.451 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.value <= 79 {
Intensity::High
} else {
Intensity::High
}
}
} else {
Intensity::Low
}
} else {
if features.luminance <= 68 {
if features.blue_difference <= 112 {
if features.green_chromaticity <= 0.470 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.320 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.465 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.308 {
if features.red_chromaticity <= 0.303 {
if features.saturation <= 128 {
if features.saturation <= 127 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 35 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.307 {
if features.red_chromaticity <= 0.305 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.235 {
if features.blue_chromaticity <= 0.233 {
if features.saturation <= 125 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.234 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_luminance <= 39 {
Intensity::Low
} else {
if features.luminance <= 65 {
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
if features.green_chromaticity <= 0.497 {
if features.value <= 88 {
if features.blue_difference <= 110 {
if features.green_chromaticity <= 0.483 {
if features.intensity <= 56 {
if features.saturation <= 154 {
if features.blue_chromaticity <= 0.191 {
if features.blue_difference <= 108 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 32 {
if features.red_chromaticity <= 0.329 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 34 {
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
if features.blue_chromaticity <= 0.222 {
if features.green_chromaticity <= 0.483 {
if features.green_chromaticity <= 0.477 {
if features.luminance <= 70 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.480 {
Intensity::High
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
} else {
if features.blue_luminance <= 24 {
if features.saturation <= 166 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 80 {
if features.green_chromaticity <= 0.497 {
if features.red_luminance <= 55 {
if features.red_chromaticity <= 0.324 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.320 {
if features.red_difference <= 117 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.blue_luminance <= 35 {
if features.green_luminance <= 81 {
if features.red_luminance <= 50 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 83 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 50 {
if features.green_luminance <= 87 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 146 {
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
if features.green_chromaticity <= 0.483 {
if features.blue_difference <= 113 {
if features.green_chromaticity <= 0.473 {
if features.red_luminance <= 53 {
if features.red_luminance <= 52 {
if features.red_luminance <= 49 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.red_luminance <= 49 {
if features.blue_luminance <= 38 {
if features.luminance <= 59 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.233 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.306 {
if features.value <= 81 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.475 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.blue_chromaticity <= 0.252 {
if features.blue_luminance <= 29 {
if features.red_difference <= 119 {
Intensity::High
} else {
if features.intensity <= 43 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_luminance <= 31 {
if features.green_luminance <= 67 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.242 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.477 {
if features.red_chromaticity <= 0.272 {
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
if features.red_difference <= 118 {
if features.green_chromaticity <= 0.494 {
if features.saturation <= 142 {
if features.blue_luminance <= 40 {
if features.intensity <= 48 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 41 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_luminance <= 29 {
if features.red_chromaticity <= 0.296 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 47 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.268 {
if features.green_chromaticity <= 0.497 {
if features.green_chromaticity <= 0.494 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 85 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.282 {
if features.green_chromaticity <= 0.494 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.luminance <= 57 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.intensity <= 40 {
if features.red_chromaticity <= 0.311 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.496 {
if features.blue_chromaticity <= 0.184 {
Intensity::High
} else {
if features.value <= 66 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.intensity <= 44 {
if features.hue <= 47 {
Intensity::High
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
} else {
if features.blue_difference <= 108 {
if features.blue_difference <= 103 {
if features.green_luminance <= 131 {
if features.saturation <= 135 {
if features.saturation <= 129 {
Intensity::High
} else {
if features.red_luminance <= 73 {
if features.red_difference <= 105 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.value <= 123 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.luminance <= 74 {
if features.saturation <= 172 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 54 {
if features.red_chromaticity <= 0.355 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.489 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.green_luminance <= 158 {
if features.green_luminance <= 134 {
if features.blue_chromaticity <= 0.236 {
if features.red_chromaticity <= 0.328 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.luminance <= 121 {
if features.red_luminance <= 95 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 132 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_luminance <= 92 {
if features.red_chromaticity <= 0.278 {
Intensity::High
} else {
if features.green_chromaticity <= 0.484 {
Intensity::Low
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
if features.green_chromaticity <= 0.483 {
if features.value <= 131 {
if features.blue_luminance <= 59 {
if features.value <= 98 {
if features.green_chromaticity <= 0.473 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.478 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_difference <= 104 {
if features.blue_chromaticity <= 0.233 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.472 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.intensity <= 93 {
if features.green_chromaticity <= 0.479 {
if features.red_chromaticity <= 0.266 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 73 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.saturation <= 119 {
if features.red_chromaticity <= 0.276 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.247 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.green_luminance <= 130 {
if features.blue_chromaticity <= 0.240 {
if features.green_luminance <= 100 {
if features.red_chromaticity <= 0.314 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.value <= 120 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.489 {
if features.intensity <= 84 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 63 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.value <= 134 {
if features.blue_chromaticity <= 0.263 {
if features.blue_chromaticity <= 0.255 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.green_luminance <= 141 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.246 {
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
if features.green_chromaticity <= 0.480 {
if features.luminance <= 79 {
if features.blue_difference <= 112 {
if features.red_difference <= 112 {
if features.saturation <= 121 {
if features.blue_luminance <= 50 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.277 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.value <= 90 {
if features.green_chromaticity <= 0.476 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.239 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.274 {
if features.green_chromaticity <= 0.473 {
Intensity::High
} else {
if features.red_chromaticity <= 0.265 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.saturation <= 121 {
if features.saturation <= 120 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 52 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.green_luminance <= 127 {
if features.blue_chromaticity <= 0.258 {
if features.red_chromaticity <= 0.297 {
if features.luminance <= 82 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.476 {
if features.blue_chromaticity <= 0.276 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 54 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.479 {
if features.blue_chromaticity <= 0.263 {
if features.blue_chromaticity <= 0.262 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.luminance <= 104 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.479 {
if features.red_chromaticity <= 0.241 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 73 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.blue_difference <= 112 {
if features.green_chromaticity <= 0.486 {
if features.luminance <= 76 {
if features.blue_chromaticity <= 0.235 {
if features.red_chromaticity <= 0.291 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.value <= 92 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 53 {
if features.red_chromaticity <= 0.268 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.value <= 130 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_luminance <= 53 {
if features.green_chromaticity <= 0.492 {
if features.value <= 92 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 135 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.488 {
if features.green_chromaticity <= 0.487 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.489 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.luminance <= 79 {
if features.saturation <= 119 {
if features.blue_chromaticity <= 0.257 {
Intensity::High
} else {
if features.blue_luminance <= 49 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.264 {
if features.blue_chromaticity <= 0.254 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_difference <= 106 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_luminance <= 100 {
Intensity::High
} else {
if features.red_chromaticity <= 0.247 {
if features.red_chromaticity <= 0.231 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.269 {
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
}
} else {
if features.blue_difference <= 109 {
if features.green_chromaticity <= 0.537 {
if features.green_luminance <= 104 {
if features.blue_difference <= 107 {
if features.red_difference <= 118 {
if features.blue_difference <= 103 {
if features.saturation <= 155 {
Intensity::High
} else {
if features.green_chromaticity <= 0.531 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.saturation <= 149 {
if features.blue_chromaticity <= 0.221 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 166 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.blue_chromaticity <= 0.156 {
if features.blue_luminance <= 27 {
if features.blue_chromaticity <= 0.148 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.value <= 92 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_luminance <= 76 {
if features.intensity <= 49 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.503 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.red_difference <= 113 {
if features.saturation <= 141 {
if features.blue_luminance <= 48 {
if features.value <= 96 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.287 {
if features.blue_difference <= 108 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 91 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.509 {
if features.red_chromaticity <= 0.316 {
if features.red_luminance <= 47 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.322 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 42 {
if features.blue_chromaticity <= 0.184 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.513 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.saturation <= 141 {
if features.green_chromaticity <= 0.506 {
if features.blue_luminance <= 62 {
if features.green_chromaticity <= 0.498 {
if features.red_chromaticity <= 0.264 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 54 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.saturation <= 127 {
if features.blue_difference <= 106 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 63 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_luminance <= 124 {
if features.blue_luminance <= 57 {
if features.luminance <= 85 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.240 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.264 {
if features.blue_chromaticity <= 0.252 {
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
}
}
} else {
if features.green_luminance <= 109 {
if features.green_chromaticity <= 0.502 {
if features.red_chromaticity <= 0.316 {
if features.green_chromaticity <= 0.499 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_difference <= 100 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.saturation <= 188 {
if features.value <= 107 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.326 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.347 {
if features.green_chromaticity <= 0.498 {
if features.luminance <= 96 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.519 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.347 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
}
} else {
if features.green_chromaticity <= 0.587 {
if features.luminance <= 83 {
if features.red_luminance <= 65 {
if features.saturation <= 162 {
if features.intensity <= 61 {
if features.green_luminance <= 100 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.206 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.277 {
if features.green_chromaticity <= 0.560 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_difference <= 101 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.546 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_difference <= 112 {
if features.blue_difference <= 98 {
if features.red_luminance <= 55 {
if features.green_chromaticity <= 0.561 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.intensity <= 68 {
if features.green_chromaticity <= 0.586 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 154 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.intensity <= 64 {
if features.red_chromaticity <= 0.326 {
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
if features.green_chromaticity <= 0.615 {
if features.red_luminance <= 32 {
if features.green_chromaticity <= 0.615 {
if features.saturation <= 174 {
if features.intensity <= 55 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.612 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.219 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.value <= 109 {
if features.blue_luminance <= 27 {
if features.red_chromaticity <= 0.254 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 188 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.intensity <= 63 {
if features.value <= 114 {
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
if features.green_chromaticity <= 0.651 {
if features.green_chromaticity <= 0.651 {
if features.green_luminance <= 63 {
if features.blue_chromaticity <= 0.119 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 202 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_difference <= 107 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.saturation <= 211 {
if features.blue_chromaticity <= 0.116 {
if features.blue_chromaticity <= 0.116 {
Intensity::High
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
}
}
}
} else {
if features.green_chromaticity <= 0.531 {
if features.red_difference <= 119 {
if features.blue_difference <= 112 {
if features.red_difference <= 110 {
if features.green_luminance <= 109 {
if features.intensity <= 71 {
if features.red_chromaticity <= 0.260 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.intensity <= 72 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_luminance <= 60 {
Intensity::High
} else {
if features.saturation <= 145 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.510 {
if features.blue_difference <= 111 {
if features.red_chromaticity <= 0.315 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.value <= 66 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.saturation <= 148 {
if features.red_chromaticity <= 0.261 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 60 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.red_difference <= 117 {
if features.green_chromaticity <= 0.518 {
if features.luminance <= 69 {
if features.blue_chromaticity <= 0.225 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.517 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_difference <= 102 {
if features.red_chromaticity <= 0.188 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.229 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.saturation <= 162 {
if features.green_chromaticity <= 0.504 {
if features.blue_luminance <= 25 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.291 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.186 {
if features.red_difference <= 118 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 33 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.blue_difference <= 110 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.164 {
if features.saturation <= 181 {
if features.blue_chromaticity <= 0.158 {
if features.red_chromaticity <= 0.316 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 176 {
Intensity::High
} else {
Intensity::High
}
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.509 {
if features.blue_chromaticity <= 0.173 {
if features.intensity <= 39 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.188 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.177 {
if features.green_chromaticity <= 0.513 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.512 {
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
if features.green_chromaticity <= 0.559 {
if features.luminance <= 41 {
if features.green_chromaticity <= 0.553 {
if features.green_chromaticity <= 0.539 {
Intensity::High
} else {
if features.luminance <= 38 {
Intensity::High
} else {
if features.red_difference <= 119 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.hue <= 48 {
if features.green_luminance <= 50 {
if features.blue_chromaticity <= 0.136 {
Intensity::High
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
if features.blue_chromaticity <= 0.121 {
Intensity::High
} else {
if features.blue_difference <= 112 {
if features.green_luminance <= 53 {
Intensity::High
} else {
if features.red_chromaticity <= 0.321 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_difference <= 117 {
if features.saturation <= 150 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.537 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.red_chromaticity <= 0.284 {
if features.green_chromaticity <= 0.582 {
if features.green_luminance <= 55 {
if features.green_luminance <= 51 {
if features.green_chromaticity <= 0.571 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_difference <= 112 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.213 {
if features.blue_chromaticity <= 0.147 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.intensity <= 53 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.274 {
if features.green_chromaticity <= 0.616 {
if features.green_chromaticity <= 0.616 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.intensity <= 12 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.saturation <= 194 {
Intensity::High
} else {
if features.value <= 43 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.blue_difference <= 112 {
if features.red_difference <= 118 {
if features.blue_difference <= 111 {
if features.red_chromaticity <= 0.303 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.578 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_difference <= 110 {
Intensity::High
} else {
if features.value <= 43 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.292 {
if features.red_chromaticity <= 0.291 {
if features.green_chromaticity <= 0.595 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.305 {
if features.red_luminance <= 16 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.intensity <= 24 {
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
}
}
}
} else {
if features.red_difference <= 122 {
if features.blue_chromaticity <= 0.216 {
if features.blue_difference <= 112 {
if features.green_chromaticity <= 0.464 {
if features.red_difference <= 121 {
if features.blue_chromaticity <= 0.216 {
if features.red_chromaticity <= 0.346 {
if features.blue_luminance <= 44 {
if features.green_chromaticity <= 0.453 {
if features.red_luminance <= 73 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 146 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.value <= 97 {
if features.green_luminance <= 95 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.342 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.value <= 110 {
Intensity::High
} else {
if features.green_luminance <= 112 {
if features.blue_difference <= 102 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.351 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.462 {
if features.blue_chromaticity <= 0.215 {
if features.blue_chromaticity <= 0.202 {
if features.value <= 99 {
if features.red_chromaticity <= 0.352 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 106 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.351 {
if features.blue_luminance <= 42 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.214 {
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
if features.green_chromaticity <= 0.462 {
Intensity::Low
} else {
if features.red_luminance <= 76 {
if features.saturation <= 151 {
if features.green_chromaticity <= 0.464 {
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
}
}
}
} else {
if features.blue_difference <= 111 {
if features.red_difference <= 121 {
if features.blue_chromaticity <= 0.147 {
if features.red_chromaticity <= 0.329 {
if features.intensity <= 26 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.intensity <= 37 {
if features.saturation <= 181 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 144 {
Intensity::High
} else {
if features.green_luminance <= 97 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.blue_chromaticity <= 0.101 {
Intensity::High
} else {
if features.luminance <= 41 {
if features.blue_luminance <= 9 {
Intensity::Low
} else {
if features.saturation <= 202 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.343 {
if features.saturation <= 156 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_difference <= 107 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.green_chromaticity <= 0.514 {
if features.green_chromaticity <= 0.484 {
if features.red_chromaticity <= 0.326 {
Intensity::Low
} else {
if features.saturation <= 146 {
if features.blue_chromaticity <= 0.203 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 150 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.494 {
Intensity::High
} else {
if features.saturation <= 170 {
if features.luminance <= 49 {
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
if features.red_luminance <= 27 {
if features.green_chromaticity <= 0.618 {
if features.red_luminance <= 24 {
if features.red_chromaticity <= 0.320 {
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
if features.saturation <= 194 {
if features.green_chromaticity <= 0.531 {
if features.green_chromaticity <= 0.525 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.547 {
Intensity::Low
} else {
if features.luminance <= 37 {
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
} else {
if features.red_chromaticity <= 0.330 {
if features.red_luminance <= 22 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.181 {
if features.red_chromaticity <= 0.322 {
if features.blue_chromaticity <= 0.171 {
if features.green_chromaticity <= 0.529 {
if features.saturation <= 173 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 12 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 18 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_luminance <= 54 {
if features.saturation <= 183 {
Intensity::High
} else {
if features.saturation <= 187 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.324 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.173 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.red_luminance <= 42 {
if features.saturation <= 147 {
if features.green_chromaticity <= 0.475 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 23 {
if features.green_chromaticity <= 0.494 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.484 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.321 {
if features.green_luminance <= 63 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.luminance <= 53 {
Intensity::High
} else {
if features.intensity <= 46 {
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
if features.red_chromaticity <= 0.338 {
if features.green_chromaticity <= 0.529 {
if features.red_chromaticity <= 0.338 {
if features.saturation <= 139 {
if features.red_luminance <= 46 {
Intensity::High
} else {
if features.green_chromaticity <= 0.450 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 39 {
if features.green_luminance <= 54 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 43 {
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
if features.luminance <= 35 {
if features.green_luminance <= 35 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.604 {
if features.green_luminance <= 42 {
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
if features.blue_chromaticity <= 0.163 {
if features.red_chromaticity <= 0.341 {
if features.green_chromaticity <= 0.524 {
if features.red_luminance <= 33 {
if features.luminance <= 40 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 51 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 4 {
if features.blue_chromaticity <= 0.040 {
if features.blue_chromaticity <= 0.009 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.342 {
if features.red_chromaticity <= 0.341 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 221 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_luminance <= 43 {
if features.luminance <= 49 {
if features.saturation <= 159 {
Intensity::High
} else {
if features.green_chromaticity <= 0.489 {
Intensity::High
} else {
Intensity::High
}
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.468 {
if features.luminance <= 56 {
if features.blue_chromaticity <= 0.200 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 30 {
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
}
} else {
if features.saturation <= 117 {
if features.green_chromaticity <= 0.430 {
if features.blue_difference <= 113 {
if features.saturation <= 110 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.429 {
if features.saturation <= 114 {
if features.luminance <= 83 {
if features.green_chromaticity <= 0.429 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.337 {
if features.luminance <= 82 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 95 {
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
} else {
if features.red_chromaticity <= 0.329 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_difference <= 110 {
if features.blue_chromaticity <= 0.234 {
Intensity::High
} else {
if features.blue_difference <= 109 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.244 {
if features.blue_chromaticity <= 0.243 {
if features.blue_chromaticity <= 0.241 {
if features.blue_chromaticity <= 0.234 {
if features.value <= 78 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.236 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 63 {
Intensity::Low
} else {
Intensity::Low
}
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
if features.red_difference <= 121 {
if features.red_chromaticity <= 0.341 {
if features.blue_chromaticity <= 0.230 {
if features.blue_chromaticity <= 0.228 {
if features.blue_chromaticity <= 0.216 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.219 {
if features.blue_difference <= 109 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.341 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_difference <= 108 {
if features.blue_chromaticity <= 0.229 {
if features.green_chromaticity <= 0.432 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.hue <= 45 {
Intensity::High
} else {
if features.green_luminance <= 78 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.green_chromaticity <= 0.433 {
if features.value <= 100 {
if features.red_luminance <= 74 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 46 {
if features.value <= 77 {
if features.green_chromaticity <= 0.444 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.439 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.blue_difference <= 109 {
Intensity::Low
} else {
if features.blue_luminance <= 48 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.blue_luminance <= 66 {
if features.blue_chromaticity <= 0.217 {
if features.green_chromaticity <= 0.433 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.216 {
Intensity::High
} else {
if features.red_chromaticity <= 0.344 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.blue_chromaticity <= 0.224 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.224 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.345 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.blue_luminance <= 69 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.338 {
if features.green_luminance <= 70 {
if features.blue_luminance <= 35 {
if features.luminance <= 58 {
if features.green_chromaticity <= 0.448 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 68 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.value <= 71 {
if features.green_chromaticity <= 0.441 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.435 {
if features.green_luminance <= 76 {
if features.value <= 74 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.231 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.225 {
if features.blue_chromaticity <= 0.220 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.436 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.green_luminance <= 96 {
if features.intensity <= 71 {
if features.saturation <= 127 {
if features.red_luminance <= 65 {
if features.value <= 82 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.229 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.intensity <= 65 {
if features.blue_chromaticity <= 0.217 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.227 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.348 {
if features.red_chromaticity <= 0.346 {
if features.saturation <= 124 {
if features.green_chromaticity <= 0.429 {
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
} else {
if features.intensity <= 81 {
Intensity::High
} else {
if features.red_luminance <= 95 {
Intensity::Low
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
if features.red_difference <= 124 {
if features.blue_difference <= 112 {
if features.green_chromaticity <= 0.448 {
if features.luminance <= 117 {
if features.blue_difference <= 101 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.196 {
if features.green_luminance <= 100 {
Intensity::High
} else {
if features.luminance <= 91 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.green_luminance <= 101 {
if features.red_chromaticity <= 0.350 {
if features.blue_difference <= 109 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.saturation <= 131 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.431 {
if features.value <= 105 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.356 {
Intensity::High
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
if features.blue_difference <= 98 {
Intensity::Low
} else {
if features.blue_difference <= 109 {
if features.blue_difference <= 101 {
Intensity::High
} else {
if features.intensity <= 58 {
if features.luminance <= 56 {
if features.value <= 47 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.359 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.449 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.163 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_chromaticity <= 0.054 {
Intensity::High
} else {
if features.red_chromaticity <= 0.363 {
if features.intensity <= 52 {
if features.blue_chromaticity <= 0.190 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.350 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_difference <= 110 {
if features.saturation <= 212 {
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
}
} else {
if features.red_difference <= 123 {
if features.blue_chromaticity <= 0.226 {
if features.blue_difference <= 113 {
if features.saturation <= 147 {
if features.red_luminance <= 55 {
if features.value <= 64 {
if features.intensity <= 45 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 140 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.341 {
if features.green_luminance <= 74 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 36 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.348 {
if features.saturation <= 175 {
if features.red_chromaticity <= 0.344 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.intensity <= 18 {
Intensity::High
} else {
if features.luminance <= 32 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_chromaticity <= 0.570 {
if features.saturation <= 202 {
if features.red_chromaticity <= 0.345 {
if features.saturation <= 166 {
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
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.341 {
if features.blue_chromaticity <= 0.227 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.341 {
if features.blue_difference <= 113 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.339 {
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
if features.red_chromaticity <= 0.347 {
if features.red_chromaticity <= 0.345 {
if features.blue_chromaticity <= 0.217 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.344 {
Intensity::Low
} else {
if features.value <= 66 {
Intensity::Low
} else {
if features.saturation <= 125 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.luminance <= 64 {
if features.red_chromaticity <= 0.346 {
if features.green_luminance <= 63 {
if features.blue_luminance <= 29 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.195 {
if features.red_chromaticity <= 0.347 {
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
}
} else {
if features.green_luminance <= 63 {
if features.red_chromaticity <= 0.356 {
if features.green_chromaticity <= 0.484 {
if features.red_chromaticity <= 0.348 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.351 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 177 {
if features.green_luminance <= 49 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 10 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.intensity <= 33 {
if features.blue_difference <= 113 {
if features.saturation <= 215 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 244 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
Intensity::High
}
}
} else {
if features.green_luminance <= 70 {
if features.saturation <= 132 {
if features.luminance <= 60 {
Intensity::Low
} else {
if features.intensity <= 52 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.349 {
Intensity::Low
} else {
if features.saturation <= 136 {
Intensity::High
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
if features.red_difference <= 125 {
if features.red_chromaticity <= 0.378 {
if features.luminance <= 52 {
if features.red_luminance <= 45 {
if features.green_luminance <= 56 {
if features.green_luminance <= 55 {
if features.blue_luminance <= 14 {
Intensity::Low
} else {
if features.green_luminance <= 51 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_difference <= 111 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.461 {
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
if features.luminance <= 51 {
if features.green_luminance <= 57 {
if features.green_chromaticity <= 0.449 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.blue_difference <= 112 {
Intensity::Low
} else {
if features.blue_luminance <= 26 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_chromaticity <= 0.438 {
if features.luminance <= 97 {
if features.blue_difference <= 101 {
Intensity::High
} else {
if features.green_chromaticity <= 0.430 {
if features.green_chromaticity <= 0.429 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.434 {
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
if features.intensity <= 67 {
if features.value <= 79 {
if features.blue_chromaticity <= 0.197 {
if features.blue_chromaticity <= 0.172 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.361 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.440 {
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
if features.saturation <= 186 {
if features.value <= 123 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.intensity <= 31 {
if features.intensity <= 20 {
if features.value <= 31 {
if features.green_luminance <= 29 {
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
} else {
if features.green_chromaticity <= 0.505 {
if features.intensity <= 36 {
Intensity::High
} else {
if features.luminance <= 92 {
if features.luminance <= 75 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 30 {
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
} else {
if features.green_chromaticity <= 0.510 {
if features.red_difference <= 126 {
if features.blue_difference <= 89 {
Intensity::High
} else {
if features.blue_difference <= 113 {
if features.blue_luminance <= 27 {
if features.blue_difference <= 104 {
Intensity::High
} else {
if features.red_luminance <= 42 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 63 {
if features.value <= 69 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 136 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
Intensity::Low
}
}
} else {
if features.hue <= 35 {
if features.saturation <= 163 {
if features.blue_chromaticity <= 0.158 {
if features.blue_chromaticity <= 0.157 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.401 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.401 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.blue_luminance <= 1 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.luminance <= 72 {
if features.blue_difference <= 112 {
if features.green_luminance <= 54 {
if features.intensity <= 30 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.184 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.blue_difference <= 103 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
} else {
if features.blue_difference <= 113 {
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
} else {
if features.green_chromaticity <= 0.474 {
if features.blue_difference <= 116 {
if features.red_chromaticity <= 0.298 {
if features.blue_luminance <= 112 {
if features.red_chromaticity <= 0.280 {
if features.red_difference <= 109 {
if features.luminance <= 121 {
if features.red_chromaticity <= 0.256 {
if features.red_difference <= 102 {
if features.red_luminance <= 59 {
if features.green_chromaticity <= 0.473 {
if features.luminance <= 96 {
if features.red_chromaticity <= 0.232 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.red_luminance <= 81 {
if features.red_difference <= 100 {
if features.value <= 129 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_difference <= 115 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.300 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.blue_chromaticity <= 0.292 {
if features.red_chromaticity <= 0.255 {
if features.blue_chromaticity <= 0.279 {
if features.green_chromaticity <= 0.473 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.288 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_difference <= 107 {
if features.saturation <= 112 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.256 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.blue_chromaticity <= 0.292 {
if features.green_luminance <= 114 {
if features.intensity <= 80 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.251 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.299 {
if features.red_difference <= 104 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.445 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.intensity <= 95 {
if features.blue_difference <= 115 {
if features.saturation <= 106 {
if features.green_chromaticity <= 0.449 {
if features.intensity <= 93 {
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
if features.red_chromaticity <= 0.256 {
if features.blue_chromaticity <= 0.278 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.257 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.value <= 122 {
if features.red_chromaticity <= 0.265 {
if features.green_chromaticity <= 0.446 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.271 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.437 {
if features.green_chromaticity <= 0.436 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 105 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.blue_chromaticity <= 0.295 {
if features.red_luminance <= 78 {
if features.green_luminance <= 131 {
if features.value <= 129 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.435 {
if features.red_luminance <= 87 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.268 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.blue_chromaticity <= 0.296 {
if features.red_chromaticity <= 0.268 {
if features.saturation <= 103 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 94 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 83 {
if features.blue_luminance <= 87 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.301 {
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
if features.blue_chromaticity <= 0.304 {
if features.red_chromaticity <= 0.276 {
if features.red_luminance <= 95 {
if features.red_difference <= 105 {
if features.green_luminance <= 143 {
if features.red_difference <= 104 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.304 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.274 {
Intensity::High
} else {
if features.red_chromaticity <= 0.274 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.value <= 147 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.297 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.301 {
if features.blue_luminance <= 101 {
Intensity::High
} else {
if features.green_chromaticity <= 0.422 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.277 {
Intensity::High
} else {
if features.intensity <= 122 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.green_chromaticity <= 0.420 {
Intensity::High
} else {
if features.green_chromaticity <= 0.430 {
if features.blue_chromaticity <= 0.305 {
if features.green_chromaticity <= 0.422 {
if features.red_chromaticity <= 0.275 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.intensity <= 113 {
if features.green_luminance <= 145 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.306 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_luminance <= 145 {
if features.red_luminance <= 88 {
if features.red_difference <= 102 {
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
}
}
}
}
} else {
if features.green_chromaticity <= 0.451 {
if features.red_difference <= 110 {
if features.blue_chromaticity <= 0.283 {
if features.intensity <= 77 {
Intensity::High
} else {
if features.green_chromaticity <= 0.447 {
if features.green_chromaticity <= 0.444 {
if features.green_luminance <= 110 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 68 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.saturation <= 100 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.436 {
if features.green_chromaticity <= 0.434 {
if features.red_chromaticity <= 0.278 {
if features.red_luminance <= 75 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.278 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_luminance <= 115 {
Intensity::High
} else {
if features.green_chromaticity <= 0.436 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.439 {
if features.blue_chromaticity <= 0.285 {
Intensity::High
} else {
if features.green_chromaticity <= 0.437 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 84 {
if features.blue_chromaticity <= 0.286 {
Intensity::High
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
if features.red_chromaticity <= 0.270 {
if features.red_chromaticity <= 0.269 {
Intensity::High
} else {
if features.saturation <= 101 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_difference <= 115 {
if features.red_chromaticity <= 0.278 {
if features.luminance <= 93 {
if features.red_chromaticity <= 0.276 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.intensity <= 86 {
if features.green_chromaticity <= 0.439 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.271 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.275 {
if features.red_chromaticity <= 0.274 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.intensity <= 66 {
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
if features.green_chromaticity <= 0.466 {
if features.red_difference <= 113 {
if features.green_chromaticity <= 0.452 {
if features.blue_chromaticity <= 0.277 {
if features.blue_luminance <= 61 {
if features.intensity <= 68 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.saturation <= 102 {
Intensity::High
} else {
if features.red_chromaticity <= 0.266 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.270 {
if features.blue_chromaticity <= 0.279 {
if features.luminance <= 82 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 106 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.275 {
if features.luminance <= 75 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 56 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.blue_difference <= 115 {
if features.saturation <= 110 {
if features.green_chromaticity <= 0.459 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.277 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.271 {
Intensity::High
} else {
if features.red_luminance <= 51 {
if features.saturation <= 105 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.452 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.red_luminance <= 47 {
if features.blue_chromaticity <= 0.255 {
if features.green_chromaticity <= 0.469 {
Intensity::High
} else {
if features.saturation <= 120 {
if features.blue_luminance <= 38 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 46 {
if features.green_chromaticity <= 0.472 {
if features.red_chromaticity <= 0.274 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.value <= 79 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.258 {
Intensity::High
} else {
if features.red_difference <= 112 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.blue_chromaticity <= 0.274 {
if features.saturation <= 109 {
Intensity::High
} else {
if features.red_luminance <= 48 {
if features.red_chromaticity <= 0.255 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.268 {
Intensity::High
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
}
} else {
if features.value <= 120 {
if features.green_chromaticity <= 0.444 {
if features.red_difference <= 112 {
if features.blue_difference <= 115 {
if features.red_chromaticity <= 0.280 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.284 {
if features.red_chromaticity <= 0.283 {
if features.value <= 105 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.287 {
if features.green_chromaticity <= 0.435 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.425 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.saturation <= 88 {
if features.blue_chromaticity <= 0.292 {
if features.saturation <= 87 {
if features.red_chromaticity <= 0.287 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.value <= 113 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 78 {
Intensity::High
} else {
if features.saturation <= 85 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.281 {
if features.red_chromaticity <= 0.280 {
if features.saturation <= 90 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.433 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_difference <= 115 {
if features.green_chromaticity <= 0.439 {
if features.green_luminance <= 117 {
if features.red_chromaticity <= 0.298 {
if features.red_chromaticity <= 0.294 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.275 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 85 {
if features.red_chromaticity <= 0.293 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.294 {
if features.red_chromaticity <= 0.284 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.284 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.saturation <= 101 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.437 {
if features.hue <= 59 {
if features.red_luminance <= 84 {
if features.green_chromaticity <= 0.434 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 119 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 75 {
if features.green_chromaticity <= 0.429 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 112 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.red_difference <= 113 {
if features.green_chromaticity <= 0.438 {
Intensity::High
} else {
if features.green_luminance <= 99 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.295 {
if features.blue_chromaticity <= 0.271 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 49 {
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
if features.green_chromaticity <= 0.460 {
if features.red_difference <= 117 {
if features.red_chromaticity <= 0.296 {
if features.green_chromaticity <= 0.457 {
if features.blue_chromaticity <= 0.266 {
if features.blue_chromaticity <= 0.266 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.saturation <= 99 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 48 {
if features.red_difference <= 116 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_luminance <= 51 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.luminance <= 66 {
Intensity::High
} else {
if features.value <= 79 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.294 {
Intensity::High
} else {
if features.green_chromaticity <= 0.460 {
if features.red_chromaticity <= 0.297 {
if features.luminance <= 61 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 53 {
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
if features.red_difference <= 118 {
if features.green_chromaticity <= 0.464 {
if features.red_difference <= 117 {
if features.green_chromaticity <= 0.463 {
if features.blue_chromaticity <= 0.255 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.249 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.461 {
Intensity::High
} else {
if features.intensity <= 50 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.296 {
if features.red_chromaticity <= 0.291 {
if features.saturation <= 119 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.469 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.466 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.value <= 64 {
if features.value <= 63 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.saturation <= 123 {
Intensity::High
} else {
if features.saturation <= 125 {
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
if features.intensity <= 116 {
if features.green_chromaticity <= 0.413 {
if features.red_difference <= 113 {
if features.blue_chromaticity <= 0.294 {
if features.green_chromaticity <= 0.412 {
if features.green_chromaticity <= 0.411 {
if features.green_chromaticity <= 0.410 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.296 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_luminance <= 92 {
if features.blue_luminance <= 88 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.407 {
if features.blue_chromaticity <= 0.298 {
if features.saturation <= 68 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 140 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.hue <= 62 {
if features.red_chromaticity <= 0.291 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.413 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
} else {
if features.luminance <= 108 {
if features.saturation <= 73 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 70 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.298 {
if features.saturation <= 71 {
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
} else {
if features.green_luminance <= 123 {
if features.red_chromaticity <= 0.283 {
if features.saturation <= 88 {
if features.red_chromaticity <= 0.282 {
if features.value <= 122 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.293 {
if features.red_chromaticity <= 0.296 {
if features.blue_chromaticity <= 0.287 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.intensity <= 95 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.294 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.green_chromaticity <= 0.420 {
if features.luminance <= 108 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.296 {
if features.luminance <= 123 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 76 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.red_difference <= 110 {
if features.red_chromaticity <= 0.284 {
if features.blue_chromaticity <= 0.299 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.287 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.288 {
if features.value <= 124 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 88 {
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
if features.green_chromaticity <= 0.407 {
if features.blue_chromaticity <= 0.300 {
if features.intensity <= 123 {
if features.red_chromaticity <= 0.296 {
if features.saturation <= 70 {
if features.red_chromaticity <= 0.295 {
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
if features.blue_chromaticity <= 0.299 {
if features.blue_luminance <= 111 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.value <= 146 {
if features.blue_chromaticity <= 0.302 {
if features.red_luminance <= 103 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.403 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 148 {
if features.green_chromaticity <= 0.407 {
if features.blue_chromaticity <= 0.301 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 149 {
if features.green_chromaticity <= 0.404 {
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
} else {
if features.blue_chromaticity <= 0.301 {
if features.red_chromaticity <= 0.293 {
if features.blue_chromaticity <= 0.301 {
if features.red_luminance <= 103 {
if features.green_chromaticity <= 0.416 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.value <= 148 {
Intensity::High
} else {
Intensity::High
}
}
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.294 {
if features.red_chromaticity <= 0.293 {
Intensity::Low
} else {
if features.green_luminance <= 147 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 106 {
if features.red_chromaticity <= 0.294 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.value <= 147 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.blue_chromaticity <= 0.303 {
if features.green_luminance <= 147 {
if features.blue_chromaticity <= 0.302 {
if features.value <= 144 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 78 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.value <= 154 {
if features.green_luminance <= 151 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.282 {
if features.blue_luminance <= 110 {
if features.value <= 147 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.luminance <= 129 {
if features.green_luminance <= 146 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 102 {
Intensity::Low
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
if features.green_luminance <= 166 {
if features.blue_chromaticity <= 0.303 {
if features.green_chromaticity <= 0.407 {
if features.green_chromaticity <= 0.404 {
if features.red_chromaticity <= 0.297 {
if features.red_chromaticity <= 0.297 {
if features.green_chromaticity <= 0.402 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.403 {
if features.saturation <= 67 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.297 {
Intensity::Low
} else {
if features.saturation <= 66 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.403 {
if features.red_chromaticity <= 0.298 {
if features.value <= 158 {
if features.saturation <= 66 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.298 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.292 {
if features.value <= 164 {
if features.luminance <= 142 {
if features.green_chromaticity <= 0.406 {
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
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.299 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.405 {
if features.red_chromaticity <= 0.296 {
if features.red_difference <= 109 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 114 {
if features.green_luminance <= 153 {
Intensity::Low
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
}
}
}
}
} else {
if features.red_chromaticity <= 0.287 {
if features.blue_chromaticity <= 0.303 {
if features.red_luminance <= 104 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.281 {
if features.saturation <= 84 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.luminance <= 136 {
Intensity::High
} else {
if features.intensity <= 127 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.green_luminance <= 159 {
if features.blue_chromaticity <= 0.303 {
if features.red_chromaticity <= 0.280 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 137 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.value <= 160 {
if features.red_difference <= 105 {
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
if features.green_luminance <= 158 {
if features.green_chromaticity <= 0.409 {
if features.red_luminance <= 110 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.408 {
if features.luminance <= 139 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 108 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.411 {
if features.red_chromaticity <= 0.289 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.411 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_chromaticity <= 0.409 {
if features.blue_chromaticity <= 0.302 {
Intensity::Low
} else {
if features.red_luminance <= 114 {
if features.luminance <= 142 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 120 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.intensity <= 130 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.value <= 162 {
if features.green_chromaticity <= 0.402 {
if features.green_chromaticity <= 0.399 {
if features.green_chromaticity <= 0.398 {
if features.blue_luminance <= 123 {
if features.blue_luminance <= 121 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.397 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.400 {
if features.blue_luminance <= 118 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.295 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.296 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.401 {
if features.saturation <= 66 {
Intensity::Low
} else {
if features.intensity <= 131 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 67 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.402 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.blue_chromaticity <= 0.307 {
if features.green_chromaticity <= 0.404 {
if features.red_chromaticity <= 0.291 {
if features.blue_chromaticity <= 0.306 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.292 {
Intensity::Low
} else {
if features.red_luminance <= 111 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_luminance <= 103 {
if features.intensity <= 124 {
Intensity::High
} else {
if features.red_difference <= 101 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.289 {
if features.intensity <= 129 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 73 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_chromaticity <= 0.308 {
if features.value <= 160 {
if features.green_luminance <= 156 {
if features.value <= 155 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.277 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.307 {
if features.blue_chromaticity <= 0.307 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 95 {
if features.saturation <= 91 {
if features.green_chromaticity <= 0.418 {
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
}
}
} else {
if features.hue <= 65 {
if features.blue_luminance <= 122 {
if features.red_chromaticity <= 0.284 {
if features.green_chromaticity <= 0.416 {
if features.value <= 163 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.413 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.value <= 164 {
if features.green_luminance <= 163 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 134 {
if features.red_luminance <= 116 {
if features.saturation <= 76 {
if features.green_chromaticity <= 0.406 {
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
if features.green_chromaticity <= 0.402 {
if features.intensity <= 138 {
if features.blue_luminance <= 126 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.403 {
if features.green_chromaticity <= 0.402 {
Intensity::Low
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
}
}
}
} else {
if features.red_difference <= 105 {
if features.green_luminance <= 164 {
if features.blue_chromaticity <= 0.310 {
if features.red_chromaticity <= 0.276 {
if features.value <= 163 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 85 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.311 {
if features.blue_luminance <= 123 {
if features.blue_chromaticity <= 0.305 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.hue <= 66 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 127 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.blue_chromaticity <= 0.307 {
Intensity::Low
} else {
if features.saturation <= 78 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.410 {
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
} else {
if features.hue <= 63 {
if features.blue_chromaticity <= 0.307 {
if features.blue_chromaticity <= 0.305 {
if features.green_luminance <= 171 {
if features.green_luminance <= 169 {
if features.green_luminance <= 168 {
if features.red_difference <= 108 {
if features.saturation <= 71 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 167 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 123 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.401 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.404 {
if features.green_luminance <= 170 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
}
} else {
if features.saturation <= 69 {
if features.green_chromaticity <= 0.399 {
Intensity::Low
} else {
if features.red_luminance <= 127 {
if features.green_luminance <= 172 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 144 {
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
} else {
if features.red_chromaticity <= 0.294 {
if features.red_chromaticity <= 0.294 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.398 {
if features.red_chromaticity <= 0.297 {
if features.blue_chromaticity <= 0.307 {
if features.red_luminance <= 135 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 184 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.307 {
if features.green_chromaticity <= 0.396 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 176 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.400 {
if features.value <= 179 {
if features.red_difference <= 108 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.399 {
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
} else {
if features.blue_luminance <= 138 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.396 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.309 {
if features.red_difference <= 105 {
if features.green_chromaticity <= 0.401 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.307 {
if features.blue_chromaticity <= 0.305 {
if features.green_luminance <= 167 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 173 {
if features.green_luminance <= 168 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 174 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.282 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.410 {
if features.blue_chromaticity <= 0.307 {
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
} else {
if features.intensity <= 136 {
if features.hue <= 65 {
if features.green_luminance <= 167 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.value <= 180 {
if features.blue_chromaticity <= 0.305 {
if features.blue_chromaticity <= 0.304 {
Intensity::Low
} else {
if features.intensity <= 138 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.291 {
if features.luminance <= 148 {
Intensity::Low
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
if features.green_chromaticity <= 0.401 {
if features.red_luminance <= 136 {
if features.green_luminance <= 186 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 138 {
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
} else {
if features.blue_chromaticity <= 0.310 {
if features.green_luminance <= 167 {
if features.saturation <= 85 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.292 {
if features.blue_luminance <= 124 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.406 {
if features.red_chromaticity <= 0.286 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.309 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.292 {
if features.green_luminance <= 180 {
Intensity::Low
} else {
if features.red_difference <= 106 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.luminance <= 162 {
if features.luminance <= 158 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.394 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.blue_chromaticity <= 0.311 {
if features.red_difference <= 105 {
if features.blue_chromaticity <= 0.311 {
if features.red_difference <= 97 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.311 {
if features.saturation <= 81 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.311 {
if features.blue_chromaticity <= 0.311 {
if features.intensity <= 159 {
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
if features.green_chromaticity <= 0.390 {
if features.green_chromaticity <= 0.382 {
if features.green_chromaticity <= 0.378 {
if features.green_chromaticity <= 0.375 {
if features.green_chromaticity <= 0.373 {
if features.green_chromaticity <= 0.370 {
if features.saturation <= 34 {
if features.red_chromaticity <= 0.349 {
if features.green_chromaticity <= 0.355 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.355 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.349 {
Intensity::Low
} else {
if features.blue_luminance <= 164 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_luminance <= 143 {
Intensity::Low
} else {
if features.luminance <= 138 {
if features.green_chromaticity <= 0.364 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 49 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_luminance <= 154 {
if features.green_chromaticity <= 0.370 {
if features.saturation <= 58 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.327 {
if features.red_chromaticity <= 0.327 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.332 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.370 {
if features.green_chromaticity <= 0.370 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 159 {
if features.green_luminance <= 183 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.303 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.green_luminance <= 219 {
if features.blue_luminance <= 148 {
if features.red_chromaticity <= 0.332 {
if features.saturation <= 54 {
if features.blue_chromaticity <= 0.298 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.374 {
if features.green_chromaticity <= 0.374 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.374 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.saturation <= 42 {
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
if features.blue_luminance <= 145 {
if features.blue_luminance <= 103 {
if features.blue_chromaticity <= 0.293 {
if features.blue_luminance <= 89 {
Intensity::Low
} else {
if features.blue_luminance <= 94 {
if features.red_chromaticity <= 0.335 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.376 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.377 {
if features.green_luminance <= 133 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.304 {
if features.value <= 156 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.375 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.328 {
if features.red_chromaticity <= 0.326 {
if features.luminance <= 137 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.296 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.hue <= 47 {
if features.green_chromaticity <= 0.378 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.377 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.blue_chromaticity <= 0.304 {
if features.red_chromaticity <= 0.320 {
if features.green_chromaticity <= 0.376 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.375 {
Intensity::Low
} else {
if features.intensity <= 160 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.321 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_chromaticity <= 0.312 {
if features.red_luminance <= 151 {
if features.red_luminance <= 150 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.311 {
if features.red_luminance <= 156 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 186 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_luminance <= 177 {
if features.red_difference <= 111 {
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
if features.blue_luminance <= 144 {
if features.red_luminance <= 110 {
if features.saturation <= 65 {
if features.blue_chromaticity <= 0.288 {
if features.intensity <= 106 {
if features.luminance <= 112 {
if features.value <= 112 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 106 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.332 {
if features.green_chromaticity <= 0.379 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 78 {
if features.red_difference <= 127 {
Intensity::Low
} else {
if features.red_luminance <= 68 {
if features.blue_chromaticity <= 0.249 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 84 {
Intensity::Low
} else {
if features.intensity <= 100 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_luminance <= 131 {
if features.green_chromaticity <= 0.382 {
if features.value <= 139 {
if features.saturation <= 58 {
if features.green_luminance <= 136 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.328 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.382 {
if features.green_chromaticity <= 0.379 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_difference <= 118 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_luminance <= 137 {
if features.red_difference <= 121 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 58 {
if features.red_luminance <= 123 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
}
} else {
if features.saturation <= 49 {
if features.red_chromaticity <= 0.313 {
if features.green_chromaticity <= 0.380 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.315 {
if features.hue <= 56 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.379 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.hue <= 56 {
if features.blue_difference <= 115 {
if features.red_difference <= 117 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.381 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.381 {
if features.value <= 174 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.382 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
}
} else {
if features.luminance <= 171 {
if features.value <= 180 {
if features.blue_luminance <= 145 {
if features.blue_chromaticity <= 0.307 {
Intensity::Low
} else {
if features.red_difference <= 114 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.value <= 183 {
if features.red_chromaticity <= 0.316 {
if features.saturation <= 48 {
if features.red_luminance <= 150 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.382 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.304 {
if features.value <= 181 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.382 {
if features.saturation <= 50 {
if features.red_luminance <= 150 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 161 {
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
if features.value <= 187 {
if features.red_luminance <= 152 {
Intensity::Low
} else {
if features.red_luminance <= 154 {
if features.green_chromaticity <= 0.380 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.381 {
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
if features.blue_chromaticity <= 0.309 {
Intensity::Low
} else {
if features.value <= 189 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.382 {
if features.red_luminance <= 163 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.382 {
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
} else {
if features.blue_luminance <= 141 {
if features.green_luminance <= 122 {
if features.luminance <= 108 {
if features.blue_chromaticity <= 0.287 {
if features.green_chromaticity <= 0.389 {
if features.value <= 116 {
if features.blue_chromaticity <= 0.286 {
if features.red_luminance <= 90 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 99 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 59 {
if features.saturation <= 106 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.390 {
if features.green_chromaticity <= 0.389 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.229 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.saturation <= 65 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.388 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.321 {
Intensity::Low
} else {
if features.red_luminance <= 104 {
if features.red_chromaticity <= 0.331 {
if features.red_chromaticity <= 0.327 {
if features.red_luminance <= 99 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.327 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 88 {
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
if features.saturation <= 57 {
if features.saturation <= 54 {
if features.red_chromaticity <= 0.307 {
if features.green_chromaticity <= 0.386 {
Intensity::Low
} else {
if features.intensity <= 146 {
if features.red_chromaticity <= 0.306 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.387 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.314 {
if features.red_chromaticity <= 0.308 {
if features.green_chromaticity <= 0.388 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.382 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 147 {
if features.luminance <= 143 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.315 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_difference <= 112 {
if features.red_luminance <= 138 {
if features.red_chromaticity <= 0.305 {
if features.green_chromaticity <= 0.389 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.306 {
if features.saturation <= 55 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 152 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.saturation <= 56 {
if features.value <= 176 {
if features.green_luminance <= 168 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 151 {
if features.red_luminance <= 120 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.298 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.red_chromaticity <= 0.322 {
if features.saturation <= 58 {
if features.red_luminance <= 131 {
if features.red_difference <= 117 {
if features.blue_chromaticity <= 0.299 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.383 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_difference <= 113 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 118 {
if features.green_luminance <= 130 {
if features.red_luminance <= 103 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.388 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.hue <= 51 {
if features.luminance <= 136 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 115 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_chromaticity <= 0.325 {
if features.red_luminance <= 105 {
if features.saturation <= 62 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.322 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.value <= 128 {
if features.green_luminance <= 127 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.385 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_luminance <= 108 {
if features.red_chromaticity <= 0.326 {
if features.green_chromaticity <= 0.383 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.285 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.327 {
if features.green_chromaticity <= 0.384 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 131 {
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
} else {
if features.blue_luminance <= 144 {
if features.hue <= 59 {
if features.red_chromaticity <= 0.310 {
if features.hue <= 58 {
if features.red_chromaticity <= 0.309 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.386 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.luminance <= 166 {
if features.value <= 181 {
if features.blue_chromaticity <= 0.306 {
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
}
} else {
if features.value <= 180 {
if features.green_luminance <= 179 {
if features.value <= 178 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.310 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.309 {
if features.red_chromaticity <= 0.305 {
if features.blue_chromaticity <= 0.309 {
if features.blue_chromaticity <= 0.307 {
Intensity::Low
} else {
if features.intensity <= 154 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.308 {
if features.red_chromaticity <= 0.306 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.307 {
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
Intensity::Low
}
}
} else {
if features.blue_luminance <= 147 {
if features.saturation <= 55 {
if features.intensity <= 159 {
if features.red_chromaticity <= 0.308 {
if features.hue <= 61 {
if features.red_luminance <= 144 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.384 {
if features.blue_chromaticity <= 0.305 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 145 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.311 {
if features.red_difference <= 112 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.309 {
if features.red_chromaticity <= 0.303 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.390 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 142 {
Intensity::Low
} else {
if features.saturation <= 57 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.luminance <= 170 {
if features.red_luminance <= 147 {
if features.hue <= 62 {
if features.red_luminance <= 146 {
if features.blue_chromaticity <= 0.310 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.389 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 148 {
if features.saturation <= 51 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.311 {
if features.blue_chromaticity <= 0.306 {
if features.green_chromaticity <= 0.385 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.386 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 151 {
if features.green_chromaticity <= 0.386 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.311 {
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
} else {
if features.red_chromaticity <= 0.318 {
if features.blue_luminance <= 112 {
if features.value <= 120 {
if features.green_chromaticity <= 0.434 {
if features.red_chromaticity <= 0.306 {
if features.green_luminance <= 114 {
if features.blue_chromaticity <= 0.269 {
if features.red_chromaticity <= 0.300 {
Intensity::Low
} else {
if features.red_luminance <= 62 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_difference <= 115 {
if features.red_difference <= 115 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.299 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.luminance <= 105 {
if features.green_chromaticity <= 0.414 {
if features.green_chromaticity <= 0.409 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.416 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.406 {
if features.luminance <= 106 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 88 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_difference <= 115 {
if features.blue_chromaticity <= 0.282 {
if features.green_chromaticity <= 0.428 {
if features.red_chromaticity <= 0.309 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.432 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.405 {
if features.blue_chromaticity <= 0.284 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.311 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.424 {
if features.blue_chromaticity <= 0.288 {
if features.red_chromaticity <= 0.309 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.290 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.426 {
if features.red_difference <= 119 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.306 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.blue_difference <= 115 {
if features.red_chromaticity <= 0.301 {
if features.blue_luminance <= 42 {
if features.red_chromaticity <= 0.301 {
if features.red_chromaticity <= 0.300 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.301 {
if features.intensity <= 60 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 82 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.314 {
if features.green_luminance <= 82 {
if features.red_chromaticity <= 0.305 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 83 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.316 {
if features.green_chromaticity <= 0.440 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.317 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_chromaticity <= 0.312 {
if features.value <= 72 {
if features.blue_chromaticity <= 0.247 {
if features.green_chromaticity <= 0.471 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 70 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.302 {
if features.red_chromaticity <= 0.301 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.253 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.312 {
if features.red_chromaticity <= 0.312 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 115 {
if features.saturation <= 113 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 64 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
}
} else {
if features.green_chromaticity <= 0.401 {
if features.blue_luminance <= 107 {
if features.red_difference <= 117 {
if features.saturation <= 66 {
if features.green_chromaticity <= 0.392 {
if features.blue_chromaticity <= 0.293 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 140 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.luminance <= 111 {
if features.saturation <= 69 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.397 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.hue <= 52 {
if features.blue_chromaticity <= 0.286 {
if features.luminance <= 110 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 70 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.292 {
if features.blue_chromaticity <= 0.292 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 125 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.saturation <= 64 {
if features.saturation <= 61 {
if features.blue_chromaticity <= 0.299 {
if features.green_chromaticity <= 0.392 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 112 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.296 {
if features.blue_chromaticity <= 0.295 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.300 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.intensity <= 121 {
Intensity::Low
} else {
if features.blue_luminance <= 108 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.398 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.green_chromaticity <= 0.406 {
if features.blue_luminance <= 104 {
if features.red_luminance <= 91 {
if features.intensity <= 99 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.304 {
if features.red_chromaticity <= 0.301 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.313 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.297 {
if features.blue_chromaticity <= 0.296 {
if features.hue <= 58 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 109 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.hue <= 59 {
if features.value <= 150 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 105 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_chromaticity <= 0.286 {
if features.red_luminance <= 89 {
Intensity::Low
} else {
if features.red_difference <= 115 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.407 {
if features.value <= 122 {
if features.blue_chromaticity <= 0.289 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 102 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.luminance <= 120 {
if features.green_chromaticity <= 0.407 {
Intensity::High
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
} else {
if features.blue_chromaticity <= 0.303 {
if features.saturation <= 61 {
if features.saturation <= 60 {
if features.luminance <= 142 {
if features.red_chromaticity <= 0.305 {
if features.luminance <= 135 {
if features.blue_luminance <= 113 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_difference <= 113 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.393 {
if features.blue_difference <= 115 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.hue <= 58 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.saturation <= 59 {
if features.intensity <= 139 {
if features.red_chromaticity <= 0.308 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 130 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 123 {
if features.blue_chromaticity <= 0.299 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 139 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.luminance <= 137 {
if features.red_chromaticity <= 0.302 {
Intensity::Low
} else {
if features.value <= 150 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 122 {
if features.green_luminance <= 152 {
Intensity::Low
} else {
if features.intensity <= 130 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 121 {
if features.value <= 157 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.304 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.saturation <= 63 {
if features.blue_difference <= 115 {
if features.green_chromaticity <= 0.397 {
if features.blue_chromaticity <= 0.300 {
if features.blue_chromaticity <= 0.298 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.luminance <= 141 {
if features.red_difference <= 112 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.300 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.intensity <= 129 {
if features.red_chromaticity <= 0.300 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.301 {
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
if features.red_luminance <= 112 {
Intensity::Low
} else {
if features.luminance <= 146 {
if features.green_luminance <= 155 {
if features.red_chromaticity <= 0.301 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.299 {
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
} else {
if features.blue_luminance <= 138 {
if features.green_chromaticity <= 0.396 {
if features.blue_chromaticity <= 0.308 {
if features.red_difference <= 110 {
if features.intensity <= 148 {
if features.green_chromaticity <= 0.395 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.394 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.304 {
if features.red_luminance <= 122 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.304 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.393 {
if features.red_difference <= 110 {
Intensity::Low
} else {
if features.saturation <= 58 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.393 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.luminance <= 154 {
if features.green_chromaticity <= 0.397 {
if features.green_chromaticity <= 0.397 {
if features.red_chromaticity <= 0.301 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.300 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 126 {
if features.intensity <= 139 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.396 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.301 {
if features.luminance <= 166 {
if features.blue_chromaticity <= 0.309 {
if features.blue_difference <= 115 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.392 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.value <= 181 {
Intensity::Low
} else {
if features.blue_luminance <= 144 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.299 {
if features.green_luminance <= 190 {
if features.green_chromaticity <= 0.393 {
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
}
} else {
if features.blue_chromaticity <= 0.305 {
if features.green_chromaticity <= 0.392 {
if features.green_chromaticity <= 0.392 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 183 {
if features.red_chromaticity <= 0.301 {
if features.green_chromaticity <= 0.393 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.302 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_luminance <= 186 {
if features.blue_chromaticity <= 0.306 {
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
}
} else {
if features.value <= 123 {
if features.green_chromaticity <= 0.426 {
if features.red_chromaticity <= 0.332 {
if features.hue <= 50 {
if features.blue_chromaticity <= 0.268 {
if features.red_chromaticity <= 0.323 {
if features.blue_luminance <= 48 {
if features.green_luminance <= 79 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.323 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 70 {
if features.red_chromaticity <= 0.332 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 79 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_luminance <= 113 {
if features.green_chromaticity <= 0.405 {
if features.red_chromaticity <= 0.326 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 64 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.323 {
if features.red_chromaticity <= 0.323 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 68 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_chromaticity <= 0.318 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.397 {
if features.luminance <= 111 {
if features.green_chromaticity <= 0.396 {
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
}
}
} else {
if features.hue <= 40 {
if features.value <= 44 {
if features.blue_chromaticity <= 0.187 {
if features.red_chromaticity <= 0.401 {
if features.green_luminance <= 42 {
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
} else {
if features.blue_difference <= 115 {
if features.green_chromaticity <= 0.404 {
if features.red_chromaticity <= 0.353 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.406 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.391 {
if features.green_chromaticity <= 0.391 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
}
} else {
if features.luminance <= 73 {
if features.value <= 66 {
if features.red_luminance <= 52 {
if features.green_chromaticity <= 0.419 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_difference <= 124 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.234 {
if features.intensity <= 53 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.419 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.value <= 93 {
if features.red_luminance <= 74 {
if features.intensity <= 73 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.saturation <= 79 {
if features.red_chromaticity <= 0.335 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 97 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
}
} else {
if features.red_chromaticity <= 0.338 {
if features.red_chromaticity <= 0.325 {
if features.blue_chromaticity <= 0.248 {
if features.saturation <= 135 {
if features.luminance <= 65 {
if features.red_chromaticity <= 0.320 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.saturation <= 138 {
if features.luminance <= 48 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.467 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.saturation <= 101 {
Intensity::Low
} else {
if features.saturation <= 106 {
if features.blue_chromaticity <= 0.255 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
}
} else {
if features.blue_difference <= 115 {
if features.green_luminance <= 62 {
if features.saturation <= 128 {
Intensity::High
} else {
if features.red_chromaticity <= 0.328 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.336 {
if features.red_chromaticity <= 0.329 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.327 {
if features.saturation <= 122 {
if features.green_chromaticity <= 0.436 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 51 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_luminance <= 64 {
if features.saturation <= 141 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 50 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.red_chromaticity <= 0.355 {
if features.saturation <= 146 {
if features.value <= 54 {
if features.green_chromaticity <= 0.448 {
if features.green_chromaticity <= 0.433 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 38 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_luminance <= 55 {
if features.intensity <= 40 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.220 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.luminance <= 42 {
if features.value <= 47 {
Intensity::Low
} else {
if features.blue_luminance <= 19 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.345 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.465 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_chromaticity <= 0.168 {
if features.red_luminance <= 38 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.391 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_difference <= 125 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.luminance <= 118 {
if features.green_chromaticity <= 0.393 {
if features.luminance <= 117 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.395 {
if features.blue_chromaticity <= 0.287 {
Intensity::Low
} else {
if features.saturation <= 68 {
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
if features.green_chromaticity <= 0.391 {
Intensity::Low
} else {
if features.saturation <= 66 {
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
} else {
if features.blue_difference <= 118 {
if features.red_chromaticity <= 0.288 {
if features.blue_luminance <= 116 {
if features.red_difference <= 110 {
if features.blue_luminance <= 106 {
if features.red_difference <= 106 {
if features.luminance <= 101 {
if features.red_chromaticity <= 0.227 {
if features.green_chromaticity <= 0.470 {
Intensity::High
} else {
if features.saturation <= 137 {
if features.red_chromaticity <= 0.225 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 123 {
if features.luminance <= 89 {
if features.green_chromaticity <= 0.459 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.saturation <= 108 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.474 {
if features.luminance <= 95 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.309 {
if features.blue_chromaticity <= 0.307 {
if features.luminance <= 125 {
if features.blue_chromaticity <= 0.307 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_difference <= 105 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
Intensity::High
}
} else {
if features.red_difference <= 102 {
if features.blue_chromaticity <= 0.316 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.316 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 89 {
if features.blue_chromaticity <= 0.310 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.419 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.blue_difference <= 117 {
if features.blue_chromaticity <= 0.297 {
if features.green_chromaticity <= 0.463 {
if features.saturation <= 117 {
if features.green_chromaticity <= 0.458 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.251 {
if features.green_chromaticity <= 0.464 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.253 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.431 {
if features.blue_luminance <= 102 {
if features.blue_chromaticity <= 0.299 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 83 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.268 {
if features.red_chromaticity <= 0.264 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.269 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.red_luminance <= 51 {
if features.green_chromaticity <= 0.471 {
if features.red_chromaticity <= 0.238 {
Intensity::Low
} else {
if features.value <= 96 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.saturation <= 125 {
if features.red_difference <= 109 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.intensity <= 90 {
if features.red_chromaticity <= 0.255 {
if features.blue_chromaticity <= 0.295 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.300 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_difference <= 109 {
if features.blue_chromaticity <= 0.307 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.luminance <= 116 {
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
if features.blue_difference <= 117 {
if features.green_chromaticity <= 0.425 {
if features.blue_chromaticity <= 0.308 {
if features.blue_chromaticity <= 0.305 {
if features.hue <= 65 {
if features.red_difference <= 108 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.green_luminance <= 154 {
if features.blue_chromaticity <= 0.307 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 115 {
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
if features.red_luminance <= 92 {
Intensity::High
} else {
if features.luminance <= 129 {
if features.saturation <= 97 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.276 {
if features.red_difference <= 104 {
if features.green_chromaticity <= 0.424 {
if features.red_chromaticity <= 0.267 {
Intensity::Low
} else {
if features.saturation <= 92 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 90 {
Intensity::High
} else {
Intensity::High
}
}
} else {
Intensity::High
}
} else {
if features.green_luminance <= 144 {
if features.luminance <= 125 {
if features.red_chromaticity <= 0.278 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.407 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_luminance <= 143 {
if features.blue_chromaticity <= 0.308 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_luminance <= 97 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.411 {
if features.green_chromaticity <= 0.404 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.311 {
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
if features.green_chromaticity <= 0.454 {
if features.luminance <= 101 {
if features.green_chromaticity <= 0.439 {
if features.blue_difference <= 117 {
if features.blue_chromaticity <= 0.293 {
if features.saturation <= 90 {
if features.value <= 110 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 92 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 81 {
Intensity::Low
} else {
if features.red_luminance <= 74 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.279 {
if features.saturation <= 96 {
if features.green_chromaticity <= 0.428 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 103 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.430 {
if features.green_chromaticity <= 0.428 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 91 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_difference <= 117 {
if features.red_chromaticity <= 0.278 {
if features.green_chromaticity <= 0.450 {
if features.green_chromaticity <= 0.448 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.452 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.285 {
if features.red_chromaticity <= 0.283 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.287 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.268 {
if features.green_chromaticity <= 0.453 {
if features.green_chromaticity <= 0.448 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 90 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.445 {
if features.green_chromaticity <= 0.439 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.454 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.blue_luminance <= 91 {
if features.green_chromaticity <= 0.416 {
if features.green_luminance <= 119 {
if features.red_luminance <= 81 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.412 {
Intensity::Low
} else {
if features.saturation <= 77 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.423 {
if features.saturation <= 84 {
if features.blue_chromaticity <= 0.298 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 119 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.281 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.blue_chromaticity <= 0.300 {
if features.red_luminance <= 88 {
Intensity::High
} else {
if features.saturation <= 78 {
if features.green_chromaticity <= 0.414 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.blue_difference <= 117 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.305 {
if features.green_chromaticity <= 0.411 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.408 {
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
if features.blue_difference <= 117 {
if features.green_chromaticity <= 0.459 {
if features.red_chromaticity <= 0.263 {
if features.green_chromaticity <= 0.457 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 103 {
if features.blue_chromaticity <= 0.272 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.456 {
if features.green_chromaticity <= 0.455 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.457 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_chromaticity <= 0.251 {
if features.red_chromaticity <= 0.286 {
if features.red_difference <= 117 {
if features.blue_chromaticity <= 0.249 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 39 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.saturation <= 116 {
if features.red_chromaticity <= 0.268 {
if features.red_chromaticity <= 0.267 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.460 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.473 {
if features.green_luminance <= 86 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
}
}
} else {
if features.red_chromaticity <= 0.269 {
if features.saturation <= 110 {
if features.red_chromaticity <= 0.264 {
if features.green_luminance <= 79 {
Intensity::Low
} else {
if features.value <= 86 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.264 {
Intensity::Low
} else {
if features.green_luminance <= 72 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.270 {
if features.red_chromaticity <= 0.264 {
if features.red_chromaticity <= 0.260 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_difference <= 115 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.251 {
if features.red_chromaticity <= 0.251 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.255 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
} else {
if features.green_luminance <= 60 {
if features.luminance <= 49 {
if features.blue_luminance <= 30 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.luminance <= 53 {
if features.blue_chromaticity <= 0.253 {
if features.red_chromaticity <= 0.280 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.255 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 109 {
if features.red_difference <= 115 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.279 {
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
} else {
if features.green_luminance <= 160 {
if features.blue_chromaticity <= 0.307 {
if features.green_chromaticity <= 0.408 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.311 {
if features.red_chromaticity <= 0.287 {
if features.red_chromaticity <= 0.287 {
if features.saturation <= 92 {
if features.green_chromaticity <= 0.408 {
if features.saturation <= 76 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.310 {
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
} else {
if features.blue_chromaticity <= 0.309 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.410 {
if features.green_chromaticity <= 0.409 {
if features.red_luminance <= 107 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.313 {
if features.blue_chromaticity <= 0.312 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 106 {
Intensity::Low
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
if features.value <= 164 {
if features.blue_chromaticity <= 0.309 {
if features.red_chromaticity <= 0.286 {
Intensity::Low
} else {
if features.saturation <= 75 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.279 {
if features.green_chromaticity <= 0.412 {
Intensity::Low
} else {
if features.red_difference <= 103 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.273 {
Intensity::High
} else {
if features.red_chromaticity <= 0.274 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_chromaticity <= 0.281 {
if features.intensity <= 132 {
if features.blue_difference <= 117 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 163 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.313 {
if features.saturation <= 72 {
if features.blue_chromaticity <= 0.312 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.310 {
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
} else {
if features.blue_chromaticity <= 0.312 {
if features.red_chromaticity <= 0.279 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.404 {
if features.blue_chromaticity <= 0.311 {
if features.green_luminance <= 170 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.283 {
if features.red_chromaticity <= 0.283 {
if features.luminance <= 148 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.saturation <= 76 {
if features.value <= 167 {
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
} else {
if features.green_luminance <= 170 {
if features.blue_chromaticity <= 0.314 {
if features.blue_chromaticity <= 0.314 {
if features.luminance <= 147 {
Intensity::Low
} else {
if features.green_luminance <= 166 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_luminance <= 165 {
Intensity::Low
} else {
if features.luminance <= 144 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_difference <= 117 {
Intensity::High
} else {
if features.green_luminance <= 167 {
if features.blue_luminance <= 127 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.312 {
if features.blue_chromaticity <= 0.312 {
Intensity::Low
} else {
if features.green_luminance <= 174 {
if features.saturation <= 83 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 175 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.value <= 177 {
if features.green_chromaticity <= 0.399 {
Intensity::Low
} else {
if features.luminance <= 153 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_difference <= 97 {
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
} else {
if features.red_chromaticity <= 0.301 {
if features.red_luminance <= 111 {
if features.green_luminance <= 117 {
if features.green_chromaticity <= 0.435 {
if features.blue_difference <= 117 {
if features.value <= 112 {
if features.green_chromaticity <= 0.419 {
if features.blue_chromaticity <= 0.290 {
if features.green_chromaticity <= 0.411 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.hue <= 60 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 70 {
if features.saturation <= 92 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 71 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.290 {
if features.green_chromaticity <= 0.418 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 116 {
if features.blue_chromaticity <= 0.294 {
Intensity::Low
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
}
} else {
if features.green_chromaticity <= 0.427 {
if features.blue_luminance <= 82 {
if features.green_chromaticity <= 0.419 {
if features.red_chromaticity <= 0.291 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.421 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.411 {
if features.intensity <= 95 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 116 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.428 {
if features.saturation <= 91 {
if features.red_difference <= 116 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.430 {
if features.green_chromaticity <= 0.429 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.295 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.blue_difference <= 117 {
if features.green_chromaticity <= 0.451 {
if features.green_chromaticity <= 0.441 {
if features.red_luminance <= 54 {
if features.red_chromaticity <= 0.297 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 99 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.295 {
if features.red_chromaticity <= 0.290 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.299 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.293 {
if features.red_chromaticity <= 0.289 {
if features.intensity <= 50 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 114 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.luminance <= 53 {
if features.saturation <= 122 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.295 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_chromaticity <= 0.292 {
if features.green_chromaticity <= 0.448 {
if features.green_chromaticity <= 0.443 {
if features.blue_luminance <= 47 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 103 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.455 {
if features.blue_luminance <= 36 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 114 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.437 {
if features.green_chromaticity <= 0.436 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 73 {
if features.red_chromaticity <= 0.299 {
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
if features.blue_luminance <= 106 {
if features.red_difference <= 113 {
if features.blue_chromaticity <= 0.305 {
if features.value <= 128 {
if features.blue_chromaticity <= 0.298 {
if features.value <= 125 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 92 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 95 {
if features.red_chromaticity <= 0.296 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.value <= 133 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.green_luminance <= 137 {
if features.hue <= 64 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.405 {
if features.green_chromaticity <= 0.401 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 73 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.luminance <= 109 {
if features.green_chromaticity <= 0.407 {
if features.red_chromaticity <= 0.301 {
if features.saturation <= 66 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.295 {
if features.intensity <= 95 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.hue <= 61 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.399 {
if features.red_chromaticity <= 0.298 {
Intensity::Low
} else {
if features.saturation <= 61 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_difference <= 114 {
if features.blue_luminance <= 103 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 124 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.red_luminance <= 108 {
if features.blue_chromaticity <= 0.306 {
if features.red_luminance <= 104 {
if features.red_luminance <= 102 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.292 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.302 {
if features.red_difference <= 112 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.293 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.value <= 140 {
if features.hue <= 62 {
if features.value <= 139 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.307 {
if features.value <= 141 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.291 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_chromaticity <= 0.402 {
if features.luminance <= 131 {
if features.red_chromaticity <= 0.297 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.300 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 109 {
if features.red_chromaticity <= 0.291 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.399 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_luminance <= 154 {
if features.red_chromaticity <= 0.292 {
if features.saturation <= 70 {
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
}
}
}
}
} else {
if features.blue_chromaticity <= 0.308 {
if features.green_chromaticity <= 0.395 {
if features.green_chromaticity <= 0.394 {
if features.blue_chromaticity <= 0.308 {
if features.blue_luminance <= 115 {
Intensity::Low
} else {
if features.luminance <= 137 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.301 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.intensity <= 139 {
if features.red_luminance <= 119 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.395 {
if features.red_luminance <= 120 {
if features.blue_chromaticity <= 0.305 {
Intensity::Low
} else {
if features.green_luminance <= 152 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.307 {
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
if features.red_luminance <= 117 {
if features.blue_chromaticity <= 0.306 {
if features.luminance <= 136 {
if features.green_chromaticity <= 0.397 {
if features.blue_luminance <= 115 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 125 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.401 {
if features.red_chromaticity <= 0.300 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 113 {
if features.blue_luminance <= 119 {
if features.green_luminance <= 155 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.296 {
if features.intensity <= 128 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.297 {
if features.green_chromaticity <= 0.397 {
if features.blue_chromaticity <= 0.308 {
if features.red_chromaticity <= 0.296 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 64 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.307 {
if features.red_chromaticity <= 0.295 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 63 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_luminance <= 135 {
if features.red_luminance <= 122 {
if features.red_luminance <= 112 {
if features.value <= 152 {
Intensity::Low
} else {
if features.blue_luminance <= 120 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.309 {
if features.blue_chromaticity <= 0.309 {
if features.intensity <= 128 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.311 {
if features.blue_chromaticity <= 0.310 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 117 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_difference <= 107 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.290 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.293 {
if features.red_chromaticity <= 0.291 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.393 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.blue_chromaticity <= 0.310 {
if features.red_chromaticity <= 0.301 {
if features.intensity <= 147 {
if features.value <= 172 {
Intensity::Low
} else {
if features.hue <= 63 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 138 {
if features.red_luminance <= 133 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.392 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.390 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 150 {
if features.red_difference <= 108 {
if features.blue_chromaticity <= 0.312 {
if features.green_chromaticity <= 0.395 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.314 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 59 {
if features.green_chromaticity <= 0.387 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.297 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_luminance <= 190 {
if features.blue_luminance <= 154 {
Intensity::Low
} else {
if features.hue <= 65 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.luminance <= 172 {
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
} else {
if features.green_chromaticity <= 0.384 {
if features.green_chromaticity <= 0.378 {
if features.green_chromaticity <= 0.372 {
if features.green_chromaticity <= 0.371 {
if features.blue_chromaticity <= 0.317 {
if features.red_chromaticity <= 0.337 {
if features.red_chromaticity <= 0.337 {
if features.saturation <= 44 {
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
if features.blue_chromaticity <= 0.317 {
if features.red_luminance <= 223 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 135 {
if features.green_luminance <= 136 {
if features.green_chromaticity <= 0.371 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.372 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 50 {
if features.green_chromaticity <= 0.372 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.371 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.321 {
if features.red_chromaticity <= 0.319 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.322 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.luminance <= 165 {
if features.blue_luminance <= 98 {
if features.red_chromaticity <= 0.327 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.334 {
if features.red_chromaticity <= 0.334 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.307 {
if features.red_chromaticity <= 0.321 {
if features.red_chromaticity <= 0.320 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 109 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.value <= 176 {
if features.red_chromaticity <= 0.313 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.311 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_luminance <= 182 {
if features.red_luminance <= 152 {
if features.red_luminance <= 150 {
if features.green_chromaticity <= 0.376 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 45 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.377 {
if features.green_luminance <= 183 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.377 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_luminance <= 184 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_chromaticity <= 0.309 {
if features.red_difference <= 120 {
if features.saturation <= 50 {
if features.green_chromaticity <= 0.379 {
if features.red_luminance <= 136 {
if features.red_chromaticity <= 0.316 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.313 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 127 {
if features.blue_chromaticity <= 0.306 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.308 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.luminance <= 126 {
if features.saturation <= 59 {
if features.blue_luminance <= 94 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.saturation <= 54 {
if features.saturation <= 51 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.316 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_chromaticity <= 0.329 {
if features.saturation <= 63 {
if features.green_chromaticity <= 0.382 {
if features.red_chromaticity <= 0.327 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 108 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.290 {
if features.red_chromaticity <= 0.360 {
if features.blue_chromaticity <= 0.286 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.360 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.value <= 106 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.291 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.red_luminance <= 146 {
if features.red_luminance <= 134 {
if features.red_chromaticity <= 0.306 {
if features.green_chromaticity <= 0.384 {
Intensity::Low
} else {
if features.blue_luminance <= 130 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.383 {
if features.red_luminance <= 133 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 154 {
if features.red_difference <= 113 {
if features.blue_chromaticity <= 0.309 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.311 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.304 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.309 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_luminance <= 150 {
if features.green_chromaticity <= 0.381 {
if features.blue_chromaticity <= 0.310 {
if features.luminance <= 168 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_difference <= 112 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.383 {
if features.green_chromaticity <= 0.381 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 169 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.378 {
Intensity::Low
} else {
if features.value <= 191 {
if features.blue_chromaticity <= 0.311 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.378 {
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
} else {
if features.red_chromaticity <= 0.316 {
if features.blue_chromaticity <= 0.306 {
if features.value <= 120 {
if features.green_chromaticity <= 0.432 {
if features.green_chromaticity <= 0.422 {
if features.red_difference <= 117 {
if features.value <= 114 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.418 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.264 {
if features.value <= 69 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 73 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_difference <= 120 {
if features.saturation <= 115 {
if features.green_chromaticity <= 0.436 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 131 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_difference <= 117 {
if features.red_chromaticity <= 0.308 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.306 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_chromaticity <= 0.394 {
if features.saturation <= 57 {
if features.luminance <= 137 {
if features.green_chromaticity <= 0.388 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.385 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_luminance <= 124 {
if features.blue_chromaticity <= 0.296 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 112 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.hue <= 58 {
if features.hue <= 57 {
if features.blue_chromaticity <= 0.294 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 136 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.luminance <= 130 {
if features.red_chromaticity <= 0.301 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.302 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.blue_chromaticity <= 0.309 {
if features.intensity <= 125 {
if features.green_chromaticity <= 0.392 {
if features.green_luminance <= 145 {
if features.green_chromaticity <= 0.392 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 124 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.307 {
if features.green_chromaticity <= 0.387 {
if features.red_luminance <= 119 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 162 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.308 {
if features.saturation <= 51 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.308 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_luminance <= 141 {
if features.green_chromaticity <= 0.385 {
if features.red_difference <= 113 {
if features.green_chromaticity <= 0.385 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.value <= 157 {
if features.green_chromaticity <= 0.386 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.310 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.311 {
if features.green_chromaticity <= 0.385 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.304 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.301 {
if features.red_difference <= 110 {
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
if features.luminance <= 113 {
if features.red_chromaticity <= 0.327 {
if features.blue_chromaticity <= 0.262 {
if features.blue_difference <= 117 {
if features.saturation <= 104 {
if features.green_chromaticity <= 0.423 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.438 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.316 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.429 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.value <= 120 {
if features.red_chromaticity <= 0.326 {
if features.blue_difference <= 117 {
Intensity::Low
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
if features.red_chromaticity <= 0.316 {
if features.luminance <= 111 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.387 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_chromaticity <= 0.336 {
if features.saturation <= 99 {
if features.blue_luminance <= 57 {
if features.green_chromaticity <= 0.409 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.328 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.331 {
if features.blue_luminance <= 30 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 54 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.362 {
if features.red_chromaticity <= 0.362 {
if features.saturation <= 89 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.value <= 56 {
if features.luminance <= 21 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.401 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.blue_chromaticity <= 0.296 {
if features.blue_chromaticity <= 0.295 {
if features.hue <= 52 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.388 {
if features.red_luminance <= 103 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.value <= 132 {
if features.green_chromaticity <= 0.385 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.386 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.386 {
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
if features.green_chromaticity <= 0.438 {
if features.red_difference <= 113 {
if features.blue_chromaticity <= 0.313 {
if features.red_difference <= 109 {
if features.blue_chromaticity <= 0.310 {
if features.red_difference <= 107 {
if features.blue_chromaticity <= 0.310 {
if features.green_chromaticity <= 0.437 {
if features.red_chromaticity <= 0.266 {
if features.red_chromaticity <= 0.264 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 123 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.259 {
if features.blue_chromaticity <= 0.310 {
if features.green_chromaticity <= 0.436 {
if features.hue <= 68 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 105 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.265 {
if features.red_chromaticity <= 0.260 {
Intensity::High
} else {
if features.red_chromaticity <= 0.262 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.intensity <= 100 {
if features.green_chromaticity <= 0.422 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 95 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.red_difference <= 106 {
if features.green_chromaticity <= 0.428 {
if features.green_chromaticity <= 0.421 {
if features.blue_luminance <= 101 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.luminance <= 110 {
if features.luminance <= 109 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.310 {
if features.red_chromaticity <= 0.257 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.311 {
if features.saturation <= 101 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.intensity <= 96 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_luminance <= 114 {
if features.red_luminance <= 62 {
Intensity::Low
} else {
if features.value <= 113 {
if features.red_chromaticity <= 0.259 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 98 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_luminance <= 92 {
if features.red_chromaticity <= 0.278 {
if features.green_chromaticity <= 0.421 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.intensity <= 116 {
if features.green_chromaticity <= 0.410 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 126 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
}
} else {
if features.red_difference <= 111 {
if features.green_luminance <= 116 {
if features.hue <= 67 {
if features.value <= 111 {
if features.saturation <= 101 {
if features.intensity <= 83 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.422 {
if features.red_luminance <= 73 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 114 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_difference <= 110 {
if features.blue_chromaticity <= 0.306 {
Intensity::High
} else {
if features.green_chromaticity <= 0.434 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.310 {
if features.blue_chromaticity <= 0.310 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 96 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.value <= 139 {
if features.green_luminance <= 124 {
if features.blue_chromaticity <= 0.312 {
if features.green_chromaticity <= 0.420 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.409 {
if features.blue_chromaticity <= 0.311 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 101 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.green_luminance <= 145 {
if features.red_luminance <= 103 {
if features.blue_chromaticity <= 0.312 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 69 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_luminance <= 146 {
if features.red_chromaticity <= 0.289 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.289 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.value <= 115 {
if features.blue_chromaticity <= 0.302 {
if features.blue_luminance <= 61 {
if features.green_chromaticity <= 0.436 {
if features.luminance <= 74 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.267 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 65 {
if features.green_luminance <= 98 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.424 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.value <= 108 {
if features.red_chromaticity <= 0.272 {
if features.blue_difference <= 121 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.277 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.280 {
if features.hue <= 66 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.302 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.value <= 144 {
if features.value <= 127 {
if features.green_chromaticity <= 0.409 {
if features.red_difference <= 112 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 103 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 101 {
if features.value <= 129 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 63 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.value <= 148 {
if features.red_chromaticity <= 0.295 {
if features.blue_luminance <= 114 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.298 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 128 {
if features.value <= 149 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.299 {
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
} else {
if features.green_chromaticity <= 0.393 {
if features.green_chromaticity <= 0.385 {
if features.green_chromaticity <= 0.379 {
if features.green_luminance <= 124 {
if features.green_chromaticity <= 0.376 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.376 {
if features.hue <= 86 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::Low
}
}
} else {
if features.blue_difference <= 128 {
if features.blue_chromaticity <= 0.319 {
if features.red_chromaticity <= 0.305 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 126 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 110 {
if features.saturation <= 79 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.hue <= 90 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_chromaticity <= 0.379 {
Intensity::Low
} else {
if features.red_difference <= 112 {
if features.red_difference <= 103 {
if features.saturation <= 79 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.323 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 117 {
if features.blue_chromaticity <= 0.332 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 135 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.blue_chromaticity <= 0.321 {
if features.luminance <= 134 {
if features.blue_chromaticity <= 0.314 {
Intensity::Low
} else {
if features.red_difference <= 111 {
if features.blue_luminance <= 121 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.318 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_difference <= 109 {
if features.blue_chromaticity <= 0.320 {
if features.blue_chromaticity <= 0.317 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 171 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.luminance <= 141 {
if features.green_chromaticity <= 0.388 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.392 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_luminance <= 86 {
if features.red_difference <= 105 {
if features.value <= 110 {
if features.green_luminance <= 109 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_difference <= 111 {
if features.hue <= 84 {
if features.red_difference <= 104 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 91 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.blue_luminance <= 100 {
if features.green_chromaticity <= 0.392 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 125 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
}
} else {
if features.blue_difference <= 121 {
if features.blue_luminance <= 114 {
if features.green_luminance <= 118 {
if features.red_chromaticity <= 0.245 {
if features.blue_luminance <= 82 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.435 {
if features.intensity <= 96 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 95 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.404 {
if features.green_luminance <= 131 {
if features.green_chromaticity <= 0.398 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 95 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.436 {
if features.blue_chromaticity <= 0.316 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 121 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.blue_luminance <= 125 {
if features.blue_chromaticity <= 0.316 {
if features.saturation <= 85 {
if features.green_chromaticity <= 0.395 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 152 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_luminance <= 121 {
if features.green_luminance <= 153 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.397 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_luminance <= 130 {
if features.red_chromaticity <= 0.284 {
if features.blue_chromaticity <= 0.324 {
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
if features.luminance <= 150 {
if features.red_luminance <= 115 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.415 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.blue_difference <= 122 {
if features.red_chromaticity <= 0.251 {
if features.green_chromaticity <= 0.428 {
if features.blue_luminance <= 103 {
if features.red_difference <= 104 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 107 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.hue <= 70 {
if features.red_chromaticity <= 0.251 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.436 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.266 {
if features.luminance <= 95 {
if features.green_chromaticity <= 0.429 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 88 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 89 {
if features.green_chromaticity <= 0.419 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 136 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_chromaticity <= 0.427 {
if features.green_luminance <= 118 {
if features.green_chromaticity <= 0.417 {
if features.red_difference <= 105 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.247 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.399 {
if features.blue_chromaticity <= 0.373 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.399 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.352 {
if features.blue_chromaticity <= 0.351 {
if features.red_difference <= 112 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_difference <= 103 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 36 {
if features.green_chromaticity <= 0.436 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.380 {
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
} else {
if features.green_chromaticity <= 0.377 {
if features.green_chromaticity <= 0.373 {
if features.blue_difference <= 137 {
if features.green_chromaticity <= 0.370 {
if features.green_chromaticity <= 0.364 {
if features.blue_difference <= 133 {
if features.red_difference <= 115 {
if features.blue_chromaticity <= 0.348 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.360 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 195 {
if features.hue <= 93 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 221 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_difference <= 121 {
if features.intensity <= 152 {
if features.blue_luminance <= 118 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.320 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 32 {
if features.green_chromaticity <= 0.364 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 103 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_luminance <= 95 {
if features.blue_chromaticity <= 0.281 {
if features.red_chromaticity <= 0.348 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.314 {
if features.red_luminance <= 121 {
if features.red_chromaticity <= 0.331 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.321 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 134 {
if features.red_luminance <= 118 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 157 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.red_luminance <= 218 {
if features.saturation <= 31 {
if features.red_chromaticity <= 0.313 {
if features.green_chromaticity <= 0.332 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.329 {
if features.blue_difference <= 139 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.314 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.luminance <= 159 {
if features.green_luminance <= 153 {
if features.blue_luminance <= 158 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 168 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.313 {
if features.green_luminance <= 175 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.313 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_chromaticity <= 0.326 {
Intensity::High
} else {
if features.luminance <= 231 {
if features.intensity <= 233 {
if features.green_chromaticity <= 0.328 {
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
}
}
}
} else {
if features.red_luminance <= 93 {
if features.intensity <= 103 {
if features.saturation <= 47 {
if features.red_chromaticity <= 0.320 {
if features.blue_chromaticity <= 0.321 {
if features.blue_difference <= 121 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.306 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_difference <= 122 {
Intensity::Low
} else {
if features.blue_luminance <= 63 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_luminance <= 66 {
Intensity::Low
} else {
if features.blue_luminance <= 68 {
if features.saturation <= 51 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
}
} else {
if features.red_difference <= 116 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.314 {
if features.green_chromaticity <= 0.376 {
if features.red_luminance <= 115 {
if features.green_luminance <= 109 {
if features.saturation <= 52 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 41 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 131 {
if features.green_luminance <= 154 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 143 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_luminance <= 116 {
if features.intensity <= 95 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.376 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.316 {
if features.green_chromaticity <= 0.377 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 133 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_chromaticity <= 0.319 {
if features.green_luminance <= 170 {
if features.blue_chromaticity <= 0.317 {
if features.red_chromaticity <= 0.312 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.309 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.374 {
if features.red_chromaticity <= 0.312 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 159 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.301 {
if features.green_chromaticity <= 0.373 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.324 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 127 {
if features.blue_luminance <= 133 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 45 {
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
} else {
if features.blue_difference <= 120 {
if features.red_chromaticity <= 0.303 {
if features.green_chromaticity <= 0.428 {
if features.green_luminance <= 115 {
if features.green_chromaticity <= 0.415 {
if features.blue_difference <= 119 {
if features.red_chromaticity <= 0.298 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 83 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_difference <= 119 {
if features.red_chromaticity <= 0.294 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 68 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.388 {
if features.green_chromaticity <= 0.384 {
if features.green_chromaticity <= 0.380 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 119 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.value <= 125 {
if features.green_chromaticity <= 0.403 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.393 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_difference <= 118 {
if features.green_chromaticity <= 0.433 {
if features.green_luminance <= 72 {
if features.blue_luminance <= 46 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 73 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_difference <= 119 {
if features.saturation <= 96 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 57 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_luminance <= 39 {
if features.blue_chromaticity <= 0.265 {
Intensity::Low
} else {
if features.red_luminance <= 39 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.269 {
if features.blue_luminance <= 40 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 45 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.value <= 116 {
if features.red_chromaticity <= 0.321 {
if features.blue_chromaticity <= 0.285 {
if features.blue_difference <= 119 {
if features.red_chromaticity <= 0.307 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 54 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_difference <= 118 {
if features.green_chromaticity <= 0.399 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.293 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_luminance <= 66 {
if features.red_chromaticity <= 0.342 {
if features.red_chromaticity <= 0.342 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.415 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 53 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.393 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_chromaticity <= 0.382 {
if features.red_luminance <= 129 {
if features.red_difference <= 118 {
if features.blue_chromaticity <= 0.309 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.321 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.315 {
if features.red_luminance <= 132 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.303 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.intensity <= 108 {
if features.green_chromaticity <= 0.384 {
if features.intensity <= 107 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 96 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.387 {
if features.blue_luminance <= 116 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.304 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
}
} else {
if features.blue_difference <= 122 {
if features.red_chromaticity <= 0.276 {
if features.blue_chromaticity <= 0.308 {
if features.red_luminance <= 41 {
if features.value <= 64 {
if features.red_chromaticity <= 0.274 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 55 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.429 {
if features.blue_chromaticity <= 0.305 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.307 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.310 {
if features.intensity <= 73 {
if features.green_chromaticity <= 0.416 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.313 {
if features.green_chromaticity <= 0.422 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 72 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_chromaticity <= 0.304 {
if features.blue_difference <= 121 {
if features.value <= 119 {
if features.red_chromaticity <= 0.291 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 136 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.281 {
if features.blue_chromaticity <= 0.300 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.317 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.intensity <= 106 {
if features.red_chromaticity <= 0.316 {
if features.blue_luminance <= 43 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_difference <= 121 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.hue <= 64 {
if features.red_difference <= 117 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 116 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.red_difference <= 119 {
if features.green_chromaticity <= 0.411 {
if features.blue_difference <= 124 {
if features.blue_luminance <= 54 {
if features.luminance <= 58 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 58 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.377 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.397 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_difference <= 125 {
if features.green_chromaticity <= 0.437 {
if features.green_chromaticity <= 0.421 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.437 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 38 {
if features.green_chromaticity <= 0.412 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.434 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_difference <= 122 {
if features.blue_chromaticity <= 0.317 {
if features.green_chromaticity <= 0.425 {
if features.red_chromaticity <= 0.262 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_difference <= 120 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_difference <= 120 {
if features.blue_chromaticity <= 0.337 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 33 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_luminance <= 58 {
if features.luminance <= 41 {
if features.green_luminance <= 28 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.386 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.378 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.316 {
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
if features.value <= 57 {
if features.green_luminance <= 50 {
if features.green_luminance <= 44 {
if features.green_luminance <= 40 {
if features.green_luminance <= 30 {
if features.green_chromaticity <= 0.472 {
if features.blue_chromaticity <= 0.013 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 18 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.470 {
if features.blue_chromaticity <= 0.405 {
if features.green_luminance <= 35 {
if features.green_chromaticity <= 0.439 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.354 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 28 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.406 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.saturation <= 127 {
if features.blue_chromaticity <= 0.268 {
if features.green_chromaticity <= 0.472 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.240 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.352 {
if features.green_luminance <= 33 {
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
} else {
if features.green_chromaticity <= 0.472 {
if features.green_chromaticity <= 0.457 {
if features.value <= 43 {
if features.saturation <= 132 {
if features.green_chromaticity <= 0.446 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 34 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.284 {
if features.blue_luminance <= 23 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_difference <= 123 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.461 {
if features.saturation <= 133 {
if features.red_chromaticity <= 0.231 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 28 {
if features.red_difference <= 121 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.326 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.value <= 43 {
if features.luminance <= 35 {
if features.hue <= 73 {
if features.saturation <= 119 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_difference <= 126 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.247 {
if features.red_chromaticity <= 0.210 {
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
} else {
if features.green_chromaticity <= 0.456 {
if features.green_chromaticity <= 0.448 {
if features.green_chromaticity <= 0.441 {
if features.luminance <= 42 {
if features.red_chromaticity <= 0.292 {
Intensity::Low
} else {
if features.red_difference <= 122 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_difference <= 121 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.luminance <= 40 {
if features.hue <= 60 {
if features.red_luminance <= 33 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.321 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 108 {
if features.saturation <= 98 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.315 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.luminance <= 38 {
if features.green_luminance <= 45 {
if features.red_difference <= 118 {
if features.red_luminance <= 21 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.281 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 33 {
if features.hue <= 58 {
if features.green_luminance <= 48 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 110 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 140 {
if features.value <= 48 {
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
} else {
if features.luminance <= 39 {
if features.saturation <= 117 {
if features.red_chromaticity <= 0.270 {
if features.red_luminance <= 26 {
if features.red_luminance <= 25 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.272 {
if features.saturation <= 108 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 110 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_luminance <= 48 {
if features.red_chromaticity <= 0.181 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.193 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.467 {
if features.green_chromaticity <= 0.465 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
}
} else {
if features.saturation <= 121 {
if features.intensity <= 35 {
if features.green_chromaticity <= 0.469 {
if features.red_chromaticity <= 0.261 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.255 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 29 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 133 {
if features.red_chromaticity <= 0.235 {
Intensity::Low
} else {
if features.red_luminance <= 27 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_difference <= 125 {
if features.blue_chromaticity <= 0.318 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_difference <= 114 {
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
} else {
if features.green_chromaticity <= 0.458 {
if features.blue_chromaticity <= 0.296 {
if features.red_difference <= 119 {
if features.red_chromaticity <= 0.275 {
if features.red_difference <= 117 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.270 {
if features.green_luminance <= 53 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 41 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.value <= 56 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.284 {
if features.red_chromaticity <= 0.280 {
if features.saturation <= 94 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 99 {
if features.red_chromaticity <= 0.283 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.300 {
if features.value <= 55 {
if features.luminance <= 43 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 37 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 36 {
if features.green_luminance <= 52 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.439 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.green_chromaticity <= 0.457 {
if features.green_chromaticity <= 0.442 {
if features.luminance <= 44 {
Intensity::Low
} else {
if features.blue_luminance <= 38 {
if features.red_chromaticity <= 0.262 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 54 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.328 {
if features.blue_chromaticity <= 0.328 {
if features.green_chromaticity <= 0.443 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.447 {
Intensity::Low
} else {
if features.luminance <= 42 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_difference <= 114 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.305 {
Intensity::Low
} else {
if features.blue_difference <= 124 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.green_chromaticity <= 0.470 {
if features.luminance <= 43 {
if features.green_chromaticity <= 0.467 {
if features.saturation <= 117 {
if features.red_luminance <= 28 {
Intensity::Low
} else {
if features.value <= 51 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 23 {
if features.saturation <= 150 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 33 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.252 {
if features.red_difference <= 116 {
if features.blue_luminance <= 35 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.288 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_difference <= 119 {
if features.green_chromaticity <= 0.469 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
}
} else {
if features.saturation <= 117 {
if features.green_chromaticity <= 0.465 {
if features.green_chromaticity <= 0.462 {
if features.saturation <= 108 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.262 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.278 {
if features.hue <= 61 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.blue_difference <= 125 {
if features.saturation <= 128 {
if features.red_chromaticity <= 0.251 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 40 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.luminance <= 44 {
if features.red_chromaticity <= 0.188 {
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
} else {
if features.red_luminance <= 27 {
if features.red_chromaticity <= 0.227 {
if features.green_chromaticity <= 0.473 {
if features.green_chromaticity <= 0.470 {
if features.blue_chromaticity <= 0.359 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 37 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.236 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.473 {
if features.red_difference <= 116 {
Intensity::Low
} else {
if features.intensity <= 36 {
if features.saturation <= 116 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 117 {
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
if features.blue_difference <= 120 {
if features.red_chromaticity <= 0.268 {
if features.green_chromaticity <= 0.453 {
if features.value <= 114 {
if features.blue_chromaticity <= 0.286 {
if features.red_luminance <= 42 {
if features.green_chromaticity <= 0.449 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.hue <= 63 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.442 {
if features.blue_chromaticity <= 0.292 {
if features.value <= 82 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.262 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.309 {
if features.green_luminance <= 71 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.313 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_difference <= 119 {
if features.red_chromaticity <= 0.250 {
if features.green_chromaticity <= 0.442 {
Intensity::High
} else {
if features.green_chromaticity <= 0.442 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.253 {
if features.red_luminance <= 70 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.240 {
if features.green_chromaticity <= 0.449 {
if features.red_difference <= 100 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.245 {
Intensity::High
} else {
if features.luminance <= 100 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.blue_difference <= 119 {
if features.blue_chromaticity <= 0.315 {
if features.intensity <= 75 {
if features.saturation <= 117 {
if features.red_luminance <= 41 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_difference <= 104 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_difference <= 98 {
Intensity::High
} else {
if features.luminance <= 101 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.220 {
if features.green_chromaticity <= 0.473 {
if features.intensity <= 77 {
if features.green_chromaticity <= 0.468 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.320 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.saturation <= 119 {
if features.blue_chromaticity <= 0.272 {
if features.green_luminance <= 59 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.252 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.288 {
if features.green_chromaticity <= 0.472 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.231 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
}
} else {
if features.red_difference <= 118 {
if features.blue_difference <= 119 {
if features.green_chromaticity <= 0.441 {
if features.saturation <= 93 {
Intensity::Low
} else {
if features.saturation <= 95 {
if features.red_difference <= 116 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.439 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.441 {
if features.red_chromaticity <= 0.275 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 43 {
if features.red_chromaticity <= 0.279 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 97 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_chromaticity <= 0.274 {
if features.red_chromaticity <= 0.270 {
if features.red_difference <= 115 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.449 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_difference <= 117 {
if features.intensity <= 52 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 59 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.438 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.446 {
if features.red_chromaticity <= 0.277 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 51 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.green_chromaticity <= 0.450 {
if features.value <= 65 {
if features.red_chromaticity <= 0.280 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.276 {
if features.saturation <= 104 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.442 {
if features.intensity <= 50 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 106 {
Intensity::Low
} else {
if features.intensity <= 44 {
if features.saturation <= 110 {
Intensity::Low
} else {
if features.luminance <= 49 {
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
if features.blue_difference <= 122 {
if features.green_chromaticity <= 0.459 {
if features.red_chromaticity <= 0.256 {
if features.blue_chromaticity <= 0.323 {
if features.saturation <= 117 {
if features.red_chromaticity <= 0.239 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.304 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.449 {
if features.green_chromaticity <= 0.441 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.449 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.237 {
if features.green_chromaticity <= 0.446 {
if features.green_chromaticity <= 0.445 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.330 {
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
if features.green_chromaticity <= 0.448 {
if features.luminance <= 54 {
if features.hue <= 64 {
if features.red_difference <= 117 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.444 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.297 {
if features.red_luminance <= 41 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.260 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.282 {
Intensity::Low
} else {
if features.intensity <= 43 {
if features.red_chromaticity <= 0.259 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.hue <= 65 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.blue_difference <= 121 {
if features.green_chromaticity <= 0.465 {
if features.green_chromaticity <= 0.462 {
if features.red_luminance <= 44 {
if features.red_luminance <= 41 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.hue <= 73 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.saturation <= 128 {
if features.value <= 76 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 135 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_difference <= 101 {
if features.blue_luminance <= 86 {
if features.green_chromaticity <= 0.466 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.value <= 81 {
if features.green_chromaticity <= 0.469 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.219 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
} else {
if features.red_chromaticity <= 0.196 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.323 {
if features.green_chromaticity <= 0.460 {
if features.green_chromaticity <= 0.460 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_luminance <= 29 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.199 {
if features.saturation <= 148 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.467 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
}
} else {
if features.blue_chromaticity <= 0.335 {
if features.green_chromaticity <= 0.455 {
if features.blue_chromaticity <= 0.335 {
if features.green_luminance <= 76 {
if features.red_luminance <= 32 {
if features.blue_luminance <= 48 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 50 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.327 {
if features.intensity <= 72 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_luminance <= 39 {
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
if features.green_chromaticity <= 0.464 {
if features.green_chromaticity <= 0.455 {
Intensity::High
} else {
if features.red_chromaticity <= 0.216 {
if features.green_chromaticity <= 0.456 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 54 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.value <= 101 {
if features.green_chromaticity <= 0.470 {
if features.red_chromaticity <= 0.198 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.value <= 96 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.335 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.194 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
}
} else {
if features.blue_luminance <= 85 {
if features.green_luminance <= 116 {
if features.value <= 75 {
if features.red_difference <= 112 {
if features.red_difference <= 111 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.337 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.352 {
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
if features.red_difference <= 101 {
if features.red_luminance <= 49 {
if features.red_chromaticity <= 0.182 {
if features.intensity <= 75 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 155 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.saturation <= 127 {
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
}
} else {
if features.red_difference <= 118 {
if features.blue_difference <= 120 {
if features.green_chromaticity <= 0.510 {
if features.blue_difference <= 118 {
if features.blue_difference <= 116 {
if features.green_chromaticity <= 0.485 {
if features.blue_difference <= 115 {
if features.red_luminance <= 50 {
if features.red_luminance <= 43 {
if features.intensity <= 44 {
Intensity::High
} else {
if features.saturation <= 132 {
if features.hue <= 52 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_difference <= 117 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_luminance <= 79 {
if features.value <= 78 {
if features.red_chromaticity <= 0.273 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.saturation <= 121 {
if features.blue_chromaticity <= 0.258 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.480 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.green_chromaticity <= 0.474 {
Intensity::High
} else {
if features.red_chromaticity <= 0.251 {
if features.blue_chromaticity <= 0.284 {
if features.green_chromaticity <= 0.475 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.285 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.475 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.saturation <= 117 {
if features.green_chromaticity <= 0.478 {
if features.saturation <= 115 {
if features.red_difference <= 112 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.477 {
if features.blue_luminance <= 44 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.286 {
if features.intensity <= 72 {
if features.red_chromaticity <= 0.248 {
if features.red_chromaticity <= 0.245 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.485 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.231 {
if features.blue_chromaticity <= 0.301 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.232 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.red_luminance <= 37 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.236 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.green_chromaticity <= 0.497 {
if features.red_luminance <= 41 {
if features.green_luminance <= 59 {
if features.blue_chromaticity <= 0.224 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.489 {
if features.green_chromaticity <= 0.487 {
if features.saturation <= 127 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 124 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 38 {
if features.red_chromaticity <= 0.289 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.494 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.blue_chromaticity <= 0.292 {
if features.red_chromaticity <= 0.244 {
if features.red_chromaticity <= 0.236 {
if features.blue_luminance <= 53 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.492 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.248 {
if features.blue_chromaticity <= 0.268 {
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
} else {
if features.green_luminance <= 122 {
if features.red_chromaticity <= 0.208 {
if features.red_chromaticity <= 0.207 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.215 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 83 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.red_luminance <= 31 {
if features.blue_chromaticity <= 0.215 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.218 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.503 {
if features.intensity <= 54 {
if features.red_luminance <= 37 {
if features.blue_chromaticity <= 0.215 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.503 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 46 {
if features.red_luminance <= 42 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.502 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.504 {
if features.intensity <= 63 {
if features.blue_difference <= 115 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.203 {
Intensity::High
} else {
if features.red_chromaticity <= 0.204 {
Intensity::Low
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
if features.green_chromaticity <= 0.481 {
if features.luminance <= 49 {
if features.value <= 59 {
Intensity::Low
} else {
if features.intensity <= 41 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 58 {
if features.green_chromaticity <= 0.479 {
if features.green_chromaticity <= 0.474 {
if features.blue_luminance <= 51 {
Intensity::High
} else {
if features.red_difference <= 108 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.289 {
if features.saturation <= 127 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.intensity <= 63 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.saturation <= 129 {
if features.green_chromaticity <= 0.480 {
if features.red_luminance <= 37 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.value <= 87 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.intensity <= 66 {
if features.green_chromaticity <= 0.480 {
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
if features.red_chromaticity <= 0.212 {
if features.blue_chromaticity <= 0.311 {
if features.blue_chromaticity <= 0.308 {
Intensity::High
} else {
if features.green_chromaticity <= 0.480 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.480 {
if features.green_chromaticity <= 0.475 {
if features.green_chromaticity <= 0.475 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.288 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_difference <= 102 {
if features.blue_chromaticity <= 0.298 {
Intensity::High
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
if features.red_difference <= 117 {
if features.green_chromaticity <= 0.489 {
if features.luminance <= 88 {
if features.red_luminance <= 34 {
if features.saturation <= 121 {
Intensity::High
} else {
if features.saturation <= 122 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_luminance <= 94 {
if features.blue_luminance <= 50 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.297 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.value <= 119 {
if features.green_chromaticity <= 0.485 {
if features.red_luminance <= 52 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_luminance <= 50 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 55 {
Intensity::High
} else {
if features.intensity <= 91 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
} else {
if features.green_luminance <= 57 {
if features.saturation <= 133 {
if features.saturation <= 131 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_difference <= 116 {
Intensity::High
} else {
if features.value <= 54 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_difference <= 98 {
if features.red_chromaticity <= 0.193 {
if features.green_chromaticity <= 0.501 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 75 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.green_luminance <= 95 {
if features.blue_chromaticity <= 0.275 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_difference <= 117 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.red_chromaticity <= 0.273 {
if features.red_chromaticity <= 0.271 {
if features.red_chromaticity <= 0.266 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.502 {
if features.value <= 54 {
Intensity::Low
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
} else {
if features.red_chromaticity <= 0.279 {
if features.green_chromaticity <= 0.498 {
if features.intensity <= 39 {
if features.hue <= 55 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.intensity <= 36 {
if features.green_chromaticity <= 0.507 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.233 {
if features.blue_chromaticity <= 0.230 {
Intensity::High
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
} else {
if features.red_difference <= 115 {
if features.blue_chromaticity <= 0.288 {
if features.red_difference <= 114 {
if features.green_chromaticity <= 0.493 {
if features.red_chromaticity <= 0.225 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.237 {
if features.red_chromaticity <= 0.235 {
if features.red_chromaticity <= 0.233 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.luminance <= 55 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 35 {
if features.blue_chromaticity <= 0.274 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_difference <= 113 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.blue_luminance <= 31 {
if features.blue_chromaticity <= 0.267 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.hue <= 67 {
if features.blue_luminance <= 38 {
if features.green_chromaticity <= 0.504 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.luminance <= 56 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.saturation <= 151 {
if features.red_chromaticity <= 0.209 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.204 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.saturation <= 133 {
if features.red_chromaticity <= 0.238 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.276 {
if features.luminance <= 46 {
Intensity::High
} else {
if features.intensity <= 40 {
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
if features.saturation <= 135 {
if features.green_chromaticity <= 0.500 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.509 {
if features.value <= 55 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.261 {
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
if features.green_chromaticity <= 0.499 {
if features.blue_luminance <= 71 {
if features.saturation <= 135 {
if features.blue_chromaticity <= 0.296 {
if features.red_chromaticity <= 0.226 {
if features.blue_chromaticity <= 0.295 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.479 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.301 {
if features.intensity <= 63 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.476 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.313 {
if features.value <= 91 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_difference <= 98 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
} else {
if features.value <= 125 {
if features.blue_chromaticity <= 0.322 {
if features.blue_chromaticity <= 0.322 {
if features.luminance <= 91 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 88 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.487 {
if features.saturation <= 153 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.luminance <= 93 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.317 {
if features.intensity <= 88 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
}
} else {
if features.blue_chromaticity <= 0.315 {
if features.red_difference <= 103 {
if features.value <= 109 {
if features.value <= 104 {
if features.red_chromaticity <= 0.194 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.186 {
if features.intensity <= 72 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.505 {
if features.blue_chromaticity <= 0.295 {
if features.blue_chromaticity <= 0.295 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.194 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.510 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.326 {
Intensity::High
} else {
if features.blue_luminance <= 80 {
if features.green_luminance <= 122 {
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
} else {
if features.blue_difference <= 119 {
if features.red_chromaticity <= 0.264 {
if features.green_chromaticity <= 0.507 {
if features.blue_chromaticity <= 0.257 {
if features.red_difference <= 117 {
if features.green_luminance <= 55 {
if features.green_chromaticity <= 0.493 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.253 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.saturation <= 131 {
if features.green_chromaticity <= 0.493 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 134 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.482 {
if features.green_luminance <= 62 {
Intensity::Low
} else {
if features.green_luminance <= 63 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.490 {
if features.green_luminance <= 60 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
}
} else {
if features.red_difference <= 116 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.477 {
if features.luminance <= 47 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 53 {
Intensity::Low
} else {
if features.red_luminance <= 30 {
Intensity::Low
} else {
if features.red_difference <= 117 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.red_difference <= 117 {
if features.saturation <= 128 {
if features.blue_luminance <= 31 {
if features.blue_chromaticity <= 0.262 {
if features.green_chromaticity <= 0.493 {
if features.luminance <= 42 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.luminance <= 44 {
Intensity::Low
} else {
if features.red_difference <= 116 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.intensity <= 39 {
Intensity::Low
} else {
if features.luminance <= 47 {
if features.green_luminance <= 57 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
}
} else {
if features.blue_luminance <= 25 {
Intensity::High
} else {
if features.red_luminance <= 23 {
Intensity::Low
} else {
if features.red_luminance <= 24 {
Intensity::Low
} else {
if features.saturation <= 133 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.luminance <= 39 {
if features.red_chromaticity <= 0.251 {
if features.green_chromaticity <= 0.503 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.479 {
Intensity::Low
} else {
if features.green_luminance <= 50 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.484 {
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
} else {
if features.red_difference <= 116 {
if features.green_chromaticity <= 0.548 {
if features.blue_difference <= 118 {
if features.blue_difference <= 117 {
if features.green_chromaticity <= 0.527 {
if features.blue_chromaticity <= 0.241 {
if features.green_luminance <= 58 {
if features.green_chromaticity <= 0.516 {
Intensity::High
} else {
if features.green_chromaticity <= 0.521 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.242 {
if features.luminance <= 48 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.260 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.blue_chromaticity <= 0.259 {
if features.red_luminance <= 34 {
if features.red_chromaticity <= 0.227 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.519 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.526 {
if features.green_luminance <= 112 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_difference <= 100 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.blue_chromaticity <= 0.290 {
if features.green_chromaticity <= 0.530 {
if features.green_chromaticity <= 0.529 {
if features.intensity <= 36 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.284 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_luminance <= 25 {
if features.saturation <= 148 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.luminance <= 51 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_difference <= 97 {
if features.blue_chromaticity <= 0.304 {
if features.green_luminance <= 103 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_difference <= 91 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.176 {
if features.blue_luminance <= 52 {
Intensity::High
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
if features.blue_chromaticity <= 0.279 {
if features.hue <= 59 {
if features.green_chromaticity <= 0.533 {
if features.green_chromaticity <= 0.516 {
Intensity::High
} else {
if features.green_chromaticity <= 0.522 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.236 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.511 {
Intensity::High
} else {
if features.green_chromaticity <= 0.547 {
if features.green_chromaticity <= 0.517 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.luminance <= 47 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.red_difference <= 100 {
if features.blue_chromaticity <= 0.305 {
if features.blue_chromaticity <= 0.294 {
if features.saturation <= 175 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.luminance <= 68 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.value <= 108 {
if features.blue_chromaticity <= 0.306 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.513 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.203 {
if features.blue_chromaticity <= 0.287 {
if features.luminance <= 62 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.181 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.513 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.red_chromaticity <= 0.172 {
if features.blue_chromaticity <= 0.320 {
if features.red_chromaticity <= 0.169 {
if features.red_chromaticity <= 0.150 {
if features.red_chromaticity <= 0.150 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.169 {
Intensity::High
} else {
if features.green_chromaticity <= 0.541 {
Intensity::High
} else {
if features.green_chromaticity <= 0.544 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.red_difference <= 93 {
Intensity::High
} else {
if features.red_chromaticity <= 0.165 {
if features.red_chromaticity <= 0.151 {
Intensity::Low
} else {
if features.red_luminance <= 34 {
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
} else {
if features.blue_difference <= 119 {
if features.green_luminance <= 80 {
if features.green_chromaticity <= 0.547 {
if features.hue <= 61 {
if features.green_chromaticity <= 0.517 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 23 {
Intensity::High
} else {
Intensity::High
}
}
} else {
Intensity::Low
}
} else {
if features.saturation <= 157 {
Intensity::Low
} else {
if features.value <= 96 {
if features.blue_luminance <= 51 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 167 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
} else {
if features.green_chromaticity <= 0.529 {
if features.green_chromaticity <= 0.527 {
if features.green_chromaticity <= 0.524 {
if features.red_difference <= 102 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 22 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.196 {
if features.red_chromaticity <= 0.179 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_difference <= 114 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.179 {
Intensity::High
} else {
if features.green_chromaticity <= 0.532 {
if features.blue_luminance <= 34 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.533 {
Intensity::Low
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
if features.blue_difference <= 118 {
if features.green_chromaticity <= 0.590 {
if features.luminance <= 36 {
if features.green_chromaticity <= 0.575 {
if features.intensity <= 27 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.210 {
Intensity::High
} else {
if features.hue <= 56 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.207 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.blue_difference <= 117 {
if features.red_chromaticity <= 0.232 {
if features.red_luminance <= 25 {
if features.blue_chromaticity <= 0.204 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.luminance <= 66 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_difference <= 115 {
if features.value <= 53 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.553 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_luminance <= 57 {
if features.red_luminance <= 20 {
if features.red_chromaticity <= 0.197 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.239 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.hue <= 67 {
if features.red_chromaticity <= 0.157 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.555 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.red_difference <= 113 {
if features.saturation <= 186 {
if features.blue_chromaticity <= 0.174 {
Intensity::High
} else {
if features.red_chromaticity <= 0.175 {
if features.red_chromaticity <= 0.175 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.591 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.blue_difference <= 116 {
Intensity::High
} else {
if features.hue <= 61 {
if features.saturation <= 200 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.hue <= 65 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.saturation <= 183 {
if features.value <= 45 {
if features.red_chromaticity <= 0.213 {
if features.red_chromaticity <= 0.197 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.179 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.187 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_luminance <= 19 {
if features.blue_chromaticity <= 0.060 {
if features.green_luminance <= 39 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.hue <= 58 {
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
if features.red_chromaticity <= 0.178 {
if features.green_luminance <= 26 {
if features.green_chromaticity <= 0.981 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.590 {
if features.green_chromaticity <= 0.589 {
if features.red_chromaticity <= 0.142 {
Intensity::High
} else {
if features.red_chromaticity <= 0.143 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_difference <= 109 {
Intensity::High
} else {
if features.green_luminance <= 51 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.blue_chromaticity <= 0.120 {
if features.green_chromaticity <= 0.857 {
if features.luminance <= 19 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.luminance <= 18 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.300 {
if features.red_chromaticity <= 0.090 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.301 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
}
} else {
if features.green_luminance <= 50 {
if features.blue_chromaticity <= 0.236 {
if features.red_chromaticity <= 0.179 {
Intensity::High
} else {
if features.value <= 45 {
if features.red_luminance <= 14 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.234 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.intensity <= 29 {
if features.green_chromaticity <= 0.563 {
if features.red_chromaticity <= 0.205 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.567 {
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
if features.red_chromaticity <= 0.194 {
if features.saturation <= 170 {
if features.green_chromaticity <= 0.553 {
Intensity::High
} else {
if features.hue <= 64 {
Intensity::High
} else {
Intensity::High
}
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.552 {
Intensity::High
} else {
if features.green_chromaticity <= 0.559 {
if features.green_chromaticity <= 0.555 {
Intensity::High
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
if features.blue_difference <= 118 {
if features.blue_difference <= 116 {
if features.green_chromaticity <= 0.560 {
if features.blue_difference <= 115 {
if features.red_chromaticity <= 0.278 {
if features.red_chromaticity <= 0.272 {
if features.red_chromaticity <= 0.269 {
if features.green_chromaticity <= 0.550 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.547 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.274 {
if features.red_chromaticity <= 0.273 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 55 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.283 {
if features.red_chromaticity <= 0.281 {
if features.red_chromaticity <= 0.280 {
Intensity::High
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
if features.red_chromaticity <= 0.261 {
if features.green_luminance <= 50 {
Intensity::High
} else {
if features.luminance <= 39 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.value <= 55 {
if features.red_chromaticity <= 0.265 {
if features.value <= 47 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.268 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 29 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.red_difference <= 117 {
if features.green_chromaticity <= 0.626 {
if features.green_chromaticity <= 0.608 {
if features.blue_chromaticity <= 0.155 {
if features.blue_luminance <= 11 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.blue_difference <= 115 {
Intensity::High
} else {
if features.green_chromaticity <= 0.617 {
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
if features.blue_chromaticity <= 0.151 {
if features.saturation <= 206 {
Intensity::High
} else {
if features.red_luminance <= 16 {
if features.blue_difference <= 115 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.saturation <= 185 {
if features.saturation <= 181 {
if features.blue_difference <= 115 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.hue <= 52 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.588 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.red_difference <= 117 {
if features.green_chromaticity <= 0.603 {
if features.red_chromaticity <= 0.255 {
if features.blue_luminance <= 16 {
if features.green_chromaticity <= 0.577 {
if features.green_luminance <= 44 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 45 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.223 {
if features.intensity <= 27 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.219 {
if features.saturation <= 149 {
Intensity::High
} else {
if features.green_chromaticity <= 0.527 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.224 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.value <= 34 {
if features.saturation <= 235 {
if features.value <= 32 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.099 {
Intensity::High
} else {
Intensity::High
}
}
} else {
Intensity::High
}
} else {
if features.blue_difference <= 117 {
if features.red_chromaticity <= 0.216 {
if features.green_chromaticity <= 0.664 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.red_luminance <= 12 {
if features.value <= 35 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 182 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.blue_chromaticity <= 0.190 {
if features.red_chromaticity <= 0.204 {
if features.red_chromaticity <= 0.164 {
if features.red_luminance <= 5 {
if features.green_luminance <= 29 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 247 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.736 {
if features.green_chromaticity <= 0.721 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.211 {
Intensity::High
} else {
if features.green_chromaticity <= 0.557 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.182 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.green_chromaticity <= 0.547 {
if features.saturation <= 153 {
if features.luminance <= 38 {
Intensity::Low
} else {
if features.saturation <= 146 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.value <= 48 {
if features.luminance <= 36 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.529 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.blue_chromaticity <= 0.195 {
Intensity::High
} else {
if features.blue_luminance <= 16 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
}
} else {
if features.saturation <= 149 {
if features.green_luminance <= 45 {
if features.saturation <= 144 {
if features.red_luminance <= 19 {
Intensity::Low
} else {
if features.saturation <= 137 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.luminance <= 33 {
if features.saturation <= 147 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.237 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.229 {
if features.red_difference <= 117 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 47 {
if features.hue <= 60 {
if features.hue <= 59 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 46 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.238 {
Intensity::Low
} else {
if features.saturation <= 139 {
if features.green_chromaticity <= 0.511 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.intensity <= 30 {
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
if features.red_difference <= 117 {
if features.red_chromaticity <= 0.216 {
if features.red_chromaticity <= 0.194 {
if features.red_chromaticity <= 0.086 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.165 {
if features.red_chromaticity <= 0.136 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.value <= 33 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.211 {
if features.luminance <= 28 {
if features.green_chromaticity <= 0.616 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 169 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.saturation <= 163 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.blue_chromaticity <= 0.222 {
if features.saturation <= 154 {
Intensity::High
} else {
if features.luminance <= 32 {
if features.green_chromaticity <= 0.571 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.233 {
if features.intensity <= 27 {
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
if features.blue_difference <= 119 {
if features.green_chromaticity <= 0.557 {
if features.saturation <= 156 {
if features.green_chromaticity <= 0.540 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.577 {
if features.green_chromaticity <= 0.567 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.708 {
if features.blue_chromaticity <= 0.137 {
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
if features.green_chromaticity <= 0.909 {
if features.blue_luminance <= 3 {
Intensity::High
} else {
if features.red_luminance <= 15 {
if features.intensity <= 22 {
Intensity::Low
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
}
}
}
}
} else {
if features.green_chromaticity <= 0.536 {
if features.blue_difference <= 122 {
if features.red_difference <= 115 {
if features.green_chromaticity <= 0.505 {
if features.blue_difference <= 121 {
if features.red_luminance <= 36 {
if features.green_chromaticity <= 0.492 {
if features.green_chromaticity <= 0.486 {
if features.red_difference <= 110 {
if features.blue_chromaticity <= 0.310 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.saturation <= 131 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.green_luminance <= 77 {
if features.red_chromaticity <= 0.219 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.491 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.493 {
if features.green_chromaticity <= 0.492 {
if features.red_chromaticity <= 0.204 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.red_luminance <= 35 {
if features.blue_luminance <= 38 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
}
} else {
if features.green_luminance <= 82 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.333 {
if features.blue_chromaticity <= 0.321 {
if features.red_chromaticity <= 0.192 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.intensity <= 89 {
Intensity::Low
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
if features.red_luminance <= 24 {
if features.saturation <= 143 {
if features.blue_luminance <= 30 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 150 {
if features.green_chromaticity <= 0.498 {
Intensity::Low
} else {
if features.value <= 50 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.502 {
Intensity::High
} else {
if features.red_difference <= 111 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.saturation <= 136 {
if features.red_chromaticity <= 0.224 {
if features.red_difference <= 112 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.481 {
if features.green_luminance <= 60 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 166 {
if features.saturation <= 164 {
if features.red_difference <= 93 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_difference <= 100 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
Intensity::Low
}
}
}
}
} else {
if features.intensity <= 48 {
if features.green_luminance <= 74 {
if features.green_chromaticity <= 0.525 {
if features.green_chromaticity <= 0.507 {
Intensity::High
} else {
if features.red_chromaticity <= 0.173 {
if features.red_chromaticity <= 0.167 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.187 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.526 {
Intensity::High
} else {
if features.red_luminance <= 16 {
Intensity::High
} else {
if features.intensity <= 30 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
} else {
if features.blue_chromaticity <= 0.310 {
if features.red_difference <= 104 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.308 {
Intensity::Low
} else {
if features.red_luminance <= 25 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.165 {
if features.saturation <= 178 {
Intensity::High
} else {
if features.intensity <= 47 {
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
if features.blue_chromaticity <= 0.333 {
if features.green_chromaticity <= 0.513 {
if features.saturation <= 165 {
Intensity::High
} else {
if features.red_difference <= 101 {
if features.red_chromaticity <= 0.168 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.512 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.hue <= 72 {
if features.green_luminance <= 78 {
Intensity::High
} else {
if features.green_luminance <= 80 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.322 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.323 {
Intensity::High
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
if features.red_chromaticity <= 0.220 {
if features.blue_chromaticity <= 0.272 {
if features.blue_luminance <= 19 {
if features.saturation <= 152 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.531 {
if features.green_chromaticity <= 0.513 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.265 {
if features.red_chromaticity <= 0.210 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.268 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.luminance <= 31 {
Intensity::High
} else {
if features.green_chromaticity <= 0.534 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.luminance <= 35 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.508 {
Intensity::Low
} else {
if features.red_luminance <= 19 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_difference <= 117 {
if features.green_chromaticity <= 0.486 {
if features.red_luminance <= 26 {
if features.red_chromaticity <= 0.244 {
if features.value <= 53 {
if features.saturation <= 129 {
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
if features.green_chromaticity <= 0.481 {
if features.green_luminance <= 56 {
if features.green_chromaticity <= 0.478 {
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
}
} else {
if features.saturation <= 132 {
if features.red_difference <= 116 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.281 {
if features.blue_chromaticity <= 0.277 {
if features.luminance <= 40 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 40 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 138 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_chromaticity <= 0.509 {
if features.blue_chromaticity <= 0.273 {
if features.green_luminance <= 48 {
if features.blue_chromaticity <= 0.264 {
if features.luminance <= 36 {
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
} else {
if features.intensity <= 34 {
if features.green_chromaticity <= 0.483 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 24 {
if features.red_chromaticity <= 0.244 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.green_luminance <= 41 {
if features.green_luminance <= 40 {
Intensity::Low
} else {
if features.saturation <= 146 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 26 {
Intensity::Low
} else {
if features.luminance <= 33 {
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
} else {
if features.green_luminance <= 49 {
if features.green_chromaticity <= 0.511 {
if features.red_luminance <= 12 {
if features.blue_chromaticity <= 0.323 {
if features.intensity <= 21 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 178 {
if features.red_chromaticity <= 0.148 {
Intensity::Low
} else {
if features.blue_luminance <= 25 {
Intensity::Low
} else {
if features.luminance <= 27 {
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
} else {
if features.green_chromaticity <= 0.494 {
if features.green_chromaticity <= 0.475 {
if features.red_difference <= 113 {
Intensity::High
} else {
if features.saturation <= 161 {
if features.saturation <= 149 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.487 {
if features.red_luminance <= 19 {
if features.luminance <= 36 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 34 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 20 {
if features.green_chromaticity <= 0.488 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_difference <= 116 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.value <= 44 {
if features.green_chromaticity <= 0.503 {
if features.blue_difference <= 125 {
if features.hue <= 71 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.506 {
if features.saturation <= 154 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 161 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.luminance <= 34 {
Intensity::High
} else {
if features.red_difference <= 112 {
Intensity::Low
} else {
if features.red_luminance <= 17 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
}
} else {
if features.value <= 39 {
if features.luminance <= 25 {
if features.hue <= 73 {
if features.red_chromaticity <= 0.160 {
if features.red_difference <= 117 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 20 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.luminance <= 24 {
if features.green_chromaticity <= 0.526 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.527 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 22 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.371 {
if features.blue_chromaticity <= 0.273 {
Intensity::Low
} else {
if features.blue_luminance <= 23 {
if features.blue_luminance <= 22 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.514 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.525 {
if features.blue_difference <= 125 {
if features.blue_luminance <= 24 {
if features.blue_chromaticity <= 0.285 {
if features.saturation <= 153 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 15 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.145 {
Intensity::High
} else {
if features.green_luminance <= 48 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.518 {
Intensity::Low
} else {
if features.red_difference <= 112 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.saturation <= 187 {
if features.blue_chromaticity <= 0.301 {
if features.green_chromaticity <= 0.526 {
Intensity::High
} else {
if features.red_luminance <= 15 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.152 {
if features.hue <= 74 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.161 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.intensity <= 25 {
if features.blue_chromaticity <= 0.360 {
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
} else {
if features.green_chromaticity <= 0.495 {
if features.blue_difference <= 123 {
if features.blue_luminance <= 82 {
if features.green_chromaticity <= 0.491 {
if features.red_chromaticity <= 0.173 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.335 {
if features.saturation <= 161 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.479 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.492 {
Intensity::High
} else {
if features.green_chromaticity <= 0.494 {
if features.red_difference <= 106 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.hue <= 69 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_luminance <= 84 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.hue <= 77 {
if features.saturation <= 164 {
if features.red_chromaticity <= 0.207 {
if features.intensity <= 38 {
if features.saturation <= 157 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_difference <= 111 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 143 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.342 {
if features.luminance <= 69 {
if features.blue_chromaticity <= 0.340 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.value <= 78 {
if features.green_chromaticity <= 0.484 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 26 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_chromaticity <= 0.482 {
if features.red_difference <= 102 {
if features.green_luminance <= 83 {
Intensity::High
} else {
if features.blue_luminance <= 64 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 35 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.482 {
Intensity::High
} else {
if features.green_luminance <= 72 {
if features.luminance <= 43 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 182 {
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
if features.red_difference <= 102 {
if features.green_chromaticity <= 0.521 {
if features.red_chromaticity <= 0.136 {
if features.blue_luminance <= 63 {
if features.saturation <= 196 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.342 {
if features.blue_chromaticity <= 0.329 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.331 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.343 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_luminance <= 17 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_difference <= 124 {
if features.green_chromaticity <= 0.514 {
if features.green_chromaticity <= 0.513 {
if features.green_chromaticity <= 0.513 {
if features.red_chromaticity <= 0.191 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.green_luminance <= 55 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_luminance <= 50 {
if features.blue_chromaticity <= 0.301 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.332 {
if features.green_chromaticity <= 0.515 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_difference <= 103 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.red_difference <= 104 {
if features.blue_chromaticity <= 0.347 {
if features.intensity <= 51 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.529 {
if features.luminance <= 43 {
if features.red_difference <= 112 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_difference <= 107 {
Intensity::Low
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
}
}
}
} else {
if features.red_difference <= 115 {
if features.blue_difference <= 124 {
if features.blue_difference <= 122 {
if features.red_chromaticity <= 0.146 {
if features.red_difference <= 114 {
if features.red_difference <= 109 {
Intensity::High
} else {
if features.value <= 52 {
if features.blue_difference <= 121 {
if features.saturation <= 211 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.118 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.573 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.888 {
if features.green_luminance <= 35 {
Intensity::High
} else {
if features.green_luminance <= 38 {
if features.hue <= 66 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.300 {
if features.blue_difference <= 121 {
if features.red_chromaticity <= 0.186 {
if features.hue <= 67 {
Intensity::High
} else {
if features.blue_luminance <= 21 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.193 {
if features.red_difference <= 114 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.169 {
if features.red_chromaticity <= 0.146 {
Intensity::Low
} else {
if features.blue_luminance <= 22 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.541 {
if features.blue_chromaticity <= 0.283 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_difference <= 113 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
} else {
if features.saturation <= 184 {
if features.green_chromaticity <= 0.541 {
if features.green_chromaticity <= 0.539 {
if features.luminance <= 48 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 61 {
Intensity::Low
} else {
Intensity::Low
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
if features.red_difference <= 113 {
if features.red_chromaticity <= 0.135 {
if features.green_chromaticity <= 0.537 {
Intensity::Low
} else {
if features.value <= 32 {
if features.green_chromaticity <= 0.733 {
if features.blue_chromaticity <= 0.284 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.saturation <= 217 {
if features.saturation <= 216 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.intensity <= 16 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.saturation <= 182 {
Intensity::High
} else {
if features.red_chromaticity <= 0.154 {
if features.red_chromaticity <= 0.140 {
if features.green_chromaticity <= 0.565 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.541 {
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
if features.blue_chromaticity <= 0.309 {
if features.luminance <= 28 {
if features.hue <= 72 {
if features.blue_luminance <= 15 {
if features.blue_chromaticity <= 0.267 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.145 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 6 {
if features.green_luminance <= 35 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 205 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.intensity <= 24 {
if features.blue_chromaticity <= 0.283 {
Intensity::High
} else {
if features.green_luminance <= 40 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.luminance <= 30 {
Intensity::High
} else {
if features.red_chromaticity <= 0.148 {
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
if features.red_difference <= 113 {
if features.green_chromaticity <= 0.594 {
if features.blue_chromaticity <= 0.340 {
if features.saturation <= 195 {
Intensity::High
} else {
if features.saturation <= 221 {
if features.luminance <= 34 {
if features.luminance <= 31 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.542 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.367 {
if features.red_luminance <= 15 {
if features.red_chromaticity <= 0.114 {
Intensity::High
} else {
if features.red_difference <= 111 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 57 {
if features.blue_chromaticity <= 0.414 {
if features.blue_chromaticity <= 0.395 {
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
} else {
if features.red_difference <= 109 {
if features.green_chromaticity <= 0.644 {
Intensity::High
} else {
if features.green_chromaticity <= 0.648 {
Intensity::Low
} else {
if features.value <= 41 {
if features.red_difference <= 108 {
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
if features.saturation <= 242 {
if features.saturation <= 238 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
}
} else {
if features.blue_luminance <= 24 {
if features.red_difference <= 114 {
if features.blue_chromaticity <= 0.340 {
if features.green_chromaticity <= 0.553 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.327 {
if features.blue_difference <= 125 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.592 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.saturation <= 251 {
if features.blue_difference <= 127 {
Intensity::Low
} else {
if features.luminance <= 23 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.blue_luminance <= 19 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.122 {
if features.green_chromaticity <= 0.575 {
if features.intensity <= 20 {
Intensity::Low
} else {
if features.blue_luminance <= 21 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.578 {
Intensity::High
} else {
if features.luminance <= 22 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.luminance <= 26 {
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
} else {
if features.blue_difference <= 121 {
if features.red_difference <= 117 {
if features.red_difference <= 116 {
if features.green_chromaticity <= 0.610 {
if features.luminance <= 30 {
if features.luminance <= 29 {
if features.value <= 39 {
if features.saturation <= 185 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.intensity <= 26 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.luminance <= 23 {
if features.blue_luminance <= 9 {
Intensity::High
} else {
if features.green_chromaticity <= 0.677 {
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
if features.green_chromaticity <= 0.544 {
Intensity::Low
} else {
if features.blue_luminance <= 12 {
if features.green_luminance <= 26 {
Intensity::High
} else {
if features.green_chromaticity <= 0.775 {
if features.saturation <= 216 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.237 {
if features.saturation <= 179 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.luminance <= 28 {
Intensity::High
} else {
if features.saturation <= 161 {
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
if features.green_luminance <= 29 {
Intensity::Low
} else {
if features.saturation <= 200 {
if features.green_luminance <= 33 {
if features.luminance <= 21 {
Intensity::Low
} else {
if features.saturation <= 188 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.587 {
if features.intensity <= 20 {
Intensity::Low
} else {
if features.saturation <= 163 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 177 {
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
} else {
if features.green_luminance <= 34 {
if features.value <= 31 {
if features.red_difference <= 116 {
if features.blue_chromaticity <= 0.284 {
if features.blue_chromaticity <= 0.218 {
if features.green_chromaticity <= 0.806 {
if features.blue_luminance <= 8 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 3 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 3 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 12 {
if features.blue_chromaticity <= 0.309 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.371 {
if features.red_chromaticity <= 0.077 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 25 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
} else {
if features.red_chromaticity <= 0.157 {
if features.green_luminance <= 29 {
if features.hue <= 69 {
if features.green_chromaticity <= 0.626 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.645 {
if features.red_chromaticity <= 0.121 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.hue <= 66 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 209 {
if features.blue_chromaticity <= 0.289 {
if features.blue_chromaticity <= 0.260 {
if features.red_chromaticity <= 0.137 {
Intensity::High
} else {
if features.red_chromaticity <= 0.158 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.287 {
if features.blue_chromaticity <= 0.266 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 18 {
if features.green_luminance <= 32 {
Intensity::High
} else {
if features.value <= 33 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.345 {
if features.hue <= 73 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.109 {
if features.red_chromaticity <= 0.094 {
Intensity::High
} else {
if features.red_chromaticity <= 0.101 {
if features.green_chromaticity <= 0.628 {
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
}
} else {
if features.red_difference <= 117 {
if features.blue_difference <= 122 {
if features.intensity <= 19 {
if features.red_chromaticity <= 0.154 {
if features.saturation <= 201 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.588 {
if features.green_chromaticity <= 0.581 {
if features.hue <= 65 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.152 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.566 {
if features.blue_luminance <= 20 {
if features.green_chromaticity <= 0.554 {
if features.intensity <= 22 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.561 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 186 {
if features.value <= 39 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.value <= 35 {
if features.intensity <= 19 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.148 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 188 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.green_chromaticity <= 0.541 {
Intensity::Low
} else {
if features.intensity <= 20 {
Intensity::Low
} else {
if features.value <= 35 {
Intensity::Low
} else {
if features.red_luminance <= 13 {
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
}
} else {
if features.blue_difference <= 117 {
if features.red_difference <= 120 {
if features.green_chromaticity <= 0.565 {
if features.blue_difference <= 115 {
if features.red_luminance <= 23 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.289 {
if features.blue_luminance <= 18 {
if features.blue_chromaticity <= 0.176 {
if features.red_chromaticity <= 0.282 {
if features.green_chromaticity <= 0.554 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.saturation <= 168 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_luminance <= 19 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.482 {
if features.saturation <= 141 {
if features.value <= 64 {
if features.intensity <= 43 {
if features.value <= 61 {
Intensity::High
} else {
Intensity::High
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
} else {
if features.red_chromaticity <= 0.302 {
if features.red_chromaticity <= 0.299 {
if features.red_chromaticity <= 0.298 {
if features.red_luminance <= 35 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
if features.saturation <= 173 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.intensity <= 37 {
if features.green_chromaticity <= 0.498 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.304 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.luminance <= 47 {
if features.green_chromaticity <= 0.489 {
Intensity::High
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
} else {
if features.red_chromaticity <= 0.288 {
if features.blue_difference <= 116 {
if features.blue_luminance <= 19 {
if features.green_chromaticity <= 0.560 {
if features.blue_chromaticity <= 0.179 {
if features.green_chromaticity <= 0.551 {
Intensity::High
} else {
if features.green_luminance <= 45 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.535 {
if features.green_chromaticity <= 0.529 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 168 {
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
if features.intensity <= 37 {
if features.value <= 53 {
if features.blue_chromaticity <= 0.202 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.luminance <= 44 {
if features.blue_chromaticity <= 0.209 {
Intensity::High
} else {
Intensity::High
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
if features.red_chromaticity <= 0.276 {
if features.green_chromaticity <= 0.531 {
Intensity::High
} else {
if features.saturation <= 172 {
if features.red_chromaticity <= 0.272 {
if features.red_chromaticity <= 0.269 {
Intensity::High
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
if features.saturation <= 145 {
if features.saturation <= 143 {
if features.blue_chromaticity <= 0.222 {
Intensity::High
} else {
if features.blue_luminance <= 25 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.saturation <= 174 {
if features.red_chromaticity <= 0.279 {
if features.red_luminance <= 26 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 148 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 12 {
if features.green_luminance <= 40 {
Intensity::High
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
if features.green_luminance <= 58 {
if features.green_luminance <= 44 {
Intensity::High
} else {
if features.green_chromaticity <= 0.491 {
if features.blue_chromaticity <= 0.214 {
if features.green_luminance <= 53 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 56 {
if features.green_chromaticity <= 0.482 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 27 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.300 {
if features.red_chromaticity <= 0.299 {
if features.blue_chromaticity <= 0.185 {
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
}
} else {
if features.intensity <= 42 {
if features.green_chromaticity <= 0.482 {
if features.green_chromaticity <= 0.478 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 137 {
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
if features.green_luminance <= 35 {
if features.blue_difference <= 116 {
if features.hue <= 52 {
if features.red_chromaticity <= 0.262 {
if features.green_chromaticity <= 0.715 {
if features.luminance <= 23 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.215 {
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
} else {
if features.value <= 29 {
if features.green_chromaticity <= 0.877 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_luminance <= 34 {
if features.green_luminance <= 33 {
if features.green_chromaticity <= 0.731 {
if features.blue_chromaticity <= 0.055 {
Intensity::Low
} else {
if features.hue <= 52 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.793 {
if features.blue_luminance <= 1 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.668 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_difference <= 119 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_chromaticity <= 0.115 {
if features.green_chromaticity <= 0.655 {
if features.intensity <= 20 {
if features.red_luminance <= 14 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 18 {
if features.saturation <= 213 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.064 {
Intensity::High
} else {
if features.green_chromaticity <= 0.685 {
if features.red_chromaticity <= 0.239 {
Intensity::High
} else {
if features.green_chromaticity <= 0.658 {
Intensity::High
} else {
if features.red_luminance <= 14 {
Intensity::High
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
} else {
if features.green_luminance <= 43 {
if features.red_chromaticity <= 0.259 {
if features.blue_chromaticity <= 0.137 {
if features.value <= 39 {
if features.green_chromaticity <= 0.630 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 40 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 17 {
Intensity::High
} else {
if features.value <= 41 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.262 {
if features.saturation <= 186 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.value <= 40 {
if features.blue_chromaticity <= 0.135 {
if features.red_luminance <= 16 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 38 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.126 {
if features.red_luminance <= 19 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.value <= 42 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.saturation <= 182 {
Intensity::High
} else {
if features.saturation <= 186 {
Intensity::High
} else {
if features.red_chromaticity <= 0.274 {
if features.saturation <= 195 {
Intensity::High
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
if features.red_difference <= 122 {
if features.blue_difference <= 115 {
if features.red_chromaticity <= 0.316 {
if features.green_chromaticity <= 0.597 {
if features.green_chromaticity <= 0.498 {
if features.blue_chromaticity <= 0.192 {
Intensity::Low
} else {
if features.blue_luminance <= 21 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.200 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_luminance <= 28 {
if features.green_chromaticity <= 0.539 {
if features.blue_luminance <= 14 {
if features.green_luminance <= 45 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.304 {
Intensity::Low
} else {
if features.red_luminance <= 23 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.513 {
if features.red_chromaticity <= 0.313 {
if features.green_chromaticity <= 0.508 {
Intensity::High
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
}
} else {
if features.red_chromaticity <= 0.299 {
if features.intensity <= 16 {
Intensity::High
} else {
if features.green_chromaticity <= 0.657 {
if features.blue_luminance <= 3 {
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
} else {
if features.red_chromaticity <= 0.325 {
if features.red_chromaticity <= 0.324 {
if features.green_chromaticity <= 0.651 {
if features.red_chromaticity <= 0.323 {
if features.red_chromaticity <= 0.322 {
if features.saturation <= 186 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.saturation <= 212 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.red_luminance <= 24 {
if features.green_chromaticity <= 0.559 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.saturation <= 149 {
Intensity::Low
} else {
if features.luminance <= 43 {
if features.green_chromaticity <= 0.503 {
if features.saturation <= 158 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.327 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.514 {
if features.value <= 47 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 36 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_luminance <= 53 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.red_chromaticity <= 0.315 {
if features.red_chromaticity <= 0.291 {
if features.red_chromaticity <= 0.267 {
if features.green_luminance <= 24 {
Intensity::High
} else {
if features.value <= 27 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.750 {
if features.green_chromaticity <= 0.702 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.276 {
if features.red_chromaticity <= 0.272 {
if features.saturation <= 243 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.290 {
if features.red_chromaticity <= 0.289 {
if features.red_chromaticity <= 0.285 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.saturation <= 222 {
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
} else {
if features.hue <= 49 {
if features.green_luminance <= 35 {
if features.green_luminance <= 28 {
Intensity::High
} else {
if features.red_chromaticity <= 0.309 {
if features.value <= 32 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_difference <= 116 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_luminance <= 21 {
if features.luminance <= 28 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.293 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.302 {
if features.red_luminance <= 22 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 32 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_luminance <= 22 {
if features.value <= 40 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 48 {
if features.red_chromaticity <= 0.299 {
if features.green_luminance <= 43 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 46 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 22 {
if features.blue_chromaticity <= 0.204 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 36 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
}
} else {
if features.blue_difference <= 116 {
if features.intensity <= 31 {
if features.blue_chromaticity <= 0.157 {
if features.luminance <= 26 {
Intensity::Low
} else {
if features.red_luminance <= 22 {
if features.green_chromaticity <= 0.566 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 24 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.503 {
if features.saturation <= 162 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.322 {
if features.saturation <= 174 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
}
} else {
if features.blue_luminance <= 19 {
if features.green_luminance <= 47 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 149 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.530 {
if features.red_chromaticity <= 0.319 {
if features.green_luminance <= 41 {
if features.red_luminance <= 24 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.497 {
if features.green_chromaticity <= 0.492 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 26 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.value <= 47 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.478 {
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
} else {
if features.red_difference <= 123 {
if features.red_chromaticity <= 0.337 {
if features.blue_difference <= 116 {
if features.luminance <= 26 {
if features.blue_chromaticity <= 0.049 {
if features.green_luminance <= 30 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 35 {
Intensity::Low
} else {
if features.saturation <= 152 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.164 {
if features.saturation <= 182 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.169 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.saturation <= 241 {
if features.saturation <= 218 {
if features.value <= 32 {
Intensity::Low
} else {
if features.saturation <= 200 {
if features.saturation <= 179 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 15 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 0 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.saturation <= 205 {
if features.green_luminance <= 39 {
Intensity::High
} else {
if features.luminance <= 35 {
if features.intensity <= 24 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.525 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.600 {
if features.intensity <= 20 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
}
} else {
if features.green_luminance <= 39 {
if features.luminance <= 21 {
if features.red_luminance <= 15 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.390 {
if features.saturation <= 250 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.luminance <= 29 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.532 {
if features.saturation <= 187 {
if features.saturation <= 171 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.369 {
if features.blue_chromaticity <= 0.120 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.luminance <= 30 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
} else {
if features.saturation <= 184 {
if features.green_chromaticity <= 0.478 {
Intensity::Low
} else {
if features.intensity <= 31 {
if features.blue_chromaticity <= 0.167 {
if features.green_chromaticity <= 0.503 {
if features.green_chromaticity <= 0.482 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.337 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.491 {
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
} else {
if features.blue_chromaticity <= 0.136 {
if features.hue <= 42 {
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
if features.green_luminance <= 35 {
if features.green_luminance <= 31 {
if features.red_difference <= 119 {
if features.blue_difference <= 118 {
if features.blue_luminance <= 1 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.061 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.blue_difference <= 122 {
if features.green_chromaticity <= 0.948 {
if features.blue_chromaticity <= 0.104 {
if features.green_chromaticity <= 0.782 {
if features.red_luminance <= 6 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.185 {
if features.green_chromaticity <= 0.619 {
if features.green_chromaticity <= 0.609 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.675 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_difference <= 121 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.intensity <= 5 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.283 {
if features.green_chromaticity <= 0.596 {
if features.blue_chromaticity <= 0.273 {
if features.intensity <= 16 {
if features.blue_chromaticity <= 0.258 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.570 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_luminance <= 30 {
if features.green_chromaticity <= 0.562 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.luminance <= 11 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.179 {
if features.blue_chromaticity <= 0.308 {
if features.saturation <= 185 {
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
}
}
}
} else {
if features.value <= 29 {
if features.blue_difference <= 121 {
if features.red_difference <= 120 {
if features.blue_chromaticity <= 0.183 {
if features.hue <= 55 {
if features.red_chromaticity <= 0.221 {
if features.green_luminance <= 27 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.184 {
if features.green_chromaticity <= 0.711 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 19 {
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
if features.red_difference <= 122 {
if features.blue_chromaticity <= 0.080 {
if features.blue_chromaticity <= 0.069 {
if features.red_difference <= 121 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 28 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.saturation <= 210 {
if features.saturation <= 207 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.blue_difference <= 118 {
if features.red_chromaticity <= 0.312 {
if features.saturation <= 232 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 210 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.114 {
if features.blue_chromaticity <= 0.113 {
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
} else {
if features.blue_luminance <= 19 {
if features.value <= 28 {
if features.saturation <= 118 {
Intensity::Low
} else {
if features.red_difference <= 122 {
if features.saturation <= 158 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 133 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_luminance <= 11 {
if features.blue_chromaticity <= 0.208 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.212 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.223 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.581 {
if features.green_chromaticity <= 0.550 {
if features.red_chromaticity <= 0.372 {
if features.red_luminance <= 16 {
if features.blue_chromaticity <= 0.207 {
if features.red_chromaticity <= 0.270 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 24 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.350 {
if features.luminance <= 25 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.353 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_difference <= 126 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.565 {
if features.red_luminance <= 17 {
if features.green_chromaticity <= 0.560 {
if features.red_difference <= 121 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_difference <= 120 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.217 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.649 {
if features.luminance <= 22 {
if features.red_chromaticity <= 0.227 {
if features.red_chromaticity <= 0.220 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 5 {
Intensity::Low
} else {
if features.hue <= 53 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.620 {
if features.blue_chromaticity <= 0.171 {
if features.red_luminance <= 15 {
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
}
} else {
if features.luminance <= 21 {
Intensity::Low
} else {
if features.hue <= 51 {
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
if features.green_chromaticity <= 0.583 {
if features.green_chromaticity <= 0.536 {
if features.green_chromaticity <= 0.519 {
if features.hue <= 54 {
if features.red_chromaticity <= 0.305 {
if features.blue_difference <= 120 {
if features.green_chromaticity <= 0.504 {
if features.blue_luminance <= 13 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 33 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.479 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.134 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.496 {
if features.red_chromaticity <= 0.221 {
if features.value <= 33 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.315 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 129 {
if features.red_luminance <= 17 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.482 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.saturation <= 155 {
if features.red_luminance <= 16 {
if features.red_chromaticity <= 0.240 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.259 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 12 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.luminance <= 25 {
if features.saturation <= 156 {
if features.blue_chromaticity <= 0.223 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.187 {
Intensity::Low
} else {
if features.value <= 32 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.206 {
Intensity::Low
} else {
if features.red_luminance <= 20 {
if features.red_chromaticity <= 0.237 {
if features.hue <= 63 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 146 {
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
} else {
if features.hue <= 55 {
if features.red_chromaticity <= 0.297 {
if features.blue_luminance <= 9 {
if features.red_chromaticity <= 0.286 {
if features.saturation <= 187 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.569 {
if features.red_chromaticity <= 0.260 {
if features.luminance <= 25 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.268 {
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
if features.red_chromaticity <= 0.316 {
if features.blue_chromaticity <= 0.128 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 190 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.luminance <= 25 {
if features.saturation <= 149 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.239 {
if features.red_luminance <= 13 {
if features.saturation <= 166 {
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
}
} else {
if features.saturation <= 151 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.561 {
if features.intensity <= 20 {
if features.hue <= 59 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 156 {
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
if features.value <= 33 {
if features.green_chromaticity <= 0.613 {
if features.green_luminance <= 32 {
if features.hue <= 50 {
Intensity::Low
} else {
if features.blue_difference <= 120 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 12 {
if features.hue <= 60 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.257 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.198 {
if features.green_chromaticity <= 0.654 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 32 {
if features.saturation <= 203 {
Intensity::Low
} else {
if features.hue <= 53 {
if features.blue_luminance <= 5 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.luminance <= 23 {
if features.red_chromaticity <= 0.204 {
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
} else {
if features.saturation <= 196 {
if features.red_chromaticity <= 0.212 {
if features.luminance <= 24 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 11 {
if features.blue_chromaticity <= 0.164 {
if features.red_chromaticity <= 0.261 {
if features.red_luminance <= 13 {
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
Intensity::Low
}
}
} else {
if features.hue <= 53 {
if features.luminance <= 25 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 207 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
}
} else {
if features.green_chromaticity <= 0.537 {
if features.green_luminance <= 44 {
if features.green_chromaticity <= 0.516 {
if features.green_chromaticity <= 0.494 {
if features.green_luminance <= 39 {
if features.green_chromaticity <= 0.494 {
if features.saturation <= 136 {
if features.green_chromaticity <= 0.478 {
if features.value <= 37 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.493 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.493 {
if features.red_chromaticity <= 0.301 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 130 {
Intensity::Low
} else {
if features.blue_difference <= 121 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.476 {
if features.saturation <= 127 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 153 {
if features.red_chromaticity <= 0.243 {
if features.saturation <= 132 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.259 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_luminance <= 40 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_luminance <= 39 {
if features.green_chromaticity <= 0.510 {
if features.saturation <= 147 {
if features.saturation <= 134 {
if features.red_luminance <= 18 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 142 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.value <= 37 {
if features.green_chromaticity <= 0.503 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 158 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.220 {
if features.green_luminance <= 36 {
Intensity::Low
} else {
if features.value <= 37 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 142 {
if features.green_chromaticity <= 0.514 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 24 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_chromaticity <= 0.269 {
if features.green_chromaticity <= 0.509 {
if features.blue_chromaticity <= 0.257 {
if features.saturation <= 131 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 42 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 20 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.509 {
if features.saturation <= 156 {
if features.luminance <= 33 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 41 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.287 {
if features.red_chromaticity <= 0.278 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_difference <= 121 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
}
} else {
if features.red_difference <= 120 {
if features.green_chromaticity <= 0.523 {
if features.luminance <= 28 {
Intensity::Low
} else {
if features.saturation <= 142 {
if features.red_luminance <= 19 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.277 {
if features.luminance <= 29 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
}
} else {
if features.blue_luminance <= 14 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.530 {
if features.blue_luminance <= 15 {
Intensity::Low
} else {
if features.blue_luminance <= 17 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.luminance <= 33 {
if features.saturation <= 147 {
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
} else {
if features.intensity <= 24 {
if features.green_chromaticity <= 0.525 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.529 {
if features.red_luminance <= 21 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 38 {
if features.blue_luminance <= 13 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
}
} else {
if features.value <= 40 {
if features.luminance <= 31 {
if features.blue_difference <= 118 {
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
}
}
}
} else {
if features.green_chromaticity <= 0.503 {
if features.green_luminance <= 50 {
if features.green_chromaticity <= 0.484 {
if features.luminance <= 40 {
if features.green_chromaticity <= 0.480 {
if features.red_luminance <= 28 {
if features.green_chromaticity <= 0.479 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.313 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.306 {
if features.red_chromaticity <= 0.259 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 24 {
Intensity::Low
} else {
if features.saturation <= 122 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_luminance <= 48 {
if features.red_luminance <= 27 {
if features.red_chromaticity <= 0.284 {
if features.blue_luminance <= 21 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 146 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.485 {
Intensity::Low
} else {
if features.hue <= 53 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.luminance <= 40 {
if features.red_chromaticity <= 0.266 {
Intensity::Low
} else {
if features.blue_difference <= 118 {
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
if features.blue_difference <= 118 {
if features.red_chromaticity <= 0.293 {
if features.green_luminance <= 52 {
if features.blue_chromaticity <= 0.227 {
if features.red_difference <= 119 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.484 {
if features.intensity <= 39 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 43 {
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
if features.red_chromaticity <= 0.270 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.476 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.247 {
if features.blue_luminance <= 26 {
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
if features.blue_chromaticity <= 0.223 {
if features.blue_luminance <= 21 {
if features.green_chromaticity <= 0.532 {
if features.blue_chromaticity <= 0.210 {
if features.red_luminance <= 24 {
Intensity::Low
} else {
if features.green_luminance <= 45 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.value <= 48 {
if features.hue <= 56 {
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
Intensity::Low
}
} else {
if features.green_luminance <= 46 {
Intensity::Low
} else {
if features.saturation <= 139 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.red_difference <= 119 {
if features.blue_chromaticity <= 0.210 {
if features.red_chromaticity <= 0.252 {
if features.green_chromaticity <= 0.607 {
if features.red_chromaticity <= 0.239 {
if features.red_chromaticity <= 0.233 {
if features.red_luminance <= 14 {
if features.red_chromaticity <= 0.224 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.hue <= 56 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.200 {
if features.red_chromaticity <= 0.244 {
if features.saturation <= 174 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.248 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.550 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.145 {
Intensity::High
} else {
if features.luminance <= 26 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.luminance <= 32 {
Intensity::High
} else {
if features.value <= 43 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.233 {
if features.green_chromaticity <= 0.552 {
if features.blue_chromaticity <= 0.215 {
Intensity::Low
} else {
if features.blue_luminance <= 15 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.luminance <= 27 {
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
if features.green_chromaticity <= 0.575 {
if features.luminance <= 28 {
if features.red_difference <= 121 {
if features.blue_difference <= 118 {
Intensity::Low
} else {
if features.red_luminance <= 16 {
Intensity::Low
} else {
if features.saturation <= 171 {
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
if features.saturation <= 180 {
if features.red_luminance <= 19 {
if features.green_luminance <= 38 {
if features.intensity <= 22 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.183 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 12 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.275 {
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
} else {
if features.green_chromaticity <= 0.584 {
Intensity::Low
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
}