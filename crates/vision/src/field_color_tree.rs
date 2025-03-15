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
if features.green_chromaticity <= 0.429 {
if features.green_chromaticity <= 0.413 {
if features.green_chromaticity <= 0.406 {
if features.green_chromaticity <= 0.400 {
if features.green_chromaticity <= 0.395 {
if features.green_chromaticity <= 0.389 {
if features.blue_difference <= 136 {
if features.blue_difference <= 112 {
if features.red_chromaticity <= 0.357 {
if features.hue <= 49 {
if features.red_luminance <= 174 {
if features.blue_difference <= 108 {
if features.green_chromaticity <= 0.374 {
if features.luminance <= 150 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_difference <= 103 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.387 {
if features.red_chromaticity <= 0.349 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 134 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.value <= 210 {
if features.red_luminance <= 208 {
if features.blue_chromaticity <= 0.295 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.hue <= 47 {
if features.saturation <= 44 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 194 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_difference <= 117 {
if features.intensity <= 138 {
if features.intensity <= 137 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.389 {
if features.red_chromaticity <= 0.319 {
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
}
} else {
if features.green_chromaticity <= 0.380 {
Intensity::Low
} else {
if features.red_luminance <= 156 {
if features.green_chromaticity <= 0.386 {
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
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.386 {
if features.blue_difference <= 133 {
if features.green_chromaticity <= 0.375 {
if features.red_difference <= 111 {
if features.green_chromaticity <= 0.367 {
if features.green_chromaticity <= 0.367 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.369 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_difference <= 115 {
if features.saturation <= 39 {
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
}
} else {
if features.blue_chromaticity <= 0.301 {
if features.blue_chromaticity <= 0.300 {
if features.value <= 99 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 78 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 189 {
if features.green_chromaticity <= 0.375 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 199 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_luminance <= 195 {
if features.green_luminance <= 142 {
if features.hue <= 89 {
if features.saturation <= 85 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.357 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 146 {
if features.hue <= 90 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.340 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_luminance <= 211 {
if features.red_chromaticity <= 0.312 {
if features.saturation <= 28 {
Intensity::High
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
if features.blue_chromaticity <= 0.350 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.350 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.blue_difference <= 135 {
if features.blue_chromaticity <= 0.305 {
if features.blue_chromaticity <= 0.304 {
if features.luminance <= 153 {
if features.red_difference <= 118 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 60 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.388 {
if features.green_chromaticity <= 0.387 {
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
if features.red_chromaticity <= 0.308 {
if features.red_chromaticity <= 0.296 {
if features.red_difference <= 119 {
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
} else {
if features.green_chromaticity <= 0.387 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_luminance <= 96 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
} else {
if features.red_luminance <= 210 {
if features.blue_chromaticity <= 0.354 {
if features.red_luminance <= 204 {
Intensity::High
} else {
if features.value <= 233 {
if features.green_luminance <= 213 {
Intensity::Low
} else {
if features.intensity <= 217 {
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
if features.red_luminance <= 146 {
if features.red_luminance <= 117 {
Intensity::Low
} else {
if features.blue_luminance <= 165 {
if features.red_chromaticity <= 0.276 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.385 {
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
if features.red_difference <= 117 {
if features.intensity <= 171 {
if features.red_chromaticity <= 0.286 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.hue <= 98 {
if features.green_chromaticity <= 0.343 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 189 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_luminance <= 147 {
if features.red_chromaticity <= 0.305 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 209 {
if features.blue_chromaticity <= 0.364 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_difference <= 120 {
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
if features.green_chromaticity <= 0.324 {
Intensity::High
} else {
if features.intensity <= 225 {
if features.green_luminance <= 219 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.326 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.356 {
Intensity::Low
} else {
if features.blue_luminance <= 253 {
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
if features.blue_chromaticity <= 0.294 {
if features.blue_luminance <= 89 {
if features.blue_luminance <= 79 {
if features.green_chromaticity <= 0.389 {
if features.red_difference <= 122 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.255 {
if features.blue_chromaticity <= 0.252 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 64 {
if features.blue_luminance <= 61 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.317 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.314 {
if features.blue_chromaticity <= 0.292 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 92 {
if features.red_chromaticity <= 0.359 {
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
if features.green_chromaticity <= 0.394 {
if features.blue_luminance <= 83 {
if features.green_chromaticity <= 0.390 {
if features.green_chromaticity <= 0.390 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.hue <= 47 {
Intensity::Low
} else {
if features.saturation <= 72 {
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
if features.green_chromaticity <= 0.394 {
Intensity::High
} else {
if features.blue_luminance <= 88 {
if features.red_luminance <= 90 {
if features.green_luminance <= 112 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 123 {
Intensity::Low
} else {
if features.red_difference <= 120 {
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
if features.blue_chromaticity <= 0.287 {
if features.intensity <= 157 {
if features.red_luminance <= 129 {
if features.green_chromaticity <= 0.395 {
if features.luminance <= 121 {
if features.blue_luminance <= 90 {
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
if features.red_luminance <= 128 {
if features.blue_chromaticity <= 0.270 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.281 {
if features.blue_luminance <= 131 {
if features.green_chromaticity <= 0.394 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.red_luminance <= 153 {
if features.saturation <= 69 {
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
}
} else {
if features.blue_chromaticity <= 0.282 {
if features.red_luminance <= 155 {
Intensity::High
} else {
if features.red_difference <= 117 {
if features.blue_chromaticity <= 0.279 {
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
if features.value <= 185 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.286 {
if features.red_luminance <= 153 {
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
}
} else {
if features.green_chromaticity <= 0.389 {
if features.green_chromaticity <= 0.389 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.393 {
if features.blue_difference <= 108 {
if features.blue_chromaticity <= 0.291 {
if features.green_luminance <= 192 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.321 {
if features.blue_chromaticity <= 0.293 {
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
if features.value <= 182 {
if features.red_chromaticity <= 0.319 {
if features.green_luminance <= 125 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 188 {
if features.luminance <= 170 {
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
if features.blue_chromaticity <= 0.301 {
if features.blue_luminance <= 119 {
if features.value <= 128 {
if features.green_chromaticity <= 0.395 {
if features.blue_chromaticity <= 0.301 {
if features.red_difference <= 118 {
if features.blue_chromaticity <= 0.294 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 61 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.luminance <= 87 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 111 {
if features.red_difference <= 115 {
if features.blue_chromaticity <= 0.299 {
if features.blue_luminance <= 105 {
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
if features.value <= 134 {
if features.blue_luminance <= 97 {
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
if features.value <= 151 {
if features.green_chromaticity <= 0.393 {
if features.blue_luminance <= 113 {
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
if features.green_luminance <= 157 {
if features.blue_chromaticity <= 0.298 {
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
if features.red_luminance <= 122 {
Intensity::Low
} else {
if features.saturation <= 60 {
if features.red_difference <= 114 {
Intensity::Low
} else {
if features.luminance <= 143 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.299 {
if features.blue_luminance <= 125 {
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
}
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.393 {
if features.red_chromaticity <= 0.309 {
if features.green_luminance <= 165 {
if features.blue_luminance <= 110 {
if features.red_difference <= 121 {
if features.green_chromaticity <= 0.393 {
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
if features.luminance <= 131 {
if features.green_chromaticity <= 0.392 {
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
}
} else {
if features.green_chromaticity <= 0.392 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.392 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.309 {
if features.blue_luminance <= 112 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.blue_difference <= 135 {
if features.red_luminance <= 98 {
if features.blue_chromaticity <= 0.318 {
if features.red_chromaticity <= 0.291 {
if features.red_difference <= 118 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_difference <= 111 {
if features.blue_luminance <= 92 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 11 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_luminance <= 113 {
if features.hue <= 60 {
if features.red_chromaticity <= 0.304 {
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
} else {
if features.blue_luminance <= 128 {
if features.red_chromaticity <= 0.293 {
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
} else {
if features.value <= 98 {
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
if features.blue_difference <= 113 {
if features.blue_chromaticity <= 0.288 {
if features.blue_luminance <= 84 {
if features.blue_chromaticity <= 0.272 {
if features.blue_chromaticity <= 0.249 {
if features.intensity <= 42 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.250 {
if features.green_chromaticity <= 0.397 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_difference <= 107 {
if features.saturation <= 93 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.252 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.396 {
if features.saturation <= 89 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 91 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.luminance <= 109 {
if features.green_chromaticity <= 0.397 {
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
}
} else {
if features.saturation <= 78 {
if features.green_chromaticity <= 0.400 {
if features.red_chromaticity <= 0.330 {
if features.blue_luminance <= 112 {
if features.green_chromaticity <= 0.399 {
if features.red_chromaticity <= 0.315 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 125 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.323 {
if features.red_luminance <= 151 {
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
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.400 {
Intensity::High
} else {
if features.red_chromaticity <= 0.317 {
if features.red_difference <= 111 {
Intensity::High
} else {
if features.red_luminance <= 117 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 90 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.323 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
}
} else {
if features.green_luminance <= 186 {
if features.blue_chromaticity <= 0.274 {
if features.blue_chromaticity <= 0.248 {
Intensity::Low
} else {
if features.hue <= 38 {
Intensity::High
} else {
if features.blue_difference <= 102 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.274 {
Intensity::High
} else {
if features.blue_difference <= 110 {
if features.saturation <= 79 {
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
}
} else {
if features.intensity <= 175 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_luminance <= 127 {
if features.blue_chromaticity <= 0.293 {
if features.saturation <= 70 {
if features.green_chromaticity <= 0.398 {
if features.red_chromaticity <= 0.314 {
if features.green_chromaticity <= 0.396 {
if features.blue_luminance <= 112 {
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
} else {
if features.red_chromaticity <= 0.315 {
if features.green_chromaticity <= 0.396 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.291 {
if features.red_chromaticity <= 0.311 {
if features.red_luminance <= 121 {
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
if features.blue_chromaticity <= 0.291 {
if features.blue_chromaticity <= 0.291 {
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
}
}
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 160 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.398 {
if features.luminance <= 146 {
Intensity::Low
} else {
if features.red_difference <= 113 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.308 {
if features.red_chromaticity <= 0.308 {
if features.value <= 163 {
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
if features.intensity <= 183 {
if features.saturation <= 66 {
if features.green_chromaticity <= 0.399 {
if features.intensity <= 141 {
if features.saturation <= 65 {
Intensity::Low
} else {
if features.green_luminance <= 166 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.396 {
if features.green_chromaticity <= 0.396 {
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
}
} else {
if features.blue_difference <= 112 {
if features.saturation <= 63 {
if features.red_chromaticity <= 0.301 {
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
} else {
if features.blue_chromaticity <= 0.298 {
Intensity::Low
} else {
if features.intensity <= 147 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_luminance <= 133 {
if features.blue_luminance <= 125 {
if features.green_chromaticity <= 0.396 {
Intensity::Low
} else {
if features.value <= 169 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.308 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_luminance <= 179 {
if features.value <= 177 {
if features.blue_chromaticity <= 0.291 {
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
if features.saturation <= 69 {
if features.blue_chromaticity <= 0.294 {
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
}
}
} else {
Intensity::High
}
}
}
} else {
if features.luminance <= 111 {
if features.blue_difference <= 121 {
if features.intensity <= 97 {
if features.green_luminance <= 102 {
if features.red_chromaticity <= 0.295 {
if features.red_luminance <= 71 {
if features.luminance <= 85 {
if features.luminance <= 84 {
Intensity::Low
} else {
Intensity::Low
}
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
if features.luminance <= 92 {
if features.blue_difference <= 115 {
if features.green_chromaticity <= 0.398 {
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
if features.red_difference <= 120 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.399 {
if features.red_luminance <= 87 {
Intensity::Low
} else {
if features.green_luminance <= 111 {
if features.blue_luminance <= 80 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 64 {
Intensity::Low
} else {
if features.saturation <= 65 {
if features.red_luminance <= 82 {
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
if features.luminance <= 105 {
if features.saturation <= 71 {
if features.saturation <= 64 {
Intensity::Low
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
if features.green_chromaticity <= 0.400 {
if features.red_luminance <= 98 {
if features.saturation <= 63 {
if features.green_luminance <= 119 {
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
if features.red_luminance <= 99 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 70 {
if features.luminance <= 107 {
Intensity::Low
} else {
if features.saturation <= 65 {
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
if features.intensity <= 39 {
Intensity::Low
} else {
if features.luminance <= 41 {
if features.blue_luminance <= 47 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 27 {
if features.red_difference <= 116 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.301 {
if features.blue_luminance <= 52 {
if features.saturation <= 62 {
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
if features.green_chromaticity <= 0.399 {
if features.blue_luminance <= 53 {
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
}
}
}
}
} else {
if features.red_difference <= 114 {
if features.blue_luminance <= 118 {
if features.green_chromaticity <= 0.398 {
if features.red_chromaticity <= 0.302 {
if features.blue_luminance <= 106 {
if features.red_luminance <= 100 {
if features.blue_chromaticity <= 0.304 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.301 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 111 {
if features.luminance <= 129 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 122 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.397 {
if features.green_chromaticity <= 0.396 {
if features.blue_chromaticity <= 0.301 {
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
if features.value <= 146 {
if features.green_chromaticity <= 0.398 {
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
if features.green_luminance <= 138 {
if features.saturation <= 66 {
if features.red_chromaticity <= 0.296 {
Intensity::Low
} else {
if features.saturation <= 63 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 107 {
if features.green_chromaticity <= 0.399 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 108 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.299 {
if features.red_chromaticity <= 0.302 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.303 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.400 {
if features.blue_chromaticity <= 0.303 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 122 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.blue_chromaticity <= 0.299 {
if features.red_chromaticity <= 0.308 {
if features.intensity <= 133 {
Intensity::Low
} else {
if features.green_luminance <= 160 {
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
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.398 {
if features.green_luminance <= 168 {
if features.red_luminance <= 126 {
if features.saturation <= 65 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 130 {
if features.green_chromaticity <= 0.396 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.398 {
if features.red_luminance <= 109 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 190 {
if features.value <= 166 {
Intensity::Low
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
}
}
}
}
} else {
if features.blue_chromaticity <= 0.286 {
if features.green_chromaticity <= 0.398 {
if features.green_chromaticity <= 0.396 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.321 {
if features.green_chromaticity <= 0.398 {
if features.green_chromaticity <= 0.397 {
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
if features.red_chromaticity <= 0.318 {
if features.red_luminance <= 105 {
if features.value <= 132 {
if features.green_luminance <= 129 {
Intensity::Low
} else {
Intensity::Low
}
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
if features.value <= 127 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.310 {
if features.saturation <= 68 {
if features.green_chromaticity <= 0.399 {
if features.red_chromaticity <= 0.302 {
if features.blue_chromaticity <= 0.301 {
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
if features.green_luminance <= 129 {
if features.blue_chromaticity <= 0.295 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.399 {
Intensity::Low
} else {
if features.green_luminance <= 132 {
Intensity::Low
} else {
if features.saturation <= 69 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_difference <= 117 {
if features.blue_chromaticity <= 0.290 {
if features.green_chromaticity <= 0.399 {
if features.red_luminance <= 101 {
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
} else {
if features.blue_luminance <= 103 {
if features.blue_chromaticity <= 0.292 {
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
if features.saturation <= 72 {
if features.value <= 210 {
if features.green_chromaticity <= 0.402 {
if features.blue_chromaticity <= 0.301 {
if features.red_chromaticity <= 0.307 {
if features.blue_chromaticity <= 0.301 {
if features.red_luminance <= 134 {
if features.red_chromaticity <= 0.303 {
if features.green_chromaticity <= 0.402 {
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
} else {
if features.red_chromaticity <= 0.299 {
if features.luminance <= 169 {
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
if features.green_chromaticity <= 0.400 {
Intensity::Low
} else {
if features.luminance <= 168 {
if features.blue_chromaticity <= 0.287 {
if features.blue_luminance <= 127 {
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
if features.red_chromaticity <= 0.310 {
if features.green_chromaticity <= 0.400 {
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
}
}
} else {
Intensity::High
}
} else {
if features.blue_difference <= 108 {
if features.red_chromaticity <= 0.307 {
Intensity::High
} else {
if features.hue <= 55 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.300 {
if features.saturation <= 65 {
Intensity::Low
} else {
if features.red_luminance <= 134 {
if features.luminance <= 157 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.300 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 141 {
if features.blue_chromaticity <= 0.295 {
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
}
} else {
if features.red_chromaticity <= 0.301 {
Intensity::High
} else {
if features.green_chromaticity <= 0.404 {
if features.value <= 190 {
if features.green_chromaticity <= 0.403 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 70 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.291 {
if features.blue_chromaticity <= 0.290 {
Intensity::Low
} else {
Intensity::Low
}
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
}
}
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 86 {
if features.red_chromaticity <= 0.350 {
if features.red_chromaticity <= 0.349 {
if features.red_luminance <= 102 {
if features.red_chromaticity <= 0.349 {
if features.blue_difference <= 111 {
if features.red_chromaticity <= 0.339 {
if features.value <= 111 {
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
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.404 {
if features.green_chromaticity <= 0.401 {
Intensity::High
} else {
if features.green_chromaticity <= 0.403 {
if features.blue_chromaticity <= 0.269 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 94 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.250 {
Intensity::High
} else {
if features.green_chromaticity <= 0.404 {
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
if features.red_luminance <= 100 {
Intensity::High
} else {
Intensity::High
}
}
} else {
Intensity::Low
}
} else {
if features.saturation <= 77 {
if features.red_luminance <= 156 {
if features.blue_luminance <= 111 {
if features.blue_chromaticity <= 0.282 {
if features.green_chromaticity <= 0.404 {
if features.red_chromaticity <= 0.319 {
if features.red_chromaticity <= 0.319 {
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
if features.red_luminance <= 112 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.404 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.401 {
if features.blue_chromaticity <= 0.286 {
if features.saturation <= 74 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.403 {
if features.red_difference <= 115 {
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
if features.green_chromaticity <= 0.405 {
if features.red_difference <= 112 {
if features.blue_chromaticity <= 0.283 {
if features.blue_chromaticity <= 0.283 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.saturation <= 73 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.316 {
if features.red_chromaticity <= 0.314 {
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
if features.red_luminance <= 145 {
if features.blue_chromaticity <= 0.283 {
if features.blue_luminance <= 121 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_luminance <= 181 {
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
if features.intensity <= 167 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.luminance <= 173 {
if features.green_chromaticity <= 0.404 {
if features.green_chromaticity <= 0.400 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.252 {
Intensity::Low
} else {
if features.blue_difference <= 102 {
Intensity::High
} else {
if features.saturation <= 83 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_chromaticity <= 0.327 {
if features.red_luminance <= 151 {
if features.blue_luminance <= 113 {
if features.intensity <= 138 {
Intensity::Low
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
Intensity::High
}
} else {
if features.green_chromaticity <= 0.404 {
Intensity::High
} else {
if features.red_luminance <= 112 {
if features.red_chromaticity <= 0.328 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.334 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.value <= 192 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.red_luminance <= 92 {
if features.red_luminance <= 89 {
if features.blue_difference <= 114 {
if features.red_chromaticity <= 0.339 {
if features.red_luminance <= 81 {
if features.blue_chromaticity <= 0.263 {
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
if features.blue_chromaticity <= 0.249 {
Intensity::Low
} else {
if features.saturation <= 94 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.402 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.246 {
if features.intensity <= 71 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 89 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.luminance <= 87 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.luminance <= 111 {
if features.blue_difference <= 118 {
if features.green_chromaticity <= 0.405 {
if features.green_chromaticity <= 0.401 {
if features.luminance <= 96 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 74 {
if features.red_chromaticity <= 0.311 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 87 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.405 {
if features.hue <= 56 {
if features.saturation <= 108 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_difference <= 122 {
if features.intensity <= 68 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.261 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.saturation <= 70 {
if features.red_luminance <= 81 {
if features.green_chromaticity <= 0.403 {
if features.green_chromaticity <= 0.401 {
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
} else {
if features.red_chromaticity <= 0.296 {
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
if features.blue_chromaticity <= 0.444 {
if features.red_difference <= 120 {
if features.blue_luminance <= 54 {
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
if features.blue_chromaticity <= 0.450 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
} else {
if features.intensity <= 104 {
Intensity::High
} else {
if features.green_chromaticity <= 0.403 {
if features.intensity <= 105 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 113 {
if features.green_luminance <= 128 {
if features.red_chromaticity <= 0.271 {
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
} else {
if features.green_chromaticity <= 0.405 {
if features.green_luminance <= 120 {
if features.red_luminance <= 90 {
if features.red_chromaticity <= 0.310 {
if features.green_chromaticity <= 0.401 {
Intensity::Low
} else {
if features.value <= 119 {
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
if features.green_chromaticity <= 0.404 {
if features.green_chromaticity <= 0.403 {
if features.red_chromaticity <= 0.324 {
if features.saturation <= 80 {
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
if features.red_difference <= 118 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.404 {
if features.green_chromaticity <= 0.401 {
if features.red_luminance <= 90 {
if features.green_luminance <= 131 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.hue <= 63 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.405 {
Intensity::Low
} else {
if features.saturation <= 70 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.405 {
if features.saturation <= 71 {
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
if features.red_difference <= 109 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.blue_luminance <= 104 {
if features.red_luminance <= 94 {
if features.green_chromaticity <= 0.403 {
if features.red_chromaticity <= 0.314 {
Intensity::Low
} else {
if features.green_luminance <= 119 {
if features.blue_chromaticity <= 0.270 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.403 {
Intensity::Low
} else {
if features.red_luminance <= 93 {
if features.green_chromaticity <= 0.403 {
Intensity::Low
} else {
if features.green_luminance <= 131 {
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
if features.red_difference <= 114 {
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
}
} else {
if features.blue_difference <= 114 {
if features.red_chromaticity <= 0.310 {
if features.red_chromaticity <= 0.310 {
if features.intensity <= 110 {
if features.saturation <= 74 {
Intensity::Low
} else {
if features.saturation <= 75 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.288 {
if features.saturation <= 73 {
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
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.401 {
if features.red_chromaticity <= 0.313 {
if features.luminance <= 126 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 139 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 77 {
if features.red_chromaticity <= 0.314 {
if features.red_chromaticity <= 0.313 {
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
if features.red_chromaticity <= 0.317 {
if features.green_chromaticity <= 0.404 {
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
}
} else {
if features.green_chromaticity <= 0.401 {
if features.red_luminance <= 101 {
if features.blue_luminance <= 93 {
if features.red_chromaticity <= 0.314 {
if features.green_luminance <= 125 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.401 {
if features.value <= 134 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.300 {
if features.red_chromaticity <= 0.301 {
Intensity::Low
} else {
if features.saturation <= 66 {
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
if features.red_chromaticity <= 0.294 {
if features.blue_luminance <= 99 {
if features.value <= 131 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.292 {
if features.intensity <= 109 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.403 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_difference <= 114 {
if features.red_chromaticity <= 0.302 {
if features.blue_luminance <= 95 {
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
if features.red_chromaticity <= 0.304 {
if features.green_chromaticity <= 0.402 {
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
}
}
} else {
if features.blue_chromaticity <= 0.297 {
if features.green_chromaticity <= 0.402 {
if features.hue <= 55 {
if features.intensity <= 124 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.309 {
if features.green_chromaticity <= 0.401 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.302 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.293 {
if features.blue_chromaticity <= 0.292 {
if features.green_luminance <= 156 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 146 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.294 {
if features.red_chromaticity <= 0.305 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 145 {
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
if features.saturation <= 72 {
if features.red_luminance <= 125 {
if features.blue_chromaticity <= 0.296 {
if features.saturation <= 68 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.403 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 123 {
if features.green_chromaticity <= 0.405 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.297 {
if features.green_chromaticity <= 0.405 {
if features.saturation <= 70 {
if features.red_chromaticity <= 0.302 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.405 {
if features.red_luminance <= 112 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
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
if features.green_luminance <= 133 {
Intensity::High
} else {
if features.blue_luminance <= 123 {
if features.green_chromaticity <= 0.404 {
if features.luminance <= 144 {
if features.red_difference <= 112 {
if features.green_chromaticity <= 0.403 {
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
if features.green_chromaticity <= 0.401 {
if features.green_chromaticity <= 0.400 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 135 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.289 {
if features.red_chromaticity <= 0.287 {
if features.blue_chromaticity <= 0.311 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.299 {
if features.blue_chromaticity <= 0.298 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 72 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.luminance <= 152 {
if features.intensity <= 142 {
if features.green_chromaticity <= 0.405 {
if features.green_chromaticity <= 0.400 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.311 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.saturation <= 64 {
if features.red_luminance <= 132 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.302 {
if features.green_chromaticity <= 0.404 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 156 {
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
}
} else {
if features.blue_difference <= 112 {
if features.green_chromaticity <= 0.409 {
if features.blue_luminance <= 84 {
if features.blue_luminance <= 72 {
if features.saturation <= 106 {
if features.blue_difference <= 108 {
if features.green_chromaticity <= 0.407 {
if features.blue_chromaticity <= 0.246 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.357 {
if features.green_chromaticity <= 0.408 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.408 {
if features.hue <= 44 {
Intensity::High
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
if features.blue_chromaticity <= 0.155 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.406 {
Intensity::High
} else {
if features.red_luminance <= 110 {
if features.green_chromaticity <= 0.409 {
if features.red_chromaticity <= 0.348 {
if features.saturation <= 98 {
if features.intensity <= 100 {
if features.intensity <= 98 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 128 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.345 {
Intensity::High
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
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.281 {
if features.saturation <= 84 {
if features.green_chromaticity <= 0.407 {
if features.blue_luminance <= 132 {
if features.value <= 154 {
if features.green_luminance <= 147 {
if features.red_chromaticity <= 0.313 {
if features.red_luminance <= 104 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 129 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.276 {
if features.blue_difference <= 108 {
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
}
} else {
if features.red_luminance <= 133 {
if features.luminance <= 150 {
if features.green_chromaticity <= 0.406 {
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
if features.red_difference <= 113 {
if features.green_chromaticity <= 0.406 {
Intensity::High
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
}
}
} else {
Intensity::High
}
} else {
if features.red_luminance <= 104 {
if features.green_chromaticity <= 0.407 {
Intensity::High
} else {
if features.green_chromaticity <= 0.408 {
if features.saturation <= 81 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_difference <= 111 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.313 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_luminance <= 148 {
if features.green_chromaticity <= 0.409 {
if features.red_chromaticity <= 0.311 {
Intensity::High
} else {
if features.green_chromaticity <= 0.408 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.value <= 163 {
Intensity::High
} else {
if features.value <= 177 {
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
if features.intensity <= 110 {
if features.red_luminance <= 110 {
if features.green_chromaticity <= 0.408 {
if features.green_chromaticity <= 0.406 {
if features.hue <= 47 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 86 {
if features.blue_chromaticity <= 0.267 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.407 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.408 {
Intensity::Low
} else {
if features.luminance <= 119 {
if features.blue_chromaticity <= 0.265 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 108 {
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
if features.red_chromaticity <= 0.341 {
if features.value <= 184 {
if features.value <= 161 {
if features.red_chromaticity <= 0.339 {
if features.blue_chromaticity <= 0.271 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_difference <= 103 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.407 {
if features.red_luminance <= 145 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.318 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.hue <= 47 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.luminance <= 133 {
if features.blue_chromaticity <= 0.249 {
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
if features.blue_chromaticity <= 0.287 {
if features.luminance <= 170 {
if features.green_chromaticity <= 0.408 {
if features.blue_luminance <= 104 {
if features.red_chromaticity <= 0.311 {
if features.red_chromaticity <= 0.308 {
if features.green_chromaticity <= 0.407 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 145 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 114 {
Intensity::Low
} else {
if features.saturation <= 77 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.value <= 172 {
if features.intensity <= 138 {
if features.red_chromaticity <= 0.308 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_difference <= 108 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.value <= 180 {
if features.red_chromaticity <= 0.308 {
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
}
}
} else {
if features.red_difference <= 109 {
Intensity::High
} else {
if features.green_chromaticity <= 0.408 {
if features.blue_luminance <= 127 {
if features.green_luminance <= 179 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.blue_difference <= 109 {
Intensity::Low
} else {
if features.blue_difference <= 111 {
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
if features.intensity <= 141 {
if features.red_chromaticity <= 0.300 {
if features.red_chromaticity <= 0.300 {
if features.blue_chromaticity <= 0.295 {
if features.green_luminance <= 164 {
Intensity::Low
} else {
if features.saturation <= 71 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.408 {
if features.red_luminance <= 125 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 116 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.407 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.292 {
if features.blue_chromaticity <= 0.287 {
if features.value <= 165 {
if features.red_chromaticity <= 0.307 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.304 {
if features.green_chromaticity <= 0.408 {
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
if features.red_chromaticity <= 0.301 {
if features.blue_luminance <= 119 {
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
}
}
} else {
if features.blue_chromaticity <= 0.290 {
if features.red_chromaticity <= 0.305 {
if features.blue_chromaticity <= 0.287 {
Intensity::High
} else {
if features.luminance <= 169 {
if features.blue_luminance <= 129 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 127 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.408 {
if features.blue_chromaticity <= 0.291 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.296 {
if features.blue_luminance <= 125 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.296 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.408 {
if features.green_luminance <= 177 {
Intensity::High
} else {
if features.red_luminance <= 134 {
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
}
} else {
if features.blue_luminance <= 78 {
if features.blue_luminance <= 70 {
if features.blue_chromaticity <= 0.263 {
if features.green_chromaticity <= 0.409 {
if features.blue_difference <= 108 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.217 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.217 {
Intensity::Low
} else {
if features.green_luminance <= 114 {
Intensity::Low
} else {
if features.luminance <= 104 {
Intensity::High
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
if features.blue_chromaticity <= 0.239 {
Intensity::Low
} else {
if features.saturation <= 105 {
if features.green_chromaticity <= 0.409 {
Intensity::High
} else {
if features.saturation <= 90 {
if features.green_chromaticity <= 0.412 {
if features.blue_luminance <= 74 {
Intensity::Low
} else {
if features.saturation <= 87 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_difference <= 117 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.412 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.324 {
if features.saturation <= 91 {
if features.blue_chromaticity <= 0.266 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.409 {
if features.green_chromaticity <= 0.409 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 76 {
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
}
}
} else {
if features.blue_difference <= 110 {
if features.green_chromaticity <= 0.411 {
if features.red_chromaticity <= 0.315 {
if features.blue_chromaticity <= 0.287 {
if features.red_chromaticity <= 0.305 {
if features.green_chromaticity <= 0.410 {
if features.blue_chromaticity <= 0.287 {
if features.blue_chromaticity <= 0.286 {
Intensity::High
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
} else {
if features.green_chromaticity <= 0.411 {
if features.green_chromaticity <= 0.410 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 159 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.intensity <= 154 {
if features.green_luminance <= 154 {
if features.green_luminance <= 152 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_difference <= 109 {
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
if features.blue_chromaticity <= 0.294 {
if features.red_chromaticity <= 0.303 {
if features.blue_luminance <= 129 {
if features.green_chromaticity <= 0.411 {
Intensity::Low
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
} else {
if features.green_chromaticity <= 0.409 {
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
if features.luminance <= 121 {
if features.green_chromaticity <= 0.410 {
if features.luminance <= 114 {
Intensity::High
} else {
if features.green_chromaticity <= 0.410 {
if features.hue <= 46 {
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
}
} else {
if features.blue_luminance <= 79 {
Intensity::High
} else {
if features.green_chromaticity <= 0.411 {
if features.red_chromaticity <= 0.333 {
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
if features.value <= 184 {
if features.luminance <= 130 {
if features.red_luminance <= 112 {
if features.green_chromaticity <= 0.410 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.258 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.336 {
if features.red_chromaticity <= 0.318 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_difference <= 101 {
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
if features.blue_difference <= 108 {
if features.intensity <= 153 {
if features.green_chromaticity <= 0.412 {
if features.luminance <= 156 {
if features.blue_difference <= 105 {
if features.luminance <= 134 {
Intensity::High
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
if features.green_chromaticity <= 0.412 {
if features.green_chromaticity <= 0.411 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 114 {
if features.blue_luminance <= 85 {
Intensity::High
} else {
if features.intensity <= 110 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.luminance <= 150 {
if features.green_chromaticity <= 0.412 {
Intensity::High
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
}
}
} else {
Intensity::High
}
} else {
if features.red_luminance <= 126 {
if features.red_difference <= 110 {
if features.red_chromaticity <= 0.305 {
if features.blue_chromaticity <= 0.285 {
if features.blue_chromaticity <= 0.285 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.intensity <= 139 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.307 {
if features.green_chromaticity <= 0.413 {
if features.blue_chromaticity <= 0.282 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 144 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.264 {
if features.red_luminance <= 104 {
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
if features.red_chromaticity <= 0.302 {
if features.blue_luminance <= 137 {
if features.green_chromaticity <= 0.412 {
if features.green_chromaticity <= 0.412 {
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
Intensity::High
}
} else {
if features.blue_luminance <= 122 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
}
} else {
if features.green_chromaticity <= 0.412 {
if features.blue_chromaticity <= 0.294 {
if features.blue_difference <= 111 {
if features.value <= 129 {
if features.value <= 123 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.321 {
if features.red_chromaticity <= 0.320 {
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
} else {
if features.red_luminance <= 113 {
if features.green_chromaticity <= 0.411 {
if features.red_luminance <= 110 {
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
if features.green_chromaticity <= 0.409 {
if features.intensity <= 129 {
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
}
}
} else {
if features.red_chromaticity <= 0.300 {
if features.green_chromaticity <= 0.410 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.411 {
if features.blue_luminance <= 112 {
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
}
} else {
if features.red_chromaticity <= 0.300 {
if features.green_chromaticity <= 0.410 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 93 {
if features.value <= 120 {
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
}
} else {
if features.value <= 176 {
if features.red_luminance <= 121 {
if features.luminance <= 148 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.295 {
if features.luminance <= 150 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.411 {
if features.green_chromaticity <= 0.410 {
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
Intensity::Low
}
}
} else {
if features.red_luminance <= 101 {
if features.green_chromaticity <= 0.412 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.269 {
if features.green_chromaticity <= 0.413 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.273 {
if features.green_luminance <= 123 {
Intensity::Low
} else {
if features.green_luminance <= 128 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.413 {
if features.red_luminance <= 99 {
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
if features.blue_chromaticity <= 0.283 {
if features.green_chromaticity <= 0.413 {
if features.saturation <= 80 {
if features.red_chromaticity <= 0.305 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.306 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.413 {
if features.green_chromaticity <= 0.412 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.green_luminance <= 141 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.296 {
if features.blue_chromaticity <= 0.290 {
if features.blue_chromaticity <= 0.288 {
if features.value <= 161 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 76 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.295 {
if features.green_chromaticity <= 0.412 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.413 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.saturation <= 76 {
if features.green_luminance <= 178 {
if features.luminance <= 153 {
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
}
}
}
} else {
if features.red_luminance <= 92 {
if features.luminance <= 102 {
if features.blue_difference <= 118 {
if features.green_luminance <= 114 {
if features.blue_chromaticity <= 0.298 {
if features.blue_chromaticity <= 0.290 {
if features.blue_chromaticity <= 0.290 {
if features.red_chromaticity <= 0.300 {
if features.blue_chromaticity <= 0.288 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.412 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 53 {
if features.intensity <= 47 {
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
}
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 83 {
if features.saturation <= 71 {
if features.intensity <= 87 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_difference <= 117 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.406 {
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
if features.red_luminance <= 79 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.hue <= 57 {
if features.red_chromaticity <= 0.301 {
Intensity::Low
} else {
if features.blue_difference <= 115 {
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
Intensity::Low
}
}
} else {
if features.blue_difference <= 121 {
if features.value <= 83 {
if features.intensity <= 50 {
if features.red_luminance <= 39 {
Intensity::Low
} else {
if features.blue_luminance <= 33 {
if features.red_chromaticity <= 0.341 {
if features.hue <= 43 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.280 {
Intensity::Low
} else {
if features.saturation <= 80 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_luminance <= 50 {
if features.intensity <= 53 {
if features.red_chromaticity <= 0.314 {
if features.saturation <= 76 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.value <= 66 {
if features.saturation <= 77 {
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
}
} else {
if features.red_luminance <= 61 {
if features.green_chromaticity <= 0.412 {
if features.saturation <= 74 {
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
if features.red_luminance <= 62 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_chromaticity <= 0.276 {
if features.red_chromaticity <= 0.276 {
if features.green_chromaticity <= 0.413 {
if features.value <= 108 {
if features.blue_luminance <= 80 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 84 {
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
if features.green_chromaticity <= 0.413 {
if features.value <= 111 {
if features.green_chromaticity <= 0.409 {
if features.red_chromaticity <= 0.278 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 67 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 90 {
Intensity::Low
} else {
if features.blue_luminance <= 85 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.413 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.413 {
if features.intensity <= 83 {
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
if features.saturation <= 68 {
Intensity::Low
} else {
if features.blue_luminance <= 41 {
if features.red_difference <= 116 {
if features.green_chromaticity <= 0.411 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.413 {
if features.blue_luminance <= 24 {
Intensity::Low
} else {
if features.value <= 31 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.luminance <= 43 {
Intensity::Low
} else {
if features.red_luminance <= 35 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_chromaticity <= 0.298 {
if features.green_luminance <= 50 {
if features.saturation <= 110 {
if features.luminance <= 42 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.hue <= 85 {
if features.red_luminance <= 75 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.213 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.293 {
Intensity::Low
} else {
if features.green_luminance <= 64 {
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
if features.blue_chromaticity <= 0.316 {
if features.value <= 121 {
if features.saturation <= 79 {
if features.red_chromaticity <= 0.290 {
if features.saturation <= 78 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.293 {
if features.red_chromaticity <= 0.292 {
if features.value <= 120 {
if features.green_chromaticity <= 0.410 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.407 {
Intensity::Low
} else {
if features.saturation <= 73 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.408 {
if features.luminance <= 106 {
Intensity::Low
} else {
if features.green_luminance <= 120 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 89 {
if features.red_chromaticity <= 0.305 {
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
if features.saturation <= 84 {
if features.hue <= 51 {
if features.blue_chromaticity <= 0.275 {
Intensity::Low
} else {
if features.luminance <= 103 {
Intensity::Low
} else {
if features.red_luminance <= 90 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.411 {
if features.green_chromaticity <= 0.409 {
Intensity::Low
} else {
if features.red_difference <= 113 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 80 {
Intensity::Low
} else {
if features.red_luminance <= 89 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_chromaticity <= 0.412 {
if features.green_luminance <= 115 {
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
if features.saturation <= 74 {
if features.green_chromaticity <= 0.411 {
if features.green_chromaticity <= 0.407 {
Intensity::Low
} else {
if features.blue_luminance <= 90 {
if features.value <= 124 {
if features.saturation <= 70 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 91 {
if features.red_chromaticity <= 0.294 {
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
}
}
} else {
if features.green_chromaticity <= 0.412 {
Intensity::Low
} else {
if features.green_luminance <= 124 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_luminance <= 82 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.305 {
if features.saturation <= 80 {
if features.red_chromaticity <= 0.285 {
if features.red_difference <= 110 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 75 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 100 {
if features.red_chromaticity <= 0.278 {
if features.green_chromaticity <= 0.410 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.407 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.406 {
Intensity::High
} else {
if features.red_chromaticity <= 0.281 {
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
if features.green_chromaticity <= 0.413 {
if features.intensity <= 112 {
if features.blue_luminance <= 92 {
if features.hue <= 70 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.luminance <= 120 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.hue <= 72 {
if features.red_chromaticity <= 0.266 {
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
if features.green_chromaticity <= 0.409 {
if features.red_chromaticity <= 0.299 {
if features.blue_luminance <= 121 {
if features.green_chromaticity <= 0.407 {
if features.red_luminance <= 99 {
if features.green_chromaticity <= 0.406 {
Intensity::Low
} else {
if features.saturation <= 70 {
if features.green_luminance <= 132 {
if features.blue_chromaticity <= 0.295 {
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
if features.red_luminance <= 93 {
if features.green_chromaticity <= 0.407 {
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
}
}
} else {
if features.green_chromaticity <= 0.406 {
if features.intensity <= 115 {
Intensity::Low
} else {
if features.luminance <= 137 {
if features.red_chromaticity <= 0.286 {
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
}
} else {
if features.blue_chromaticity <= 0.295 {
if features.blue_difference <= 114 {
if features.blue_chromaticity <= 0.295 {
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
} else {
if features.blue_chromaticity <= 0.311 {
if features.red_chromaticity <= 0.282 {
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
if features.red_luminance <= 102 {
if features.green_chromaticity <= 0.408 {
if features.red_luminance <= 96 {
if features.red_chromaticity <= 0.286 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 71 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.408 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.294 {
if features.blue_chromaticity <= 0.299 {
if features.blue_luminance <= 96 {
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
} else {
if features.red_chromaticity <= 0.299 {
if features.blue_chromaticity <= 0.296 {
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
if features.green_chromaticity <= 0.408 {
Intensity::Low
} else {
if features.red_luminance <= 115 {
if features.blue_difference <= 115 {
if features.red_chromaticity <= 0.293 {
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
if features.red_luminance <= 116 {
if features.saturation <= 71 {
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
}
}
}
} else {
if features.blue_luminance <= 144 {
if features.blue_luminance <= 125 {
if features.green_chromaticity <= 0.406 {
if features.blue_luminance <= 123 {
Intensity::Low
} else {
if features.red_difference <= 107 {
Intensity::Low
} else {
if features.saturation <= 71 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.297 {
if features.blue_luminance <= 122 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.300 {
if features.green_chromaticity <= 0.408 {
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
}
} else {
if features.blue_luminance <= 127 {
if features.green_chromaticity <= 0.406 {
if features.hue <= 65 {
if features.saturation <= 72 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.301 {
if features.red_luminance <= 123 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.red_difference <= 106 {
if features.intensity <= 132 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 71 {
Intensity::Low
} else {
if features.intensity <= 141 {
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
}
} else {
if features.green_chromaticity <= 0.408 {
if features.blue_chromaticity <= 0.280 {
if features.saturation <= 80 {
Intensity::Low
} else {
if features.blue_luminance <= 84 {
if features.green_chromaticity <= 0.406 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.273 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.407 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_difference <= 112 {
if features.red_chromaticity <= 0.301 {
if features.red_chromaticity <= 0.300 {
if features.red_chromaticity <= 0.300 {
if features.value <= 163 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.301 {
if features.green_chromaticity <= 0.407 {
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
if features.red_chromaticity <= 0.304 {
if features.red_luminance <= 114 {
if features.luminance <= 130 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 72 {
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
if features.green_chromaticity <= 0.407 {
if features.green_chromaticity <= 0.406 {
if features.intensity <= 105 {
if features.intensity <= 102 {
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
if features.green_chromaticity <= 0.406 {
if features.red_luminance <= 99 {
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
}
} else {
if features.green_chromaticity <= 0.408 {
if features.green_luminance <= 128 {
if features.blue_luminance <= 88 {
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
if features.saturation <= 74 {
if features.red_difference <= 113 {
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
}
}
}
}
} else {
if features.red_chromaticity <= 0.303 {
if features.luminance <= 136 {
if features.green_chromaticity <= 0.409 {
if features.blue_luminance <= 96 {
Intensity::Low
} else {
if features.value <= 142 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.409 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.409 {
if features.blue_chromaticity <= 0.291 {
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
Intensity::Low
}
} else {
if features.hue <= 56 {
if features.blue_chromaticity <= 0.278 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.286 {
if features.blue_chromaticity <= 0.285 {
if features.blue_chromaticity <= 0.280 {
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
if features.red_difference <= 113 {
if features.green_luminance <= 144 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_difference <= 114 {
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
}
} else {
if features.blue_difference <= 114 {
if features.blue_chromaticity <= 0.299 {
if features.red_chromaticity <= 0.289 {
if features.green_chromaticity <= 0.412 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.298 {
Intensity::High
} else {
if features.value <= 169 {
if features.intensity <= 130 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.412 {
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
if features.green_chromaticity <= 0.412 {
if features.red_chromaticity <= 0.301 {
if features.red_difference <= 110 {
if features.blue_luminance <= 111 {
if features.saturation <= 73 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 175 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.value <= 140 {
if features.blue_chromaticity <= 0.289 {
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
}
} else {
if features.value <= 147 {
if features.blue_chromaticity <= 0.278 {
if features.green_chromaticity <= 0.410 {
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
if features.green_chromaticity <= 0.410 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.413 {
if features.green_chromaticity <= 0.413 {
if features.red_luminance <= 94 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.413 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_difference <= 109 {
if features.value <= 154 {
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
} else {
if features.hue <= 58 {
if features.red_chromaticity <= 0.301 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.306 {
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
if features.green_luminance <= 169 {
if features.saturation <= 76 {
if features.green_chromaticity <= 0.411 {
if features.red_chromaticity <= 0.290 {
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
if features.blue_chromaticity <= 0.307 {
if features.green_chromaticity <= 0.413 {
if features.green_luminance <= 150 {
if features.blue_chromaticity <= 0.301 {
if features.blue_chromaticity <= 0.300 {
if features.green_chromaticity <= 0.413 {
if features.blue_difference <= 115 {
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
if features.blue_chromaticity <= 0.301 {
if features.green_chromaticity <= 0.411 {
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
if features.red_chromaticity <= 0.284 {
if features.intensity <= 118 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.285 {
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
if features.red_luminance <= 111 {
if features.red_chromaticity <= 0.287 {
if features.red_chromaticity <= 0.286 {
if features.blue_difference <= 116 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.287 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.value <= 152 {
if features.red_luminance <= 106 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 155 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.value <= 167 {
if features.red_chromaticity <= 0.287 {
if features.saturation <= 77 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.hue <= 63 {
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
if features.blue_chromaticity <= 0.302 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.blue_luminance <= 123 {
if features.luminance <= 130 {
if features.green_chromaticity <= 0.411 {
if features.green_chromaticity <= 0.410 {
if features.red_chromaticity <= 0.278 {
if features.red_chromaticity <= 0.275 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 98 {
if features.intensity <= 112 {
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
Intensity::Low
}
} else {
if features.red_luminance <= 101 {
Intensity::High
} else {
if features.red_chromaticity <= 0.278 {
if features.blue_chromaticity <= 0.314 {
Intensity::Low
} else {
if features.green_luminance <= 156 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 108 {
if features.luminance <= 138 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.310 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.blue_luminance <= 129 {
if features.value <= 157 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 171 {
Intensity::Low
} else {
if features.intensity <= 140 {
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
if features.luminance <= 111 {
if features.luminance <= 104 {
if features.blue_difference <= 116 {
if features.green_luminance <= 113 {
if features.green_chromaticity <= 0.424 {
if features.green_chromaticity <= 0.418 {
if features.green_chromaticity <= 0.415 {
if features.blue_luminance <= 71 {
if features.red_luminance <= 77 {
if features.blue_difference <= 111 {
if features.green_chromaticity <= 0.414 {
Intensity::High
} else {
if features.red_luminance <= 75 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.value <= 97 {
if features.green_chromaticity <= 0.414 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.hue <= 52 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_difference <= 96 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.414 {
if features.saturation <= 83 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 75 {
if features.blue_chromaticity <= 0.271 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_difference <= 115 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_luminance <= 43 {
Intensity::High
} else {
if features.red_chromaticity <= 0.298 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.226 {
if features.green_chromaticity <= 0.418 {
if features.green_chromaticity <= 0.416 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.saturation <= 115 {
if features.blue_difference <= 108 {
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
if features.red_chromaticity <= 0.366 {
if features.red_chromaticity <= 0.366 {
if features.red_luminance <= 79 {
if features.hue <= 38 {
if features.saturation <= 123 {
Intensity::High
} else {
if features.value <= 81 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 56 {
if features.blue_luminance <= 42 {
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
}
} else {
if features.red_chromaticity <= 0.331 {
if features.red_chromaticity <= 0.331 {
if features.intensity <= 81 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.424 {
if features.red_luminance <= 80 {
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
if features.red_luminance <= 41 {
if features.red_luminance <= 39 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.422 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.423 {
Intensity::High
} else {
if features.green_chromaticity <= 0.423 {
if features.green_chromaticity <= 0.423 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.369 {
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
if features.intensity <= 80 {
if features.blue_luminance <= 29 {
Intensity::Low
} else {
if features.red_luminance <= 78 {
if features.luminance <= 90 {
if features.red_luminance <= 72 {
if features.blue_chromaticity <= 0.220 {
if features.green_chromaticity <= 0.424 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.saturation <= 86 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.428 {
if features.saturation <= 99 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_difference <= 121 {
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
} else {
if features.blue_chromaticity <= 0.239 {
if features.blue_luminance <= 56 {
Intensity::Low
} else {
if features.blue_luminance <= 57 {
if features.green_chromaticity <= 0.426 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_luminance <= 90 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.426 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
} else {
if features.saturation <= 97 {
if features.green_luminance <= 106 {
if features.saturation <= 89 {
if features.saturation <= 84 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.284 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.luminance <= 90 {
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
if features.red_luminance <= 75 {
if features.green_chromaticity <= 0.427 {
if features.saturation <= 85 {
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
} else {
if features.green_chromaticity <= 0.428 {
if features.red_luminance <= 80 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 110 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_chromaticity <= 0.424 {
if features.blue_luminance <= 65 {
if features.blue_luminance <= 64 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 110 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.311 {
Intensity::High
} else {
if features.blue_difference <= 108 {
if features.red_luminance <= 85 {
Intensity::High
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
}
}
}
}
} else {
if features.saturation <= 91 {
if features.green_chromaticity <= 0.425 {
if features.green_chromaticity <= 0.418 {
if features.blue_luminance <= 76 {
if features.blue_chromaticity <= 0.271 {
if features.green_chromaticity <= 0.416 {
if features.luminance <= 103 {
if features.blue_chromaticity <= 0.268 {
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
} else {
if features.red_luminance <= 86 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.415 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 83 {
if features.red_chromaticity <= 0.301 {
if features.red_chromaticity <= 0.292 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.416 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.278 {
Intensity::Low
} else {
if features.saturation <= 83 {
if features.green_luminance <= 116 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 117 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.saturation <= 79 {
if features.luminance <= 102 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 89 {
Intensity::Low
} else {
if features.luminance <= 99 {
Intensity::Low
} else {
if features.intensity <= 93 {
if features.blue_chromaticity <= 0.278 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.423 {
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
if features.green_chromaticity <= 0.426 {
if features.intensity <= 91 {
if features.red_luminance <= 78 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 93 {
if features.saturation <= 85 {
Intensity::Low
} else {
if features.blue_luminance <= 78 {
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
if features.red_difference <= 112 {
if features.blue_chromaticity <= 0.281 {
Intensity::High
} else {
if features.red_luminance <= 78 {
if features.intensity <= 90 {
if features.luminance <= 99 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 92 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 78 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.427 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_chromaticity <= 0.293 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.294 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.295 {
Intensity::Low
} else {
if features.red_luminance <= 81 {
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
if features.green_chromaticity <= 0.422 {
if features.blue_chromaticity <= 0.256 {
if features.green_chromaticity <= 0.415 {
if features.blue_chromaticity <= 0.255 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.value <= 116 {
if features.value <= 115 {
if features.red_chromaticity <= 0.335 {
if features.red_luminance <= 90 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 91 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.saturation <= 103 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.blue_luminance <= 70 {
Intensity::High
} else {
if features.green_chromaticity <= 0.418 {
if features.green_chromaticity <= 0.414 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.265 {
if features.hue <= 47 {
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
} else {
if features.red_luminance <= 89 {
if features.value <= 116 {
if features.blue_luminance <= 73 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.263 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.324 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.luminance <= 100 {
if features.green_chromaticity <= 0.425 {
if features.blue_chromaticity <= 0.270 {
if features.blue_luminance <= 71 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
if features.saturation <= 95 {
Intensity::Low
} else {
if features.intensity <= 88 {
if features.hue <= 50 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.hue <= 50 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.saturation <= 117 {
if features.green_luminance <= 114 {
Intensity::High
} else {
if features.green_chromaticity <= 0.426 {
if features.green_chromaticity <= 0.423 {
Intensity::High
} else {
if features.blue_luminance <= 71 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.323 {
if features.red_chromaticity <= 0.308 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 106 {
Intensity::High
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
}
}
} else {
if features.blue_difference <= 119 {
if features.green_chromaticity <= 0.421 {
if features.red_luminance <= 78 {
if features.red_chromaticity <= 0.274 {
Intensity::Low
} else {
if features.red_luminance <= 48 {
if features.red_chromaticity <= 0.317 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.420 {
if features.green_chromaticity <= 0.415 {
if features.green_chromaticity <= 0.415 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 44 {
if features.red_difference <= 124 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 50 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_chromaticity <= 0.415 {
if features.green_chromaticity <= 0.414 {
if features.intensity <= 76 {
if features.red_luminance <= 62 {
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
} else {
if features.luminance <= 76 {
if features.red_difference <= 119 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 80 {
if features.green_luminance <= 85 {
if features.green_chromaticity <= 0.415 {
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
if features.green_luminance <= 76 {
if features.saturation <= 85 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.420 {
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
if features.red_luminance <= 81 {
if features.green_chromaticity <= 0.414 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.292 {
if features.saturation <= 76 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.414 {
if features.red_chromaticity <= 0.285 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.417 {
if features.red_chromaticity <= 0.278 {
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
}
}
}
} else {
if features.blue_luminance <= 85 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_luminance <= 67 {
if features.blue_luminance <= 38 {
if features.green_luminance <= 65 {
if features.green_luminance <= 62 {
if features.red_chromaticity <= 0.343 {
if features.red_chromaticity <= 0.321 {
if features.saturation <= 104 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.233 {
if features.red_chromaticity <= 0.344 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 37 {
if features.blue_chromaticity <= 0.244 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.325 {
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
} else {
if features.saturation <= 91 {
if features.green_chromaticity <= 0.426 {
if features.blue_chromaticity <= 0.297 {
if features.blue_chromaticity <= 0.297 {
if features.green_chromaticity <= 0.426 {
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
if features.blue_luminance <= 62 {
if features.hue <= 61 {
if features.saturation <= 90 {
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
} else {
if features.green_chromaticity <= 0.427 {
if features.red_chromaticity <= 0.277 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 83 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_difference <= 120 {
if features.red_luminance <= 54 {
if features.blue_chromaticity <= 0.268 {
if features.saturation <= 95 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.hue <= 54 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.428 {
if features.intensity <= 66 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 93 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.intensity <= 59 {
if features.blue_luminance <= 39 {
if features.green_luminance <= 66 {
Intensity::Low
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
Intensity::Low
}
}
}
}
} else {
if features.red_chromaticity <= 0.265 {
if features.red_chromaticity <= 0.263 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_luminance <= 74 {
if features.red_chromaticity <= 0.274 {
if features.blue_luminance <= 84 {
if features.blue_luminance <= 80 {
if features.saturation <= 90 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.305 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_luminance <= 118 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 90 {
if features.green_chromaticity <= 0.425 {
if features.blue_luminance <= 76 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 77 {
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
if features.saturation <= 95 {
if features.red_chromaticity <= 0.279 {
if features.green_chromaticity <= 0.425 {
if features.blue_chromaticity <= 0.301 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 117 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.426 {
if features.luminance <= 97 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 88 {
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
if features.luminance <= 51 {
if features.blue_luminance <= 28 {
if features.red_difference <= 122 {
if features.blue_chromaticity <= 0.306 {
if features.green_chromaticity <= 0.425 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.298 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.428 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.333 {
if features.luminance <= 26 {
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
if features.green_luminance <= 42 {
if features.green_chromaticity <= 0.414 {
if features.blue_chromaticity <= 0.328 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.intensity <= 33 {
if features.blue_chromaticity <= 0.248 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 94 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_chromaticity <= 0.424 {
if features.luminance <= 44 {
if features.blue_luminance <= 43 {
if features.blue_luminance <= 34 {
if features.saturation <= 88 {
if features.blue_chromaticity <= 0.274 {
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
if features.blue_luminance <= 35 {
if features.green_chromaticity <= 0.418 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 49 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.207 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.224 {
if features.blue_chromaticity <= 0.368 {
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
if features.luminance <= 50 {
if features.green_chromaticity <= 0.423 {
if features.value <= 54 {
if features.value <= 53 {
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
} else {
if features.red_chromaticity <= 0.290 {
if features.blue_chromaticity <= 0.306 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.419 {
if features.blue_chromaticity <= 0.283 {
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
if features.blue_difference <= 125 {
if features.red_chromaticity <= 0.249 {
Intensity::Low
} else {
if features.red_difference <= 118 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.261 {
if features.saturation <= 105 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 54 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_chromaticity <= 0.424 {
if features.hue <= 78 {
Intensity::Low
} else {
if features.red_difference <= 118 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.427 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.427 {
if features.red_difference <= 118 {
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
if features.green_chromaticity <= 0.425 {
if features.blue_chromaticity <= 0.311 {
if features.blue_luminance <= 87 {
if features.green_chromaticity <= 0.419 {
if features.green_chromaticity <= 0.413 {
if features.green_chromaticity <= 0.413 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.299 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 58 {
if features.intensity <= 60 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.272 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_luminance <= 42 {
if features.red_luminance <= 39 {
Intensity::Low
} else {
if features.saturation <= 91 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 81 {
if features.blue_luminance <= 53 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.423 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.saturation <= 87 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_difference <= 120 {
if features.green_luminance <= 119 {
if features.blue_luminance <= 88 {
if features.intensity <= 92 {
if features.blue_chromaticity <= 0.313 {
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
} else {
if features.blue_chromaticity <= 0.324 {
if features.blue_chromaticity <= 0.316 {
if features.blue_chromaticity <= 0.315 {
if features.red_chromaticity <= 0.266 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.315 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 49 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.261 {
if features.green_chromaticity <= 0.420 {
if features.red_luminance <= 67 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 70 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.262 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.red_luminance <= 70 {
if features.luminance <= 102 {
if features.blue_chromaticity <= 0.310 {
if features.blue_luminance <= 48 {
if features.blue_chromaticity <= 0.295 {
if features.red_chromaticity <= 0.280 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.304 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.282 {
if features.green_chromaticity <= 0.427 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 66 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_difference <= 116 {
if features.green_luminance <= 90 {
if features.luminance <= 76 {
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
if features.luminance <= 103 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.318 {
Intensity::High
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
}
}
} else {
if features.green_chromaticity <= 0.423 {
if features.red_luminance <= 87 {
if features.green_chromaticity <= 0.422 {
if features.blue_difference <= 116 {
if features.red_difference <= 112 {
if features.green_chromaticity <= 0.418 {
if features.red_chromaticity <= 0.290 {
if features.red_difference <= 111 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.418 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.284 {
if features.green_chromaticity <= 0.421 {
Intensity::Low
} else {
if features.green_luminance <= 124 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 85 {
if features.value <= 123 {
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
}
} else {
if features.value <= 123 {
if features.red_chromaticity <= 0.294 {
if features.red_luminance <= 84 {
if features.saturation <= 79 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.292 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.293 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.287 {
if features.blue_chromaticity <= 0.282 {
if features.green_chromaticity <= 0.418 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 119 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 97 {
if features.value <= 121 {
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
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.261 {
if features.value <= 122 {
if features.blue_difference <= 123 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 65 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 98 {
if features.value <= 128 {
if features.red_chromaticity <= 0.262 {
if features.hue <= 71 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.308 {
if features.saturation <= 81 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 127 {
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
}
} else {
if features.saturation <= 99 {
if features.blue_luminance <= 89 {
if features.hue <= 62 {
if features.green_chromaticity <= 0.423 {
if features.blue_difference <= 113 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.291 {
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
if features.saturation <= 92 {
if features.luminance <= 109 {
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
} else {
if features.red_chromaticity <= 0.353 {
if features.green_chromaticity <= 0.419 {
if features.red_luminance <= 99 {
if features.red_luminance <= 92 {
if features.green_chromaticity <= 0.416 {
if features.red_chromaticity <= 0.296 {
if features.luminance <= 110 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.308 {
if features.red_chromaticity <= 0.305 {
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
if features.green_luminance <= 124 {
if features.green_chromaticity <= 0.419 {
if features.green_chromaticity <= 0.418 {
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
if features.red_chromaticity <= 0.301 {
if features.saturation <= 78 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 91 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.value <= 117 {
if features.blue_chromaticity <= 0.251 {
if features.red_luminance <= 98 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.414 {
Intensity::High
} else {
if features.blue_difference <= 108 {
if features.red_difference <= 120 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.value <= 125 {
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
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.243 {
Intensity::High
} else {
if features.red_chromaticity <= 0.314 {
if features.luminance <= 106 {
if features.red_chromaticity <= 0.309 {
Intensity::Low
} else {
if features.intensity <= 94 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.421 {
if features.green_chromaticity <= 0.420 {
if features.green_chromaticity <= 0.420 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.421 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.422 {
if features.blue_chromaticity <= 0.282 {
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
}
}
} else {
if features.red_chromaticity <= 0.329 {
if features.green_chromaticity <= 0.421 {
if features.red_luminance <= 94 {
if features.saturation <= 96 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.322 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.257 {
if features.luminance <= 109 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.luminance <= 106 {
if features.green_chromaticity <= 0.421 {
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
}
} else {
if features.blue_chromaticity <= 0.276 {
if features.green_luminance <= 124 {
if features.red_luminance <= 103 {
if features.blue_chromaticity <= 0.249 {
if features.red_difference <= 118 {
if features.saturation <= 108 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.428 {
if features.blue_luminance <= 70 {
if features.blue_luminance <= 64 {
Intensity::High
} else {
if features.green_luminance <= 118 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.green_luminance <= 123 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.227 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.311 {
if features.green_luminance <= 121 {
if features.hue <= 51 {
if features.green_chromaticity <= 0.428 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 90 {
Intensity::Low
} else {
if features.red_luminance <= 85 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.269 {
if features.red_chromaticity <= 0.306 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.274 {
if features.green_chromaticity <= 0.428 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 122 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.saturation <= 103 {
if features.green_chromaticity <= 0.426 {
if features.red_luminance <= 91 {
if features.red_chromaticity <= 0.323 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.423 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.257 {
if features.green_luminance <= 122 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 74 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.value <= 119 {
Intensity::High
} else {
if features.hue <= 48 {
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
} else {
if features.green_chromaticity <= 0.423 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 75 {
if features.blue_chromaticity <= 0.241 {
if features.red_luminance <= 99 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.luminance <= 110 {
if features.blue_difference <= 106 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.244 {
Intensity::High
} else {
if features.blue_luminance <= 73 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.green_luminance <= 127 {
if features.red_difference <= 116 {
if features.red_chromaticity <= 0.313 {
if features.green_chromaticity <= 0.425 {
if features.saturation <= 95 {
if features.blue_luminance <= 81 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.red_luminance <= 87 {
Intensity::High
} else {
if features.luminance <= 110 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.425 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.317 {
if features.green_chromaticity <= 0.426 {
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
Intensity::Low
}
} else {
Intensity::High
}
}
}
} else {
if features.red_difference <= 107 {
if features.blue_chromaticity <= 0.311 {
if features.blue_chromaticity <= 0.310 {
if features.red_chromaticity <= 0.266 {
if features.luminance <= 106 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_difference <= 118 {
if features.blue_chromaticity <= 0.305 {
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
if features.green_luminance <= 128 {
if features.green_luminance <= 123 {
if features.hue <= 66 {
if features.blue_difference <= 116 {
if features.green_chromaticity <= 0.425 {
if features.red_chromaticity <= 0.298 {
if features.luminance <= 106 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.428 {
if features.value <= 122 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.283 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.298 {
if features.blue_chromaticity <= 0.297 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.273 {
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
} else {
if features.hue <= 67 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.424 {
if features.hue <= 62 {
if features.red_chromaticity <= 0.299 {
if features.intensity <= 98 {
Intensity::Low
} else {
if features.value <= 126 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.423 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.269 {
Intensity::High
} else {
if features.green_chromaticity <= 0.426 {
if features.saturation <= 87 {
if features.blue_luminance <= 86 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.295 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 81 {
Intensity::Low
} else {
if features.saturation <= 85 {
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
if features.green_chromaticity <= 0.426 {
if features.blue_luminance <= 92 {
if features.saturation <= 90 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.277 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.289 {
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
if features.blue_difference <= 110 {
if features.green_chromaticity <= 0.421 {
if features.blue_chromaticity <= 0.239 {
if features.red_chromaticity <= 0.347 {
if features.blue_difference <= 102 {
Intensity::High
} else {
if features.saturation <= 111 {
if features.saturation <= 110 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_luminance <= 129 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.353 {
if features.red_chromaticity <= 0.353 {
if features.red_chromaticity <= 0.350 {
Intensity::Low
} else {
if features.intensity <= 112 {
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
}
} else {
if features.green_chromaticity <= 0.417 {
if features.blue_difference <= 108 {
if features.green_chromaticity <= 0.415 {
if features.value <= 165 {
if features.blue_luminance <= 94 {
if features.luminance <= 120 {
if features.green_chromaticity <= 0.414 {
if features.red_luminance <= 102 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.415 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.328 {
if features.saturation <= 94 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.251 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.310 {
if features.red_chromaticity <= 0.310 {
if features.red_chromaticity <= 0.308 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.265 {
if features.red_chromaticity <= 0.323 {
Intensity::High
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
if features.blue_chromaticity <= 0.273 {
if features.blue_chromaticity <= 0.259 {
if features.blue_luminance <= 108 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 130 {
if features.red_chromaticity <= 0.319 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.intensity <= 140 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.311 {
if features.blue_difference <= 105 {
Intensity::High
} else {
if features.green_luminance <= 185 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.275 {
if features.intensity <= 137 {
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
if features.blue_chromaticity <= 0.283 {
if features.luminance <= 147 {
if features.blue_luminance <= 109 {
if features.green_luminance <= 140 {
if features.luminance <= 123 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.325 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.hue <= 54 {
if features.green_luminance <= 165 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.blue_difference <= 107 {
if features.blue_chromaticity <= 0.279 {
if features.luminance <= 163 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.417 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.305 {
if features.saturation <= 81 {
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
if features.blue_luminance <= 121 {
if features.value <= 175 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.300 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.417 {
if features.green_luminance <= 189 {
if features.green_chromaticity <= 0.415 {
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
if features.green_chromaticity <= 0.416 {
if features.red_luminance <= 124 {
if features.blue_luminance <= 88 {
if features.blue_luminance <= 80 {
if features.hue <= 48 {
if features.red_luminance <= 99 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.value <= 127 {
Intensity::High
} else {
if features.red_chromaticity <= 0.325 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.415 {
if features.green_chromaticity <= 0.414 {
if features.red_difference <= 111 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.414 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 81 {
if features.red_chromaticity <= 0.300 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.275 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_luminance <= 132 {
if features.red_luminance <= 128 {
if features.red_luminance <= 127 {
if features.red_luminance <= 126 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 77 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.294 {
if features.saturation <= 75 {
Intensity::High
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
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.303 {
if features.red_chromaticity <= 0.299 {
if features.luminance <= 167 {
if features.blue_chromaticity <= 0.288 {
if features.blue_luminance <= 122 {
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
if features.red_chromaticity <= 0.300 {
if features.green_chromaticity <= 0.417 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.300 {
Intensity::High
} else {
if features.green_luminance <= 153 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_luminance <= 90 {
if features.blue_chromaticity <= 0.260 {
Intensity::High
} else {
if features.green_luminance <= 133 {
if features.value <= 130 {
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
} else {
if features.blue_chromaticity <= 0.275 {
if features.green_chromaticity <= 0.416 {
if features.red_luminance <= 109 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.273 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.value <= 145 {
Intensity::Low
} else {
if features.intensity <= 120 {
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
if features.blue_difference <= 108 {
if features.green_chromaticity <= 0.419 {
if features.green_chromaticity <= 0.417 {
if features.saturation <= 99 {
if features.green_chromaticity <= 0.417 {
if features.blue_luminance <= 95 {
if features.green_chromaticity <= 0.417 {
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
if features.green_chromaticity <= 0.417 {
if features.red_luminance <= 112 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.269 {
if features.green_luminance <= 140 {
if features.blue_luminance <= 81 {
if features.saturation <= 107 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.259 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_luminance <= 113 {
if features.red_luminance <= 141 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.418 {
if features.green_chromaticity <= 0.418 {
if features.red_luminance <= 125 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.hue <= 54 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.418 {
if features.blue_difference <= 106 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.luminance <= 155 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.blue_difference <= 103 {
if features.red_chromaticity <= 0.319 {
if features.red_chromaticity <= 0.319 {
if features.blue_difference <= 100 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.value <= 171 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.314 {
if features.green_luminance <= 175 {
if features.luminance <= 146 {
if features.luminance <= 139 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.311 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.419 {
if features.red_chromaticity <= 0.300 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_luminance <= 129 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.blue_luminance <= 103 {
if features.green_luminance <= 146 {
if features.red_luminance <= 105 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.255 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.420 {
if features.value <= 164 {
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
if features.value <= 136 {
if features.blue_chromaticity <= 0.259 {
Intensity::High
} else {
if features.green_luminance <= 128 {
if features.blue_chromaticity <= 0.260 {
if features.saturation <= 96 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 127 {
Intensity::Low
} else {
if features.red_luminance <= 97 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_luminance <= 96 {
Intensity::High
} else {
if features.red_chromaticity <= 0.317 {
if features.blue_chromaticity <= 0.268 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 117 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.blue_luminance <= 97 {
if features.red_luminance <= 107 {
if features.green_chromaticity <= 0.419 {
if features.red_chromaticity <= 0.315 {
if features.red_chromaticity <= 0.309 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.309 {
if features.green_chromaticity <= 0.420 {
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
if features.red_chromaticity <= 0.311 {
if features.red_luminance <= 108 {
if features.saturation <= 89 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 148 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.418 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.blue_luminance <= 118 {
if features.value <= 167 {
if features.green_chromaticity <= 0.418 {
if features.green_chromaticity <= 0.417 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_difference <= 107 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.295 {
if features.red_chromaticity <= 0.293 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 114 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.420 {
if features.red_chromaticity <= 0.291 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.290 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.421 {
if features.luminance <= 159 {
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
}
} else {
if features.blue_chromaticity <= 0.223 {
if features.blue_difference <= 99 {
if features.red_difference <= 123 {
if features.green_luminance <= 136 {
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
} else {
if features.luminance <= 120 {
if features.green_chromaticity <= 0.426 {
if features.blue_difference <= 108 {
if features.saturation <= 96 {
if features.luminance <= 119 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 103 {
if features.blue_chromaticity <= 0.256 {
if features.green_chromaticity <= 0.425 {
if features.blue_difference <= 106 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.254 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.saturation <= 100 {
if features.luminance <= 114 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 82 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.422 {
if features.green_chromaticity <= 0.422 {
Intensity::High
} else {
if features.red_luminance <= 102 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.426 {
if features.hue <= 46 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.hue <= 44 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.green_luminance <= 133 {
if features.red_chromaticity <= 0.317 {
if features.red_chromaticity <= 0.314 {
if features.red_luminance <= 93 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.267 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.315 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 79 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.saturation <= 91 {
if features.red_chromaticity <= 0.307 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.luminance <= 119 {
if features.red_chromaticity <= 0.307 {
if features.green_luminance <= 134 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 135 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_luminance <= 137 {
if features.green_chromaticity <= 0.425 {
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
if features.red_luminance <= 99 {
if features.red_chromaticity <= 0.304 {
if features.green_chromaticity <= 0.427 {
Intensity::High
} else {
if features.red_chromaticity <= 0.302 {
if features.red_chromaticity <= 0.300 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 84 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_luminance <= 97 {
if features.red_difference <= 113 {
if features.blue_chromaticity <= 0.265 {
Intensity::High
} else {
if features.saturation <= 94 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.264 {
if features.red_chromaticity <= 0.314 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 81 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.427 {
if features.value <= 129 {
Intensity::High
} else {
if features.green_chromaticity <= 0.427 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_difference <= 112 {
Intensity::High
} else {
if features.blue_difference <= 105 {
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
if features.blue_difference <= 106 {
Intensity::High
} else {
if features.red_difference <= 114 {
Intensity::High
} else {
if features.green_chromaticity <= 0.427 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.428 {
if features.blue_chromaticity <= 0.240 {
if features.value <= 127 {
Intensity::High
} else {
if features.red_luminance <= 106 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.328 {
if features.saturation <= 108 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.value <= 129 {
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
if features.green_chromaticity <= 0.424 {
if features.blue_difference <= 108 {
if features.blue_luminance <= 116 {
if features.red_chromaticity <= 0.326 {
if features.red_chromaticity <= 0.324 {
if features.intensity <= 126 {
if features.blue_luminance <= 99 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.422 {
Intensity::High
} else {
Intensity::High
}
}
} else {
Intensity::High
}
} else {
if features.blue_difference <= 100 {
if features.green_chromaticity <= 0.422 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.241 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.246 {
if features.luminance <= 122 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.423 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.green_luminance <= 182 {
if features.blue_chromaticity <= 0.278 {
if features.blue_chromaticity <= 0.272 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.304 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.293 {
if features.red_chromaticity <= 0.292 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 121 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.305 {
if features.red_chromaticity <= 0.303 {
Intensity::High
} else {
if features.green_chromaticity <= 0.422 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.intensity <= 144 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.red_luminance <= 100 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.421 {
if features.value <= 163 {
if features.green_chromaticity <= 0.421 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.275 {
Intensity::High
} else {
Intensity::High
}
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.423 {
if features.blue_chromaticity <= 0.288 {
if features.value <= 139 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.421 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_difference <= 108 {
if features.red_luminance <= 116 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.luminance <= 136 {
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
if features.blue_difference <= 108 {
if features.intensity <= 140 {
if features.green_chromaticity <= 0.425 {
if features.red_luminance <= 110 {
if features.green_chromaticity <= 0.424 {
if features.value <= 140 {
Intensity::High
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
if features.red_chromaticity <= 0.293 {
if features.red_chromaticity <= 0.291 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_difference <= 99 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.red_luminance <= 101 {
if features.red_chromaticity <= 0.307 {
if features.luminance <= 122 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.intensity <= 107 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_difference <= 102 {
if features.green_chromaticity <= 0.426 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.324 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.green_chromaticity <= 0.426 {
if features.intensity <= 150 {
if features.green_chromaticity <= 0.424 {
if features.saturation <= 88 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.luminance <= 157 {
Intensity::High
} else {
Intensity::High
}
}
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.273 {
if features.green_chromaticity <= 0.427 {
if features.red_luminance <= 133 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.300 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.green_luminance <= 181 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.red_difference <= 108 {
if features.red_luminance <= 118 {
if features.red_chromaticity <= 0.293 {
if features.red_luminance <= 110 {
if features.red_luminance <= 108 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.value <= 169 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.424 {
Intensity::High
} else {
if features.red_luminance <= 109 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.hue <= 59 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.280 {
if features.blue_luminance <= 91 {
if features.value <= 138 {
Intensity::High
} else {
if features.saturation <= 90 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.302 {
if features.saturation <= 88 {
Intensity::High
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
} else {
if features.green_chromaticity <= 0.425 {
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
if features.blue_difference <= 112 {
if features.green_chromaticity <= 0.420 {
if features.green_chromaticity <= 0.417 {
if features.blue_difference <= 111 {
if features.green_chromaticity <= 0.415 {
if features.red_chromaticity <= 0.304 {
if features.red_chromaticity <= 0.300 {
if features.red_chromaticity <= 0.293 {
Intensity::Low
} else {
if features.red_difference <= 109 {
if features.value <= 172 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.intensity <= 127 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_luminance <= 102 {
Intensity::Low
} else {
if features.blue_luminance <= 108 {
if features.green_luminance <= 153 {
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
if features.green_chromaticity <= 0.414 {
if features.red_chromaticity <= 0.308 {
if features.red_chromaticity <= 0.306 {
if features.hue <= 54 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 143 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 97 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.414 {
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
if features.saturation <= 85 {
if features.intensity <= 136 {
if features.red_luminance <= 118 {
if features.green_chromaticity <= 0.415 {
Intensity::High
} else {
if features.luminance <= 132 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.luminance <= 157 {
if features.value <= 177 {
if features.blue_luminance <= 124 {
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
if features.saturation <= 88 {
if features.blue_luminance <= 89 {
if features.green_chromaticity <= 0.415 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.273 {
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
if features.value <= 135 {
if features.green_luminance <= 128 {
if features.green_chromaticity <= 0.416 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 84 {
if features.red_luminance <= 97 {
if features.green_chromaticity <= 0.416 {
if features.red_luminance <= 96 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.281 {
Intensity::Low
} else {
if features.red_luminance <= 98 {
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
if features.green_chromaticity <= 0.416 {
if features.blue_chromaticity <= 0.292 {
if features.red_chromaticity <= 0.300 {
if features.red_chromaticity <= 0.298 {
if features.red_difference <= 110 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 98 {
if features.red_luminance <= 101 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.green_luminance <= 176 {
if features.red_luminance <= 121 {
if features.blue_chromaticity <= 0.295 {
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
if features.green_chromaticity <= 0.417 {
if features.blue_luminance <= 123 {
if features.red_chromaticity <= 0.291 {
if features.green_chromaticity <= 0.416 {
Intensity::Low
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
} else {
if features.red_chromaticity <= 0.287 {
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
}
}
} else {
if features.value <= 171 {
if features.luminance <= 136 {
if features.blue_difference <= 111 {
if features.green_chromaticity <= 0.418 {
if features.value <= 141 {
if features.red_chromaticity <= 0.307 {
if features.intensity <= 109 {
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
} else {
if features.value <= 142 {
Intensity::High
} else {
if features.green_chromaticity <= 0.417 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.419 {
if features.intensity <= 108 {
Intensity::High
} else {
if features.red_difference <= 111 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.value <= 139 {
if features.red_difference <= 114 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.298 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_chromaticity <= 0.418 {
if features.red_difference <= 113 {
if features.red_chromaticity <= 0.302 {
if features.intensity <= 122 {
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
} else {
if features.red_chromaticity <= 0.295 {
if features.blue_luminance <= 104 {
if features.red_luminance <= 103 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.luminance <= 134 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_luminance <= 140 {
if features.green_chromaticity <= 0.419 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 99 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.blue_luminance <= 117 {
if features.green_chromaticity <= 0.419 {
if features.red_luminance <= 112 {
if features.red_chromaticity <= 0.291 {
if features.luminance <= 140 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_difference <= 108 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 115 {
if features.red_chromaticity <= 0.287 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.418 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.hue <= 60 {
if features.blue_chromaticity <= 0.288 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 80 {
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
if features.intensity <= 139 {
if features.blue_chromaticity <= 0.293 {
if features.luminance <= 150 {
Intensity::Low
} else {
if features.saturation <= 79 {
if features.red_chromaticity <= 0.290 {
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
if features.intensity <= 141 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_chromaticity <= 0.423 {
if features.intensity <= 105 {
if features.red_luminance <= 92 {
if features.blue_chromaticity <= 0.278 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.421 {
if features.blue_chromaticity <= 0.279 {
if features.blue_difference <= 111 {
if features.value <= 132 {
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
if features.green_luminance <= 130 {
if features.red_luminance <= 93 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.luminance <= 115 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.301 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.blue_difference <= 111 {
if features.red_chromaticity <= 0.288 {
if features.luminance <= 148 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.299 {
if features.red_chromaticity <= 0.293 {
if features.luminance <= 146 {
if features.green_luminance <= 158 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.420 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.293 {
Intensity::High
} else {
if features.red_chromaticity <= 0.294 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.303 {
if features.red_chromaticity <= 0.301 {
if features.saturation <= 86 {
Intensity::Low
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
if features.green_luminance <= 136 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
} else {
if features.blue_chromaticity <= 0.291 {
if features.saturation <= 81 {
if features.red_chromaticity <= 0.291 {
if features.blue_chromaticity <= 0.288 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.289 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_difference <= 109 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.421 {
if features.red_chromaticity <= 0.299 {
if features.blue_chromaticity <= 0.282 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.295 {
if features.red_chromaticity <= 0.294 {
Intensity::High
} else {
Intensity::Low
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
} else {
if features.red_difference <= 103 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.421 {
Intensity::High
} else {
if features.intensity <= 128 {
if features.green_chromaticity <= 0.422 {
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
if features.luminance <= 123 {
if features.green_chromaticity <= 0.428 {
if features.saturation <= 87 {
if features.blue_chromaticity <= 0.279 {
Intensity::High
} else {
if features.blue_luminance <= 88 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.281 {
if features.hue <= 56 {
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
}
} else {
if features.blue_luminance <= 90 {
if features.red_luminance <= 95 {
if features.blue_luminance <= 84 {
if features.saturation <= 92 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.saturation <= 90 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.hue <= 55 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_luminance <= 141 {
if features.hue <= 56 {
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
if features.red_difference <= 111 {
if features.saturation <= 90 {
if features.hue <= 58 {
if features.red_chromaticity <= 0.291 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.279 {
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
}
} else {
if features.green_chromaticity <= 0.427 {
if features.red_luminance <= 101 {
if features.green_chromaticity <= 0.424 {
if features.blue_luminance <= 97 {
Intensity::High
} else {
if features.intensity <= 114 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.425 {
if features.hue <= 57 {
Intensity::High
} else {
if features.red_luminance <= 100 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.288 {
if features.red_chromaticity <= 0.286 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.luminance <= 125 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.blue_difference <= 111 {
if features.value <= 173 {
if features.red_chromaticity <= 0.286 {
Intensity::High
} else {
if features.green_chromaticity <= 0.423 {
Intensity::High
} else {
Intensity::High
}
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.282 {
if features.green_chromaticity <= 0.426 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 156 {
if features.red_luminance <= 102 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.intensity <= 123 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
}
} else {
if features.blue_luminance <= 105 {
if features.red_chromaticity <= 0.285 {
if features.red_luminance <= 100 {
if features.red_difference <= 106 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 101 {
if features.luminance <= 126 {
if features.intensity <= 111 {
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
} else {
Intensity::High
}
}
} else {
if features.saturation <= 84 {
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
if features.blue_difference <= 115 {
if features.green_chromaticity <= 0.419 {
if features.green_chromaticity <= 0.416 {
if features.red_luminance <= 114 {
if features.green_luminance <= 153 {
if features.blue_luminance <= 90 {
if features.blue_chromaticity <= 0.283 {
if features.red_luminance <= 95 {
Intensity::Low
} else {
if features.blue_luminance <= 89 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.285 {
if features.red_luminance <= 92 {
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
}
} else {
if features.saturation <= 79 {
if features.red_chromaticity <= 0.288 {
if features.red_chromaticity <= 0.287 {
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
if features.green_chromaticity <= 0.415 {
if features.intensity <= 112 {
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
if features.green_luminance <= 158 {
if features.blue_luminance <= 110 {
if features.green_luminance <= 154 {
Intensity::Low
} else {
if features.luminance <= 136 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_luminance <= 154 {
Intensity::High
} else {
if features.intensity <= 126 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.415 {
if features.green_luminance <= 166 {
if features.blue_luminance <= 114 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.416 {
if features.green_luminance <= 164 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.283 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.blue_luminance <= 137 {
if features.red_chromaticity <= 0.288 {
if features.blue_luminance <= 125 {
if features.blue_luminance <= 121 {
if features.blue_luminance <= 120 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 168 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.414 {
if features.red_luminance <= 119 {
if features.red_difference <= 106 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_difference <= 106 {
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
if features.blue_difference <= 114 {
if features.saturation <= 79 {
if features.intensity <= 124 {
if features.green_chromaticity <= 0.418 {
if features.green_chromaticity <= 0.416 {
if features.hue <= 58 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 153 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.289 {
if features.red_luminance <= 102 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 108 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.288 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.418 {
if features.green_chromaticity <= 0.417 {
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
if features.blue_luminance <= 116 {
if features.red_chromaticity <= 0.283 {
Intensity::High
} else {
if features.intensity <= 102 {
if features.blue_chromaticity <= 0.282 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.281 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.418 {
if features.red_chromaticity <= 0.286 {
if features.green_chromaticity <= 0.416 {
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
if features.red_chromaticity <= 0.284 {
if features.luminance <= 147 {
if features.green_luminance <= 166 {
if features.green_chromaticity <= 0.417 {
if features.green_chromaticity <= 0.417 {
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
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
if features.saturation <= 78 {
if features.red_chromaticity <= 0.293 {
if features.green_chromaticity <= 0.417 {
if features.saturation <= 77 {
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
if features.red_chromaticity <= 0.285 {
if features.red_chromaticity <= 0.284 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.418 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 113 {
if features.red_chromaticity <= 0.289 {
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
}
}
}
}
}
} else {
if features.green_chromaticity <= 0.423 {
if features.red_luminance <= 96 {
if features.red_difference <= 109 {
if features.green_chromaticity <= 0.421 {
if features.red_luminance <= 95 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 86 {
if features.green_chromaticity <= 0.422 {
if features.saturation <= 83 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.423 {
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
if features.value <= 137 {
if features.red_chromaticity <= 0.286 {
if features.red_luminance <= 87 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 90 {
if features.green_chromaticity <= 0.420 {
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
}
} else {
if features.green_chromaticity <= 0.422 {
if features.green_chromaticity <= 0.420 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 138 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.red_chromaticity <= 0.279 {
if features.red_chromaticity <= 0.274 {
Intensity::High
} else {
if features.blue_difference <= 113 {
Intensity::High
} else {
if features.value <= 166 {
if features.blue_difference <= 114 {
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
if features.blue_luminance <= 113 {
if features.green_chromaticity <= 0.421 {
if features.value <= 140 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.291 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 98 {
if features.saturation <= 82 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.282 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.420 {
if features.green_chromaticity <= 0.420 {
Intensity::High
} else {
if features.red_chromaticity <= 0.283 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.intensity <= 127 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.red_difference <= 104 {
if features.red_difference <= 101 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.278 {
if features.green_chromaticity <= 0.424 {
Intensity::Low
} else {
if features.value <= 152 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.297 {
if features.blue_luminance <= 113 {
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
if features.green_chromaticity <= 0.424 {
if features.red_chromaticity <= 0.280 {
if features.value <= 154 {
if features.green_luminance <= 150 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.424 {
if features.blue_difference <= 113 {
if features.blue_chromaticity <= 0.286 {
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
if features.green_chromaticity <= 0.427 {
if features.value <= 136 {
if features.red_luminance <= 91 {
if features.saturation <= 86 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 90 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.value <= 139 {
if features.red_chromaticity <= 0.278 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_difference <= 114 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.278 {
if features.red_luminance <= 96 {
if features.green_chromaticity <= 0.427 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.saturation <= 84 {
if features.red_chromaticity <= 0.286 {
Intensity::Low
} else {
Intensity::Low
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
}
}
}
}
} else {
if features.blue_chromaticity <= 0.311 {
if features.green_chromaticity <= 0.417 {
if features.value <= 153 {
if features.red_chromaticity <= 0.277 {
if features.blue_chromaticity <= 0.310 {
Intensity::Low
} else {
if features.green_luminance <= 131 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 84 {
if features.red_luminance <= 87 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.415 {
if features.red_chromaticity <= 0.291 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 102 {
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
if features.green_chromaticity <= 0.415 {
if features.red_chromaticity <= 0.280 {
if features.blue_luminance <= 117 {
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
if features.red_difference <= 102 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.302 {
if features.red_chromaticity <= 0.277 {
if features.red_luminance <= 88 {
if features.luminance <= 118 {
if features.blue_chromaticity <= 0.302 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.saturation <= 88 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.luminance <= 115 {
if features.green_chromaticity <= 0.417 {
if features.green_luminance <= 129 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.278 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.281 {
if features.green_chromaticity <= 0.418 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.282 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_chromaticity <= 0.417 {
if features.blue_chromaticity <= 0.304 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 95 {
if features.value <= 130 {
if features.saturation <= 86 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.red_luminance <= 83 {
Intensity::High
} else {
if features.green_chromaticity <= 0.424 {
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
if features.blue_luminance <= 97 {
if features.red_chromaticity <= 0.261 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.intensity <= 143 {
if features.red_chromaticity <= 0.275 {
if features.value <= 130 {
if features.blue_difference <= 120 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 102 {
if features.green_chromaticity <= 0.427 {
if features.value <= 136 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 101 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.312 {
if features.blue_chromaticity <= 0.312 {
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
if features.green_chromaticity <= 0.465 {
if features.blue_difference <= 111 {
if features.value <= 128 {
if features.green_chromaticity <= 0.447 {
if features.value <= 120 {
if features.green_chromaticity <= 0.437 {
if features.blue_luminance <= 52 {
if features.red_chromaticity <= 0.350 {
if features.red_chromaticity <= 0.330 {
if features.green_chromaticity <= 0.437 {
Intensity::Low
} else {
if features.red_luminance <= 70 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.430 {
if features.red_chromaticity <= 0.346 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.347 {
if features.red_luminance <= 78 {
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
if features.red_difference <= 125 {
if features.green_chromaticity <= 0.430 {
Intensity::High
} else {
if features.saturation <= 144 {
if features.green_luminance <= 105 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.luminance <= 61 {
if features.red_luminance <= 59 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 37 {
if features.saturation <= 135 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 91 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.red_luminance <= 87 {
if features.green_chromaticity <= 0.436 {
if features.green_luminance <= 106 {
if features.green_chromaticity <= 0.432 {
if features.red_chromaticity <= 0.330 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 80 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.green_luminance <= 107 {
if features.blue_difference <= 110 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_difference <= 108 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_luminance <= 80 {
if features.saturation <= 115 {
if features.green_chromaticity <= 0.436 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.304 {
if features.green_chromaticity <= 0.437 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.315 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
} else {
if features.green_chromaticity <= 0.435 {
if features.red_luminance <= 92 {
if features.red_luminance <= 91 {
if features.value <= 117 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.434 {
if features.red_chromaticity <= 0.342 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.328 {
Intensity::High
} else {
if features.intensity <= 89 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.234 {
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
if features.red_chromaticity <= 0.371 {
if features.green_chromaticity <= 0.441 {
if features.red_luminance <= 84 {
if features.red_luminance <= 73 {
if features.red_luminance <= 65 {
if features.blue_chromaticity <= 0.195 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 39 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.230 {
if features.green_chromaticity <= 0.439 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.305 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.red_luminance <= 90 {
if features.red_chromaticity <= 0.334 {
if features.green_luminance <= 114 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.luminance <= 104 {
if features.red_chromaticity <= 0.348 {
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
if features.luminance <= 99 {
if features.blue_chromaticity <= 0.262 {
if features.intensity <= 86 {
if features.red_chromaticity <= 0.326 {
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
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 68 {
if features.green_chromaticity <= 0.445 {
if features.red_chromaticity <= 0.336 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 87 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.301 {
if features.saturation <= 103 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.254 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.green_chromaticity <= 0.444 {
if features.green_chromaticity <= 0.438 {
if features.red_chromaticity <= 0.398 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.luminance <= 46 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.182 {
if features.green_chromaticity <= 0.444 {
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
if features.luminance <= 107 {
if features.blue_chromaticity <= 0.250 {
if features.saturation <= 111 {
if features.green_chromaticity <= 0.440 {
if features.green_chromaticity <= 0.435 {
Intensity::High
} else {
if features.red_chromaticity <= 0.316 {
Intensity::High
} else {
Intensity::High
}
}
} else {
Intensity::High
}
} else {
if features.saturation <= 116 {
if features.green_chromaticity <= 0.446 {
if features.intensity <= 90 {
Intensity::High
} else {
if features.saturation <= 112 {
Intensity::High
} else {
Intensity::High
}
}
} else {
Intensity::High
}
} else {
if features.red_luminance <= 95 {
if features.red_chromaticity <= 0.335 {
if features.red_chromaticity <= 0.332 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.value <= 121 {
if features.intensity <= 92 {
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
if features.green_chromaticity <= 0.437 {
if features.red_luminance <= 84 {
if features.red_chromaticity <= 0.299 {
if features.blue_chromaticity <= 0.268 {
if features.blue_chromaticity <= 0.267 {
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
if features.red_luminance <= 85 {
if features.green_chromaticity <= 0.435 {
if features.green_chromaticity <= 0.433 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.saturation <= 103 {
if features.blue_luminance <= 73 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.312 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_chromaticity <= 0.307 {
if features.red_luminance <= 83 {
if features.green_chromaticity <= 0.445 {
if features.green_chromaticity <= 0.443 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 72 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.438 {
if features.green_chromaticity <= 0.437 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.440 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.307 {
if features.green_chromaticity <= 0.440 {
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
if features.saturation <= 103 {
if features.green_chromaticity <= 0.432 {
if features.red_chromaticity <= 0.298 {
if features.red_chromaticity <= 0.298 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 90 {
if features.green_chromaticity <= 0.431 {
if features.red_chromaticity <= 0.299 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.431 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.430 {
if features.blue_difference <= 108 {
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
if features.green_luminance <= 127 {
if features.green_chromaticity <= 0.443 {
if features.green_chromaticity <= 0.437 {
if features.blue_chromaticity <= 0.259 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 97 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_difference <= 109 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.261 {
Intensity::High
} else {
if features.green_chromaticity <= 0.437 {
if features.green_chromaticity <= 0.435 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.286 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.red_difference <= 124 {
if features.saturation <= 111 {
if features.green_chromaticity <= 0.431 {
if features.red_chromaticity <= 0.319 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.430 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.blue_luminance <= 74 {
if features.saturation <= 104 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.260 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.437 {
if features.red_difference <= 116 {
if features.blue_chromaticity <= 0.241 {
Intensity::Low
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
} else {
if features.green_chromaticity <= 0.445 {
if features.green_chromaticity <= 0.438 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.luminance <= 109 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.red_chromaticity <= 0.376 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.saturation <= 117 {
if features.green_chromaticity <= 0.456 {
if features.luminance <= 106 {
if features.saturation <= 111 {
if features.blue_chromaticity <= 0.264 {
if features.red_chromaticity <= 0.292 {
if features.blue_difference <= 109 {
if features.green_luminance <= 122 {
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
} else {
if features.green_chromaticity <= 0.448 {
if features.blue_luminance <= 66 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_luminance <= 118 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.276 {
if features.value <= 126 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.456 {
if features.green_chromaticity <= 0.448 {
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
if features.blue_difference <= 110 {
if features.green_chromaticity <= 0.453 {
if features.red_chromaticity <= 0.299 {
if features.red_chromaticity <= 0.294 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.300 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.454 {
if features.red_difference <= 112 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.454 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.hue <= 51 {
if features.blue_chromaticity <= 0.245 {
Intensity::High
} else {
if features.green_chromaticity <= 0.448 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.saturation <= 112 {
if features.hue <= 54 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.448 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
}
} else {
if features.blue_luminance <= 77 {
if features.saturation <= 105 {
if features.red_chromaticity <= 0.283 {
if features.green_chromaticity <= 0.451 {
if features.blue_chromaticity <= 0.270 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.value <= 127 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.450 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.305 {
if features.hue <= 53 {
if features.red_chromaticity <= 0.300 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.452 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_luminance <= 126 {
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
if features.green_chromaticity <= 0.459 {
if features.green_chromaticity <= 0.457 {
if features.red_luminance <= 75 {
if features.red_luminance <= 73 {
if features.green_luminance <= 114 {
if features.value <= 109 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.intensity <= 87 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.252 {
if features.intensity <= 89 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.456 {
if features.luminance <= 100 {
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
if features.blue_luminance <= 67 {
if features.intensity <= 85 {
if features.saturation <= 114 {
if features.green_chromaticity <= 0.459 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.250 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.luminance <= 99 {
Intensity::High
} else {
if features.intensity <= 88 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.luminance <= 103 {
if features.blue_luminance <= 68 {
if features.blue_chromaticity <= 0.259 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.intensity <= 88 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.282 {
if features.red_luminance <= 76 {
Intensity::High
} else {
Intensity::High
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
if features.red_chromaticity <= 0.287 {
if features.value <= 114 {
if features.red_chromaticity <= 0.274 {
Intensity::High
} else {
if features.luminance <= 91 {
if features.hue <= 56 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.283 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.464 {
if features.red_luminance <= 75 {
if features.saturation <= 107 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.255 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_luminance <= 67 {
if features.luminance <= 99 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 69 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.value <= 127 {
if features.saturation <= 115 {
Intensity::High
} else {
if features.green_chromaticity <= 0.462 {
if features.red_luminance <= 74 {
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
}
}
} else {
if features.red_chromaticity <= 0.353 {
if features.blue_difference <= 105 {
if features.green_chromaticity <= 0.458 {
if features.luminance <= 107 {
if features.blue_chromaticity <= 0.212 {
if features.blue_chromaticity <= 0.208 {
if features.value <= 100 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.340 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.229 {
if features.green_chromaticity <= 0.450 {
Intensity::High
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
if features.green_chromaticity <= 0.447 {
if features.blue_luminance <= 64 {
Intensity::High
} else {
if features.red_difference <= 114 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.240 {
if features.red_chromaticity <= 0.312 {
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
if features.red_luminance <= 84 {
if features.hue <= 51 {
if features.red_chromaticity <= 0.326 {
if features.red_chromaticity <= 0.321 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 74 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.239 {
if features.green_chromaticity <= 0.464 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 123 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.intensity <= 83 {
if features.red_luminance <= 85 {
if features.red_difference <= 118 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.luminance <= 108 {
if features.red_chromaticity <= 0.325 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.324 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.green_chromaticity <= 0.455 {
if features.luminance <= 90 {
if features.saturation <= 121 {
if features.blue_chromaticity <= 0.239 {
if features.value <= 93 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.243 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.451 {
if features.red_luminance <= 74 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.saturation <= 123 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.intensity <= 85 {
if features.green_chromaticity <= 0.450 {
if features.blue_chromaticity <= 0.241 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.luminance <= 97 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 79 {
Intensity::High
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
if features.green_luminance <= 79 {
if features.green_chromaticity <= 0.463 {
if features.red_luminance <= 51 {
if features.red_luminance <= 50 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 144 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.hue <= 47 {
if features.saturation <= 140 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.460 {
if features.red_luminance <= 81 {
if features.red_chromaticity <= 0.296 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 82 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.298 {
if features.red_chromaticity <= 0.298 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_difference <= 114 {
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
if features.red_chromaticity <= 0.370 {
if features.blue_chromaticity <= 0.185 {
if features.red_chromaticity <= 0.362 {
if features.hue <= 40 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.456 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.175 {
if features.red_luminance <= 51 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.368 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
} else {
if features.red_chromaticity <= 0.362 {
if features.value <= 111 {
if features.red_luminance <= 61 {
if features.saturation <= 150 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.187 {
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
} else {
if features.green_chromaticity <= 0.448 {
Intensity::High
} else {
if features.red_chromaticity <= 0.390 {
if features.blue_chromaticity <= 0.154 {
Intensity::High
} else {
if features.value <= 67 {
Intensity::High
} else {
if features.green_chromaticity <= 0.450 {
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
if features.blue_difference <= 107 {
if features.red_chromaticity <= 0.364 {
if features.green_chromaticity <= 0.434 {
if features.green_luminance <= 179 {
if features.red_luminance <= 99 {
if features.blue_luminance <= 82 {
if features.saturation <= 108 {
if features.saturation <= 106 {
if features.saturation <= 103 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.430 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.431 {
if features.red_difference <= 116 {
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
} else {
if features.green_chromaticity <= 0.432 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.430 {
if features.blue_chromaticity <= 0.261 {
if features.blue_luminance <= 76 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.249 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.429 {
if features.luminance <= 144 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 91 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.292 {
if features.blue_chromaticity <= 0.274 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.433 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.intensity <= 107 {
if features.blue_chromaticity <= 0.237 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.value <= 156 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.red_chromaticity <= 0.301 {
if features.green_chromaticity <= 0.431 {
if features.blue_chromaticity <= 0.270 {
if features.value <= 182 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.431 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.432 {
Intensity::High
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
Intensity::High
}
}
} else {
if features.green_luminance <= 133 {
if features.saturation <= 117 {
if features.green_chromaticity <= 0.441 {
if features.intensity <= 98 {
if features.green_chromaticity <= 0.437 {
if features.red_chromaticity <= 0.314 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.440 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.322 {
if features.green_chromaticity <= 0.435 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 72 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_luminance <= 129 {
if features.hue <= 49 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.253 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.243 {
Intensity::High
} else {
if features.red_chromaticity <= 0.283 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.saturation <= 123 {
if features.blue_luminance <= 67 {
if features.blue_chromaticity <= 0.233 {
Intensity::High
} else {
if features.luminance <= 109 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.450 {
if features.value <= 131 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.457 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.436 {
if features.red_difference <= 119 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.301 {
if features.green_chromaticity <= 0.464 {
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
}
}
} else {
if features.blue_difference <= 105 {
if features.green_chromaticity <= 0.447 {
if features.red_chromaticity <= 0.329 {
if features.blue_difference <= 104 {
if features.red_chromaticity <= 0.309 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.312 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.intensity <= 100 {
if features.luminance <= 116 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.value <= 145 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.283 {
if features.intensity <= 106 {
if features.green_chromaticity <= 0.464 {
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
if features.green_chromaticity <= 0.457 {
if features.blue_chromaticity <= 0.218 {
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
}
}
} else {
if features.red_difference <= 110 {
if features.blue_chromaticity <= 0.279 {
if features.red_chromaticity <= 0.291 {
if features.red_difference <= 101 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.value <= 140 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.436 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.blue_difference <= 106 {
if features.green_chromaticity <= 0.443 {
if features.luminance <= 119 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.444 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.437 {
if features.green_chromaticity <= 0.435 {
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
}
}
}
} else {
if features.green_chromaticity <= 0.452 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_difference <= 108 {
if features.blue_difference <= 109 {
if features.value <= 131 {
if features.green_chromaticity <= 0.455 {
if features.green_luminance <= 130 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.284 {
if features.blue_luminance <= 73 {
if features.blue_chromaticity <= 0.258 {
Intensity::High
} else {
if features.green_chromaticity <= 0.459 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_luminance <= 74 {
if features.red_difference <= 106 {
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
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.280 {
if features.value <= 134 {
if features.red_chromaticity <= 0.276 {
if features.red_luminance <= 78 {
if features.red_luminance <= 77 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.464 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.266 {
if features.blue_difference <= 108 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.456 {
if features.green_chromaticity <= 0.452 {
if features.red_chromaticity <= 0.275 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 84 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.460 {
Intensity::High
} else {
if features.green_chromaticity <= 0.461 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.blue_chromaticity <= 0.275 {
if features.red_chromaticity <= 0.281 {
if features.red_difference <= 105 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.269 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.282 {
if features.intensity <= 97 {
Intensity::High
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
}
} else {
if features.intensity <= 116 {
if features.blue_chromaticity <= 0.277 {
if features.green_chromaticity <= 0.441 {
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
if features.red_luminance <= 105 {
if features.green_chromaticity <= 0.441 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.276 {
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
if features.red_difference <= 105 {
if features.green_chromaticity <= 0.443 {
if features.green_chromaticity <= 0.442 {
if features.blue_chromaticity <= 0.285 {
if features.blue_chromaticity <= 0.284 {
if features.red_luminance <= 96 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_difference <= 103 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.293 {
if features.red_chromaticity <= 0.282 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.431 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.saturation <= 98 {
if features.green_luminance <= 148 {
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
if features.green_luminance <= 129 {
Intensity::High
} else {
if features.red_luminance <= 74 {
if features.saturation <= 111 {
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
} else {
if features.value <= 157 {
if features.red_chromaticity <= 0.263 {
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
if features.luminance <= 125 {
if features.blue_chromaticity <= 0.281 {
if features.green_luminance <= 131 {
if features.luminance <= 108 {
if features.red_chromaticity <= 0.270 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.456 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.436 {
if features.green_chromaticity <= 0.435 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.446 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.434 {
if features.red_luminance <= 96 {
if features.red_chromaticity <= 0.286 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.279 {
Intensity::High
} else {
if features.saturation <= 92 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.red_chromaticity <= 0.282 {
if features.saturation <= 90 {
if features.luminance <= 128 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.436 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.290 {
if features.blue_chromaticity <= 0.282 {
if features.red_luminance <= 98 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.433 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.431 {
if features.green_luminance <= 148 {
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
if features.blue_difference <= 109 {
if features.red_chromaticity <= 0.306 {
if features.green_luminance <= 130 {
if features.red_luminance <= 82 {
if features.blue_luminance <= 74 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.435 {
if features.intensity <= 99 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 103 {
if features.luminance <= 111 {
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
}
}
} else {
if features.red_chromaticity <= 0.297 {
if features.green_luminance <= 131 {
if features.intensity <= 97 {
if features.green_chromaticity <= 0.449 {
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
} else {
if features.blue_chromaticity <= 0.273 {
if features.value <= 136 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.433 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.blue_difference <= 108 {
if features.red_chromaticity <= 0.305 {
if features.red_chromaticity <= 0.300 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.value <= 136 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.saturation <= 99 {
if features.green_chromaticity <= 0.435 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 132 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.saturation <= 98 {
Intensity::High
} else {
if features.saturation <= 100 {
if features.blue_difference <= 108 {
if features.green_chromaticity <= 0.430 {
if features.intensity <= 104 {
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
} else {
if features.red_chromaticity <= 0.308 {
if features.red_difference <= 113 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.431 {
Intensity::High
} else {
if features.red_chromaticity <= 0.309 {
if features.green_chromaticity <= 0.432 {
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
if features.green_luminance <= 132 {
if features.hue <= 53 {
if features.blue_chromaticity <= 0.267 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.293 {
if features.green_luminance <= 131 {
if features.luminance <= 110 {
if features.green_luminance <= 129 {
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
if features.hue <= 56 {
Intensity::High
} else {
if features.green_chromaticity <= 0.441 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.435 {
if features.green_chromaticity <= 0.434 {
if features.luminance <= 113 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 130 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.295 {
if features.saturation <= 98 {
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
if features.blue_difference <= 110 {
if features.red_chromaticity <= 0.297 {
if features.blue_chromaticity <= 0.271 {
if features.intensity <= 102 {
if features.blue_chromaticity <= 0.270 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.value <= 134 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_luminance <= 84 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.273 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.298 {
Intensity::High
} else {
if features.saturation <= 94 {
Intensity::High
} else {
if features.luminance <= 115 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.red_chromaticity <= 0.293 {
if features.saturation <= 93 {
if features.luminance <= 115 {
if features.intensity <= 102 {
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
} else {
if features.hue <= 57 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.295 {
if features.luminance <= 117 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.278 {
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
}
}
}
}
} else {
if features.blue_difference <= 115 {
if features.green_chromaticity <= 0.451 {
if features.value <= 125 {
if features.green_chromaticity <= 0.439 {
if features.luminance <= 98 {
if features.green_chromaticity <= 0.432 {
if features.blue_chromaticity <= 0.271 {
if features.saturation <= 130 {
if features.hue <= 40 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.270 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 67 {
if features.green_luminance <= 97 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.429 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.281 {
if features.green_chromaticity <= 0.430 {
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
}
} else {
if features.saturation <= 127 {
if features.blue_difference <= 112 {
if features.red_chromaticity <= 0.302 {
if features.green_chromaticity <= 0.436 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 125 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.432 {
if features.green_luminance <= 93 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.280 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.360 {
if features.saturation <= 132 {
if features.green_luminance <= 59 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.206 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.luminance <= 57 {
if features.blue_chromaticity <= 0.131 {
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
if features.saturation <= 90 {
if features.blue_chromaticity <= 0.283 {
if features.saturation <= 87 {
Intensity::High
} else {
if features.green_chromaticity <= 0.435 {
if features.red_chromaticity <= 0.292 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.429 {
if features.blue_chromaticity <= 0.285 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 78 {
if features.luminance <= 101 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 104 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.luminance <= 101 {
if features.value <= 114 {
if features.saturation <= 95 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 94 {
if features.saturation <= 93 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.273 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.271 {
if features.red_chromaticity <= 0.295 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.luminance <= 105 {
if features.green_chromaticity <= 0.429 {
Intensity::High
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
}
}
}
} else {
if features.green_luminance <= 73 {
if features.green_luminance <= 59 {
if features.red_chromaticity <= 0.364 {
if features.green_chromaticity <= 0.451 {
if features.intensity <= 41 {
Intensity::High
} else {
if features.saturation <= 145 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 39 {
if features.red_chromaticity <= 0.394 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 118 {
if features.saturation <= 116 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.321 {
if features.red_difference <= 120 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.red_difference <= 121 {
if features.green_chromaticity <= 0.449 {
if features.green_chromaticity <= 0.446 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.hue <= 46 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 45 {
Intensity::High
} else {
if features.red_chromaticity <= 0.352 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
}
} else {
if features.green_chromaticity <= 0.443 {
if features.red_difference <= 111 {
if features.blue_luminance <= 79 {
if features.red_chromaticity <= 0.287 {
if features.saturation <= 93 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 96 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.269 {
if features.green_chromaticity <= 0.441 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.intensity <= 92 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_difference <= 116 {
if features.blue_chromaticity <= 0.255 {
Intensity::High
} else {
if features.red_luminance <= 72 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 66 {
if features.red_difference <= 118 {
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
if features.blue_difference <= 112 {
if features.green_luminance <= 77 {
if features.red_chromaticity <= 0.337 {
if features.luminance <= 65 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.257 {
if features.green_chromaticity <= 0.449 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.288 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.saturation <= 107 {
if features.red_luminance <= 65 {
if features.blue_luminance <= 56 {
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
} else {
if features.green_chromaticity <= 0.448 {
if features.blue_chromaticity <= 0.257 {
Intensity::Low
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
}
}
}
} else {
if features.red_difference <= 108 {
if features.blue_difference <= 114 {
if features.green_luminance <= 127 {
if features.red_chromaticity <= 0.272 {
if features.blue_chromaticity <= 0.286 {
if features.blue_chromaticity <= 0.285 {
Intensity::High
} else {
if features.intensity <= 94 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.264 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.274 {
if features.red_chromaticity <= 0.273 {
Intensity::Low
} else {
if features.blue_difference <= 112 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.276 {
if features.saturation <= 96 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.hue <= 61 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_chromaticity <= 0.435 {
if features.value <= 131 {
if features.green_luminance <= 130 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.293 {
if features.blue_chromaticity <= 0.292 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.intensity <= 129 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.value <= 134 {
if features.red_chromaticity <= 0.264 {
if features.green_luminance <= 128 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.275 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.intensity <= 105 {
if features.saturation <= 101 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 100 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.green_luminance <= 142 {
if features.blue_chromaticity <= 0.298 {
if features.blue_luminance <= 90 {
if features.red_luminance <= 77 {
if features.green_chromaticity <= 0.448 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 88 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.269 {
if features.green_chromaticity <= 0.437 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.saturation <= 93 {
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
if features.blue_chromaticity <= 0.301 {
if features.green_chromaticity <= 0.430 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.431 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.299 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.303 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_luminance <= 87 {
if features.blue_chromaticity <= 0.279 {
if features.green_chromaticity <= 0.440 {
if features.red_chromaticity <= 0.290 {
if features.blue_chromaticity <= 0.278 {
if features.intensity <= 96 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.279 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.432 {
Intensity::High
} else {
if features.green_chromaticity <= 0.433 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.saturation <= 94 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.luminance <= 109 {
if features.blue_chromaticity <= 0.280 {
if features.green_chromaticity <= 0.438 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.281 {
if features.hue <= 58 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.432 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.intensity <= 98 {
if features.red_luminance <= 85 {
if features.red_difference <= 109 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.luminance <= 111 {
if features.red_luminance <= 85 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 132 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.value <= 130 {
Intensity::Low
} else {
if features.green_luminance <= 133 {
if features.green_chromaticity <= 0.430 {
if features.blue_chromaticity <= 0.280 {
if features.value <= 131 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.intensity <= 102 {
Intensity::High
} else {
Intensity::High
}
}
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.285 {
if features.blue_luminance <= 88 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.286 {
if features.red_chromaticity <= 0.285 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 91 {
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
if features.green_luminance <= 67 {
if features.red_chromaticity <= 0.346 {
if features.saturation <= 144 {
if features.red_chromaticity <= 0.339 {
if features.red_chromaticity <= 0.338 {
if features.saturation <= 126 {
if features.red_chromaticity <= 0.316 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.459 {
if features.luminance <= 51 {
Intensity::High
} else {
Intensity::Low
}
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
if features.intensity <= 46 {
if features.green_chromaticity <= 0.454 {
if features.hue <= 43 {
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
if features.value <= 65 {
Intensity::High
} else {
if features.value <= 66 {
if features.saturation <= 141 {
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
if features.green_chromaticity <= 0.460 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.193 {
Intensity::High
} else {
if features.value <= 65 {
if features.intensity <= 42 {
Intensity::High
} else {
if features.blue_difference <= 113 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.338 {
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
if features.red_difference <= 128 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 59 {
if features.red_luminance <= 44 {
if features.red_chromaticity <= 0.365 {
if features.blue_chromaticity <= 0.175 {
Intensity::High
} else {
if features.red_chromaticity <= 0.352 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.luminance <= 49 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 147 {
if features.saturation <= 145 {
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
if features.green_chromaticity <= 0.457 {
if features.luminance <= 105 {
if features.blue_difference <= 113 {
if features.green_chromaticity <= 0.454 {
if features.green_chromaticity <= 0.453 {
if features.green_chromaticity <= 0.453 {
if features.red_luminance <= 61 {
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
if features.green_chromaticity <= 0.453 {
if features.blue_chromaticity <= 0.275 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.454 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.237 {
if features.red_luminance <= 51 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.233 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 70 {
if features.green_chromaticity <= 0.456 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 72 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.saturation <= 109 {
if features.red_luminance <= 65 {
if features.green_chromaticity <= 0.452 {
if features.red_difference <= 115 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.452 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.455 {
if features.green_chromaticity <= 0.453 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.455 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.304 {
if features.red_chromaticity <= 0.290 {
if features.blue_chromaticity <= 0.285 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.294 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.322 {
if features.red_chromaticity <= 0.309 {
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
if features.red_chromaticity <= 0.264 {
if features.red_chromaticity <= 0.260 {
if features.green_chromaticity <= 0.453 {
if features.green_chromaticity <= 0.453 {
if features.red_difference <= 103 {
Intensity::High
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
if features.red_chromaticity <= 0.245 {
Intensity::Low
} else {
if features.red_luminance <= 79 {
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
if features.saturation <= 105 {
if features.green_chromaticity <= 0.452 {
if features.hue <= 62 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.453 {
Intensity::High
} else {
if features.red_chromaticity <= 0.267 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.455 {
if features.blue_difference <= 112 {
if features.red_difference <= 104 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 80 {
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
if features.luminance <= 106 {
if features.blue_difference <= 113 {
if features.blue_chromaticity <= 0.276 {
if features.green_chromaticity <= 0.459 {
if features.blue_chromaticity <= 0.253 {
if features.saturation <= 125 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 102 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 63 {
if features.value <= 107 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.464 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_luminance <= 128 {
if features.red_chromaticity <= 0.265 {
if features.green_chromaticity <= 0.463 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 107 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_luminance <= 78 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.intensity <= 63 {
if features.hue <= 51 {
if features.red_chromaticity <= 0.316 {
if features.red_luminance <= 47 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.291 {
if features.blue_luminance <= 43 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.243 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.246 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.289 {
if features.intensity <= 87 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.293 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
}
} else {
if features.blue_difference <= 112 {
Intensity::High
} else {
if features.value <= 138 {
if features.blue_luminance <= 82 {
if features.blue_chromaticity <= 0.288 {
if features.green_luminance <= 130 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 118 {
Intensity::High
} else {
Intensity::High
}
}
} else {
Intensity::High
}
} else {
if features.luminance <= 114 {
if features.saturation <= 119 {
if features.blue_chromaticity <= 0.291 {
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
if features.green_chromaticity <= 0.446 {
if features.green_luminance <= 118 {
if features.value <= 62 {
if features.red_chromaticity <= 0.331 {
if features.green_chromaticity <= 0.442 {
if features.saturation <= 120 {
if features.blue_chromaticity <= 0.239 {
if features.intensity <= 41 {
if features.green_chromaticity <= 0.440 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 45 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 111 {
if features.intensity <= 45 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 113 {
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
if features.green_chromaticity <= 0.443 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.444 {
if features.red_luminance <= 42 {
if features.saturation <= 118 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_difference <= 122 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.234 {
if features.red_difference <= 122 {
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
if features.value <= 54 {
if features.green_chromaticity <= 0.445 {
if features.green_chromaticity <= 0.438 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.438 {
Intensity::Low
} else {
if features.saturation <= 123 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.446 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.339 {
if features.red_chromaticity <= 0.338 {
if features.saturation <= 120 {
if features.red_chromaticity <= 0.338 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.339 {
if features.green_chromaticity <= 0.437 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.437 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_chromaticity <= 0.437 {
if features.green_chromaticity <= 0.433 {
if features.luminance <= 99 {
if features.blue_chromaticity <= 0.254 {
if features.blue_chromaticity <= 0.244 {
if features.red_luminance <= 50 {
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
if features.blue_luminance <= 54 {
if features.intensity <= 66 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_luminance <= 70 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.431 {
if features.red_luminance <= 75 {
if features.value <= 117 {
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
} else {
if features.value <= 117 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.blue_chromaticity <= 0.274 {
if features.blue_luminance <= 47 {
if features.saturation <= 105 {
if features.green_chromaticity <= 0.436 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_difference <= 116 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.300 {
if features.green_luminance <= 77 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 61 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_luminance <= 69 {
if features.luminance <= 78 {
if features.green_chromaticity <= 0.434 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.269 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 94 {
if features.red_difference <= 111 {
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
}
} else {
if features.blue_difference <= 116 {
if features.red_chromaticity <= 0.266 {
if features.blue_chromaticity <= 0.293 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.293 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.281 {
if features.saturation <= 100 {
if features.green_chromaticity <= 0.438 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.267 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.300 {
if features.green_luminance <= 78 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.437 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_chromaticity <= 0.445 {
if features.blue_chromaticity <= 0.277 {
if features.blue_chromaticity <= 0.266 {
if features.blue_chromaticity <= 0.263 {
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
} else {
if features.blue_difference <= 117 {
if features.red_chromaticity <= 0.263 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.254 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.445 {
if features.intensity <= 61 {
if features.red_chromaticity <= 0.286 {
Intensity::High
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
if features.blue_chromaticity <= 0.299 {
if features.red_chromaticity <= 0.277 {
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
if features.red_chromaticity <= 0.265 {
if features.blue_luminance <= 96 {
if features.red_luminance <= 76 {
if features.red_chromaticity <= 0.260 {
if features.green_luminance <= 119 {
if features.hue <= 67 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.440 {
if features.saturation <= 103 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.442 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.262 {
Intensity::High
} else {
if features.green_chromaticity <= 0.433 {
Intensity::Low
} else {
if features.saturation <= 101 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.blue_chromaticity <= 0.299 {
if features.red_luminance <= 77 {
Intensity::High
} else {
if features.luminance <= 110 {
if features.red_difference <= 105 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 104 {
if features.green_chromaticity <= 0.443 {
if features.blue_luminance <= 88 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.254 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.304 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_difference <= 102 {
Intensity::High
} else {
if features.red_chromaticity <= 0.263 {
if features.blue_chromaticity <= 0.309 {
if features.red_chromaticity <= 0.259 {
Intensity::High
} else {
if features.green_chromaticity <= 0.432 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.430 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 109 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.red_chromaticity <= 0.270 {
if features.red_chromaticity <= 0.270 {
if features.blue_difference <= 116 {
if features.green_chromaticity <= 0.429 {
if features.red_chromaticity <= 0.268 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.430 {
Intensity::High
} else {
if features.red_chromaticity <= 0.268 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.433 {
if features.green_luminance <= 134 {
if features.value <= 120 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.value <= 135 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.hue <= 65 {
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
if features.blue_difference <= 116 {
if features.green_chromaticity <= 0.433 {
if features.red_chromaticity <= 0.271 {
Intensity::High
} else {
if features.green_chromaticity <= 0.430 {
if features.blue_chromaticity <= 0.298 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.430 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.271 {
if features.red_luminance <= 75 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 81 {
Intensity::Low
} else {
if features.luminance <= 104 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_chromaticity <= 0.271 {
if features.red_chromaticity <= 0.271 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.272 {
if features.green_chromaticity <= 0.430 {
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
if features.value <= 62 {
if features.red_difference <= 122 {
if features.green_chromaticity <= 0.455 {
if features.green_luminance <= 59 {
if features.red_chromaticity <= 0.322 {
if features.green_chromaticity <= 0.452 {
if features.red_luminance <= 39 {
if features.saturation <= 122 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.227 {
Intensity::Low
} else {
if features.blue_luminance <= 30 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.447 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.228 {
if features.blue_difference <= 116 {
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
if features.blue_chromaticity <= 0.236 {
if features.red_chromaticity <= 0.312 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.251 {
if features.red_luminance <= 42 {
if features.green_chromaticity <= 0.451 {
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
if features.green_chromaticity <= 0.465 {
if features.blue_luminance <= 30 {
if features.green_chromaticity <= 0.461 {
if features.green_chromaticity <= 0.456 {
Intensity::Low
} else {
if features.saturation <= 124 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.304 {
if features.luminance <= 45 {
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
if features.green_chromaticity <= 0.461 {
if features.blue_chromaticity <= 0.241 {
if features.blue_luminance <= 31 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_luminance <= 60 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_luminance <= 61 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.306 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.340 {
if features.green_chromaticity <= 0.462 {
if features.blue_difference <= 116 {
if features.red_chromaticity <= 0.331 {
if features.blue_luminance <= 24 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.331 {
Intensity::High
} else {
if features.saturation <= 128 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.209 {
Intensity::Low
} else {
if features.saturation <= 135 {
if features.green_chromaticity <= 0.449 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 36 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_luminance <= 47 {
if features.saturation <= 140 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.463 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.446 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.454 {
if features.blue_difference <= 116 {
if features.red_chromaticity <= 0.300 {
if features.red_chromaticity <= 0.279 {
if features.red_chromaticity <= 0.253 {
if features.green_chromaticity <= 0.450 {
if features.green_chromaticity <= 0.448 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.452 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.275 {
if features.green_luminance <= 93 {
Intensity::High
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
}
} else {
if features.red_chromaticity <= 0.283 {
if features.red_luminance <= 54 {
if features.saturation <= 105 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.297 {
if features.red_chromaticity <= 0.296 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 72 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_chromaticity <= 0.308 {
if features.saturation <= 116 {
if features.saturation <= 111 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.449 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.saturation <= 117 {
Intensity::High
} else {
if features.green_chromaticity <= 0.449 {
Intensity::Low
} else {
if features.hue <= 49 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.red_luminance <= 67 {
if features.green_chromaticity <= 0.446 {
if features.luminance <= 78 {
if features.green_chromaticity <= 0.446 {
if features.blue_luminance <= 47 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 72 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 66 {
if features.green_luminance <= 109 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.446 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.intensity <= 72 {
if features.green_chromaticity <= 0.447 {
if features.red_luminance <= 57 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_luminance <= 45 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.298 {
if features.red_luminance <= 66 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.303 {
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
if features.blue_difference <= 116 {
if features.green_chromaticity <= 0.460 {
if features.red_luminance <= 65 {
if features.saturation <= 112 {
if features.red_chromaticity <= 0.274 {
if features.saturation <= 111 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.455 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.307 {
if features.green_chromaticity <= 0.459 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.hue <= 66 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_difference <= 100 {
if features.red_difference <= 99 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.saturation <= 108 {
if features.intensity <= 64 {
if features.saturation <= 107 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.saturation <= 112 {
if features.saturation <= 111 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 77 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.green_chromaticity <= 0.465 {
if features.luminance <= 55 {
if features.green_chromaticity <= 0.461 {
if features.hue <= 55 {
if features.green_luminance <= 63 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.464 {
if features.red_luminance <= 39 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.hue <= 55 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.280 {
if features.blue_chromaticity <= 0.278 {
if features.value <= 72 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 46 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 62 {
if features.green_chromaticity <= 0.463 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.462 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_chromaticity <= 0.287 {
if features.saturation <= 111 {
Intensity::High
} else {
if features.blue_difference <= 117 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.saturation <= 122 {
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
if features.red_difference <= 121 {
if features.blue_chromaticity <= 0.240 {
if features.red_difference <= 118 {
if features.saturation <= 141 {
if features.red_difference <= 114 {
if features.red_difference <= 110 {
if features.value <= 131 {
if features.saturation <= 130 {
if features.value <= 127 {
if features.green_luminance <= 120 {
if features.intensity <= 78 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.467 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.luminance <= 108 {
if features.green_chromaticity <= 0.473 {
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
if features.saturation <= 135 {
if features.green_chromaticity <= 0.493 {
if features.red_chromaticity <= 0.270 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.intensity <= 69 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_difference <= 105 {
if features.red_chromaticity <= 0.265 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.luminance <= 69 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.blue_difference <= 103 {
if features.blue_luminance <= 78 {
if features.green_chromaticity <= 0.468 {
if features.red_luminance <= 85 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.value <= 134 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 92 {
if features.red_difference <= 100 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.red_luminance <= 82 {
if features.hue <= 55 {
if features.blue_luminance <= 66 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.266 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.value <= 133 {
if features.saturation <= 125 {
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
}
}
} else {
if features.saturation <= 131 {
if features.red_chromaticity <= 0.284 {
if features.blue_chromaticity <= 0.237 {
if features.blue_chromaticity <= 0.236 {
if features.blue_chromaticity <= 0.234 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 87 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.240 {
if features.red_luminance <= 53 {
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
}
} else {
if features.green_luminance <= 104 {
if features.green_chromaticity <= 0.472 {
if features.green_chromaticity <= 0.471 {
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
if features.value <= 117 {
if features.luminance <= 89 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.237 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.blue_chromaticity <= 0.221 {
if features.green_luminance <= 130 {
if features.red_chromaticity <= 0.315 {
if features.green_chromaticity <= 0.475 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.316 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.310 {
if features.saturation <= 137 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.475 {
if features.blue_difference <= 102 {
if features.green_chromaticity <= 0.470 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.value <= 118 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_difference <= 113 {
if features.saturation <= 134 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.221 {
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
if features.value <= 62 {
if features.green_chromaticity <= 0.502 {
if features.saturation <= 133 {
if features.red_difference <= 116 {
Intensity::High
} else {
if features.hue <= 54 {
if features.blue_luminance <= 30 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.490 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_difference <= 117 {
if features.luminance <= 48 {
if features.intensity <= 37 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.value <= 61 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.492 {
if features.green_luminance <= 60 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 138 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.red_difference <= 115 {
if features.red_luminance <= 27 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.236 {
if features.saturation <= 139 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.red_difference <= 116 {
if features.blue_chromaticity <= 0.237 {
if features.luminance <= 47 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.green_luminance <= 53 {
if features.green_chromaticity <= 0.507 {
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
if features.saturation <= 134 {
if features.red_difference <= 117 {
if features.red_chromaticity <= 0.303 {
if features.red_chromaticity <= 0.294 {
if features.red_chromaticity <= 0.292 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 44 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.228 {
if features.red_chromaticity <= 0.304 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_difference <= 110 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.blue_chromaticity <= 0.229 {
if features.blue_chromaticity <= 0.226 {
if features.blue_chromaticity <= 0.222 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.235 {
if features.red_chromaticity <= 0.294 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.238 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.red_luminance <= 46 {
if features.saturation <= 137 {
if features.red_chromaticity <= 0.285 {
if features.hue <= 56 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 70 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.218 {
if features.intensity <= 47 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.220 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.301 {
if features.green_chromaticity <= 0.485 {
if features.green_chromaticity <= 0.483 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.hue <= 51 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.value <= 97 {
if features.red_difference <= 115 {
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
}
}
}
}
}
} else {
if features.saturation <= 164 {
if features.red_difference <= 116 {
if features.value <= 108 {
if features.saturation <= 149 {
if features.red_difference <= 114 {
if features.blue_chromaticity <= 0.239 {
if features.green_chromaticity <= 0.495 {
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
if features.blue_luminance <= 36 {
if features.red_luminance <= 42 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.318 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.blue_difference <= 117 {
if features.saturation <= 157 {
if features.blue_chromaticity <= 0.231 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 45 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.550 {
if features.red_difference <= 115 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.luminance <= 36 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.green_chromaticity <= 0.498 {
if features.green_chromaticity <= 0.466 {
if features.green_chromaticity <= 0.466 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 131 {
if features.blue_chromaticity <= 0.209 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.186 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.saturation <= 152 {
if features.red_difference <= 110 {
if features.green_luminance <= 126 {
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
if features.blue_chromaticity <= 0.217 {
if features.red_difference <= 109 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 55 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.blue_difference <= 115 {
if features.red_difference <= 117 {
if features.green_chromaticity <= 0.468 {
if features.luminance <= 95 {
if features.red_luminance <= 74 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.216 {
if features.blue_luminance <= 38 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.219 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.blue_chromaticity <= 0.191 {
if features.green_chromaticity <= 0.497 {
if features.saturation <= 159 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.502 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.191 {
Intensity::Low
} else {
if features.red_luminance <= 39 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.red_difference <= 117 {
if features.blue_chromaticity <= 0.203 {
Intensity::High
} else {
if features.value <= 50 {
if features.blue_luminance <= 18 {
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
if features.blue_chromaticity <= 0.220 {
if features.saturation <= 163 {
if features.red_chromaticity <= 0.267 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.intensity <= 35 {
if features.value <= 51 {
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
if features.red_difference <= 116 {
if features.green_chromaticity <= 0.592 {
if features.red_chromaticity <= 0.347 {
if features.value <= 49 {
if features.red_chromaticity <= 0.209 {
Intensity::High
} else {
if features.blue_luminance <= 16 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.239 {
if features.value <= 111 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 79 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.blue_difference <= 91 {
Intensity::High
} else {
if features.green_chromaticity <= 0.554 {
if features.green_chromaticity <= 0.534 {
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
if features.blue_difference <= 116 {
if features.green_luminance <= 37 {
if features.red_chromaticity <= 0.152 {
if features.blue_difference <= 112 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.intensity <= 14 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.612 {
if features.blue_chromaticity <= 0.213 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.647 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_difference <= 113 {
if features.hue <= 61 {
if features.red_luminance <= 10 {
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
} else {
if features.blue_chromaticity <= 0.203 {
if features.red_chromaticity <= 0.207 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 181 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.blue_difference <= 116 {
if features.blue_difference <= 112 {
if features.blue_chromaticity <= 0.181 {
if features.blue_chromaticity <= 0.165 {
if features.blue_difference <= 108 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 174 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.523 {
if features.luminance <= 52 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.103 {
Intensity::High
} else {
if features.value <= 38 {
Intensity::High
} else {
if features.green_chromaticity <= 0.533 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.green_luminance <= 31 {
if features.red_chromaticity <= 0.148 {
if features.red_chromaticity <= 0.118 {
Intensity::High
} else {
if features.green_luminance <= 29 {
Intensity::Low
} else {
Intensity::High
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
if features.blue_chromaticity <= 0.189 {
if features.red_difference <= 117 {
if features.blue_luminance <= 11 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.609 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_luminance <= 48 {
if features.blue_chromaticity <= 0.193 {
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
if features.blue_difference <= 115 {
if features.saturation <= 146 {
if features.red_difference <= 119 {
if features.blue_chromaticity <= 0.212 {
if features.luminance <= 69 {
if features.green_chromaticity <= 0.479 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.208 {
Intensity::High
} else {
if features.blue_luminance <= 28 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.blue_chromaticity <= 0.204 {
if features.blue_chromaticity <= 0.201 {
if features.blue_chromaticity <= 0.200 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.326 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.value <= 88 {
if features.value <= 83 {
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
if features.saturation <= 139 {
if features.blue_chromaticity <= 0.224 {
if features.red_chromaticity <= 0.310 {
if features.hue <= 50 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.466 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.225 {
if features.blue_difference <= 114 {
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
if features.blue_chromaticity <= 0.215 {
if features.hue <= 49 {
if features.green_chromaticity <= 0.478 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 62 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.217 {
if features.blue_chromaticity <= 0.215 {
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
if features.red_chromaticity <= 0.321 {
if features.red_difference <= 120 {
if features.green_chromaticity <= 0.468 {
if features.green_chromaticity <= 0.466 {
Intensity::High
} else {
if features.saturation <= 136 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.value <= 59 {
if features.red_chromaticity <= 0.309 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.value <= 67 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.blue_chromaticity <= 0.213 {
if features.saturation <= 142 {
Intensity::High
} else {
if features.green_chromaticity <= 0.474 {
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
if features.green_luminance <= 66 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.207 {
if features.saturation <= 145 {
if features.red_difference <= 120 {
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
} else {
if features.saturation <= 140 {
Intensity::High
} else {
if features.green_luminance <= 68 {
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
if features.blue_difference <= 114 {
if features.saturation <= 181 {
if features.red_difference <= 119 {
if features.red_chromaticity <= 0.326 {
if features.blue_difference <= 111 {
if features.saturation <= 166 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 25 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.501 {
if features.green_chromaticity <= 0.467 {
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
} else {
if features.blue_difference <= 112 {
if features.green_luminance <= 95 {
if features.red_chromaticity <= 0.325 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_difference <= 99 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.315 {
if features.red_chromaticity <= 0.311 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.317 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.blue_difference <= 110 {
if features.red_chromaticity <= 0.327 {
if features.red_chromaticity <= 0.327 {
if features.green_luminance <= 67 {
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
if features.red_chromaticity <= 0.330 {
if features.red_luminance <= 22 {
if features.blue_difference <= 113 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 44 {
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
if features.red_chromaticity <= 0.306 {
if features.red_chromaticity <= 0.299 {
if features.green_chromaticity <= 0.779 {
if features.red_chromaticity <= 0.282 {
if features.red_chromaticity <= 0.255 {
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
if features.luminance <= 21 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.luminance <= 29 {
Intensity::High
} else {
if features.green_chromaticity <= 0.585 {
if features.red_chromaticity <= 0.301 {
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
if features.green_chromaticity <= 0.519 {
if features.value <= 55 {
if features.saturation <= 155 {
if features.saturation <= 152 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.498 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.luminance <= 46 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.value <= 46 {
if features.saturation <= 179 {
Intensity::High
} else {
if features.red_chromaticity <= 0.309 {
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
if features.red_difference <= 119 {
if features.green_chromaticity <= 0.524 {
if features.blue_difference <= 116 {
if features.green_chromaticity <= 0.472 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.219 {
if features.saturation <= 144 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.202 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.490 {
if features.saturation <= 131 {
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
if features.blue_luminance <= 25 {
if features.green_luminance <= 52 {
if features.green_chromaticity <= 0.522 {
if features.red_chromaticity <= 0.279 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.284 {
if features.blue_difference <= 117 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.493 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.292 {
if features.blue_chromaticity <= 0.235 {
if features.green_chromaticity <= 0.481 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.238 {
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
if features.hue <= 53 {
if features.blue_difference <= 116 {
if features.green_chromaticity <= 0.535 {
if features.green_chromaticity <= 0.529 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 10 {
Intensity::High
} else {
if features.saturation <= 187 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.saturation <= 215 {
if features.green_luminance <= 36 {
Intensity::High
} else {
if features.saturation <= 198 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.089 {
if features.saturation <= 235 {
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
if features.green_chromaticity <= 0.565 {
if features.value <= 43 {
if features.saturation <= 168 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 21 {
Intensity::High
} else {
if features.saturation <= 157 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.169 {
Intensity::High
} else {
if features.saturation <= 213 {
if features.luminance <= 30 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 29 {
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
if features.blue_difference <= 117 {
if features.red_chromaticity <= 0.291 {
if features.red_chromaticity <= 0.261 {
if features.red_chromaticity <= 0.206 {
Intensity::High
} else {
if features.red_difference <= 120 {
if features.green_chromaticity <= 0.703 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.772 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_difference <= 116 {
if features.green_chromaticity <= 0.563 {
if features.saturation <= 174 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.saturation <= 208 {
if features.blue_luminance <= 9 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 14 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_chromaticity <= 0.517 {
if features.green_chromaticity <= 0.486 {
if features.green_chromaticity <= 0.484 {
if features.blue_chromaticity <= 0.211 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.308 {
if features.red_chromaticity <= 0.303 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.559 {
if features.saturation <= 173 {
if features.value <= 44 {
Intensity::High
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
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.269 {
if features.red_chromaticity <= 0.236 {
if features.red_chromaticity <= 0.217 {
if features.green_luminance <= 27 {
if features.red_chromaticity <= 0.194 {
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
if features.value <= 31 {
if features.value <= 30 {
Intensity::High
} else {
if features.intensity <= 15 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.573 {
Intensity::High
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
if features.green_chromaticity <= 0.481 {
if features.red_chromaticity <= 0.302 {
if features.green_chromaticity <= 0.479 {
if features.saturation <= 134 {
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
if features.red_difference <= 120 {
if features.saturation <= 142 {
if features.green_chromaticity <= 0.483 {
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
} else {
if features.green_luminance <= 45 {
if features.luminance <= 35 {
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
}
}
}
}
}
}
}
} else {
if features.blue_chromaticity <= 0.280 {
if features.red_difference <= 115 {
if features.blue_chromaticity <= 0.264 {
if features.red_difference <= 106 {
if features.red_difference <= 104 {
if features.blue_chromaticity <= 0.252 {
if features.blue_luminance <= 47 {
if features.green_chromaticity <= 0.575 {
if features.green_chromaticity <= 0.575 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.value <= 130 {
if features.saturation <= 124 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.246 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_difference <= 103 {
if features.green_luminance <= 105 {
if features.red_chromaticity <= 0.180 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.hue <= 61 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_luminance <= 129 {
if features.blue_difference <= 109 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.266 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.red_chromaticity <= 0.276 {
if features.blue_luminance <= 61 {
if features.green_chromaticity <= 0.493 {
if features.saturation <= 123 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 106 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.259 {
if features.luminance <= 106 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 72 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.intensity <= 93 {
if features.hue <= 55 {
if features.blue_chromaticity <= 0.243 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 133 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.468 {
if features.red_difference <= 105 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.470 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.saturation <= 119 {
if features.green_luminance <= 127 {
if features.value <= 119 {
if features.green_luminance <= 98 {
if features.red_difference <= 114 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 115 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.intensity <= 88 {
if features.green_chromaticity <= 0.471 {
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
}
} else {
if features.red_difference <= 107 {
if features.blue_chromaticity <= 0.250 {
Intensity::High
} else {
if features.hue <= 55 {
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
if features.red_difference <= 109 {
if features.luminance <= 109 {
if features.green_chromaticity <= 0.474 {
if features.blue_luminance <= 67 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.254 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.466 {
if features.intensity <= 99 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.saturation <= 124 {
if features.luminance <= 54 {
if features.saturation <= 123 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.luminance <= 98 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.249 {
if features.red_chromaticity <= 0.284 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_difference <= 114 {
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
if features.red_difference <= 104 {
if features.green_luminance <= 130 {
if features.green_chromaticity <= 0.471 {
if features.blue_luminance <= 74 {
if features.luminance <= 99 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.276 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.467 {
if features.blue_luminance <= 76 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 113 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.blue_chromaticity <= 0.275 {
if features.luminance <= 100 {
if features.red_difference <= 102 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.254 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.219 {
if features.luminance <= 67 {
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
}
} else {
if features.red_chromaticity <= 0.268 {
if features.red_difference <= 103 {
if features.blue_chromaticity <= 0.273 {
if features.blue_chromaticity <= 0.264 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.273 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.hue <= 60 {
if features.value <= 133 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.468 {
if features.saturation <= 110 {
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
if features.green_chromaticity <= 0.480 {
if features.blue_difference <= 116 {
if features.blue_chromaticity <= 0.269 {
if features.value <= 105 {
if features.red_luminance <= 56 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.intensity <= 82 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.254 {
if features.blue_chromaticity <= 0.275 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.277 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.478 {
if features.blue_luminance <= 43 {
if features.green_chromaticity <= 0.476 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 48 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.intensity <= 49 {
Intensity::High
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
if features.blue_difference <= 115 {
if features.red_difference <= 106 {
if features.red_chromaticity <= 0.247 {
if features.value <= 96 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_luminance <= 108 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.saturation <= 128 {
if features.red_chromaticity <= 0.247 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.234 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.509 {
if features.green_chromaticity <= 0.509 {
if features.blue_chromaticity <= 0.275 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_difference <= 116 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.279 {
if features.red_luminance <= 37 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.530 {
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
if features.red_difference <= 116 {
if features.blue_chromaticity <= 0.252 {
if features.blue_chromaticity <= 0.251 {
if features.blue_chromaticity <= 0.248 {
if features.blue_chromaticity <= 0.241 {
if features.green_chromaticity <= 0.479 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 122 {
if features.green_luminance <= 75 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.273 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.blue_chromaticity <= 0.248 {
Intensity::High
} else {
if features.green_luminance <= 64 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.248 {
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
} else {
if features.hue <= 56 {
if features.intensity <= 52 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.263 {
if features.blue_chromaticity <= 0.253 {
if features.value <= 65 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.luminance <= 53 {
if features.saturation <= 119 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.value <= 67 {
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
}
} else {
if features.blue_chromaticity <= 0.248 {
if features.red_difference <= 118 {
if features.red_luminance <= 41 {
if features.blue_chromaticity <= 0.243 {
if features.red_chromaticity <= 0.286 {
if features.saturation <= 127 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.luminance <= 54 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.247 {
if features.blue_chromaticity <= 0.246 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.saturation <= 121 {
Intensity::High
} else {
if features.value <= 68 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.blue_chromaticity <= 0.243 {
if features.red_luminance <= 36 {
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
}
} else {
if features.red_chromaticity <= 0.274 {
Intensity::High
} else {
if features.red_chromaticity <= 0.281 {
if features.blue_chromaticity <= 0.249 {
Intensity::Low
} else {
if features.hue <= 56 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.472 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.luminance <= 54 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.249 {
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
if features.hue <= 68 {
if features.intensity <= 84 {
if features.blue_chromaticity <= 0.292 {
if features.red_difference <= 105 {
if features.saturation <= 120 {
if features.saturation <= 117 {
Intensity::Low
} else {
if features.blue_luminance <= 69 {
if features.intensity <= 81 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.luminance <= 95 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_difference <= 100 {
if features.blue_chromaticity <= 0.285 {
if features.blue_chromaticity <= 0.282 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.208 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.blue_difference <= 114 {
if features.green_chromaticity <= 0.476 {
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
}
} else {
if features.intensity <= 62 {
if features.red_difference <= 107 {
if features.intensity <= 60 {
if features.green_chromaticity <= 0.508 {
Intensity::Low
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
} else {
if features.red_difference <= 108 {
Intensity::High
} else {
if features.red_chromaticity <= 0.247 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_luminance <= 46 {
if features.saturation <= 132 {
if features.green_luminance <= 91 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.487 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_luminance <= 107 {
if features.blue_luminance <= 64 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 58 {
Intensity::High
} else {
Intensity::High
}
}
}
}
}
} else {
if features.blue_difference <= 116 {
if features.green_chromaticity <= 0.469 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.295 {
if features.saturation <= 126 {
if features.saturation <= 125 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.saturation <= 128 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 73 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.232 {
if features.blue_chromaticity <= 0.292 {
if features.saturation <= 140 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.227 {
Intensity::High
} else {
if features.saturation <= 133 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.467 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.298 {
if features.green_chromaticity <= 0.467 {
Intensity::Low
} else {
Intensity::Low
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
}
} else {
if features.red_difference <= 101 {
if features.blue_chromaticity <= 0.290 {
if features.value <= 129 {
if features.luminance <= 103 {
Intensity::High
} else {
if features.red_chromaticity <= 0.240 {
if features.blue_chromaticity <= 0.287 {
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
} else {
if features.blue_chromaticity <= 0.290 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.297 {
if features.red_luminance <= 57 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.472 {
Intensity::High
} else {
if features.red_difference <= 98 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.blue_chromaticity <= 0.280 {
if features.green_chromaticity <= 0.468 {
Intensity::High
} else {
if features.saturation <= 120 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.245 {
Intensity::High
} else {
if features.red_chromaticity <= 0.250 {
if features.red_chromaticity <= 0.250 {
if features.red_chromaticity <= 0.246 {
Intensity::High
} else {
Intensity::High
}
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
}
}
}
}
} else {
if features.green_chromaticity <= 0.544 {
if features.hue <= 69 {
if features.red_chromaticity <= 0.230 {
if features.blue_chromaticity <= 0.297 {
if features.blue_difference <= 117 {
if features.red_difference <= 99 {
if features.blue_chromaticity <= 0.291 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.saturation <= 172 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_difference <= 103 {
if features.saturation <= 147 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.saturation <= 157 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.blue_luminance <= 79 {
if features.saturation <= 137 {
if features.red_chromaticity <= 0.224 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.intensity <= 75 {
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
} else {
if features.red_chromaticity <= 0.226 {
if features.red_difference <= 99 {
if features.blue_difference <= 116 {
if features.intensity <= 77 {
Intensity::High
} else {
if features.value <= 122 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.312 {
if features.blue_chromaticity <= 0.298 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.value <= 97 {
if features.saturation <= 172 {
if features.red_chromaticity <= 0.179 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.494 {
if features.luminance <= 86 {
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
} else {
if features.green_chromaticity <= 0.556 {
if features.red_chromaticity <= 0.166 {
if features.red_difference <= 101 {
if features.hue <= 70 {
if features.blue_difference <= 116 {
Intensity::High
} else {
if features.red_difference <= 100 {
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
} else {
if features.blue_difference <= 113 {
if features.red_difference <= 124 {
if features.red_difference <= 122 {
if features.luminance <= 51 {
if features.blue_difference <= 112 {
if features.value <= 46 {
if features.saturation <= 236 {
if features.saturation <= 213 {
Intensity::Low
} else {
if features.blue_luminance <= 3 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.value <= 33 {
Intensity::High
} else {
Intensity::High
}
}
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.340 {
Intensity::High
} else {
if features.blue_luminance <= 7 {
Intensity::High
} else {
if features.red_chromaticity <= 0.341 {
if features.green_chromaticity <= 0.518 {
if features.green_luminance <= 50 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.value <= 45 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.value <= 63 {
if features.value <= 61 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.346 {
if features.blue_chromaticity <= 0.180 {
Intensity::High
} else {
if features.green_luminance <= 62 {
if features.luminance <= 52 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.500 {
if features.blue_chromaticity <= 0.161 {
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
if features.blue_chromaticity <= 0.137 {
if features.blue_chromaticity <= 0.136 {
if features.blue_difference <= 98 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.368 {
if features.blue_chromaticity <= 0.120 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.intensity <= 45 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.481 {
if features.green_chromaticity <= 0.481 {
if features.blue_chromaticity <= 0.171 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.174 {
Intensity::Low
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
}
} else {
if features.blue_difference <= 96 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.505 {
if features.green_chromaticity <= 0.493 {
if features.red_chromaticity <= 0.351 {
if features.saturation <= 167 {
if features.red_chromaticity <= 0.344 {
if features.intensity <= 42 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.red_luminance <= 39 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.174 {
if features.saturation <= 172 {
if features.luminance <= 73 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.saturation <= 174 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.178 {
if features.blue_chromaticity <= 0.177 {
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
if features.blue_chromaticity <= 0.017 {
if features.blue_difference <= 112 {
Intensity::High
} else {
if features.red_chromaticity <= 0.359 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.hue <= 40 {
Intensity::High
} else {
if features.green_luminance <= 44 {
if features.red_chromaticity <= 0.372 {
if features.hue <= 42 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 3 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.intensity <= 28 {
Intensity::High
} else {
if features.red_chromaticity <= 0.367 {
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
if features.green_chromaticity <= 0.508 {
if features.red_chromaticity <= 0.392 {
if features.blue_difference <= 110 {
Intensity::High
} else {
if features.red_chromaticity <= 0.383 {
if features.green_chromaticity <= 0.475 {
if features.red_luminance <= 43 {
Intensity::High
} else {
if features.intensity <= 39 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 180 {
if features.value <= 50 {
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
if features.blue_chromaticity <= 0.122 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.124 {
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
if features.blue_difference <= 116 {
if features.red_difference <= 123 {
if features.green_luminance <= 34 {
if features.blue_difference <= 114 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.087 {
if features.value <= 29 {
Intensity::Low
} else {
if features.intensity <= 16 {
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
if features.saturation <= 144 {
Intensity::High
} else {
if features.red_chromaticity <= 0.344 {
if features.saturation <= 167 {
if features.green_luminance <= 54 {
if features.saturation <= 161 {
if features.saturation <= 158 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_luminance <= 52 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_luminance <= 55 {
Intensity::High
} else {
if features.saturation <= 149 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.red_luminance <= 32 {
if features.blue_difference <= 115 {
if features.red_chromaticity <= 0.322 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.099 {
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
if features.value <= 38 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.543 {
if features.red_chromaticity <= 0.345 {
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
if features.red_difference <= 124 {
if features.red_chromaticity <= 0.357 {
if features.blue_chromaticity <= 0.188 {
if features.red_luminance <= 23 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.350 {
if features.saturation <= 154 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.491 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.134 {
if features.intensity <= 25 {
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
Intensity::High
}
} else {
if features.luminance <= 42 {
if features.green_chromaticity <= 0.606 {
if features.red_chromaticity <= 0.369 {
if features.saturation <= 186 {
if features.value <= 40 {
if features.intensity <= 26 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.153 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 196 {
if features.luminance <= 31 {
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
if features.green_chromaticity <= 0.623 {
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
if features.red_difference <= 122 {
if features.hue <= 47 {
if features.value <= 38 {
if features.blue_luminance <= 1 {
if features.luminance <= 20 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.red_luminance <= 30 {
if features.green_chromaticity <= 0.530 {
if features.saturation <= 160 {
if features.saturation <= 157 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.509 {
if features.green_luminance <= 43 {
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
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.206 {
if features.green_chromaticity <= 0.478 {
Intensity::Low
} else {
if features.saturation <= 150 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.209 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_chromaticity <= 0.551 {
if features.luminance <= 32 {
if features.luminance <= 30 {
if features.green_chromaticity <= 0.541 {
if features.saturation <= 176 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.516 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.477 {
if features.saturation <= 139 {
if features.green_chromaticity <= 0.469 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.saturation <= 156 {
if features.red_luminance <= 27 {
Intensity::Low
} else {
if features.value <= 44 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.309 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.saturation <= 250 {
if features.luminance <= 24 {
if features.red_luminance <= 14 {
Intensity::Low
} else {
if features.saturation <= 210 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.intensity <= 11 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.saturation <= 144 {
if features.blue_difference <= 117 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_difference <= 123 {
if features.red_chromaticity <= 0.320 {
if features.red_luminance <= 20 {
if features.green_chromaticity <= 0.633 {
if features.green_chromaticity <= 0.613 {
if features.red_chromaticity <= 0.313 {
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
if features.green_chromaticity <= 0.641 {
if features.red_chromaticity <= 0.322 {
if features.saturation <= 161 {
if features.green_chromaticity <= 0.491 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.176 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.327 {
if features.saturation <= 150 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 23 {
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
if features.value <= 40 {
if features.red_luminance <= 20 {
if features.blue_chromaticity <= 0.128 {
if features.red_chromaticity <= 0.376 {
if features.red_luminance <= 11 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.hue <= 41 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
Intensity::High
}
} else {
if features.intensity <= 19 {
if features.saturation <= 196 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 162 {
if features.blue_luminance <= 17 {
Intensity::Low
} else {
if features.saturation <= 151 {
if features.red_chromaticity <= 0.335 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.172 {
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
} else {
if features.red_difference <= 117 {
if features.green_chromaticity <= 0.513 {
if features.blue_chromaticity <= 0.290 {
if features.green_chromaticity <= 0.473 {
if features.red_difference <= 115 {
if features.red_luminance <= 46 {
if features.blue_chromaticity <= 0.289 {
if features.blue_chromaticity <= 0.286 {
if features.red_luminance <= 39 {
if features.blue_chromaticity <= 0.273 {
if features.green_chromaticity <= 0.470 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.285 {
if features.green_chromaticity <= 0.465 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 42 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.value <= 70 {
if features.green_chromaticity <= 0.458 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 41 {
if features.green_chromaticity <= 0.471 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.258 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_difference <= 111 {
Intensity::High
} else {
if features.saturation <= 119 {
if features.hue <= 64 {
if features.value <= 72 {
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
if features.red_chromaticity <= 0.245 {
if features.red_chromaticity <= 0.240 {
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
if features.blue_chromaticity <= 0.289 {
Intensity::High
} else {
if features.red_difference <= 110 {
Intensity::Low
} else {
if features.value <= 78 {
if features.green_luminance <= 73 {
if features.red_difference <= 113 {
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
} else {
if features.saturation <= 102 {
if features.blue_chromaticity <= 0.289 {
if features.luminance <= 69 {
if features.green_chromaticity <= 0.442 {
if features.red_chromaticity <= 0.274 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.439 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 47 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.434 {
if features.green_chromaticity <= 0.432 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 70 {
if features.red_difference <= 114 {
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
Intensity::Low
}
} else {
if features.green_luminance <= 83 {
if features.value <= 81 {
if features.green_luminance <= 80 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 105 {
Intensity::Low
} else {
if features.saturation <= 108 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.luminance <= 70 {
if features.red_difference <= 112 {
if features.red_luminance <= 47 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.450 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.blue_chromaticity <= 0.281 {
if features.blue_chromaticity <= 0.280 {
if features.red_chromaticity <= 0.276 {
if features.saturation <= 107 {
if features.intensity <= 49 {
if features.blue_chromaticity <= 0.276 {
if features.blue_chromaticity <= 0.275 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 39 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.value <= 74 {
if features.saturation <= 102 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 110 {
if features.blue_chromaticity <= 0.265 {
Intensity::Low
} else {
if features.luminance <= 55 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.472 {
if features.blue_luminance <= 34 {
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
}
} else {
if features.green_chromaticity <= 0.437 {
if features.luminance <= 64 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 60 {
if features.red_chromaticity <= 0.278 {
Intensity::Low
} else {
if features.red_luminance <= 42 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 93 {
Intensity::Low
} else {
if features.green_luminance <= 72 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.intensity <= 51 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_difference <= 116 {
if features.saturation <= 102 {
if features.blue_chromaticity <= 0.283 {
if features.red_chromaticity <= 0.276 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.287 {
if features.luminance <= 69 {
if features.blue_chromaticity <= 0.284 {
Intensity::Low
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
} else {
if features.blue_chromaticity <= 0.288 {
if features.green_chromaticity <= 0.430 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.441 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.saturation <= 116 {
if features.blue_chromaticity <= 0.285 {
if features.red_luminance <= 39 {
if features.blue_luminance <= 41 {
Intensity::Low
} else {
Intensity::High
}
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
if features.red_chromaticity <= 0.247 {
if features.saturation <= 122 {
Intensity::Low
} else {
if features.intensity <= 40 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 32 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_luminance <= 31 {
Intensity::Low
} else {
if features.luminance <= 57 {
if features.red_luminance <= 29 {
if features.red_chromaticity <= 0.249 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.459 {
if features.blue_chromaticity <= 0.287 {
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
if features.green_luminance <= 70 {
if features.blue_chromaticity <= 0.283 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.288 {
if features.red_luminance <= 52 {
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
if features.blue_difference <= 120 {
if features.green_chromaticity <= 0.487 {
if features.blue_difference <= 119 {
if features.saturation <= 119 {
if features.red_chromaticity <= 0.265 {
if features.green_chromaticity <= 0.480 {
if features.green_luminance <= 63 {
if features.green_luminance <= 60 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.hue <= 61 {
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
if features.green_chromaticity <= 0.484 {
if features.green_luminance <= 81 {
if features.saturation <= 131 {
if features.value <= 70 {
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
if features.green_chromaticity <= 0.485 {
if features.saturation <= 128 {
if features.green_chromaticity <= 0.484 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 76 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.value <= 70 {
if features.value <= 61 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 132 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
}
} else {
if features.red_chromaticity <= 0.235 {
if features.luminance <= 53 {
if features.blue_luminance <= 38 {
if features.red_difference <= 113 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.green_luminance <= 68 {
Intensity::Low
} else {
if features.blue_luminance <= 41 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.237 {
if features.red_chromaticity <= 0.236 {
if features.luminance <= 55 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.478 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_difference <= 113 {
Intensity::High
} else {
if features.red_difference <= 115 {
if features.green_chromaticity <= 0.478 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.475 {
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
if features.blue_luminance <= 31 {
if features.blue_chromaticity <= 0.256 {
if features.blue_chromaticity <= 0.241 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.253 {
if features.value <= 55 {
if features.green_luminance <= 54 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.luminance <= 42 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.254 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.red_difference <= 116 {
if features.red_chromaticity <= 0.233 {
if features.red_chromaticity <= 0.229 {
if features.green_luminance <= 56 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.267 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.240 {
if features.hue <= 63 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.260 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.value <= 52 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.blue_chromaticity <= 0.282 {
if features.blue_chromaticity <= 0.265 {
if features.green_luminance <= 60 {
if features.blue_chromaticity <= 0.261 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 29 {
if features.red_luminance <= 27 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.489 {
if features.blue_chromaticity <= 0.276 {
if features.blue_chromaticity <= 0.272 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.277 {
if features.intensity <= 43 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.278 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.red_chromaticity <= 0.206 {
Intensity::High
} else {
if features.saturation <= 138 {
if features.red_chromaticity <= 0.227 {
Intensity::High
} else {
if features.red_chromaticity <= 0.228 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.493 {
if features.red_chromaticity <= 0.221 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 27 {
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
if features.red_difference <= 115 {
if features.red_luminance <= 21 {
Intensity::High
} else {
if features.red_chromaticity <= 0.204 {
Intensity::High
} else {
if features.value <= 51 {
Intensity::Low
} else {
if features.saturation <= 137 {
if features.green_chromaticity <= 0.485 {
if features.intensity <= 40 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.489 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.491 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.289 {
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
if features.blue_chromaticity <= 0.272 {
if features.saturation <= 145 {
if features.blue_chromaticity <= 0.263 {
Intensity::Low
} else {
if features.luminance <= 37 {
if features.blue_chromaticity <= 0.271 {
if features.blue_chromaticity <= 0.269 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 48 {
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
if features.green_chromaticity <= 0.475 {
if features.red_difference <= 116 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.237 {
if features.red_chromaticity <= 0.233 {
if features.blue_chromaticity <= 0.273 {
Intensity::High
} else {
if features.green_chromaticity <= 0.488 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_difference <= 121 {
if features.luminance <= 43 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 41 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.244 {
if features.green_chromaticity <= 0.476 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.240 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 124 {
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
if features.blue_chromaticity <= 0.310 {
if features.green_chromaticity <= 0.459 {
if features.green_chromaticity <= 0.440 {
if features.green_luminance <= 113 {
if features.red_difference <= 109 {
if features.value <= 101 {
Intensity::High
} else {
if features.green_chromaticity <= 0.435 {
if features.luminance <= 93 {
if features.value <= 107 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 68 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 106 {
if features.blue_chromaticity <= 0.304 {
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
}
}
} else {
if features.luminance <= 56 {
if features.green_chromaticity <= 0.439 {
if features.blue_chromaticity <= 0.309 {
if features.blue_chromaticity <= 0.296 {
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
if features.green_chromaticity <= 0.431 {
if features.green_chromaticity <= 0.430 {
if features.red_chromaticity <= 0.264 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 78 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 95 {
if features.red_chromaticity <= 0.274 {
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
}
}
} else {
if features.blue_luminance <= 87 {
if features.intensity <= 89 {
if features.blue_chromaticity <= 0.307 {
if features.blue_chromaticity <= 0.306 {
Intensity::High
} else {
if features.red_difference <= 108 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.red_luminance <= 72 {
if features.green_chromaticity <= 0.433 {
if features.intensity <= 90 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 122 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.308 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.blue_luminance <= 88 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.value <= 67 {
if features.red_chromaticity <= 0.239 {
if features.blue_chromaticity <= 0.309 {
if features.green_chromaticity <= 0.457 {
Intensity::Low
} else {
if features.intensity <= 39 {
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
if features.blue_chromaticity <= 0.290 {
Intensity::Low
} else {
if features.green_luminance <= 61 {
if features.green_chromaticity <= 0.457 {
if features.saturation <= 113 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_difference <= 122 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 118 {
if features.red_chromaticity <= 0.251 {
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
if features.blue_chromaticity <= 0.302 {
if features.red_chromaticity <= 0.259 {
if features.red_difference <= 107 {
if features.saturation <= 120 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.hue <= 65 {
if features.green_chromaticity <= 0.453 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.441 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.intensity <= 69 {
if features.value <= 88 {
if features.green_chromaticity <= 0.440 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 60 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.440 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_luminance <= 120 {
if features.green_chromaticity <= 0.458 {
if features.red_difference <= 104 {
if features.blue_luminance <= 76 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 63 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 57 {
if features.green_chromaticity <= 0.458 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.234 {
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
if features.blue_difference <= 121 {
if features.blue_chromaticity <= 0.299 {
if features.green_chromaticity <= 0.479 {
if features.blue_chromaticity <= 0.299 {
if features.blue_chromaticity <= 0.293 {
if features.red_luminance <= 42 {
if features.blue_chromaticity <= 0.290 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.245 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_luminance <= 47 {
if features.red_chromaticity <= 0.234 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.242 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.green_luminance <= 71 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.value <= 88 {
if features.saturation <= 146 {
if features.green_chromaticity <= 0.490 {
if features.green_chromaticity <= 0.488 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 41 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.496 {
if features.green_chromaticity <= 0.494 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 85 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.intensity <= 62 {
if features.red_chromaticity <= 0.214 {
if features.red_luminance <= 38 {
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
if features.saturation <= 142 {
if features.red_chromaticity <= 0.218 {
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
if features.red_difference <= 103 {
if features.green_chromaticity <= 0.470 {
if features.intensity <= 76 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 39 {
if features.hue <= 71 {
if features.green_chromaticity <= 0.506 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.303 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.306 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_chromaticity <= 0.232 {
if features.saturation <= 128 {
Intensity::High
} else {
if features.red_difference <= 110 {
if features.green_chromaticity <= 0.499 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.302 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.234 {
if features.green_chromaticity <= 0.465 {
if features.red_chromaticity <= 0.232 {
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
} else {
if features.blue_chromaticity <= 0.299 {
if features.blue_difference <= 120 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.464 {
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
if features.red_difference <= 116 {
if features.blue_chromaticity <= 0.301 {
if features.blue_luminance <= 36 {
if features.red_difference <= 112 {
if features.green_luminance <= 60 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.511 {
if features.red_luminance <= 23 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.460 {
Intensity::High
} else {
if features.luminance <= 49 {
if features.green_luminance <= 62 {
Intensity::Low
} else {
Intensity::Low
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
} else {
if features.green_chromaticity <= 0.485 {
if features.red_luminance <= 33 {
if features.blue_chromaticity <= 0.308 {
if features.red_luminance <= 26 {
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
} else {
if features.blue_chromaticity <= 0.303 {
if features.green_luminance <= 67 {
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
}
} else {
if features.red_luminance <= 19 {
if features.saturation <= 156 {
if features.blue_chromaticity <= 0.307 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.182 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.511 {
if features.red_chromaticity <= 0.198 {
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
if features.blue_chromaticity <= 0.291 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.301 {
if features.blue_chromaticity <= 0.299 {
if features.blue_chromaticity <= 0.294 {
if features.saturation <= 129 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 27 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 44 {
if features.red_chromaticity <= 0.215 {
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
if features.blue_chromaticity <= 0.331 {
if features.green_chromaticity <= 0.496 {
if features.blue_chromaticity <= 0.315 {
if features.green_chromaticity <= 0.442 {
if features.green_luminance <= 128 {
if features.red_luminance <= 49 {
if features.intensity <= 43 {
if features.blue_chromaticity <= 0.312 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 58 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_difference <= 121 {
if features.red_difference <= 109 {
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
if features.red_luminance <= 77 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 89 {
if features.green_luminance <= 123 {
if features.red_difference <= 103 {
if features.luminance <= 89 {
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
Intensity::High
}
} else {
if features.luminance <= 104 {
if features.luminance <= 101 {
if features.blue_chromaticity <= 0.311 {
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
} else {
if features.blue_luminance <= 64 {
if features.green_chromaticity <= 0.454 {
if features.value <= 63 {
if features.blue_chromaticity <= 0.328 {
if features.green_chromaticity <= 0.453 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.218 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.330 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.blue_chromaticity <= 0.316 {
if features.luminance <= 55 {
Intensity::High
} else {
if features.saturation <= 132 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.330 {
if features.blue_chromaticity <= 0.327 {
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
}
}
} else {
if features.red_luminance <= 61 {
if features.blue_chromaticity <= 0.319 {
if features.blue_chromaticity <= 0.318 {
if features.blue_chromaticity <= 0.317 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.249 {
if features.green_luminance <= 98 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.blue_difference <= 119 {
if features.hue <= 71 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 93 {
if features.saturation <= 126 {
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
if features.green_chromaticity <= 0.496 {
Intensity::High
} else {
if features.intensity <= 40 {
if features.green_chromaticity <= 0.513 {
if features.blue_chromaticity <= 0.316 {
if features.blue_chromaticity <= 0.311 {
if features.red_luminance <= 21 {
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
if features.saturation <= 171 {
if features.blue_chromaticity <= 0.320 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_difference <= 113 {
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
if features.blue_chromaticity <= 0.331 {
if features.red_difference <= 100 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.509 {
if features.red_chromaticity <= 0.168 {
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
}
}
} else {
if features.red_difference <= 107 {
if features.green_chromaticity <= 0.510 {
if features.green_luminance <= 64 {
if features.saturation <= 194 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.331 {
if features.blue_chromaticity <= 0.331 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.499 {
if features.blue_chromaticity <= 0.332 {
if features.red_chromaticity <= 0.219 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 41 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.501 {
if features.red_luminance <= 23 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 50 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.blue_difference <= 125 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.465 {
if features.green_luminance <= 77 {
if features.blue_chromaticity <= 0.354 {
if features.blue_chromaticity <= 0.353 {
if features.green_chromaticity <= 0.461 {
if features.saturation <= 148 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 143 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.red_difference <= 108 {
if features.blue_luminance <= 56 {
if features.blue_luminance <= 55 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.183 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 28 {
if features.red_luminance <= 8 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.value <= 33 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_chromaticity <= 0.450 {
if features.value <= 78 {
if features.red_chromaticity <= 0.208 {
Intensity::Low
} else {
if features.saturation <= 131 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.337 {
if features.blue_chromaticity <= 0.337 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.442 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.value <= 80 {
if features.blue_difference <= 125 {
if features.green_chromaticity <= 0.453 {
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
if features.blue_chromaticity <= 0.353 {
if features.blue_chromaticity <= 0.352 {
if features.red_chromaticity <= 0.200 {
if features.red_luminance <= 12 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.148 {
Intensity::High
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
} else {
if features.red_difference <= 108 {
if features.green_chromaticity <= 0.491 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_difference <= 116 {
if features.blue_luminance <= 37 {
Intensity::Low
} else {
if features.blue_luminance <= 38 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.368 {
if features.red_luminance <= 11 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 24 {
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
if features.blue_chromaticity <= 0.297 {
if features.blue_difference <= 121 {
if features.green_chromaticity <= 0.540 {
if features.blue_difference <= 119 {
if features.red_chromaticity <= 0.196 {
if features.red_luminance <= 27 {
if features.intensity <= 46 {
if features.red_chromaticity <= 0.180 {
Intensity::Low
} else {
if features.value <= 71 {
if features.red_luminance <= 25 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.192 {
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
if features.saturation <= 167 {
if features.red_chromaticity <= 0.186 {
Intensity::High
} else {
if features.green_chromaticity <= 0.520 {
if features.value <= 82 {
Intensity::High
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
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 22 {
if features.saturation <= 142 {
Intensity::High
} else {
if features.green_chromaticity <= 0.531 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.232 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.518 {
if features.green_chromaticity <= 0.514 {
Intensity::High
} else {
if features.green_chromaticity <= 0.517 {
if features.green_chromaticity <= 0.516 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.198 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.224 {
if features.green_luminance <= 64 {
if features.intensity <= 39 {
Intensity::High
} else {
Intensity::High
}
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
}
}
}
} else {
if features.green_luminance <= 44 {
if features.intensity <= 27 {
if features.green_chromaticity <= 0.528 {
Intensity::High
} else {
if features.value <= 41 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.529 {
if features.blue_chromaticity <= 0.292 {
if features.blue_difference <= 120 {
if features.green_chromaticity <= 0.527 {
if features.luminance <= 37 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.529 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.525 {
if features.green_chromaticity <= 0.513 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.527 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.blue_chromaticity <= 0.294 {
if features.red_difference <= 107 {
Intensity::High
} else {
if features.value <= 61 {
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
if features.blue_chromaticity <= 0.254 {
if features.green_chromaticity <= 0.535 {
Intensity::High
} else {
if features.blue_luminance <= 21 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_luminance <= 51 {
Intensity::High
} else {
if features.red_chromaticity <= 0.177 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.289 {
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
if features.red_difference <= 114 {
if features.green_chromaticity <= 0.582 {
if features.green_luminance <= 62 {
if features.green_chromaticity <= 0.581 {
if features.value <= 57 {
if features.intensity <= 34 {
if features.red_chromaticity <= 0.190 {
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
if features.red_luminance <= 15 {
if features.value <= 53 {
if features.blue_chromaticity <= 0.261 {
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
if features.blue_luminance <= 31 {
if features.blue_chromaticity <= 0.271 {
if features.saturation <= 176 {
Intensity::High
} else {
if features.red_luminance <= 17 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.566 {
Intensity::Low
} else {
if features.green_luminance <= 64 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_luminance <= 23 {
if features.value <= 69 {
if features.value <= 65 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.542 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_chromaticity <= 0.088 {
if features.blue_chromaticity <= 0.085 {
Intensity::High
} else {
if features.blue_difference <= 119 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.251 {
if features.red_chromaticity <= 0.090 {
Intensity::High
} else {
if features.red_chromaticity <= 0.092 {
if features.blue_chromaticity <= 0.209 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.saturation <= 216 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.157 {
if features.intensity <= 19 {
if features.red_difference <= 109 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.hue <= 69 {
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
if features.blue_chromaticity <= 0.235 {
if features.green_luminance <= 34 {
if features.red_difference <= 116 {
if features.value <= 26 {
if features.blue_chromaticity <= 0.019 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.691 {
if features.saturation <= 213 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.120 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.hue <= 60 {
if features.green_chromaticity <= 0.760 {
Intensity::High
} else {
if features.saturation <= 226 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.782 {
if features.red_chromaticity <= 0.132 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.value <= 26 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
} else {
if features.green_chromaticity <= 0.617 {
if features.red_chromaticity <= 0.216 {
if features.green_chromaticity <= 0.614 {
if features.red_chromaticity <= 0.201 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_difference <= 116 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.232 {
if features.value <= 47 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::High
}
}
} else {
if features.hue <= 65 {
if features.saturation <= 180 {
Intensity::High
} else {
if features.saturation <= 194 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.676 {
if features.saturation <= 209 {
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
if features.hue <= 65 {
if features.blue_chromaticity <= 0.252 {
if features.red_luminance <= 17 {
if features.blue_luminance <= 16 {
if features.red_luminance <= 10 {
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
if features.blue_chromaticity <= 0.238 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.249 {
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
if features.red_chromaticity <= 0.188 {
Intensity::High
} else {
if features.saturation <= 164 {
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
if features.red_difference <= 115 {
if features.green_chromaticity <= 0.563 {
if features.green_luminance <= 54 {
if features.red_luminance <= 16 {
if features.red_difference <= 110 {
Intensity::Low
} else {
if features.luminance <= 33 {
if features.value <= 41 {
Intensity::Low
} else {
if features.hue <= 70 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.292 {
if features.green_luminance <= 48 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_luminance <= 26 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
} else {
if features.blue_chromaticity <= 0.289 {
if features.blue_chromaticity <= 0.288 {
if features.red_chromaticity <= 0.202 {
if features.saturation <= 165 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.184 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.value <= 53 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.red_chromaticity <= 0.191 {
Intensity::High
} else {
if features.green_chromaticity <= 0.514 {
Intensity::High
} else {
Intensity::High
}
}
}
} else {
if features.blue_difference <= 123 {
if features.red_difference <= 113 {
if features.intensity <= 30 {
if features.red_chromaticity <= 0.148 {
if features.blue_chromaticity <= 0.245 {
if features.saturation <= 251 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.intensity <= 27 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.571 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.293 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.287 {
if features.green_chromaticity <= 0.886 {
if features.intensity <= 22 {
if features.red_luminance <= 8 {
Intensity::High
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
if features.green_luminance <= 42 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.value <= 32 {
if features.red_chromaticity <= 0.052 {
if features.red_chromaticity <= 0.012 {
if features.value <= 30 {
Intensity::Low
} else {
if features.intensity <= 14 {
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
if features.saturation <= 214 {
if features.green_chromaticity <= 0.597 {
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
if features.green_luminance <= 33 {
if features.blue_chromaticity <= 0.208 {
if features.intensity <= 11 {
Intensity::Low
} else {
if features.intensity <= 12 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.069 {
Intensity::High
} else {
if features.green_luminance <= 29 {
if features.green_chromaticity <= 0.687 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.green_luminance <= 30 {
Intensity::High
} else {
if features.red_chromaticity <= 0.101 {
if features.value <= 31 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.216 {
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
if features.red_chromaticity <= 0.129 {
Intensity::High
} else {
if features.value <= 37 {
if features.luminance <= 27 {
if features.intensity <= 21 {
if features.green_luminance <= 35 {
if features.blue_luminance <= 15 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 17 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.red_chromaticity <= 0.178 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.green_chromaticity <= 0.524 {
if features.luminance <= 32 {
Intensity::Low
} else {
if features.blue_luminance <= 24 {
if features.hue <= 67 {
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
if features.green_chromaticity <= 0.588 {
if features.red_chromaticity <= 0.200 {
Intensity::Low
} else {
Intensity::High
}
} else {
Intensity::High
}
} else {
if features.value <= 41 {
Intensity::High
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
}
} else {
if features.red_difference <= 114 {
if features.green_chromaticity <= 0.540 {
if features.blue_luminance <= 42 {
if features.blue_chromaticity <= 0.299 {
if features.red_chromaticity <= 0.177 {
Intensity::High
} else {
if features.blue_luminance <= 31 {
Intensity::High
} else {
if features.value <= 58 {
Intensity::Low
} else {
if features.hue <= 70 {
if features.red_luminance <= 21 {
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
} else {
if features.green_chromaticity <= 0.536 {
if features.blue_chromaticity <= 0.337 {
if features.green_chromaticity <= 0.514 {
if features.green_chromaticity <= 0.513 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.522 {
if features.green_chromaticity <= 0.520 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.332 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.green_chromaticity <= 0.526 {
if features.red_difference <= 112 {
Intensity::Low
} else {
if features.blue_luminance <= 28 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
if features.blue_luminance <= 28 {
if features.intensity <= 25 {
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
if features.red_luminance <= 17 {
if features.green_chromaticity <= 0.540 {
Intensity::High
} else {
if features.red_luminance <= 11 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.156 {
Intensity::High
} else {
if features.red_chromaticity <= 0.157 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.303 {
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
if features.red_chromaticity <= 0.159 {
if features.blue_difference <= 119 {
Intensity::High
} else {
if features.luminance <= 59 {
if features.green_luminance <= 74 {
if features.blue_luminance <= 46 {
if features.saturation <= 188 {
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
if features.value <= 85 {
Intensity::High
} else {
if features.hue <= 73 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
} else {
if features.saturation <= 175 {
if features.green_chromaticity <= 0.530 {
if features.red_chromaticity <= 0.162 {
Intensity::High
} else {
if features.green_chromaticity <= 0.517 {
if features.red_luminance <= 25 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.517 {
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
if features.hue <= 77 {
if features.red_difference <= 110 {
if features.red_luminance <= 20 {
if features.green_chromaticity <= 0.541 {
Intensity::Low
} else {
if features.luminance <= 26 {
if features.red_difference <= 109 {
if features.blue_difference <= 124 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.008 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_difference <= 120 {
if features.blue_chromaticity <= 0.300 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.314 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.blue_difference <= 119 {
if features.blue_chromaticity <= 0.300 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.298 {
if features.saturation <= 186 {
Intensity::Low
} else {
if features.blue_luminance <= 23 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.blue_chromaticity <= 0.328 {
if features.red_difference <= 113 {
if features.green_chromaticity <= 0.699 {
if features.saturation <= 205 {
Intensity::High
} else {
Intensity::High
}
} else {
Intensity::Low
}
} else {
if features.saturation <= 211 {
if features.saturation <= 200 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.blue_chromaticity <= 0.327 {
Intensity::Low
} else {
Intensity::High
}
}
}
} else {
if features.blue_chromaticity <= 0.338 {
if features.blue_luminance <= 25 {
if features.blue_chromaticity <= 0.329 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.saturation <= 196 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.344 {
Intensity::High
} else {
if features.green_luminance <= 37 {
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
if features.green_chromaticity <= 0.608 {
if features.blue_chromaticity <= 0.417 {
if features.red_difference <= 113 {
if features.red_difference <= 112 {
if features.green_chromaticity <= 0.599 {
if features.intensity <= 24 {
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
if features.blue_chromaticity <= 0.380 {
Intensity::Low
} else {
if features.hue <= 81 {
Intensity::High
} else {
Intensity::Low
}
}
}
} else {
if features.red_difference <= 113 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.intensity <= 18 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.blue_difference <= 125 {
if features.red_difference <= 115 {
if features.red_chromaticity <= 0.095 {
if features.value <= 30 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 40 {
if features.green_chromaticity <= 0.555 {
if features.green_chromaticity <= 0.540 {
if features.red_luminance <= 11 {
Intensity::High
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 8 {
Intensity::Low
} else {
if features.red_luminance <= 10 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.299 {
if features.blue_luminance <= 19 {
Intensity::High
} else {
Intensity::High
}
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
}
} else {
if features.red_luminance <= 4 {
Intensity::Low
} else {
if features.value <= 33 {
Intensity::High
} else {
if features.blue_chromaticity <= 0.320 {
if features.blue_chromaticity <= 0.313 {
if features.value <= 37 {
if features.blue_chromaticity <= 0.306 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 21 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.314 {
Intensity::High
} else {
if features.intensity <= 22 {
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
if features.green_luminance <= 37 {
if features.blue_chromaticity <= 0.353 {
if features.intensity <= 18 {
if features.green_chromaticity <= 0.667 {
if features.green_chromaticity <= 0.590 {
if features.green_chromaticity <= 0.584 {
Intensity::Low
} else {
Intensity::High
}
} else {
if features.red_luminance <= 0 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.680 {
Intensity::High
} else {
if features.luminance <= 14 {
Intensity::Low
} else {
if features.value <= 25 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
} else {
if features.intensity <= 19 {
if features.red_difference <= 115 {
Intensity::Low
} else {
if features.red_difference <= 116 {
Intensity::High
} else {
if features.green_chromaticity <= 0.534 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.145 {
if features.red_chromaticity <= 0.141 {
if features.luminance <= 23 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.153 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.158 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.green_chromaticity <= 0.600 {
if features.blue_chromaticity <= 0.381 {
if features.luminance <= 17 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.383 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.red_difference <= 116 {
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
}
}
} else {
if features.red_difference <= 119 {
if features.blue_chromaticity <= 0.265 {
if features.blue_difference <= 120 {
if features.red_difference <= 118 {
if features.blue_chromaticity <= 0.252 {
if features.hue <= 59 {
if features.red_chromaticity <= 0.251 {
if features.green_chromaticity <= 0.567 {
if features.blue_luminance <= 19 {
if features.saturation <= 149 {
Intensity::High
} else {
if features.saturation <= 156 {
Intensity::High
} else {
Intensity::High
}
}
} else {
Intensity::High
}
} else {
if features.red_luminance <= 15 {
if features.green_chromaticity <= 0.645 {
if features.green_chromaticity <= 0.600 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.intensity <= 14 {
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
if features.green_chromaticity <= 0.488 {
if features.value <= 53 {
Intensity::High
} else {
if features.luminance <= 44 {
Intensity::High
} else {
Intensity::High
}
}
} else {
if features.saturation <= 136 {
if features.intensity <= 32 {
Intensity::Low
} else {
if features.green_luminance <= 51 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.232 {
Intensity::High
} else {
Intensity::High
}
}
}
}
} else {
if features.luminance <= 15 {
Intensity::Low
} else {
if features.blue_luminance <= 17 {
if features.green_chromaticity <= 0.638 {
if features.green_chromaticity <= 0.607 {
if features.saturation <= 163 {
Intensity::High
} else {
Intensity::High
}
} else {
if features.red_luminance <= 10 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.798 {
if features.luminance <= 18 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
}
} else {
if features.green_chromaticity <= 0.521 {
if features.saturation <= 134 {
if features.saturation <= 131 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::High
}
} else {
if features.red_chromaticity <= 0.233 {
Intensity::Low
} else {
if features.saturation <= 140 {
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
if features.red_chromaticity <= 0.258 {
if features.luminance <= 39 {
Intensity::Low
} else {
if features.luminance <= 40 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.261 {
if features.green_chromaticity <= 0.466 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.255 {
Intensity::Low
} else {
if features.red_luminance <= 30 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.469 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_chromaticity <= 0.262 {
if features.blue_difference <= 119 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 48 {
if features.saturation <= 112 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 35 {
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
if features.blue_chromaticity <= 0.252 {
if features.green_luminance <= 33 {
if features.green_chromaticity <= 0.948 {
if features.green_luminance <= 32 {
if features.blue_chromaticity <= 0.104 {
if features.red_luminance <= 4 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.164 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.180 {
if features.saturation <= 199 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 31 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.636 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::High
}
} else {
if features.blue_difference <= 119 {
if features.intensity <= 34 {
if features.blue_chromaticity <= 0.165 {
if features.blue_chromaticity <= 0.156 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 149 {
if features.green_chromaticity <= 0.508 {
Intensity::Low
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
}
} else {
if features.saturation <= 122 {
if features.value <= 55 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.480 {
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
} else {
if features.green_luminance <= 38 {
if features.green_chromaticity <= 0.577 {
if features.saturation <= 161 {
if features.value <= 37 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 11 {
Intensity::High
} else {
Intensity::Low
}
}
} else {
if features.value <= 45 {
if features.green_luminance <= 44 {
if features.red_luminance <= 19 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.intensity <= 31 {
Intensity::High
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.blue_chromaticity <= 0.255 {
if features.blue_chromaticity <= 0.255 {
if features.blue_difference <= 119 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.260 {
if features.blue_chromaticity <= 0.257 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.259 {
if features.red_chromaticity <= 0.274 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 31 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.262 {
if features.saturation <= 111 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.263 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.265 {
if features.red_luminance <= 35 {
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
if features.green_luminance <= 32 {
if features.value <= 30 {
if features.hue <= 63 {
if features.green_luminance <= 28 {
if features.blue_chromaticity <= 0.171 {
if features.green_chromaticity <= 0.772 {
if features.red_luminance <= 5 {
Intensity::High
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 27 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.670 {
if features.blue_difference <= 121 {
Intensity::Low
} else {
if features.saturation <= 182 {
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
if features.green_chromaticity <= 0.600 {
if features.value <= 29 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.175 {
if features.green_chromaticity <= 0.589 {
Intensity::High
} else {
if features.green_chromaticity <= 0.600 {
Intensity::Low
} else {
if features.green_luminance <= 31 {
if features.green_chromaticity <= 0.640 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.165 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.red_chromaticity <= 0.185 {
if features.green_chromaticity <= 0.592 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 31 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.588 {
if features.green_chromaticity <= 0.562 {
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
if features.red_difference <= 118 {
if features.blue_chromaticity <= 0.243 {
if features.saturation <= 176 {
if features.blue_chromaticity <= 0.236 {
if features.green_chromaticity <= 0.564 {
Intensity::High
} else {
if features.red_chromaticity <= 0.190 {
Intensity::High
} else {
if features.red_luminance <= 12 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_luminance <= 14 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.557 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_luminance <= 33 {
if features.saturation <= 181 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.497 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.260 {
if features.blue_chromaticity <= 0.252 {
if features.saturation <= 161 {
if features.green_chromaticity <= 0.529 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.luminance <= 29 {
if features.intensity <= 21 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.523 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.262 {
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
if features.blue_luminance <= 23 {
if features.blue_difference <= 121 {
if features.intensity <= 20 {
if features.saturation <= 167 {
if features.intensity <= 19 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 18 {
if features.red_luminance <= 14 {
Intensity::Low
} else {
if features.saturation <= 150 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 135 {
if features.green_chromaticity <= 0.497 {
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
if features.green_chromaticity <= 0.525 {
if features.blue_luminance <= 20 {
if features.red_luminance <= 16 {
Intensity::Low
} else {
if features.value <= 38 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 16 {
if features.intensity <= 19 {
Intensity::Low
} else {
if features.green_luminance <= 34 {
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
if features.saturation <= 123 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.blue_chromaticity <= 0.282 {
if features.red_difference <= 118 {
if features.blue_chromaticity <= 0.273 {
if features.blue_difference <= 119 {
if features.red_chromaticity <= 0.281 {
Intensity::Low
} else {
if features.luminance <= 55 {
if features.value <= 64 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 40 {
Intensity::Low
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
if features.red_chromaticity <= 0.251 {
if features.saturation <= 123 {
Intensity::High
} else {
if features.saturation <= 149 {
if features.saturation <= 126 {
Intensity::Low
} else {
if features.intensity <= 27 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.268 {
if features.red_chromaticity <= 0.149 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_difference <= 123 {
Intensity::Low
} else {
Intensity::High
}
}
}
}
} else {
if features.blue_chromaticity <= 0.271 {
if features.blue_chromaticity <= 0.267 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.269 {
Intensity::Low
} else {
if features.blue_luminance <= 31 {
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
if features.saturation <= 103 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.intensity <= 51 {
if features.blue_chromaticity <= 0.281 {
if features.blue_chromaticity <= 0.279 {
if features.luminance <= 25 {
Intensity::Low
} else {
if features.blue_luminance <= 39 {
if features.blue_chromaticity <= 0.276 {
Intensity::Low
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
} else {
if features.blue_difference <= 123 {
if features.saturation <= 100 {
if features.green_luminance <= 66 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_difference <= 122 {
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
} else {
if features.green_luminance <= 73 {
if features.saturation <= 94 {
if features.intensity <= 54 {
Intensity::Low
} else {
if features.saturation <= 91 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.287 {
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
if features.blue_chromaticity <= 0.273 {
if features.luminance <= 37 {
if features.saturation <= 194 {
if features.blue_chromaticity <= 0.265 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.268 {
if features.blue_chromaticity <= 0.267 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.269 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.231 {
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
if features.saturation <= 95 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.267 {
if features.saturation <= 105 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.469 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.469 {
if features.green_luminance <= 53 {
if features.saturation <= 111 {
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
} else {
if features.red_chromaticity <= 0.259 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.saturation <= 89 {
Intensity::Low
} else {
if features.green_luminance <= 58 {
if features.value <= 39 {
if features.value <= 37 {
if features.blue_chromaticity <= 0.276 {
if features.red_luminance <= 12 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 30 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.luminance <= 30 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.459 {
if features.saturation <= 103 {
if features.saturation <= 99 {
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
if features.green_chromaticity <= 0.463 {
if features.blue_luminance <= 30 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.279 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_chromaticity <= 0.277 {
if features.blue_chromaticity <= 0.275 {
if features.blue_luminance <= 41 {
if features.blue_chromaticity <= 0.275 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.276 {
Intensity::Low
} else {
if features.luminance <= 58 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.luminance <= 55 {
if features.intensity <= 48 {
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
if features.blue_chromaticity <= 0.289 {
if features.red_luminance <= 35 {
if features.intensity <= 42 {
if features.blue_chromaticity <= 0.283 {
if features.green_luminance <= 41 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.239 {
Intensity::Low
} else {
if features.green_luminance <= 50 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_difference <= 118 {
if features.blue_chromaticity <= 0.288 {
if features.hue <= 68 {
if features.saturation <= 106 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 21 {
Intensity::Low
} else {
Intensity::High
}
}
} else {
Intensity::Low
}
} else {
if features.value <= 48 {
if features.blue_difference <= 123 {
if features.red_luminance <= 22 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.519 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.447 {
if features.red_chromaticity <= 0.271 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 32 {
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
if features.red_chromaticity <= 0.284 {
if features.blue_chromaticity <= 0.284 {
if features.luminance <= 58 {
if features.luminance <= 49 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.438 {
Intensity::Low
} else {
if features.green_luminance <= 59 {
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
if features.luminance <= 53 {
if features.red_luminance <= 38 {
if features.red_difference <= 118 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 62 {
if features.green_chromaticity <= 0.435 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.431 {
Intensity::Low
} else {
if features.red_luminance <= 40 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_luminance <= 45 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.hue <= 72 {
if features.blue_luminance <= 34 {
if features.red_difference <= 118 {
if features.blue_chromaticity <= 0.302 {
if features.intensity <= 30 {
if features.saturation <= 145 {
if features.value <= 41 {
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
if features.value <= 51 {
if features.green_chromaticity <= 0.460 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.value <= 36 {
if features.blue_chromaticity <= 0.314 {
if features.luminance <= 21 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 35 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.465 {
if features.luminance <= 40 {
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
if features.saturation <= 106 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.249 {
if features.blue_chromaticity <= 0.305 {
if features.green_chromaticity <= 0.460 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.313 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 108 {
Intensity::Low
} else {
if features.saturation <= 110 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.green_chromaticity <= 0.442 {
if features.blue_chromaticity <= 0.302 {
if features.red_chromaticity <= 0.275 {
if features.blue_luminance <= 39 {
if features.green_chromaticity <= 0.439 {
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
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.242 {
if features.red_chromaticity <= 0.240 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.448 {
if features.luminance <= 48 {
if features.blue_chromaticity <= 0.307 {
if features.green_chromaticity <= 0.444 {
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
if features.green_chromaticity <= 0.430 {
if features.saturation <= 115 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.343 {
if features.blue_chromaticity <= 0.343 {
if features.red_chromaticity <= 0.203 {
if features.luminance <= 28 {
if features.red_difference <= 118 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.value <= 37 {
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
Intensity::Low
}
}
}
}
}
}
} else {
if features.blue_difference <= 120 {
if features.red_difference <= 121 {
if features.saturation <= 114 {
if features.green_luminance <= 55 {
if features.green_chromaticity <= 0.444 {
if features.red_chromaticity <= 0.296 {
if features.saturation <= 103 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.452 {
if features.red_luminance <= 36 {
if features.green_chromaticity <= 0.446 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.451 {
if features.green_chromaticity <= 0.449 {
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
if features.green_chromaticity <= 0.453 {
Intensity::Low
} else {
if features.saturation <= 111 {
Intensity::Low
} else {
if features.blue_luminance <= 27 {
if features.red_chromaticity <= 0.286 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.hue <= 55 {
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
if features.green_luminance <= 61 {
if features.red_chromaticity <= 0.295 {
if features.green_luminance <= 56 {
Intensity::Low
} else {
if features.luminance <= 50 {
if features.blue_chromaticity <= 0.270 {
if features.green_chromaticity <= 0.451 {
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
if features.green_chromaticity <= 0.446 {
if features.green_chromaticity <= 0.435 {
if features.luminance <= 52 {
if features.blue_luminance <= 37 {
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
if features.blue_chromaticity <= 0.266 {
if features.luminance <= 49 {
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
if features.blue_luminance <= 37 {
Intensity::Low
} else {
if features.intensity <= 49 {
if features.luminance <= 54 {
if features.hue <= 53 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.430 {
Intensity::Low
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
}
} else {
if features.red_difference <= 120 {
if features.red_chromaticity <= 0.230 {
if features.saturation <= 190 {
Intensity::Low
} else {
if features.value <= 27 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.692 {
if features.blue_chromaticity <= 0.146 {
if features.green_chromaticity <= 0.659 {
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
if features.blue_chromaticity <= 0.234 {
if features.green_luminance <= 45 {
if features.blue_chromaticity <= 0.170 {
if features.saturation <= 190 {
if features.blue_chromaticity <= 0.162 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.619 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.269 {
if features.red_chromaticity <= 0.266 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.522 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.280 {
Intensity::Low
} else {
if features.green_luminance <= 47 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_difference <= 119 {
if features.green_chromaticity <= 0.478 {
if features.saturation <= 117 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.242 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.saturation <= 120 {
if features.saturation <= 118 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.491 {
if features.blue_luminance <= 21 {
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
if features.hue <= 54 {
if features.blue_chromaticity <= 0.178 {
if features.saturation <= 188 {
if features.saturation <= 182 {
if features.green_chromaticity <= 0.561 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 31 {
if features.intensity <= 15 {
if features.saturation <= 223 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 200 {
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
if features.red_chromaticity <= 0.294 {
if features.blue_luminance <= 12 {
Intensity::Low
} else {
if features.value <= 40 {
if features.green_luminance <= 38 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.497 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.295 {
Intensity::Low
} else {
if features.saturation <= 121 {
if features.green_chromaticity <= 0.458 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 41 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.green_luminance <= 40 {
if features.saturation <= 190 {
if features.intensity <= 22 {
if features.saturation <= 172 {
if features.saturation <= 158 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.578 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.510 {
if features.green_chromaticity <= 0.503 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 15 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_luminance <= 23 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.732 {
if features.value <= 26 {
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
if features.saturation <= 118 {
Intensity::Low
} else {
if features.luminance <= 35 {
if features.blue_chromaticity <= 0.231 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 31 {
if features.saturation <= 126 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 120 {
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
if features.red_difference <= 122 {
if features.hue <= 50 {
if features.saturation <= 112 {
if features.green_chromaticity <= 0.434 {
if features.saturation <= 106 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.saturation <= 186 {
if features.intensity <= 19 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.442 {
Intensity::Low
} else {
if features.intensity <= 37 {
if features.blue_luminance <= 20 {
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
if features.luminance <= 15 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.663 {
if features.saturation <= 221 {
if features.luminance <= 20 {
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
if features.green_chromaticity <= 0.440 {
Intensity::Low
} else {
if features.saturation <= 188 {
if features.red_luminance <= 28 {
if features.red_luminance <= 19 {
if features.saturation <= 170 {
if features.saturation <= 164 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.162 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.298 {
if features.green_chromaticity <= 0.479 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.465 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.saturation <= 115 {
if features.red_luminance <= 32 {
Intensity::Low
} else {
if features.saturation <= 110 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.blue_luminance <= 23 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.303 {
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
if features.green_chromaticity <= 0.430 {
Intensity::Low
} else {
if features.blue_difference <= 119 {
if features.red_chromaticity <= 0.315 {
if features.intensity <= 26 {
if features.intensity <= 17 {
Intensity::Low
} else {
if features.saturation <= 170 {
if features.blue_chromaticity <= 0.186 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.528 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_luminance <= 41 {
if features.blue_chromaticity <= 0.207 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.223 {
if features.blue_chromaticity <= 0.149 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.151 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.354 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.green_chromaticity <= 0.437 {
if features.red_chromaticity <= 0.327 {
if features.green_luminance <= 50 {
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
if features.red_chromaticity <= 0.327 {
if features.green_chromaticity <= 0.442 {
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
if features.red_chromaticity <= 0.342 {
if features.red_chromaticity <= 0.341 {
if features.red_chromaticity <= 0.337 {
if features.blue_chromaticity <= 0.249 {
if features.red_chromaticity <= 0.305 {
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
} else {
if features.green_chromaticity <= 0.448 {
if features.red_luminance <= 28 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.338 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.saturation <= 206 {
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
if features.red_difference <= 120 {
if features.hue <= 61 {
if features.blue_chromaticity <= 0.265 {
if features.saturation <= 160 {
if features.blue_chromaticity <= 0.228 {
if features.value <= 31 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.219 {
if features.blue_luminance <= 12 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 13 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.535 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_chromaticity <= 0.258 {
if features.green_chromaticity <= 0.507 {
if features.saturation <= 122 {
if features.saturation <= 120 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 40 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.527 {
if features.red_chromaticity <= 0.248 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.536 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.red_chromaticity <= 0.260 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.263 {
if features.blue_chromaticity <= 0.261 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.hue <= 59 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.hue <= 59 {
if features.luminance <= 22 {
if features.value <= 28 {
if features.value <= 23 {
Intensity::Low
} else {
if features.saturation <= 192 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.596 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 7 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.584 {
Intensity::Low
} else {
if features.value <= 28 {
if features.green_chromaticity <= 0.618 {
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
if features.green_chromaticity <= 0.437 {
if features.saturation <= 87 {
Intensity::Low
} else {
if features.red_luminance <= 36 {
if features.blue_luminance <= 35 {
if features.intensity <= 41 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_luminance <= 37 {
Intensity::Low
} else {
if features.luminance <= 49 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_chromaticity <= 0.276 {
if features.saturation <= 102 {
if features.blue_chromaticity <= 0.275 {
if features.blue_chromaticity <= 0.273 {
if features.green_chromaticity <= 0.449 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.274 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.459 {
if features.green_chromaticity <= 0.455 {
Intensity::Low
} else {
if features.hue <= 59 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 109 {
if features.saturation <= 106 {
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
if features.blue_chromaticity <= 0.280 {
if features.intensity <= 37 {
Intensity::Low
} else {
if features.red_luminance <= 32 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.440 {
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
if features.blue_chromaticity <= 0.294 {
if features.red_chromaticity <= 0.210 {
Intensity::Low
} else {
if features.red_luminance <= 25 {
if features.red_chromaticity <= 0.240 {
if features.green_chromaticity <= 0.483 {
if features.luminance <= 28 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.luminance <= 25 {
if features.red_chromaticity <= 0.220 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.saturation <= 137 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_chromaticity <= 0.282 {
if features.red_luminance <= 22 {
if features.green_luminance <= 36 {
Intensity::Low
} else {
Intensity::Low
}
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
}
} else {
if features.green_chromaticity <= 0.447 {
if features.green_chromaticity <= 0.432 {
Intensity::Low
} else {
if features.intensity <= 33 {
Intensity::Low
} else {
if features.saturation <= 95 {
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
if features.luminance <= 36 {
if features.blue_chromaticity <= 0.338 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.339 {
if features.green_chromaticity <= 0.471 {
if features.red_chromaticity <= 0.203 {
if features.value <= 29 {
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
if features.green_luminance <= 44 {
if features.green_chromaticity <= 0.436 {
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
if features.value <= 31 {
if features.blue_difference <= 121 {
if features.red_difference <= 122 {
if features.blue_chromaticity <= 0.200 {
if features.blue_chromaticity <= 0.184 {
if features.blue_chromaticity <= 0.150 {
Intensity::Low
} else {
if features.saturation <= 182 {
if features.green_chromaticity <= 0.587 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_chromaticity <= 0.163 {
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
if features.blue_chromaticity <= 0.207 {
if features.red_luminance <= 14 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
}
} else {
if features.blue_chromaticity <= 0.215 {
if features.red_chromaticity <= 0.345 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.346 {
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
if features.green_luminance <= 28 {
if features.red_difference <= 123 {
if features.saturation <= 158 {
if features.green_chromaticity <= 0.511 {
if features.blue_luminance <= 14 {
if features.saturation <= 120 {
Intensity::Low
} else {
Intensity::Low
}
} else {
Intensity::Low
}
} else {
if features.red_chromaticity <= 0.198 {
Intensity::Low
} else {
if features.saturation <= 136 {
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
if features.hue <= 28 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.484 {
if features.red_chromaticity <= 0.266 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.269 {
Intensity::Low
} else {
if features.green_chromaticity <= 0.462 {
Intensity::Low
} else {
if features.red_difference <= 123 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
} else {
if features.blue_chromaticity <= 0.243 {
if features.blue_chromaticity <= 0.213 {
if features.green_chromaticity <= 0.533 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.hue <= 54 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.red_chromaticity <= 0.264 {
if features.saturation <= 134 {
if features.blue_difference <= 123 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_luminance <= 30 {
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
if features.green_chromaticity <= 0.479 {
if features.green_luminance <= 40 {
if features.green_chromaticity <= 0.470 {
if features.blue_chromaticity <= 0.253 {
if features.blue_chromaticity <= 0.245 {
if features.blue_chromaticity <= 0.237 {
Intensity::Low
} else {
if features.green_luminance <= 37 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.green_chromaticity <= 0.442 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.251 {
Intensity::Low
} else {
Intensity::Low
}
}
}
} else {
if features.blue_difference <= 124 {
if features.blue_luminance <= 24 {
if features.green_chromaticity <= 0.455 {
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
if features.blue_chromaticity <= 0.309 {
if features.saturation <= 118 {
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
if features.blue_chromaticity <= 0.285 {
if features.saturation <= 116 {
Intensity::Low
} else {
if features.saturation <= 125 {
if features.saturation <= 122 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.intensity <= 23 {
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
if features.blue_chromaticity <= 0.249 {
Intensity::Low
} else {
if features.blue_chromaticity <= 0.274 {
if features.blue_chromaticity <= 0.273 {
if features.red_chromaticity <= 0.285 {
if features.red_chromaticity <= 0.282 {
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
Intensity::Low
}
} else {
if features.blue_luminance <= 29 {
if features.saturation <= 96 {
if features.red_chromaticity <= 0.281 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 25 {
Intensity::Low
} else {
Intensity::Low
}
}
} else {
if features.saturation <= 90 {
if features.blue_chromaticity <= 0.280 {
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
if features.luminance <= 27 {
if features.hue <= 54 {
Intensity::Low
} else {
if features.red_chromaticity <= 0.252 {
if features.red_chromaticity <= 0.240 {
Intensity::Low
} else {
if features.saturation <= 141 {
if features.blue_chromaticity <= 0.246 {
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
} else {
if features.red_chromaticity <= 0.256 {
if features.blue_difference <= 121 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.green_chromaticity <= 0.489 {
Intensity::Low
} else {
if features.green_luminance <= 33 {
Intensity::Low
} else {
Intensity::Low
}
}
}
}
}
} else {
if features.green_chromaticity <= 0.493 {
if features.value <= 38 {
if features.value <= 35 {
if features.blue_difference <= 122 {
Intensity::Low
} else {
Intensity::Low
}
} else {
if features.blue_luminance <= 17 {
Intensity::Low
} else {
if features.luminance <= 29 {
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
if features.green_chromaticity <= 0.497 {
Intensity::Low
} else {
if features.blue_luminance <= 16 {
Intensity::Low
} else {
if features.intensity <= 23 {
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
}
}
}
}