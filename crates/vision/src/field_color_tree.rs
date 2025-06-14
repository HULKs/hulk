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
if features.blue_difference <= 110 {
if features.green_chromaticity <= 0.399 {
if features.green_chromaticity <= 0.394 {
if features.green_chromaticity <= 0.391 {
if features.green_chromaticity <= 0.388 {
if features.green_chromaticity <= 0.386 {
if features.green_chromaticity <= 0.382 {
if features.green_chromaticity <= 0.381 {
if features.red_chromaticity <= 0.338 {
if features.red_chromaticity <= 0.338 {
if features.green_chromaticity <= 0.374 {
if features.green_chromaticity <= 0.374 {
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
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.381 {
if features.red_difference <= 120 {
Intensity::Low
} else {
if features.saturation <= 98 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 155 {
if features.blue_luminance <= 133 {
if features.blue_chromaticity <= 0.269 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.382 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.269 {
Intensity::High
} else {
if features.red_luminance <= 141 {
if features.red_chromaticity <= 0.338 {
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
}
}
} else {
if features.intensity <= 155 {
Intensity::High
} else {
if features.green_chromaticity <= 0.384 {
if features.red_chromaticity <= 0.328 {
if features.red_difference <= 118 {
if features.blue_luminance <= 136 {
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
}
} else {
if features.green_chromaticity <= 0.382 {
Intensity::Low
} else {
if features.blue_difference <= 107 {
if features.blue_chromaticity <= 0.293 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.293 {
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
if features.red_luminance <= 151 {
if features.blue_chromaticity <= 0.271 {
if features.red_chromaticity <= 0.351 {
if features.saturation <= 82 {
if features.green_chromaticity <= 0.386 {
if features.green_chromaticity <= 0.386 {
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
Intensity::Low
}
} else {
if features.blue_luminance <= 113 {
if features.blue_chromaticity <= 0.272 {
Intensity::High
} else {
if features.blue_luminance <= 112 {
if features.red_luminance <= 133 {
if features.intensity <= 128 {
if features.green_luminance <= 144 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 110 {
if features.saturation <= 75 {
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
} else {
if features.blue_difference <= 108 {
if features.green_chromaticity <= 0.388 {
if features.green_chromaticity <= 0.388 {
if features.intensity <= 142 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.387 {
Intensity::Low
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
} else {
if features.red_chromaticity <= 0.327 {
if features.red_chromaticity <= 0.327 {
if features.green_chromaticity <= 0.388 {
if features.value <= 172 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 67 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.283 {
if features.red_luminance <= 146 {
if features.green_chromaticity <= 0.387 {
Intensity::Low
} else {
Intensity::Low
}
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
}
}
}
} else {
if features.green_chromaticity <= 0.386 {
if features.luminance <= 166 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.287 {
if features.blue_chromaticity <= 0.287 {
if features.blue_chromaticity <= 0.287 {
if features.blue_chromaticity <= 0.285 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.285 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.387 {
Intensity::Low
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
}
}
} else {
if features.red_luminance <= 154 {
if features.red_chromaticity <= 0.345 {
if features.red_luminance <= 135 {
if features.red_luminance <= 123 {
if features.green_chromaticity <= 0.390 {
if features.red_chromaticity <= 0.342 {
Intensity::High
} else {
if features.hue <= 41 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 155 {
if features.intensity <= 125 {
if features.blue_difference <= 109 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.intensity <= 134 {
if features.blue_chromaticity <= 0.280 {
if features.saturation <= 73 {
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
if features.red_chromaticity <= 0.320 {
if features.red_chromaticity <= 0.320 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.388 {
if features.blue_difference <= 108 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.luminance <= 152 {
if features.red_luminance <= 140 {
if features.red_difference <= 122 {
if features.red_luminance <= 139 {
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
if features.blue_luminance <= 124 {
if features.red_chromaticity <= 0.329 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.330 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.388 {
Intensity::High
} else {
if features.red_chromaticity <= 0.331 {
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
Intensity::Low
}
} else {
if features.value <= 186 {
if features.green_chromaticity <= 0.391 {
if features.red_chromaticity <= 0.330 {
if features.red_chromaticity <= 0.328 {
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
if features.luminance <= 176 {
if features.intensity <= 163 {
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
}
}
}
} else {
if features.red_luminance <= 156 {
if features.red_chromaticity <= 0.341 {
if features.luminance <= 150 {
if features.value <= 160 {
if features.red_luminance <= 123 {
if features.blue_chromaticity <= 0.267 {
Intensity::High
} else {
if features.green_chromaticity <= 0.393 {
Intensity::Low
} else {
if features.blue_luminance <= 95 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.392 {
if features.red_luminance <= 127 {
Intensity::High
} else {
if features.blue_luminance <= 106 {
if features.blue_luminance <= 105 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_difference <= 109 {
Intensity::High
} else {
if features.green_luminance <= 159 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
} else {
if features.green_luminance <= 159 {
if features.luminance <= 143 {
if features.red_chromaticity <= 0.330 {
Intensity::Low
} else {
Intensity::High
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
if features.green_chromaticity <= 0.392 {
if features.red_difference <= 118 {
if features.value <= 161 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.391 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.393 {
if features.blue_luminance <= 114 {
if features.green_chromaticity <= 0.393 {
if features.green_chromaticity <= 0.393 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.hue <= 45 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.394 {
if features.hue <= 45 {
Intensity::Low
} else {
if features.hue <= 47 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 137 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.red_chromaticity <= 0.324 {
if features.red_chromaticity <= 0.322 {
if features.red_difference <= 113 {
if features.green_chromaticity <= 0.393 {
if features.luminance <= 174 {
if features.intensity <= 161 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 154 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.intensity <= 158 {
if features.red_luminance <= 141 {
if features.luminance <= 158 {
if features.value <= 172 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.value <= 177 {
if features.green_chromaticity <= 0.393 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 145 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.289 {
if features.blue_chromaticity <= 0.287 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.288 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 139 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_chromaticity <= 0.285 {
if features.green_chromaticity <= 0.393 {
if features.green_chromaticity <= 0.392 {
if features.green_chromaticity <= 0.392 {
if features.saturation <= 69 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.saturation <= 69 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_difference <= 108 {
Intensity::Low
} else {
if features.intensity <= 146 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.saturation <= 71 {
if features.blue_difference <= 108 {
Intensity::High
} else {
if features.red_difference <= 116 {
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
}
} else {
if features.red_difference <= 118 {
if features.blue_luminance <= 118 {
if features.green_chromaticity <= 0.394 {
Intensity::Low
} else {
if features.luminance <= 152 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.391 {
if features.blue_chromaticity <= 0.282 {
if features.green_luminance <= 175 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.281 {
if features.blue_chromaticity <= 0.281 {
if features.intensity <= 150 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.blue_difference <= 108 {
if features.saturation <= 71 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.326 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.blue_luminance <= 114 {
if features.value <= 166 {
if features.blue_chromaticity <= 0.270 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.278 {
if features.red_chromaticity <= 0.331 {
if features.red_chromaticity <= 0.331 {
if features.saturation <= 76 {
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
if features.blue_chromaticity <= 0.278 {
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
if features.blue_luminance <= 80 {
Intensity::Low
} else {
if features.blue_luminance <= 88 {
if features.blue_chromaticity <= 0.250 {
Intensity::High
} else {
if features.green_luminance <= 126 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.344 {
if features.blue_chromaticity <= 0.264 {
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
if features.red_luminance <= 159 {
if features.blue_luminance <= 143 {
if features.green_chromaticity <= 0.391 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 146 {
if features.green_chromaticity <= 0.392 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.274 {
if features.blue_chromaticity <= 0.273 {
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
if features.blue_difference <= 107 {
if features.blue_difference <= 105 {
if features.blue_difference <= 104 {
if features.red_difference <= 121 {
if features.red_chromaticity <= 0.346 {
if features.red_luminance <= 142 {
if features.blue_chromaticity <= 0.262 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_luminance <= 154 {
if features.blue_chromaticity <= 0.269 {
if features.red_chromaticity <= 0.339 {
if features.red_chromaticity <= 0.339 {
if features.green_chromaticity <= 0.397 {
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
if features.intensity <= 151 {
Intensity::High
} else {
if features.red_chromaticity <= 0.332 {
Intensity::Low
} else {
if features.value <= 183 {
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
Intensity::High
}
} else {
if features.green_luminance <= 74 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.332 {
if features.blue_chromaticity <= 0.276 {
if features.saturation <= 82 {
if features.green_chromaticity <= 0.399 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 172 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.327 {
if features.hue <= 47 {
if features.blue_chromaticity <= 0.277 {
Intensity::High
} else {
if features.intensity <= 160 {
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
} else {
if features.luminance <= 166 {
if features.green_chromaticity <= 0.394 {
Intensity::High
} else {
if features.saturation <= 81 {
if features.luminance <= 161 {
if features.saturation <= 80 {
Intensity::High
} else {
if features.red_luminance <= 146 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.333 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.value <= 179 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.337 {
if features.value <= 167 {
if features.intensity <= 138 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 144 {
Intensity::Low
} else {
if features.red_difference <= 121 {
if features.green_luminance <= 161 {
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
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.397 {
if features.red_luminance <= 154 {
if features.red_luminance <= 128 {
if features.saturation <= 95 {
if features.red_chromaticity <= 0.357 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.269 {
if features.green_chromaticity <= 0.397 {
if features.blue_difference <= 106 {
if features.green_chromaticity <= 0.396 {
Intensity::High
} else {
if features.green_chromaticity <= 0.397 {
if features.red_chromaticity <= 0.341 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 129 {
Intensity::High
} else {
if features.red_chromaticity <= 0.335 {
Intensity::High
} else {
if features.saturation <= 82 {
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
} else {
if features.blue_chromaticity <= 0.282 {
if features.red_chromaticity <= 0.329 {
if features.red_chromaticity <= 0.329 {
if features.blue_difference <= 106 {
if features.saturation <= 77 {
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
} else {
if features.red_luminance <= 142 {
if features.red_chromaticity <= 0.329 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 148 {
if features.green_luminance <= 165 {
if features.blue_chromaticity <= 0.273 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_difference <= 106 {
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
if features.green_luminance <= 189 {
if features.green_chromaticity <= 0.395 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 136 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
}
} else {
if features.red_chromaticity <= 0.320 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.320 {
if features.green_chromaticity <= 0.395 {
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
if features.hue <= 41 {
Intensity::Low
} else {
if features.luminance <= 145 {
if features.green_chromaticity <= 0.398 {
if features.hue <= 43 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.323 {
if features.luminance <= 177 {
if features.blue_chromaticity <= 0.283 {
if features.green_luminance <= 192 {
if features.red_chromaticity <= 0.322 {
if features.luminance <= 167 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.280 {
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
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.328 {
if features.blue_chromaticity <= 0.278 {
if features.green_chromaticity <= 0.399 {
if features.saturation <= 77 {
if features.value <= 178 {
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
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.279 {
if features.blue_chromaticity <= 0.278 {
if features.luminance <= 164 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.intensity <= 155 {
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
if features.saturation <= 82 {
if features.blue_chromaticity <= 0.271 {
if features.green_chromaticity <= 0.398 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.272 {
if features.intensity <= 138 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.329 {
Intensity::Low
} else {
Intensity::High
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
if features.red_chromaticity <= 0.341 {
if features.blue_luminance <= 146 {
if features.red_luminance <= 131 {
if features.value <= 144 {
if features.green_chromaticity <= 0.396 {
if features.red_chromaticity <= 0.340 {
if features.green_chromaticity <= 0.394 {
Intensity::High
} else {
if features.green_chromaticity <= 0.395 {
if features.blue_chromaticity <= 0.270 {
if features.green_chromaticity <= 0.395 {
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
Intensity::High
}
} else {
if features.red_luminance <= 117 {
if features.blue_chromaticity <= 0.261 {
Intensity::High
} else {
if features.red_chromaticity <= 0.340 {
if features.blue_chromaticity <= 0.268 {
Intensity::High
} else {
if features.green_chromaticity <= 0.397 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.329 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.saturation <= 74 {
if features.red_chromaticity <= 0.323 {
if features.red_chromaticity <= 0.318 {
Intensity::High
} else {
if features.red_chromaticity <= 0.319 {
if features.red_chromaticity <= 0.319 {
if features.green_luminance <= 163 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.intensity <= 134 {
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
if features.green_chromaticity <= 0.394 {
Intensity::High
} else {
if features.green_chromaticity <= 0.395 {
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
if features.green_chromaticity <= 0.394 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.327 {
if features.red_luminance <= 128 {
Intensity::High
} else {
if features.green_chromaticity <= 0.398 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.327 {
if features.green_chromaticity <= 0.398 {
if features.blue_chromaticity <= 0.277 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.saturation <= 82 {
Intensity::High
} else {
if features.blue_luminance <= 99 {
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
if features.red_chromaticity <= 0.319 {
if features.luminance <= 168 {
if features.green_chromaticity <= 0.398 {
if features.blue_chromaticity <= 0.290 {
if features.green_chromaticity <= 0.397 {
if features.green_chromaticity <= 0.397 {
if features.green_chromaticity <= 0.395 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 133 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_difference <= 113 {
if features.green_luminance <= 182 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 132 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.395 {
if features.red_chromaticity <= 0.315 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 183 {
Intensity::Low
} else {
if features.red_difference <= 112 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.value <= 174 {
if features.luminance <= 154 {
Intensity::High
} else {
if features.saturation <= 71 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 141 {
if features.green_chromaticity <= 0.399 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.311 {
Intensity::High
} else {
if features.red_chromaticity <= 0.312 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
}
} else {
if features.blue_chromaticity <= 0.289 {
if features.red_chromaticity <= 0.314 {
Intensity::High
} else {
if features.red_chromaticity <= 0.315 {
if features.green_chromaticity <= 0.399 {
if features.luminance <= 171 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.288 {
if features.green_chromaticity <= 0.398 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 153 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
} else {
if features.green_luminance <= 199 {
if features.blue_chromaticity <= 0.293 {
if features.green_chromaticity <= 0.395 {
if features.green_chromaticity <= 0.394 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.309 {
Intensity::High
} else {
if features.green_chromaticity <= 0.396 {
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
} else {
if features.red_luminance <= 135 {
if features.red_chromaticity <= 0.327 {
if features.intensity <= 139 {
if features.green_chromaticity <= 0.394 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.320 {
Intensity::High
} else {
if features.red_chromaticity <= 0.320 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.blue_difference <= 109 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.hue <= 46 {
if features.green_chromaticity <= 0.394 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.276 {
if features.green_chromaticity <= 0.397 {
if features.green_chromaticity <= 0.396 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.intensity <= 136 {
if features.luminance <= 146 {
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
}
}
}
} else {
if features.green_chromaticity <= 0.396 {
if features.green_chromaticity <= 0.394 {
if features.green_chromaticity <= 0.394 {
if features.red_difference <= 115 {
if features.luminance <= 162 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 161 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.intensity <= 153 {
if features.red_luminance <= 137 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.395 {
if features.blue_chromaticity <= 0.287 {
if features.value <= 181 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.282 {
if features.red_chromaticity <= 0.324 {
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
}
}
} else {
if features.luminance <= 158 {
if features.red_chromaticity <= 0.322 {
if features.red_difference <= 115 {
if features.intensity <= 142 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.396 {
Intensity::High
} else {
if features.green_chromaticity <= 0.398 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.320 {
if features.red_chromaticity <= 0.320 {
if features.green_chromaticity <= 0.397 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.red_luminance <= 142 {
Intensity::High
} else {
if features.red_luminance <= 144 {
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
if features.luminance <= 181 {
if features.hue <= 55 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 108 {
if features.red_luminance <= 104 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.357 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_difference <= 124 {
if features.green_chromaticity <= 0.396 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.258 {
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
} else {
if features.red_difference <= 121 {
if features.hue <= 66 {
if features.green_chromaticity <= 0.409 {
if features.blue_difference <= 105 {
if features.green_chromaticity <= 0.406 {
if features.luminance <= 148 {
if features.blue_chromaticity <= 0.256 {
if features.red_chromaticity <= 0.340 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.399 {
Intensity::Low
} else {
if features.green_luminance <= 162 {
if features.red_chromaticity <= 0.332 {
if features.value <= 159 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 96 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.334 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.260 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.red_luminance <= 157 {
if features.blue_chromaticity <= 0.271 {
if features.red_chromaticity <= 0.329 {
if features.red_chromaticity <= 0.327 {
if features.blue_chromaticity <= 0.271 {
if features.green_chromaticity <= 0.404 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.luminance <= 171 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.value <= 178 {
if features.green_chromaticity <= 0.405 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.405 {
if features.red_luminance <= 137 {
if features.red_chromaticity <= 0.333 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 113 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.406 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.262 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_chromaticity <= 0.321 {
if features.green_chromaticity <= 0.405 {
Intensity::High
} else {
if features.luminance <= 169 {
if features.red_chromaticity <= 0.320 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.intensity <= 153 {
if features.green_chromaticity <= 0.401 {
Intensity::High
} else {
if features.red_chromaticity <= 0.324 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.272 {
Intensity::High
} else {
if features.green_chromaticity <= 0.400 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.red_chromaticity <= 0.344 {
if features.red_luminance <= 159 {
if features.blue_chromaticity <= 0.277 {
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
if features.red_luminance <= 153 {
if features.blue_luminance <= 118 {
if features.red_luminance <= 130 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.258 {
if features.green_chromaticity <= 0.409 {
if features.blue_luminance <= 101 {
if features.green_chromaticity <= 0.409 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.257 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 105 {
if features.red_chromaticity <= 0.331 {
if features.blue_difference <= 104 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 159 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.408 {
if features.green_luminance <= 172 {
Intensity::High
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
}
}
} else {
if features.blue_luminance <= 122 {
if features.red_chromaticity <= 0.322 {
if features.red_chromaticity <= 0.322 {
if features.blue_luminance <= 119 {
if features.saturation <= 85 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 141 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.value <= 183 {
if features.red_luminance <= 144 {
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
if features.blue_chromaticity <= 0.275 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.275 {
if features.hue <= 50 {
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
if features.red_chromaticity <= 0.307 {
if features.green_chromaticity <= 0.408 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.409 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.406 {
Intensity::High
} else {
if features.red_luminance <= 155 {
if features.green_chromaticity <= 0.406 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.284 {
if features.blue_difference <= 98 {
if features.saturation <= 95 {
Intensity::High
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
}
} else {
if features.green_chromaticity <= 0.403 {
if features.red_luminance <= 130 {
if features.blue_luminance <= 85 {
if features.luminance <= 113 {
if features.green_chromaticity <= 0.403 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_difference <= 120 {
if features.red_chromaticity <= 0.323 {
if features.value <= 152 {
if features.red_luminance <= 115 {
Intensity::High
} else {
if features.intensity <= 120 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.403 {
if features.saturation <= 76 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.267 {
if features.blue_chromaticity <= 0.265 {
Intensity::High
} else {
if features.value <= 156 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.399 {
if features.red_luminance <= 126 {
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
}
} else {
if features.blue_luminance <= 90 {
if features.blue_luminance <= 87 {
Intensity::High
} else {
if features.intensity <= 113 {
if features.green_chromaticity <= 0.402 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 138 {
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
}
} else {
if features.red_chromaticity <= 0.320 {
if features.luminance <= 181 {
if features.green_luminance <= 177 {
if features.red_chromaticity <= 0.312 {
if features.blue_chromaticity <= 0.288 {
if features.blue_chromaticity <= 0.286 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.289 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 118 {
if features.red_luminance <= 131 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.317 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.value <= 187 {
if features.luminance <= 162 {
if features.blue_chromaticity <= 0.283 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.314 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.288 {
if features.blue_chromaticity <= 0.287 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 67 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.green_chromaticity <= 0.403 {
if features.blue_difference <= 106 {
if features.green_luminance <= 216 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.303 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.red_luminance <= 134 {
if features.red_chromaticity <= 0.330 {
if features.value <= 165 {
if features.value <= 160 {
if features.red_chromaticity <= 0.330 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.403 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_luminance <= 166 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.401 {
if features.value <= 160 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 132 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_difference <= 117 {
if features.hue <= 47 {
if features.blue_chromaticity <= 0.271 {
if features.luminance <= 150 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.400 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.325 {
if features.intensity <= 144 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 175 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.intensity <= 138 {
if features.saturation <= 83 {
if features.green_chromaticity <= 0.400 {
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
if features.red_luminance <= 127 {
if features.intensity <= 102 {
if features.red_chromaticity <= 0.334 {
Intensity::High
} else {
if features.blue_luminance <= 74 {
Intensity::High
} else {
if features.green_luminance <= 119 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.luminance <= 142 {
if features.blue_chromaticity <= 0.262 {
if features.blue_chromaticity <= 0.261 {
if features.green_chromaticity <= 0.404 {
Intensity::Low
} else {
if features.red_luminance <= 109 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.luminance <= 125 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.404 {
if features.blue_chromaticity <= 0.277 {
Intensity::High
} else {
if features.intensity <= 125 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_difference <= 108 {
if features.green_luminance <= 157 {
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
}
}
} else {
if features.red_luminance <= 122 {
if features.blue_chromaticity <= 0.283 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.306 {
if features.red_chromaticity <= 0.306 {
if features.green_chromaticity <= 0.409 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.hue <= 53 {
if features.red_chromaticity <= 0.313 {
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
if features.red_luminance <= 156 {
if features.red_chromaticity <= 0.313 {
if features.value <= 188 {
if features.blue_chromaticity <= 0.291 {
if features.blue_chromaticity <= 0.286 {
if features.blue_chromaticity <= 0.284 {
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
if features.red_chromaticity <= 0.303 {
if features.red_difference <= 107 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.303 {
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
if features.intensity <= 146 {
if features.red_luminance <= 131 {
if features.saturation <= 77 {
if features.green_chromaticity <= 0.405 {
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
} else {
if features.green_chromaticity <= 0.406 {
if features.green_luminance <= 169 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 133 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.blue_chromaticity <= 0.281 {
if features.intensity <= 154 {
if features.blue_chromaticity <= 0.277 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.404 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.intensity <= 156 {
if features.red_luminance <= 143 {
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
if features.value <= 214 {
if features.green_chromaticity <= 0.407 {
if features.red_chromaticity <= 0.300 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 189 {
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
} else {
if features.red_luminance <= 14 {
if features.blue_difference <= 102 {
if features.blue_difference <= 100 {
if features.hue <= 54 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_difference <= 85 {
if features.red_difference <= 84 {
Intensity::High
} else {
if features.saturation <= 238 {
if features.saturation <= 228 {
Intensity::Low
} else {
if features.intensity <= 47 {
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
Intensity::High
}
}
} else {
if features.blue_difference <= 103 {
if features.red_chromaticity <= 0.094 {
if features.red_chromaticity <= 0.056 {
Intensity::High
} else {
if features.green_chromaticity <= 0.764 {
if features.red_chromaticity <= 0.091 {
if features.red_chromaticity <= 0.068 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.730 {
Intensity::Low
} else {
Intensity::High
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
Intensity::High
}
} else {
if features.blue_difference <= 105 {
if features.blue_chromaticity <= 0.135 {
if features.saturation <= 213 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.059 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_difference <= 81 {
Intensity::High
} else {
if features.red_difference <= 95 {
if features.saturation <= 236 {
if features.blue_difference <= 104 {
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
if features.saturation <= 213 {
if features.blue_chromaticity <= 0.147 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.190 {
if features.blue_difference <= 107 {
Intensity::High
} else {
if features.green_chromaticity <= 0.718 {
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
if features.red_chromaticity <= 0.106 {
if features.saturation <= 239 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.043 {
if features.blue_difference <= 107 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.107 {
Intensity::High
} else {
if features.green_luminance <= 89 {
if features.green_chromaticity <= 0.765 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.695 {
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
if features.hue <= 64 {
if features.red_difference <= 118 {
if features.green_chromaticity <= 0.414 {
if features.blue_difference <= 104 {
if features.red_chromaticity <= 0.321 {
if features.red_luminance <= 150 {
if features.luminance <= 162 {
if features.green_chromaticity <= 0.413 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.266 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.307 {
if features.hue <= 55 {
Intensity::High
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
} else {
if features.intensity <= 133 {
if features.green_luminance <= 161 {
Intensity::High
} else {
if features.blue_luminance <= 100 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.327 {
if features.blue_difference <= 103 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.value <= 190 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_luminance <= 130 {
if features.intensity <= 130 {
if features.red_chromaticity <= 0.309 {
if features.red_chromaticity <= 0.309 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 101 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.323 {
if features.red_difference <= 110 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.325 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.318 {
if features.blue_chromaticity <= 0.292 {
if features.blue_luminance <= 118 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 133 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.intensity <= 139 {
if features.blue_chromaticity <= 0.267 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.luminance <= 155 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.value <= 122 {
if features.hue <= 62 {
if features.hue <= 61 {
if features.luminance <= 48 {
if features.luminance <= 42 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.461 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.luminance <= 62 {
if features.saturation <= 193 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.208 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.blue_difference <= 104 {
if features.blue_chromaticity <= 0.201 {
if features.luminance <= 72 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.642 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.178 {
if features.blue_difference <= 105 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_difference <= 98 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
} else {
if features.green_chromaticity <= 0.422 {
if features.blue_difference <= 98 {
if features.red_chromaticity <= 0.340 {
if features.red_difference <= 110 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.blue_difference <= 101 {
if features.red_chromaticity <= 0.324 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 126 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.blue_difference <= 108 {
if features.red_luminance <= 158 {
if features.blue_difference <= 105 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.hue <= 49 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.248 {
if features.green_luminance <= 137 {
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
}
}
}
}
} else {
if features.blue_difference <= 106 {
if features.luminance <= 146 {
if features.value <= 57 {
Intensity::Low
} else {
if features.blue_difference <= 103 {
if features.green_chromaticity <= 0.411 {
if features.red_luminance <= 125 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.luminance <= 141 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.338 {
if features.blue_chromaticity <= 0.219 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.428 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.blue_difference <= 99 {
if features.blue_luminance <= 96 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.red_difference <= 120 {
if features.blue_chromaticity <= 0.122 {
if features.red_luminance <= 37 {
if features.green_chromaticity <= 0.628 {
if features.intensity <= 28 {
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
if features.luminance <= 79 {
if features.blue_chromaticity <= 0.187 {
if features.value <= 68 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.187 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.green_luminance <= 123 {
if features.blue_luminance <= 74 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.330 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.red_chromaticity <= 0.343 {
if features.red_chromaticity <= 0.339 {
if features.red_chromaticity <= 0.335 {
if features.saturation <= 140 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 13 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.340 {
if features.green_luminance <= 108 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 170 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_luminance <= 23 {
if features.blue_chromaticity <= 0.126 {
Intensity::Low
} else {
if features.saturation <= 175 {
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
} else {
if features.value <= 131 {
if features.red_difference <= 87 {
if features.blue_difference <= 104 {
Intensity::High
} else {
if features.green_chromaticity <= 0.675 {
if features.red_difference <= 85 {
if features.blue_chromaticity <= 0.232 {
if features.blue_luminance <= 44 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.hue <= 65 {
Intensity::High
} else {
if features.green_luminance <= 128 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.095 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.681 {
if features.value <= 116 {
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
if features.blue_difference <= 105 {
if features.green_chromaticity <= 0.613 {
Intensity::High
} else {
if features.blue_luminance <= 34 {
Intensity::High
} else {
if features.red_difference <= 88 {
if features.green_luminance <= 120 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 112 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
} else {
if features.green_chromaticity <= 0.638 {
if features.red_difference <= 96 {
if features.green_chromaticity <= 0.636 {
if features.hue <= 65 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.129 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.261 {
if features.saturation <= 191 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 69 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.red_difference <= 89 {
if features.blue_chromaticity <= 0.228 {
if features.blue_chromaticity <= 0.215 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.red_difference <= 92 {
if features.value <= 101 {
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
}
} else {
if features.value <= 134 {
if features.blue_chromaticity <= 0.257 {
if features.intensity <= 61 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 145 {
if features.green_chromaticity <= 0.497 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.525 {
if features.blue_difference <= 108 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.266 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.196 {
if features.intensity <= 79 {
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
if features.red_difference <= 92 {
if features.green_luminance <= 201 {
if features.green_luminance <= 147 {
if features.blue_chromaticity <= 0.260 {
if features.blue_difference <= 105 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.260 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.green_luminance <= 181 {
if features.red_difference <= 84 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.luminance <= 144 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.477 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.228 {
if features.red_difference <= 94 {
if features.blue_chromaticity <= 0.269 {
if features.blue_chromaticity <= 0.268 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.497 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.hue <= 65 {
Intensity::High
} else {
if features.red_luminance <= 64 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_luminance <= 182 {
Intensity::High
} else {
if features.green_chromaticity <= 0.455 {
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
} else {
if features.value <= 134 {
if features.green_luminance <= 130 {
if features.blue_difference <= 104 {
if features.green_luminance <= 122 {
if features.green_chromaticity <= 0.747 {
if features.green_luminance <= 120 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 72 {
Intensity::High
} else {
Intensity::High
}
}
} else {
Intensity::High
}
} else {
if features.blue_difference <= 107 {
if features.red_chromaticity <= 0.133 {
if features.hue <= 68 {
if features.value <= 129 {
if features.red_difference <= 78 {
Intensity::High
} else {
if features.value <= 122 {
if features.saturation <= 202 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.655 {
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
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.124 {
if features.green_luminance <= 128 {
if features.green_luminance <= 125 {
if features.green_chromaticity <= 0.729 {
if features.red_difference <= 80 {
if features.red_chromaticity <= 0.077 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.228 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.731 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.638 {
if features.red_difference <= 82 {
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
if features.green_chromaticity <= 0.632 {
if features.green_chromaticity <= 0.606 {
if features.luminance <= 96 {
if features.blue_difference <= 108 {
if features.red_chromaticity <= 0.149 {
Intensity::Low
} else {
Intensity::Low
}
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
if features.red_luminance <= 25 {
if features.red_chromaticity <= 0.138 {
if features.saturation <= 202 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.intensity <= 62 {
Intensity::High
} else {
if features.red_difference <= 85 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_luminance <= 108 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.green_chromaticity <= 0.561 {
if features.blue_chromaticity <= 0.273 {
if features.red_chromaticity <= 0.180 {
if features.blue_luminance <= 64 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 99 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.saturation <= 187 {
Intensity::High
} else {
if features.luminance <= 89 {
if features.red_chromaticity <= 0.079 {
Intensity::High
} else {
if features.red_chromaticity <= 0.087 {
if features.red_chromaticity <= 0.082 {
Intensity::Low
} else {
if features.red_luminance <= 16 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
}
} else {
if features.saturation <= 207 {
if features.red_luminance <= 30 {
Intensity::High
} else {
if features.red_chromaticity <= 0.143 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.152 {
if features.red_chromaticity <= 0.149 {
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
if features.blue_luminance <= 48 {
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
if features.value <= 191 {
if features.green_luminance <= 137 {
if features.blue_luminance <= 67 {
if features.blue_chromaticity <= 0.261 {
if features.blue_chromaticity <= 0.261 {
if features.red_difference <= 78 {
if features.red_difference <= 76 {
Intensity::High
} else {
if features.green_chromaticity <= 0.679 {
if features.saturation <= 211 {
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
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.value <= 136 {
if features.red_difference <= 90 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 83 {
Intensity::Low
} else {
if features.red_luminance <= 50 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.red_difference <= 76 {
if features.green_chromaticity <= 0.585 {
if features.green_chromaticity <= 0.563 {
Intensity::Low
} else {
if features.intensity <= 91 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
Intensity::High
}
} else {
if features.hue <= 68 {
if features.value <= 141 {
if features.luminance <= 106 {
Intensity::High
} else {
if features.green_chromaticity <= 0.528 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.blue_difference <= 103 {
if features.blue_chromaticity <= 0.260 {
Intensity::High
} else {
if features.saturation <= 178 {
if features.red_luminance <= 51 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.194 {
if features.red_chromaticity <= 0.194 {
if features.blue_chromaticity <= 0.272 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.value <= 153 {
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
if features.green_luminance <= 163 {
if features.value <= 140 {
if features.red_luminance <= 44 {
if features.blue_luminance <= 64 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.281 {
if features.blue_chromaticity <= 0.280 {
Intensity::High
} else {
if features.red_luminance <= 42 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
}
} else {
if features.value <= 164 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
}
} else {
if features.red_difference <= 86 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
} else {
if features.red_difference <= 123 {
if features.blue_difference <= 106 {
if features.green_chromaticity <= 0.442 {
if features.value <= 121 {
if features.green_chromaticity <= 0.426 {
if features.green_chromaticity <= 0.414 {
if features.green_chromaticity <= 0.412 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.360 {
if features.blue_difference <= 103 {
Intensity::High
} else {
if features.hue <= 41 {
if features.hue <= 40 {
Intensity::High
} else {
if features.red_luminance <= 88 {
Intensity::Low
} else {
if features.red_luminance <= 92 {
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
if features.blue_chromaticity <= 0.206 {
if features.blue_chromaticity <= 0.200 {
if features.intensity <= 86 {
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
}
} else {
if features.value <= 150 {
if features.luminance <= 119 {
if features.intensity <= 101 {
if features.intensity <= 94 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.357 {
Intensity::High
} else {
if features.red_chromaticity <= 0.357 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.353 {
if features.green_chromaticity <= 0.414 {
if features.green_luminance <= 130 {
if features.blue_difference <= 104 {
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
Intensity::Low
}
}
} else {
if features.blue_difference <= 97 {
if features.green_luminance <= 143 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.347 {
if features.red_chromaticity <= 0.347 {
if features.blue_luminance <= 81 {
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
} else {
if features.red_chromaticity <= 0.350 {
if features.value <= 152 {
Intensity::Low
} else {
if features.value <= 153 {
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
if features.red_chromaticity <= 0.362 {
Intensity::High
} else {
if features.red_chromaticity <= 0.362 {
Intensity::Low
} else {
if features.blue_difference <= 96 {
Intensity::High
} else {
if features.green_luminance <= 82 {
Intensity::High
} else {
if features.red_chromaticity <= 0.364 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.120 {
Intensity::High
} else {
if features.hue <= 39 {
Intensity::High
} else {
if features.hue <= 40 {
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
if features.red_luminance <= 66 {
if features.blue_luminance <= 23 {
if features.saturation <= 180 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.512 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.hue <= 41 {
Intensity::High
} else {
if features.value <= 82 {
if features.green_luminance <= 69 {
Intensity::High
} else {
if features.red_luminance <= 61 {
if features.red_luminance <= 59 {
if features.intensity <= 55 {
if features.saturation <= 169 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.347 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.346 {
if features.saturation <= 131 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 58 {
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
if features.red_chromaticity <= 0.343 {
if features.red_chromaticity <= 0.342 {
if features.green_luminance <= 100 {
Intensity::Low
} else {
if features.intensity <= 93 {
Intensity::High
} else {
if features.saturation <= 92 {
Intensity::Low
} else {
if features.red_luminance <= 100 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
} else {
Intensity::High
}
} else {
if features.green_luminance <= 118 {
if features.hue <= 42 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.343 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.luminance <= 108 {
Intensity::High
} else {
if features.red_chromaticity <= 0.346 {
Intensity::Low
} else {
if features.blue_luminance <= 78 {
if features.red_chromaticity <= 0.346 {
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
} else {
if features.red_difference <= 125 {
if features.blue_difference <= 104 {
if features.green_chromaticity <= 0.431 {
if features.red_chromaticity <= 0.360 {
Intensity::Low
} else {
if features.blue_luminance <= 64 {
if features.red_chromaticity <= 0.364 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 66 {
if features.red_chromaticity <= 0.361 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.228 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
} else {
if features.green_chromaticity <= 0.466 {
if features.red_chromaticity <= 0.369 {
if features.blue_difference <= 101 {
if features.blue_difference <= 100 {
if features.blue_luminance <= 51 {
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
} else {
Intensity::High
}
} else {
if features.hue <= 38 {
if features.blue_luminance <= 20 {
Intensity::High
} else {
if features.blue_difference <= 103 {
if features.saturation <= 176 {
if features.luminance <= 74 {
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
Intensity::High
}
}
}
} else {
if features.red_luminance <= 83 {
if features.intensity <= 66 {
if features.red_difference <= 124 {
if features.green_chromaticity <= 0.481 {
if features.blue_difference <= 107 {
if features.red_chromaticity <= 0.367 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.356 {
if features.intensity <= 59 {
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
if features.blue_difference <= 109 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.367 {
Intensity::Low
} else {
if features.intensity <= 49 {
if features.green_luminance <= 66 {
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
if features.blue_chromaticity <= 0.223 {
if features.green_luminance <= 92 {
if features.green_chromaticity <= 0.424 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 78 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.422 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.419 {
Intensity::Low
} else {
if features.blue_luminance <= 50 {
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
if features.blue_difference <= 88 {
if features.blue_luminance <= 53 {
Intensity::High
} else {
if features.green_chromaticity <= 0.444 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.364 {
if features.red_chromaticity <= 0.364 {
if features.blue_luminance <= 68 {
if features.green_luminance <= 103 {
if features.value <= 102 {
if features.intensity <= 80 {
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
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.hue <= 34 {
if features.hue <= 23 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.384 {
if features.red_difference <= 127 {
Intensity::Low
} else {
if features.blue_luminance <= 54 {
Intensity::Low
} else {
if features.saturation <= 118 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.blue_chromaticity <= 0.186 {
if features.red_chromaticity <= 0.385 {
Intensity::High
} else {
if features.red_chromaticity <= 0.387 {
if features.red_chromaticity <= 0.387 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.value <= 81 {
if features.blue_chromaticity <= 0.157 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 61 {
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
}
}
}
}
}
}
} else {
if features.blue_difference <= 112 {
if features.green_chromaticity <= 0.394 {
if features.green_chromaticity <= 0.390 {
if features.green_chromaticity <= 0.385 {
if features.green_chromaticity <= 0.382 {
if features.green_chromaticity <= 0.380 {
if features.green_chromaticity <= 0.378 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.378 {
if features.red_difference <= 126 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.282 {
if features.luminance <= 143 {
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
if features.red_chromaticity <= 0.328 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.290 {
if features.green_chromaticity <= 0.380 {
if features.hue <= 32 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.382 {
if features.green_chromaticity <= 0.381 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.381 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.382 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.329 {
if features.blue_difference <= 111 {
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
if features.red_luminance <= 154 {
if features.red_chromaticity <= 0.340 {
if features.blue_chromaticity <= 0.279 {
if features.blue_difference <= 111 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 110 {
if features.green_chromaticity <= 0.383 {
if features.green_chromaticity <= 0.383 {
if features.blue_luminance <= 104 {
Intensity::Low
} else {
if features.intensity <= 127 {
Intensity::High
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
} else {
if features.blue_chromaticity <= 0.289 {
if features.blue_chromaticity <= 0.288 {
if features.green_chromaticity <= 0.384 {
if features.green_luminance <= 164 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.332 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.384 {
if features.green_chromaticity <= 0.384 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.323 {
if features.red_chromaticity <= 0.322 {
if features.red_chromaticity <= 0.322 {
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
if features.blue_chromaticity <= 0.295 {
if features.red_luminance <= 149 {
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
Intensity::Low
}
} else {
if features.saturation <= 59 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.384 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.luminance <= 173 {
if features.saturation <= 72 {
if features.green_luminance <= 160 {
if features.green_luminance <= 156 {
if features.blue_luminance <= 98 {
Intensity::High
} else {
if features.green_chromaticity <= 0.390 {
if features.blue_chromaticity <= 0.281 {
if features.red_luminance <= 128 {
if features.red_luminance <= 120 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.intensity <= 132 {
if features.green_chromaticity <= 0.388 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.388 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.blue_chromaticity <= 0.286 {
if features.saturation <= 71 {
if features.value <= 151 {
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
if features.saturation <= 65 {
if features.intensity <= 135 {
Intensity::High
} else {
if features.green_chromaticity <= 0.386 {
if features.blue_chromaticity <= 0.289 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_difference <= 117 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.324 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_luminance <= 117 {
if features.red_chromaticity <= 0.330 {
if features.red_luminance <= 133 {
if features.red_chromaticity <= 0.327 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.284 {
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
}
}
} else {
if features.saturation <= 62 {
if features.red_chromaticity <= 0.314 {
if features.red_chromaticity <= 0.313 {
Intensity::Low
} else {
if features.luminance <= 167 {
if features.red_chromaticity <= 0.314 {
if features.green_chromaticity <= 0.390 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 138 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.388 {
Intensity::Low
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
if features.green_chromaticity <= 0.385 {
if features.luminance <= 165 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.294 {
if features.red_chromaticity <= 0.319 {
if features.blue_chromaticity <= 0.293 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.292 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.315 {
if features.blue_chromaticity <= 0.298 {
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
}
}
} else {
if features.red_chromaticity <= 0.323 {
if features.blue_chromaticity <= 0.293 {
if features.value <= 181 {
if features.red_chromaticity <= 0.318 {
Intensity::Low
} else {
if features.red_luminance <= 133 {
Intensity::Low
} else {
Intensity::Low
}
}
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
} else {
if features.red_chromaticity <= 0.330 {
if features.blue_chromaticity <= 0.289 {
if features.blue_chromaticity <= 0.289 {
if features.green_chromaticity <= 0.388 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 135 {
Intensity::High
} else {
if features.red_luminance <= 144 {
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
if features.saturation <= 75 {
if features.luminance <= 114 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.value <= 195 {
if features.red_chromaticity <= 0.314 {
if features.blue_chromaticity <= 0.300 {
if features.luminance <= 175 {
if features.green_chromaticity <= 0.388 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.312 {
Intensity::Low
} else {
if features.luminance <= 177 {
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
if features.blue_chromaticity <= 0.299 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.385 {
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
Intensity::Low
}
}
}
} else {
if features.red_luminance <= 110 {
if features.blue_luminance <= 90 {
Intensity::Low
} else {
if features.green_luminance <= 131 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 157 {
if features.luminance <= 149 {
if features.blue_chromaticity <= 0.285 {
if features.green_chromaticity <= 0.393 {
if features.red_chromaticity <= 0.330 {
if features.red_luminance <= 127 {
if features.red_luminance <= 122 {
if features.red_chromaticity <= 0.330 {
if features.red_chromaticity <= 0.324 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.330 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.luminance <= 135 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.283 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.intensity <= 133 {
if features.saturation <= 70 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 117 {
if features.green_chromaticity <= 0.391 {
if features.red_chromaticity <= 0.334 {
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
if features.red_chromaticity <= 0.322 {
if features.blue_difference <= 111 {
if features.blue_chromaticity <= 0.284 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.luminance <= 136 {
if features.intensity <= 120 {
if features.red_chromaticity <= 0.324 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_difference <= 118 {
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
if features.green_chromaticity <= 0.393 {
if features.green_chromaticity <= 0.392 {
if features.green_chromaticity <= 0.391 {
Intensity::High
} else {
if features.green_chromaticity <= 0.392 {
if features.red_chromaticity <= 0.322 {
if features.red_difference <= 116 {
Intensity::Low
} else {
Intensity::High
}
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
}
} else {
Intensity::High
}
} else {
if features.red_luminance <= 121 {
Intensity::High
} else {
if features.blue_luminance <= 118 {
if features.blue_chromaticity <= 0.289 {
if features.blue_chromaticity <= 0.288 {
if features.saturation <= 69 {
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
if features.saturation <= 68 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.green_luminance <= 187 {
if features.luminance <= 166 {
if features.green_chromaticity <= 0.391 {
if features.red_luminance <= 142 {
if features.red_chromaticity <= 0.318 {
if features.blue_chromaticity <= 0.296 {
if features.red_chromaticity <= 0.316 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.320 {
if features.hue <= 51 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.322 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_luminance <= 132 {
if features.luminance <= 161 {
Intensity::High
} else {
Intensity::High
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
if features.blue_chromaticity <= 0.293 {
if features.blue_chromaticity <= 0.292 {
if features.blue_chromaticity <= 0.292 {
if features.green_chromaticity <= 0.393 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.292 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 134 {
if features.red_difference <= 114 {
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
if features.green_chromaticity <= 0.392 {
if features.luminance <= 159 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 142 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 127 {
if features.red_luminance <= 133 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 136 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.green_chromaticity <= 0.394 {
if features.green_chromaticity <= 0.390 {
Intensity::High
} else {
if features.red_chromaticity <= 0.310 {
if features.blue_chromaticity <= 0.297 {
Intensity::Low
} else {
if features.luminance <= 168 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.value <= 183 {
if features.intensity <= 155 {
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
if features.green_chromaticity <= 0.394 {
Intensity::High
} else {
if features.green_chromaticity <= 0.394 {
if features.hue <= 54 {
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
if features.red_chromaticity <= 0.312 {
if features.red_luminance <= 148 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.391 {
if features.value <= 190 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.393 {
if features.green_chromaticity <= 0.392 {
if features.green_chromaticity <= 0.391 {
Intensity::High
} else {
if features.green_chromaticity <= 0.391 {
if features.value <= 193 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.313 {
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
Intensity::Low
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
if features.red_chromaticity <= 0.197 {
if features.value <= 140 {
if features.red_difference <= 102 {
if features.blue_luminance <= 69 {
if features.red_chromaticity <= 0.159 {
if features.blue_chromaticity <= 0.288 {
if features.hue <= 64 {
if features.red_difference <= 101 {
if features.blue_luminance <= 29 {
if features.green_chromaticity <= 0.641 {
Intensity::Low
} else {
if features.luminance <= 51 {
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
if features.red_chromaticity <= 0.122 {
if features.red_luminance <= 0 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 206 {
if features.saturation <= 201 {
if features.blue_chromaticity <= 0.219 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 100 {
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
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.257 {
if features.red_luminance <= 27 {
if features.green_chromaticity <= 0.608 {
if features.green_luminance <= 93 {
if features.blue_chromaticity <= 0.222 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.232 {
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
} else {
if features.blue_luminance <= 43 {
if features.hue <= 63 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.254 {
if features.blue_chromaticity <= 0.249 {
Intensity::High
} else {
if features.saturation <= 171 {
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
if features.red_chromaticity <= 0.196 {
if features.green_luminance <= 104 {
if features.red_difference <= 98 {
if features.saturation <= 175 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.527 {
if features.green_luminance <= 125 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.259 {
if features.green_luminance <= 109 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.162 {
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
if features.blue_chromaticity <= 0.288 {
Intensity::High
} else {
if features.red_chromaticity <= 0.159 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.292 {
if features.blue_chromaticity <= 0.290 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.290 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.291 {
if features.intensity <= 85 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 78 {
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
}
} else {
if features.luminance <= 53 {
if features.blue_difference <= 111 {
if features.green_chromaticity <= 0.692 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_difference <= 108 {
if features.green_chromaticity <= 0.638 {
if features.blue_chromaticity <= 0.190 {
Intensity::High
} else {
if features.saturation <= 184 {
if features.green_chromaticity <= 0.606 {
Intensity::High
} else {
if features.red_chromaticity <= 0.184 {
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
if features.hue <= 61 {
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
if features.blue_chromaticity <= 0.229 {
Intensity::High
} else {
if features.red_difference <= 103 {
Intensity::High
} else {
if features.saturation <= 168 {
Intensity::Low
} else {
if features.intensity <= 48 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.576 {
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
Intensity::High
}
} else {
if features.red_difference <= 119 {
if features.intensity <= 147 {
if features.red_difference <= 100 {
if features.value <= 141 {
if features.hue <= 65 {
if features.green_chromaticity <= 0.468 {
Intensity::High
} else {
if features.green_chromaticity <= 0.473 {
if features.saturation <= 122 {
if features.green_chromaticity <= 0.470 {
if features.green_chromaticity <= 0.469 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_luminance <= 72 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.485 {
if features.red_chromaticity <= 0.243 {
Intensity::High
} else {
if features.red_chromaticity <= 0.244 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.276 {
if features.green_chromaticity <= 0.512 {
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
}
}
}
} else {
if features.red_luminance <= 51 {
if features.blue_chromaticity <= 0.264 {
Intensity::High
} else {
if features.red_chromaticity <= 0.204 {
if features.green_chromaticity <= 0.524 {
if features.green_luminance <= 128 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.206 {
Intensity::High
} else {
if features.red_chromaticity <= 0.214 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.saturation <= 152 {
if features.red_luminance <= 69 {
if features.blue_chromaticity <= 0.290 {
if features.blue_chromaticity <= 0.289 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.saturation <= 122 {
if features.blue_luminance <= 86 {
Intensity::Low
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
}
} else {
if features.green_luminance <= 149 {
if features.red_chromaticity <= 0.230 {
Intensity::High
} else {
if features.red_chromaticity <= 0.250 {
if features.green_chromaticity <= 0.461 {
Intensity::Low
} else {
if features.red_difference <= 95 {
if features.green_chromaticity <= 0.475 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.246 {
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
if features.blue_chromaticity <= 0.288 {
if features.green_chromaticity <= 0.470 {
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
if features.red_difference <= 115 {
if features.green_chromaticity <= 0.407 {
if features.green_chromaticity <= 0.399 {
if features.red_luminance <= 130 {
if features.luminance <= 148 {
if features.red_luminance <= 123 {
if features.green_chromaticity <= 0.398 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 115 {
Intensity::High
} else {
Intensity::High
}
}
} else {
Intensity::High
}
} else {
if features.intensity <= 140 {
if features.saturation <= 69 {
if features.blue_luminance <= 122 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.288 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 137 {
if features.red_chromaticity <= 0.310 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 175 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.red_chromaticity <= 0.308 {
if features.blue_luminance <= 118 {
if features.blue_chromaticity <= 0.289 {
if features.blue_chromaticity <= 0.287 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.302 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.saturation <= 69 {
if features.green_luminance <= 176 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.290 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.400 {
if features.red_luminance <= 132 {
Intensity::High
} else {
if features.red_difference <= 112 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.315 {
if features.value <= 171 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 155 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.red_chromaticity <= 0.213 {
if features.blue_luminance <= 31 {
if features.red_chromaticity <= 0.209 {
if features.blue_chromaticity <= 0.222 {
if features.value <= 72 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.210 {
Intensity::Low
} else {
if features.green_luminance <= 73 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.red_difference <= 101 {
if features.luminance <= 78 {
if features.saturation <= 160 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.563 {
if features.red_luminance <= 31 {
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
if features.blue_difference <= 111 {
if features.blue_chromaticity <= 0.275 {
if features.intensity <= 66 {
if features.intensity <= 29 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.244 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.414 {
if features.value <= 182 {
Intensity::High
} else {
Intensity::Low
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
if features.red_luminance <= 53 {
if features.value <= 62 {
if features.red_luminance <= 25 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.hue <= 63 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.saturation <= 92 {
if features.value <= 127 {
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
}
}
}
}
} else {
if features.red_difference <= 118 {
if features.value <= 122 {
if features.red_luminance <= 60 {
if features.blue_chromaticity <= 0.140 {
if features.red_difference <= 116 {
Intensity::High
} else {
if features.green_chromaticity <= 0.573 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_difference <= 117 {
if features.red_chromaticity <= 0.280 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.307 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.blue_chromaticity <= 0.240 {
if features.red_luminance <= 61 {
Intensity::High
} else {
if features.saturation <= 120 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.431 {
if features.red_difference <= 117 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.443 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_luminance <= 112 {
if features.red_chromaticity <= 0.311 {
if features.green_chromaticity <= 0.415 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.283 {
if features.intensity <= 104 {
Intensity::High
} else {
Intensity::High
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
if features.green_luminance <= 158 {
if features.green_chromaticity <= 0.395 {
Intensity::High
} else {
if features.green_chromaticity <= 0.397 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.saturation <= 70 {
if features.green_chromaticity <= 0.395 {
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
if features.blue_chromaticity <= 0.240 {
if features.red_chromaticity <= 0.305 {
if features.red_chromaticity <= 0.300 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.303 {
Intensity::High
} else {
if features.saturation <= 168 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.228 {
if features.blue_chromaticity <= 0.220 {
if features.green_chromaticity <= 0.471 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.224 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 119 {
if features.intensity <= 64 {
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
if features.luminance <= 104 {
if features.intensity <= 75 {
if features.saturation <= 112 {
if features.green_chromaticity <= 0.431 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.244 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.blue_luminance <= 72 {
if features.blue_luminance <= 71 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 113 {
if features.luminance <= 119 {
if features.green_luminance <= 124 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.396 {
if features.green_chromaticity <= 0.395 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 95 {
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
if features.red_luminance <= 152 {
if features.value <= 190 {
if features.saturation <= 65 {
if features.saturation <= 63 {
if features.red_luminance <= 146 {
if features.blue_chromaticity <= 0.298 {
if features.red_luminance <= 143 {
Intensity::Low
} else {
if features.value <= 186 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 144 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.306 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_luminance <= 143 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.304 {
if features.value <= 180 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.301 {
if features.intensity <= 152 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.301 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.value <= 179 {
if features.value <= 176 {
Intensity::Low
} else {
if features.saturation <= 64 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.luminance <= 168 {
if features.green_chromaticity <= 0.395 {
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
}
}
}
} else {
if features.blue_luminance <= 132 {
if features.saturation <= 66 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.295 {
if features.red_luminance <= 136 {
if features.green_chromaticity <= 0.403 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 140 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.296 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.404 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
} else {
if features.blue_luminance <= 138 {
if features.green_chromaticity <= 0.409 {
if features.green_chromaticity <= 0.406 {
if features.blue_chromaticity <= 0.293 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.295 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.301 {
if features.red_luminance <= 130 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.296 {
if features.red_difference <= 108 {
Intensity::High
} else {
if features.intensity <= 158 {
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
} else {
Intensity::High
}
} else {
if features.red_luminance <= 154 {
if features.blue_chromaticity <= 0.298 {
if features.blue_luminance <= 148 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.saturation <= 60 {
if features.blue_chromaticity <= 0.303 {
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
if features.red_difference <= 122 {
if features.blue_luminance <= 88 {
if features.green_chromaticity <= 0.437 {
if features.red_difference <= 121 {
if features.red_chromaticity <= 0.327 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.327 {
Intensity::High
} else {
if features.red_chromaticity <= 0.332 {
if features.saturation <= 91 {
if features.green_luminance <= 120 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.410 {
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
if features.red_chromaticity <= 0.336 {
if features.saturation <= 88 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::Low
}
}
} else {
if features.luminance <= 46 {
Intensity::Low
} else {
if features.red_luminance <= 37 {
Intensity::High
} else {
if features.red_chromaticity <= 0.338 {
if features.green_chromaticity <= 0.438 {
Intensity::High
} else {
if features.green_luminance <= 67 {
if features.green_chromaticity <= 0.490 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 154 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.341 {
Intensity::High
} else {
if features.value <= 74 {
if features.red_luminance <= 49 {
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
if features.red_chromaticity <= 0.332 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_difference <= 123 {
if features.luminance <= 87 {
if features.red_chromaticity <= 0.349 {
if features.red_chromaticity <= 0.347 {
if features.blue_chromaticity <= 0.221 {
if features.saturation <= 151 {
Intensity::Low
} else {
if features.green_luminance <= 61 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 65 {
if features.saturation <= 115 {
Intensity::High
} else {
if features.red_chromaticity <= 0.342 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.344 {
if features.saturation <= 101 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.344 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_chromaticity <= 0.485 {
Intensity::High
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
if features.hue <= 41 {
if features.red_difference <= 125 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.463 {
if features.value <= 84 {
if features.value <= 81 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 24 {
if features.saturation <= 163 {
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
} else {
if features.blue_difference <= 114 {
if features.red_chromaticity <= 0.310 {
if features.red_chromaticity <= 0.202 {
if features.red_chromaticity <= 0.181 {
if features.blue_luminance <= 76 {
if features.blue_chromaticity <= 0.301 {
if features.red_difference <= 101 {
if features.luminance <= 94 {
if features.red_chromaticity <= 0.159 {
if features.green_chromaticity <= 0.548 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 50 {
Intensity::High
} else {
if features.red_chromaticity <= 0.160 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.174 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_chromaticity <= 0.292 {
if features.red_luminance <= 37 {
if features.red_chromaticity <= 0.154 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.171 {
if features.red_difference <= 87 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_difference <= 90 {
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
if features.saturation <= 194 {
if features.red_chromaticity <= 0.151 {
Intensity::High
} else {
if features.red_difference <= 107 {
if features.blue_difference <= 113 {
if features.red_chromaticity <= 0.177 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_difference <= 103 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_luminance <= 60 {
Intensity::Low
} else {
if features.value <= 62 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.intensity <= 39 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.626 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.145 {
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
Intensity::High
}
} else {
if features.intensity <= 55 {
if features.green_luminance <= 65 {
if features.saturation <= 173 {
Intensity::Low
} else {
if features.blue_luminance <= 17 {
Intensity::Low
} else {
if features.red_luminance <= 19 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.242 {
if features.green_luminance <= 80 {
Intensity::High
} else {
if features.blue_luminance <= 32 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.568 {
if features.red_chromaticity <= 0.194 {
Intensity::High
} else {
if features.green_chromaticity <= 0.554 {
if features.luminance <= 66 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 81 {
Intensity::Low
} else {
if features.luminance <= 63 {
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
if features.blue_luminance <= 82 {
if features.red_difference <= 97 {
if features.red_difference <= 89 {
if features.luminance <= 107 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.saturation <= 154 {
if features.green_chromaticity <= 0.505 {
if features.value <= 136 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 51 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.537 {
if features.red_chromaticity <= 0.201 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.201 {
if features.green_luminance <= 105 {
if features.red_chromaticity <= 0.185 {
Intensity::High
} else {
if features.red_chromaticity <= 0.189 {
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
if features.red_chromaticity <= 0.187 {
if features.intensity <= 95 {
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
if features.saturation <= 66 {
if features.green_chromaticity <= 0.392 {
if features.blue_chromaticity <= 0.300 {
if features.saturation <= 59 {
Intensity::High
} else {
if features.red_chromaticity <= 0.309 {
Intensity::Low
} else {
if features.intensity <= 157 {
if features.intensity <= 146 {
Intensity::Low
} else {
if features.red_difference <= 112 {
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
if features.green_chromaticity <= 0.390 {
if features.red_luminance <= 152 {
if features.blue_chromaticity <= 0.304 {
if features.luminance <= 167 {
Intensity::Low
} else {
if features.intensity <= 158 {
if features.green_luminance <= 184 {
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
} else {
if features.red_chromaticity <= 0.306 {
if features.red_chromaticity <= 0.305 {
if features.blue_difference <= 113 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 57 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.luminance <= 174 {
if features.green_chromaticity <= 0.391 {
if features.red_difference <= 112 {
if features.saturation <= 57 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.392 {
if features.blue_chromaticity <= 0.301 {
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
if features.blue_luminance <= 150 {
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
if features.blue_chromaticity <= 0.303 {
if features.value <= 190 {
if features.saturation <= 64 {
if features.hue <= 60 {
if features.red_chromaticity <= 0.309 {
if features.red_luminance <= 126 {
if features.value <= 160 {
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
if features.red_chromaticity <= 0.310 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.296 {
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
if features.blue_luminance <= 129 {
if features.blue_chromaticity <= 0.297 {
if features.red_luminance <= 132 {
if features.green_luminance <= 165 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.397 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.300 {
if features.green_chromaticity <= 0.403 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.298 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.400 {
Intensity::High
} else {
if features.red_chromaticity <= 0.298 {
Intensity::Low
} else {
if features.blue_luminance <= 137 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.red_difference <= 108 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.300 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.395 {
if features.luminance <= 170 {
Intensity::High
} else {
if features.red_difference <= 108 {
Intensity::Low
} else {
if features.intensity <= 161 {
if features.intensity <= 159 {
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
}
}
}
} else {
if features.hue <= 65 {
if features.red_difference <= 117 {
if features.blue_luminance <= 49 {
if features.saturation <= 181 {
if features.red_difference <= 114 {
if features.red_chromaticity <= 0.217 {
if features.value <= 76 {
if features.red_chromaticity <= 0.215 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 31 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.487 {
if features.blue_chromaticity <= 0.257 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.value <= 61 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.blue_difference <= 113 {
if features.green_chromaticity <= 0.465 {
if features.luminance <= 72 {
Intensity::Low
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
if features.blue_chromaticity <= 0.251 {
if features.blue_chromaticity <= 0.233 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 86 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
} else {
if features.blue_chromaticity <= 0.145 {
if features.red_chromaticity <= 0.219 {
Intensity::High
} else {
if features.red_difference <= 116 {
if features.green_chromaticity <= 0.654 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.155 {
Intensity::High
} else {
if features.red_chromaticity <= 0.223 {
if features.intensity <= 30 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.577 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
}
} else {
if features.blue_difference <= 113 {
if features.value <= 179 {
if features.red_luminance <= 79 {
if features.green_chromaticity <= 0.460 {
if features.red_chromaticity <= 0.296 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.236 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.saturation <= 72 {
if features.green_chromaticity <= 0.403 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.274 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.green_luminance <= 188 {
if features.blue_chromaticity <= 0.301 {
if features.blue_chromaticity <= 0.300 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 147 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
}
} else {
if features.hue <= 54 {
if features.red_chromaticity <= 0.300 {
Intensity::High
} else {
if features.saturation <= 80 {
if features.green_chromaticity <= 0.410 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.433 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_luminance <= 112 {
if features.red_difference <= 107 {
if features.green_chromaticity <= 0.448 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 85 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.303 {
if features.red_chromaticity <= 0.285 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.hue <= 64 {
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
if features.value <= 58 {
if features.blue_chromaticity <= 0.188 {
if features.red_chromaticity <= 0.288 {
Intensity::Low
} else {
if features.red_difference <= 118 {
if features.blue_luminance <= 16 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.300 {
if features.luminance <= 44 {
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
if features.intensity <= 37 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 159 {
if features.red_chromaticity <= 0.307 {
if features.green_chromaticity <= 0.467 {
if features.value <= 76 {
if features.green_chromaticity <= 0.464 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 122 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.intensity <= 49 {
if features.red_luminance <= 43 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 50 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.252 {
if features.red_chromaticity <= 0.307 {
Intensity::High
} else {
if features.red_chromaticity <= 0.309 {
Intensity::Low
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
}
}
} else {
if features.value <= 143 {
if features.red_difference <= 99 {
if features.green_luminance <= 114 {
Intensity::High
} else {
if features.luminance <= 108 {
if features.red_chromaticity <= 0.202 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.292 {
if features.red_difference <= 96 {
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
if features.luminance <= 113 {
if features.blue_chromaticity <= 0.297 {
if features.red_chromaticity <= 0.234 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.291 {
if features.luminance <= 114 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.477 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.red_chromaticity <= 0.254 {
if features.value <= 99 {
Intensity::High
} else {
if features.blue_difference <= 113 {
if features.value <= 105 {
Intensity::Low
} else {
if features.green_luminance <= 140 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 126 {
if features.blue_chromaticity <= 0.288 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.221 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_luminance <= 136 {
Intensity::High
} else {
if features.green_chromaticity <= 0.447 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.294 {
if features.blue_chromaticity <= 0.293 {
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
if features.green_luminance <= 153 {
if features.hue <= 68 {
if features.red_difference <= 97 {
if features.red_chromaticity <= 0.229 {
if features.saturation <= 133 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.254 {
if features.saturation <= 112 {
if features.green_chromaticity <= 0.452 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 78 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.262 {
Intensity::High
} else {
if features.value <= 145 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
} else {
if features.red_difference <= 90 {
Intensity::High
} else {
if features.red_chromaticity <= 0.226 {
if features.red_chromaticity <= 0.219 {
if features.green_luminance <= 151 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.300 {
Intensity::High
} else {
if features.green_luminance <= 147 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.blue_difference <= 113 {
Intensity::High
} else {
if features.saturation <= 84 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.301 {
if features.blue_chromaticity <= 0.301 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 73 {
if features.red_luminance <= 72 {
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
}
}
}
}
} else {
if features.green_chromaticity <= 0.387 {
if features.green_chromaticity <= 0.383 {
if features.green_chromaticity <= 0.380 {
if features.green_chromaticity <= 0.375 {
if features.green_chromaticity <= 0.364 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.364 {
Intensity::Low
} else {
if features.blue_luminance <= 134 {
if features.green_luminance <= 169 {
if features.blue_chromaticity <= 0.294 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.337 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.336 {
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
if features.green_luminance <= 180 {
if features.saturation <= 51 {
if features.green_chromaticity <= 0.375 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.hue <= 44 {
Intensity::Low
} else {
if features.intensity <= 128 {
if features.green_luminance <= 141 {
Intensity::Low
} else {
if features.green_luminance <= 142 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.375 {
if features.value <= 168 {
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
}
}
}
} else {
Intensity::Low
}
}
} else {
if features.value <= 175 {
if features.red_chromaticity <= 0.317 {
Intensity::High
} else {
if features.green_chromaticity <= 0.380 {
if features.blue_chromaticity <= 0.282 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_luminance <= 123 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.331 {
if features.red_luminance <= 147 {
if features.blue_luminance <= 131 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.324 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.381 {
if features.red_chromaticity <= 0.331 {
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
if features.blue_difference <= 113 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_luminance <= 134 {
if features.red_luminance <= 115 {
if features.intensity <= 103 {
Intensity::Low
} else {
if features.saturation <= 73 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.saturation <= 56 {
Intensity::High
} else {
if features.intensity <= 134 {
if features.value <= 155 {
if features.green_chromaticity <= 0.387 {
if features.red_chromaticity <= 0.323 {
if features.red_chromaticity <= 0.323 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.value <= 148 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.387 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.323 {
if features.blue_luminance <= 128 {
if features.saturation <= 58 {
if features.red_luminance <= 137 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.384 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 149 {
if features.green_luminance <= 167 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 150 {
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
if features.red_luminance <= 148 {
if features.luminance <= 167 {
if features.value <= 181 {
if features.luminance <= 164 {
Intensity::Low
} else {
if features.intensity <= 154 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.385 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.312 {
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
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.314 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.315 {
if features.green_chromaticity <= 0.385 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.384 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.384 {
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
if features.value <= 122 {
if features.red_chromaticity <= 0.330 {
if features.red_chromaticity <= 0.330 {
if features.value <= 87 {
if features.red_chromaticity <= 0.327 {
if features.hue <= 48 {
if features.red_chromaticity <= 0.315 {
if features.green_chromaticity <= 0.478 {
if features.intensity <= 46 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.322 {
if features.green_chromaticity <= 0.507 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_luminance <= 65 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.blue_difference <= 113 {
if features.value <= 80 {
if features.red_luminance <= 54 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.red_luminance <= 54 {
Intensity::Low
} else {
if features.saturation <= 114 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
} else {
if features.red_luminance <= 54 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.hue <= 48 {
if features.blue_chromaticity <= 0.268 {
if features.luminance <= 99 {
if features.blue_luminance <= 65 {
if features.red_chromaticity <= 0.329 {
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
if features.red_chromaticity <= 0.327 {
Intensity::Low
} else {
if features.red_luminance <= 93 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.320 {
if features.red_difference <= 118 {
if features.red_chromaticity <= 0.316 {
if features.red_chromaticity <= 0.315 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 90 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.433 {
if features.hue <= 49 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.436 {
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
}
} else {
Intensity::High
}
} else {
if features.red_difference <= 124 {
if features.saturation <= 105 {
if features.green_chromaticity <= 0.389 {
if features.green_chromaticity <= 0.389 {
if features.green_chromaticity <= 0.389 {
Intensity::Low
} else {
if features.hue <= 43 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.338 {
if features.red_chromaticity <= 0.337 {
if features.value <= 91 {
if features.blue_luminance <= 56 {
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
if features.green_chromaticity <= 0.414 {
Intensity::High
} else {
if features.red_luminance <= 46 {
if features.blue_chromaticity <= 0.198 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.467 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 142 {
if features.red_luminance <= 47 {
Intensity::High
} else {
if features.red_chromaticity <= 0.340 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.458 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.green_chromaticity <= 0.427 {
if features.value <= 41 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 66 {
if features.green_luminance <= 60 {
Intensity::Low
} else {
if features.saturation <= 141 {
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
if features.green_chromaticity <= 0.395 {
if features.saturation <= 61 {
if features.green_luminance <= 162 {
if features.intensity <= 137 {
if features.blue_chromaticity <= 0.296 {
Intensity::Low
} else {
if features.blue_luminance <= 121 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.value <= 161 {
Intensity::High
} else {
if features.blue_luminance <= 123 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_luminance <= 146 {
if features.red_chromaticity <= 0.314 {
if features.blue_luminance <= 136 {
if features.red_luminance <= 142 {
if features.green_chromaticity <= 0.391 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.value <= 188 {
if features.green_chromaticity <= 0.389 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 129 {
Intensity::High
} else {
if features.red_chromaticity <= 0.316 {
if features.blue_luminance <= 130 {
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
if features.red_luminance <= 154 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.316 {
if features.blue_chromaticity <= 0.292 {
if features.blue_chromaticity <= 0.292 {
if features.blue_chromaticity <= 0.291 {
if features.value <= 155 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.291 {
Intensity::Low
} else {
if features.saturation <= 66 {
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
if features.red_chromaticity <= 0.310 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.394 {
if features.green_chromaticity <= 0.392 {
if features.blue_chromaticity <= 0.294 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 148 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.395 {
if features.intensity <= 134 {
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
if features.green_chromaticity <= 0.388 {
if features.red_luminance <= 120 {
if features.green_chromaticity <= 0.387 {
Intensity::High
} else {
if features.green_luminance <= 133 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.394 {
if features.green_chromaticity <= 0.389 {
if features.blue_luminance <= 110 {
if features.green_chromaticity <= 0.388 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 141 {
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
if features.red_luminance <= 119 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.saturation <= 70 {
if features.blue_luminance <= 105 {
if features.red_chromaticity <= 0.318 {
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
}
}
}
} else {
if features.green_chromaticity <= 0.403 {
if features.red_luminance <= 103 {
if features.intensity <= 108 {
if features.green_chromaticity <= 0.400 {
if features.green_chromaticity <= 0.396 {
Intensity::High
} else {
if features.red_luminance <= 99 {
Intensity::High
} else {
if features.green_chromaticity <= 0.397 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.401 {
Intensity::High
} else {
if features.green_luminance <= 126 {
if features.value <= 125 {
Intensity::Low
} else {
Intensity::High
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
} else {
Intensity::High
}
} else {
if features.red_luminance <= 114 {
if features.red_chromaticity <= 0.313 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.288 {
if features.red_chromaticity <= 0.313 {
if features.luminance <= 125 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 106 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.316 {
if features.red_luminance <= 113 {
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
if features.green_chromaticity <= 0.395 {
Intensity::High
} else {
if features.red_chromaticity <= 0.311 {
Intensity::High
} else {
if features.red_chromaticity <= 0.311 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.395 {
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
if features.blue_chromaticity <= 0.286 {
if features.green_chromaticity <= 0.409 {
if features.green_chromaticity <= 0.404 {
if features.green_chromaticity <= 0.404 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 87 {
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
}
}
}
} else {
if features.blue_difference <= 116 {
if features.red_chromaticity <= 0.306 {
if features.red_difference <= 103 {
if features.red_luminance <= 69 {
if features.red_chromaticity <= 0.199 {
if features.value <= 139 {
if features.red_chromaticity <= 0.167 {
if features.red_chromaticity <= 0.156 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.157 {
Intensity::Low
} else {
if features.red_difference <= 97 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.565 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.luminance <= 68 {
if features.hue <= 66 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.172 {
if features.green_chromaticity <= 0.566 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.red_difference <= 88 {
if features.blue_chromaticity <= 0.309 {
if features.value <= 132 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.198 {
if features.blue_chromaticity <= 0.278 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.199 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.green_luminance <= 141 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.hue <= 67 {
if features.green_chromaticity <= 0.463 {
if features.saturation <= 116 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.292 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.292 {
if features.red_difference <= 101 {
Intensity::Low
} else {
if features.saturation <= 153 {
if features.intensity <= 67 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.526 {
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
} else {
if features.red_difference <= 92 {
Intensity::High
} else {
if features.green_chromaticity <= 0.509 {
if features.hue <= 68 {
if features.red_difference <= 98 {
if features.saturation <= 142 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.300 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.blue_difference <= 115 {
if features.red_chromaticity <= 0.229 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_difference <= 98 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.510 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.red_difference <= 94 {
Intensity::High
} else {
if features.luminance <= 108 {
Intensity::High
} else {
if features.red_luminance <= 89 {
if features.red_chromaticity <= 0.230 {
Intensity::High
} else {
if features.green_chromaticity <= 0.434 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.304 {
if features.red_chromaticity <= 0.235 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.value <= 155 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
} else {
if features.green_chromaticity <= 0.429 {
if features.blue_chromaticity <= 0.305 {
Intensity::High
} else {
if features.saturation <= 91 {
Intensity::Low
} else {
if features.green_luminance <= 158 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.red_luminance <= 91 {
if features.intensity <= 117 {
if features.green_luminance <= 150 {
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
}
}
}
} else {
if features.red_luminance <= 110 {
if features.red_difference <= 116 {
if features.blue_chromaticity <= 0.270 {
if features.saturation <= 166 {
if features.red_chromaticity <= 0.265 {
if features.luminance <= 54 {
if features.green_chromaticity <= 0.524 {
if features.red_luminance <= 30 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.212 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_difference <= 106 {
if features.green_chromaticity <= 0.551 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.intensity <= 67 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.457 {
if features.blue_chromaticity <= 0.269 {
if features.blue_chromaticity <= 0.268 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.blue_difference <= 115 {
if features.value <= 89 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.463 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_chromaticity <= 0.183 {
if features.green_chromaticity <= 0.602 {
if features.value <= 69 {
Intensity::High
} else {
if features.saturation <= 190 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_difference <= 106 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.153 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.red_luminance <= 26 {
if features.red_chromaticity <= 0.196 {
Intensity::High
} else {
if features.green_chromaticity <= 0.580 {
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
if features.hue <= 65 {
if features.hue <= 57 {
if features.red_chromaticity <= 0.305 {
if features.blue_luminance <= 89 {
if features.green_luminance <= 97 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.305 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.414 {
if features.blue_chromaticity <= 0.297 {
if features.green_chromaticity <= 0.410 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.intensity <= 121 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.273 {
if features.blue_chromaticity <= 0.295 {
Intensity::High
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
}
}
} else {
if features.blue_chromaticity <= 0.303 {
if features.blue_chromaticity <= 0.296 {
if features.red_chromaticity <= 0.243 {
if features.saturation <= 150 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 70 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.426 {
if features.intensity <= 123 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.427 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.421 {
if features.red_luminance <= 107 {
if features.red_luminance <= 105 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.414 {
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
} else {
if features.red_difference <= 118 {
if features.blue_luminance <= 48 {
if features.red_difference <= 117 {
if features.red_chromaticity <= 0.295 {
if features.green_luminance <= 75 {
if features.blue_chromaticity <= 0.243 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.saturation <= 104 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 57 {
if features.value <= 77 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.290 {
if features.value <= 58 {
Intensity::Low
} else {
if features.intensity <= 42 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.466 {
if features.blue_chromaticity <= 0.258 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_luminance <= 65 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
} else {
if features.blue_chromaticity <= 0.263 {
if features.green_luminance <= 87 {
if features.blue_chromaticity <= 0.260 {
Intensity::Low
} else {
if features.red_luminance <= 59 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.273 {
if features.red_chromaticity <= 0.303 {
Intensity::Low
} else {
if features.red_luminance <= 61 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 88 {
if features.luminance <= 87 {
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
if features.value <= 64 {
if features.luminance <= 41 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.522 {
if features.red_chromaticity <= 0.285 {
if features.saturation <= 149 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.227 {
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
if features.red_chromaticity <= 0.305 {
if features.luminance <= 55 {
if features.red_chromaticity <= 0.300 {
if features.red_chromaticity <= 0.297 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.300 {
if features.green_chromaticity <= 0.456 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.234 {
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
if features.green_chromaticity <= 0.397 {
if features.hue <= 60 {
if features.red_chromaticity <= 0.305 {
if features.red_chromaticity <= 0.303 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.300 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.300 {
Intensity::High
} else {
if features.red_chromaticity <= 0.304 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_chromaticity <= 0.305 {
if features.red_chromaticity <= 0.305 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.305 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.393 {
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
if features.green_chromaticity <= 0.392 {
if features.red_luminance <= 135 {
if features.blue_luminance <= 137 {
if features.red_chromaticity <= 0.301 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.303 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.305 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.388 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.blue_luminance <= 149 {
if features.red_chromaticity <= 0.300 {
if features.green_chromaticity <= 0.395 {
if features.red_chromaticity <= 0.296 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.395 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.value <= 182 {
if features.red_chromaticity <= 0.302 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.saturation <= 62 {
if features.intensity <= 162 {
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
if features.blue_difference <= 115 {
if features.red_chromaticity <= 0.283 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.306 {
if features.blue_chromaticity <= 0.304 {
if features.red_luminance <= 118 {
if features.green_chromaticity <= 0.410 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.303 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.305 {
Intensity::High
} else {
if features.hue <= 63 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.blue_luminance <= 144 {
if features.red_luminance <= 127 {
if features.green_chromaticity <= 0.406 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.399 {
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
if features.red_chromaticity <= 0.301 {
if features.saturation <= 79 {
if features.blue_luminance <= 137 {
if features.red_chromaticity <= 0.289 {
if features.green_chromaticity <= 0.404 {
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
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.412 {
if features.hue <= 66 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 164 {
Intensity::High
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
if features.green_luminance <= 149 {
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
if features.green_chromaticity <= 0.385 {
if features.green_chromaticity <= 0.380 {
if features.green_chromaticity <= 0.375 {
if features.green_chromaticity <= 0.374 {
if features.red_luminance <= 114 {
if features.red_chromaticity <= 0.340 {
if features.green_luminance <= 125 {
Intensity::High
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
if features.green_chromaticity <= 0.374 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.green_luminance <= 162 {
if features.blue_chromaticity <= 0.302 {
if features.saturation <= 51 {
if features.red_chromaticity <= 0.324 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.375 {
if features.green_chromaticity <= 0.375 {
if features.blue_difference <= 115 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.intensity <= 125 {
Intensity::Low
} else {
if features.blue_luminance <= 111 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_chromaticity <= 0.302 {
if features.green_chromaticity <= 0.378 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.302 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.378 {
if features.red_luminance <= 137 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 158 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.saturation <= 52 {
Intensity::Low
} else {
if features.red_luminance <= 143 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_luminance <= 135 {
if features.red_luminance <= 101 {
if features.red_luminance <= 99 {
Intensity::Low
} else {
if features.blue_luminance <= 83 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_difference <= 123 {
if features.intensity <= 147 {
if features.value <= 138 {
if features.green_luminance <= 137 {
if features.green_chromaticity <= 0.384 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.380 {
Intensity::Low
} else {
if features.green_luminance <= 163 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.value <= 170 {
Intensity::High
} else {
if features.luminance <= 157 {
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
} else {
if features.green_chromaticity <= 0.384 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.384 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.308 {
Intensity::Low
} else {
if features.red_difference <= 112 {
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
if features.green_luminance <= 118 {
if features.red_chromaticity <= 0.321 {
if features.green_chromaticity <= 0.424 {
if features.luminance <= 96 {
if features.blue_chromaticity <= 0.269 {
if features.value <= 85 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.421 {
if features.saturation <= 91 {
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
} else {
if features.green_chromaticity <= 0.411 {
if features.blue_chromaticity <= 0.284 {
if features.blue_chromaticity <= 0.282 {
if features.green_chromaticity <= 0.404 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.red_luminance <= 87 {
if features.green_chromaticity <= 0.401 {
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
}
} else {
if features.green_luminance <= 63 {
if features.green_chromaticity <= 0.486 {
if features.saturation <= 145 {
if features.blue_chromaticity <= 0.224 {
if features.blue_luminance <= 28 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 35 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.441 {
if features.green_chromaticity <= 0.427 {
if features.saturation <= 101 {
if features.green_luminance <= 79 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.433 {
if features.red_chromaticity <= 0.317 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.434 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.320 {
if features.red_chromaticity <= 0.313 {
if features.red_chromaticity <= 0.313 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.value <= 68 {
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
if features.hue <= 44 {
if features.hue <= 25 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.328 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.328 {
Intensity::High
} else {
if features.red_chromaticity <= 0.329 {
if features.red_chromaticity <= 0.329 {
Intensity::Low
} else {
if features.saturation <= 154 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.336 {
if features.red_chromaticity <= 0.332 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 49 {
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
if features.saturation <= 62 {
if features.value <= 141 {
if features.blue_luminance <= 107 {
if features.red_chromaticity <= 0.321 {
if features.red_luminance <= 111 {
if features.red_luminance <= 109 {
if features.green_chromaticity <= 0.395 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.296 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.value <= 136 {
Intensity::High
} else {
if features.value <= 138 {
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
} else {
if features.blue_chromaticity <= 0.303 {
if features.green_luminance <= 167 {
if features.red_chromaticity <= 0.306 {
Intensity::High
} else {
if features.green_chromaticity <= 0.395 {
if features.red_chromaticity <= 0.307 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 136 {
Intensity::High
} else {
if features.luminance <= 155 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_luminance <= 133 {
if features.blue_chromaticity <= 0.304 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.308 {
if features.red_chromaticity <= 0.307 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 132 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.389 {
if features.blue_luminance <= 148 {
if features.red_chromaticity <= 0.310 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_difference <= 111 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.luminance <= 158 {
Intensity::Low
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
}
} else {
if features.red_difference <= 118 {
if features.blue_chromaticity <= 0.295 {
if features.red_luminance <= 104 {
if features.red_chromaticity <= 0.307 {
if features.blue_chromaticity <= 0.291 {
Intensity::Low
} else {
if features.green_luminance <= 135 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.309 {
if features.blue_chromaticity <= 0.288 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.value <= 119 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.295 {
if features.saturation <= 66 {
if features.red_chromaticity <= 0.316 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.399 {
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
if features.saturation <= 64 {
if features.green_chromaticity <= 0.396 {
if features.green_chromaticity <= 0.392 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.296 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.green_luminance <= 145 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.316 {
if features.red_chromaticity <= 0.316 {
if features.green_chromaticity <= 0.394 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_luminance <= 122 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 104 {
if features.green_chromaticity <= 0.396 {
if features.blue_luminance <= 93 {
Intensity::Low
} else {
if features.luminance <= 115 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 96 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.391 {
if features.red_luminance <= 112 {
if features.luminance <= 122 {
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
}
} else {
if features.blue_difference <= 118 {
if features.red_chromaticity <= 0.305 {
if features.hue <= 67 {
if features.red_chromaticity <= 0.280 {
if features.intensity <= 35 {
if features.luminance <= 41 {
if features.blue_chromaticity <= 0.226 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.227 {
Intensity::High
} else {
if features.green_chromaticity <= 0.543 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.219 {
if features.blue_chromaticity <= 0.215 {
Intensity::High
} else {
if features.green_luminance <= 54 {
if features.luminance <= 42 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.593 {
if features.green_chromaticity <= 0.579 {
if features.green_chromaticity <= 0.564 {
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
if features.hue <= 66 {
if features.red_chromaticity <= 0.258 {
if features.red_luminance <= 44 {
if features.value <= 74 {
if features.saturation <= 169 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.hue <= 65 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.279 {
if features.red_chromaticity <= 0.252 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.288 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_difference <= 117 {
if features.green_chromaticity <= 0.450 {
if features.green_chromaticity <= 0.434 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.270 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.value <= 116 {
if features.red_luminance <= 40 {
Intensity::Low
} else {
Intensity::Low
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
if features.red_chromaticity <= 0.279 {
if features.red_difference <= 107 {
if features.blue_chromaticity <= 0.308 {
if features.value <= 150 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.value <= 160 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 79 {
if features.blue_chromaticity <= 0.301 {
Intensity::Low
} else {
Intensity::High
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
if features.red_chromaticity <= 0.279 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.blue_luminance <= 115 {
if features.blue_chromaticity <= 0.296 {
if features.green_luminance <= 57 {
if features.hue <= 51 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.291 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.225 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.449 {
if features.hue <= 62 {
if features.blue_chromaticity <= 0.280 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_difference <= 117 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 47 {
if features.red_chromaticity <= 0.282 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.451 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
} else {
if features.luminance <= 123 {
if features.green_chromaticity <= 0.404 {
if features.value <= 130 {
if features.blue_difference <= 117 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 133 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 88 {
if features.blue_chromaticity <= 0.297 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.288 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.green_luminance <= 152 {
if features.blue_luminance <= 112 {
if features.blue_chromaticity <= 0.306 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 80 {
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
if features.red_luminance <= 122 {
if features.green_luminance <= 171 {
if features.red_difference <= 106 {
if features.saturation <= 75 {
Intensity::Low
} else {
if features.blue_luminance <= 126 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.blue_difference <= 117 {
if features.green_chromaticity <= 0.398 {
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
Intensity::High
}
} else {
if features.green_luminance <= 156 {
Intensity::Low
} else {
if features.luminance <= 154 {
if features.red_chromaticity <= 0.290 {
Intensity::High
} else {
if features.green_chromaticity <= 0.390 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_difference <= 107 {
if features.blue_chromaticity <= 0.315 {
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
}
}
}
}
} else {
if features.red_chromaticity <= 0.226 {
if features.value <= 135 {
if features.red_difference <= 104 {
if features.red_chromaticity <= 0.172 {
if features.red_chromaticity <= 0.162 {
Intensity::Low
} else {
if features.saturation <= 180 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.saturation <= 173 {
if features.blue_chromaticity <= 0.301 {
if features.green_luminance <= 117 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.303 {
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
if features.red_chromaticity <= 0.214 {
if features.red_luminance <= 29 {
if features.saturation <= 194 {
if features.saturation <= 181 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
if features.value <= 86 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.312 {
if features.value <= 140 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.value <= 140 {
if features.green_chromaticity <= 0.478 {
if features.green_luminance <= 138 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.saturation <= 157 {
Intensity::Low
} else {
if features.hue <= 72 {
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
} else {
if features.red_difference <= 96 {
Intensity::High
} else {
if features.red_chromaticity <= 0.227 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.302 {
if features.value <= 118 {
if features.value <= 109 {
if features.red_luminance <= 54 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 126 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 123 {
if features.red_luminance <= 62 {
Intensity::High
} else {
if features.saturation <= 122 {
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
if features.green_chromaticity <= 0.377 {
if features.green_chromaticity <= 0.371 {
if features.green_chromaticity <= 0.369 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.369 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.371 {
Intensity::Low
} else {
if features.luminance <= 158 {
if features.intensity <= 150 {
if features.red_chromaticity <= 0.324 {
if features.red_chromaticity <= 0.324 {
if features.green_chromaticity <= 0.374 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.371 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.luminance <= 159 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.value <= 107 {
if features.hue <= 49 {
if features.red_chromaticity <= 0.336 {
if features.red_chromaticity <= 0.336 {
if features.red_chromaticity <= 0.335 {
if features.red_difference <= 122 {
if features.red_chromaticity <= 0.325 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.013 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 69 {
if features.green_luminance <= 63 {
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
if features.blue_chromaticity <= 0.249 {
if features.blue_chromaticity <= 0.248 {
if features.red_chromaticity <= 0.310 {
if features.red_luminance <= 43 {
Intensity::Low
} else {
if features.luminance <= 55 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.453 {
if features.green_chromaticity <= 0.449 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.red_difference <= 120 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.317 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.317 {
Intensity::High
} else {
if features.luminance <= 78 {
if features.green_luminance <= 86 {
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
if features.green_chromaticity <= 0.389 {
if features.blue_chromaticity <= 0.303 {
if features.green_luminance <= 155 {
if features.blue_chromaticity <= 0.285 {
Intensity::Low
} else {
if features.green_luminance <= 151 {
if features.value <= 143 {
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
} else {
Intensity::High
}
} else {
if features.red_luminance <= 111 {
if features.intensity <= 118 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.307 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 137 {
if features.intensity <= 140 {
if features.red_difference <= 117 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 160 {
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
if features.luminance <= 121 {
if features.blue_luminance <= 77 {
if features.intensity <= 88 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.306 {
if features.blue_difference <= 117 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.394 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.394 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_chromaticity <= 0.300 {
if features.green_chromaticity <= 0.392 {
if features.red_difference <= 116 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.393 {
if features.blue_luminance <= 112 {
if features.blue_chromaticity <= 0.302 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 146 {
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
}
}
} else {
if features.green_chromaticity <= 0.384 {
if features.green_chromaticity <= 0.363 {
if features.green_chromaticity <= 0.362 {
if features.green_chromaticity <= 0.353 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.353 {
if features.red_difference <= 119 {
if features.luminance <= 147 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.355 {
if features.green_chromaticity <= 0.355 {
Intensity::Low
} else {
if features.red_difference <= 118 {
if features.hue <= 83 {
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
}
} else {
if features.green_chromaticity <= 0.362 {
if features.saturation <= 37 {
if features.hue <= 57 {
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
if features.green_chromaticity <= 0.363 {
if features.red_chromaticity <= 0.310 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 94 {
if features.blue_luminance <= 0 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.377 {
if features.red_difference <= 116 {
if features.blue_chromaticity <= 0.317 {
if features.blue_chromaticity <= 0.317 {
Intensity::Low
} else {
if features.green_luminance <= 166 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_difference <= 113 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.333 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.308 {
if features.red_chromaticity <= 0.308 {
if features.green_luminance <= 148 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.313 {
if features.blue_chromaticity <= 0.313 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.312 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_chromaticity <= 0.377 {
if features.blue_difference <= 123 {
if features.red_difference <= 118 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_difference <= 113 {
if features.red_difference <= 112 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.322 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.321 {
if features.blue_chromaticity <= 0.315 {
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
}
}
}
}
}
}
} else {
if features.luminance <= 110 {
if features.blue_difference <= 120 {
if features.saturation <= 155 {
if features.red_difference <= 115 {
if features.blue_chromaticity <= 0.293 {
if features.red_difference <= 113 {
if features.blue_chromaticity <= 0.288 {
if features.blue_chromaticity <= 0.284 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_difference <= 112 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.243 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.248 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_difference <= 101 {
if features.blue_chromaticity <= 0.315 {
if features.value <= 127 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.318 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.447 {
if features.red_difference <= 105 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.313 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
} else {
if features.red_chromaticity <= 0.296 {
if features.blue_luminance <= 27 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.254 {
if features.green_luminance <= 54 {
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
}
} else {
if features.green_luminance <= 120 {
if features.red_chromaticity <= 0.305 {
if features.red_chromaticity <= 0.305 {
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
if features.blue_chromaticity <= 0.305 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
} else {
if features.saturation <= 167 {
if features.green_chromaticity <= 0.509 {
if features.blue_difference <= 119 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 63 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.530 {
if features.red_difference <= 104 {
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
Intensity::Low
}
}
} else {
if features.blue_difference <= 121 {
if features.hue <= 63 {
if features.green_luminance <= 11 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 156 {
if features.red_luminance <= 41 {
if features.saturation <= 97 {
Intensity::High
} else {
if features.red_difference <= 101 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_luminance <= 115 {
if features.hue <= 68 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.003 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_difference <= 122 {
if features.red_luminance <= 65 {
if features.hue <= 68 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.300 {
if features.intensity <= 45 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.saturation <= 181 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.410 {
Intensity::Low
} else {
if features.green_luminance <= 102 {
Intensity::High
} else {
if features.red_chromaticity <= 0.273 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.saturation <= 67 {
if features.red_chromaticity <= 0.289 {
if features.blue_chromaticity <= 0.322 {
if features.blue_chromaticity <= 0.322 {
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
} else {
if features.red_chromaticity <= 0.289 {
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
if features.blue_chromaticity <= 0.312 {
if features.green_chromaticity <= 0.395 {
if features.blue_chromaticity <= 0.308 {
if features.hue <= 61 {
if features.saturation <= 52 {
Intensity::High
} else {
if features.red_chromaticity <= 0.305 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.311 {
if features.red_luminance <= 109 {
Intensity::Low
} else {
if features.red_luminance <= 111 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 114 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.intensity <= 118 {
if features.green_chromaticity <= 0.396 {
Intensity::High
} else {
if features.red_chromaticity <= 0.281 {
if features.red_chromaticity <= 0.270 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.285 {
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
} else {
if features.red_difference <= 115 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.322 {
if features.blue_difference <= 122 {
if features.green_luminance <= 143 {
if features.luminance <= 121 {
if features.blue_chromaticity <= 0.318 {
if features.green_chromaticity <= 0.442 {
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
} else {
if features.green_chromaticity <= 0.385 {
Intensity::High
} else {
if features.green_chromaticity <= 0.392 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.235 {
if features.red_chromaticity <= 0.232 {
if features.value <= 146 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.303 {
if features.luminance <= 146 {
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
if features.red_chromaticity <= 0.292 {
if features.blue_chromaticity <= 0.325 {
if features.blue_chromaticity <= 0.325 {
if features.blue_chromaticity <= 0.325 {
if features.red_difference <= 108 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 145 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.423 {
if features.intensity <= 103 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.423 {
Intensity::High
} else {
if features.value <= 157 {
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
}
}
}
}
}
}
}
}