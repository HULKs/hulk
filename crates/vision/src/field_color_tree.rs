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
if features.blue_difference <= 113 {
            if features.green_chromaticity <= 0.421 {
            if features.green_chromaticity <= 0.409 {
            if features.green_chromaticity <= 0.401 {
            if features.green_chromaticity <= 0.395 {
            if features.green_chromaticity <= 0.390 {
            if features.green_chromaticity <= 0.385 {
            if features.red_chromaticity <= 0.361 {
            if features.blue_difference <= 107 {
            if features.red_chromaticity <= 0.361 {
            if features.blue_luminance <= 131 {
            if features.green_chromaticity <= 0.376 {
            Intensity::High
            } else {
            if features.blue_luminance <= 128 {
            if features.red_chromaticity <= 0.357 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.hue <= 39 {
            if features.green_luminance <= 237 {
            Intensity::Low
            } else {
            if features.intensity <= 215 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.value <= 201 {
            Intensity::Low
            } else {
            if features.intensity <= 184 {
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
            if features.blue_difference <= 111 {
            if features.saturation <= 45 {
            if features.green_chromaticity <= 0.362 {
            if features.value <= 210 {
            if features.saturation <= 43 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.green_luminance <= 213 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.290 {
            if features.blue_chromaticity <= 0.290 {
            if features.value <= 201 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.349 {
            if features.value <= 203 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_difference <= 128 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.red_chromaticity <= 0.314 {
            if features.red_chromaticity <= 0.313 {
            Intensity::Low
            } else {
            if features.blue_luminance <= 165 {
            if features.green_luminance <= 199 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_luminance <= 153 {
            if features.intensity <= 156 {
            if features.blue_difference <= 112 {
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
            if features.blue_chromaticity <= 0.301 {
            Intensity::Low
            } else {
            if features.blue_chromaticity <= 0.301 {
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
            Intensity::Low
            }
            } else {
            if features.luminance <= 204 {
            if features.hue <= 49 {
            if features.blue_luminance <= 107 {
            if features.green_chromaticity <= 0.386 {
            if features.saturation <= 76 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_luminance <= 104 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.387 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.385 {
            Intensity::High
            } else {
            if features.blue_difference <= 108 {
            if features.green_chromaticity <= 0.385 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.284 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_difference <= 121 {
            if features.red_difference <= 117 {
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
            if features.red_chromaticity <= 0.316 {
            if features.green_chromaticity <= 0.390 {
            if features.green_chromaticity <= 0.386 {
            if features.green_chromaticity <= 0.386 {
            if features.red_chromaticity <= 0.313 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.intensity <= 157 {
            if features.blue_chromaticity <= 0.298 {
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
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.389 {
            if features.blue_chromaticity <= 0.288 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.387 {
            if features.green_chromaticity <= 0.386 {
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
            }
            } else {
            if features.red_luminance <= 147 {
            if features.green_chromaticity <= 0.389 {
            if features.red_luminance <= 130 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.red_luminance <= 158 {
            if features.blue_difference <= 109 {
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
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.290 {
            if features.green_chromaticity <= 0.390 {
            Intensity::High
            } else {
            if features.blue_luminance <= 93 {
            if features.green_chromaticity <= 0.390 {
            if features.blue_difference <= 106 {
            Intensity::High
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
            } else {
            if features.red_chromaticity <= 0.325 {
            if features.green_luminance <= 137 {
            Intensity::High
            } else {
            if features.intensity <= 160 {
            if features.blue_luminance <= 138 {
            if features.red_luminance <= 135 {
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
            if features.green_chromaticity <= 0.391 {
            if features.green_chromaticity <= 0.390 {
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
            if features.red_chromaticity <= 0.326 {
            if features.blue_chromaticity <= 0.279 {
            Intensity::High
            } else {
            if features.red_luminance <= 159 {
            if features.green_chromaticity <= 0.394 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.luminance <= 156 {
            if features.blue_chromaticity <= 0.270 {
            if features.blue_chromaticity <= 0.265 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.luminance <= 155 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.282 {
            if features.red_chromaticity <= 0.330 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.327 {
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
            if features.green_luminance <= 212 {
            if features.hue <= 55 {
            if features.blue_difference <= 111 {
            if features.green_chromaticity <= 0.393 {
            if features.blue_chromaticity <= 0.294 {
            if features.blue_chromaticity <= 0.292 {
            if features.red_difference <= 114 {
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
            if features.blue_chromaticity <= 0.297 {
            if features.red_chromaticity <= 0.313 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.luminance <= 174 {
            if features.red_luminance <= 148 {
            if features.saturation <= 65 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_luminance <= 139 {
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
            if features.red_chromaticity <= 0.318 {
            if features.red_chromaticity <= 0.311 {
            if features.red_chromaticity <= 0.311 {
            if features.blue_luminance <= 130 {
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
            if features.blue_chromaticity <= 0.292 {
            if features.value <= 166 {
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
            }
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.395 {
            if features.red_chromaticity <= 0.309 {
            if features.red_chromaticity <= 0.309 {
            if features.blue_chromaticity <= 0.299 {
            if features.blue_chromaticity <= 0.299 {
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
            if features.green_chromaticity <= 0.395 {
            if features.hue <= 59 {
            if features.green_luminance <= 181 {
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
            Intensity::High
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.286 {
            if features.luminance <= 126 {
            if features.blue_chromaticity <= 0.249 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.354 {
            if features.red_luminance <= 114 {
            if features.blue_difference <= 107 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.396 {
            if features.green_chromaticity <= 0.396 {
            if features.green_chromaticity <= 0.396 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            } else {
            if features.red_luminance <= 94 {
            Intensity::Low
            } else {
            if features.blue_luminance <= 70 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.saturation <= 94 {
            if features.red_difference <= 122 {
            if features.green_chromaticity <= 0.396 {
            Intensity::Low
            } else {
            if features.green_luminance <= 136 {
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
            } else {
            Intensity::High
            }
            }
            } else {
            if features.saturation <= 77 {
            if features.green_chromaticity <= 0.399 {
            if features.blue_chromaticity <= 0.284 {
            if features.red_difference <= 113 {
            Intensity::High
            } else {
            if features.red_luminance <= 121 {
            if features.blue_chromaticity <= 0.281 {
            if features.intensity <= 116 {
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
            } else {
            if features.value <= 174 {
            if features.red_chromaticity <= 0.326 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_luminance <= 152 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.red_luminance <= 147 {
            if features.red_difference <= 114 {
            if features.red_difference <= 113 {
            if features.blue_luminance <= 129 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.286 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_luminance <= 157 {
            if features.saturation <= 72 {
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
            if features.red_chromaticity <= 0.316 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.286 {
            if features.green_luminance <= 185 {
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
            if features.red_chromaticity <= 0.321 {
            if features.green_chromaticity <= 0.400 {
            if features.green_chromaticity <= 0.400 {
            if features.blue_luminance <= 132 {
            if features.intensity <= 146 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.value <= 191 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_luminance <= 118 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_difference <= 115 {
            if features.blue_difference <= 111 {
            if features.green_chromaticity <= 0.401 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_luminance <= 102 {
            if features.blue_difference <= 112 {
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
            Intensity::High
            }
            }
            } else {
            if features.red_luminance <= 145 {
            if features.green_chromaticity <= 0.396 {
            Intensity::High
            } else {
            if features.blue_luminance <= 109 {
            if features.green_chromaticity <= 0.398 {
            if features.red_luminance <= 124 {
            if features.red_luminance <= 119 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.intensity <= 134 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_chromaticity <= 0.323 {
            Intensity::Low
            } else {
            if features.blue_luminance <= 107 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.intensity <= 134 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.400 {
            if features.green_chromaticity <= 0.400 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.saturation <= 78 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            }
            }
            } else {
            if features.intensity <= 170 {
            if features.blue_luminance <= 134 {
            if features.red_difference <= 115 {
            if features.blue_chromaticity <= 0.277 {
            Intensity::High
            } else {
            if features.red_luminance <= 148 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_luminance <= 131 {
            if features.red_luminance <= 149 {
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
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.luminance <= 193 {
            if features.blue_chromaticity <= 0.292 {
            if features.green_luminance <= 162 {
            if features.green_chromaticity <= 0.401 {
            if features.green_chromaticity <= 0.398 {
            if features.green_luminance <= 155 {
            if features.luminance <= 139 {
            if features.intensity <= 128 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_luminance <= 113 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.intensity <= 132 {
            if features.green_chromaticity <= 0.398 {
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
            if features.blue_chromaticity <= 0.290 {
            if features.green_chromaticity <= 0.400 {
            if features.red_chromaticity <= 0.314 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.saturation <= 69 {
            if features.blue_chromaticity <= 0.291 {
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
            }
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.intensity <= 170 {
            if features.intensity <= 139 {
            if features.blue_chromaticity <= 0.287 {
            Intensity::Low
            } else {
            if features.luminance <= 150 {
            if features.red_chromaticity <= 0.312 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_chromaticity <= 0.289 {
            if features.red_chromaticity <= 0.312 {
            if features.saturation <= 71 {
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
            if features.blue_chromaticity <= 0.292 {
            if features.red_chromaticity <= 0.310 {
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
            if features.green_chromaticity <= 0.395 {
            if features.saturation <= 65 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.saturation <= 65 {
            if features.blue_chromaticity <= 0.299 {
            if features.green_chromaticity <= 0.400 {
            if features.red_difference <= 109 {
            if features.saturation <= 63 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.green_luminance <= 176 {
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
            if features.blue_difference <= 112 {
            if features.blue_chromaticity <= 0.296 {
            if features.red_luminance <= 127 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.307 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_luminance <= 177 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.303 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.red_luminance <= 124 {
            if features.intensity <= 132 {
            if features.green_luminance <= 158 {
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
            if features.saturation <= 67 {
            if features.intensity <= 141 {
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
            Intensity::High
            }
            }
            }
            } else {
            if features.blue_difference <= 109 {
            if features.red_difference <= 121 {
            if features.blue_luminance <= 119 {
            if features.blue_difference <= 108 {
            if features.blue_chromaticity <= 0.265 {
            if features.intensity <= 110 {
            if features.red_difference <= 120 {
            if features.blue_chromaticity <= 0.261 {
            Intensity::High
            } else {
            if features.luminance <= 121 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.402 {
            if features.red_chromaticity <= 0.334 {
            Intensity::High
            } else {
            if features.luminance <= 133 {
            Intensity::Low
            } else {
            if features.saturation <= 89 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.hue <= 46 {
            if features.red_chromaticity <= 0.340 {
            if features.green_chromaticity <= 0.409 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.405 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_chromaticity <= 0.408 {
            if features.blue_difference <= 104 {
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
            if features.green_chromaticity <= 0.404 {
            if features.green_chromaticity <= 0.401 {
            if features.hue <= 45 {
            Intensity::Low
            } else {
            if features.hue <= 49 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_luminance <= 129 {
            if features.red_luminance <= 127 {
            if features.green_chromaticity <= 0.402 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_luminance <= 106 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_luminance <= 164 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.403 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.blue_difference <= 107 {
            if features.blue_chromaticity <= 0.273 {
            if features.green_chromaticity <= 0.404 {
            Intensity::High
            } else {
            if features.value <= 149 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_chromaticity <= 0.317 {
            if features.red_chromaticity <= 0.315 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.321 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.red_chromaticity <= 0.319 {
            if features.red_chromaticity <= 0.313 {
            if features.saturation <= 80 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.407 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_chromaticity <= 0.324 {
            if features.red_luminance <= 115 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.saturation <= 86 {
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
            if features.red_chromaticity <= 0.316 {
            if features.blue_chromaticity <= 0.278 {
            if features.luminance <= 142 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.403 {
            Intensity::High
            } else {
            if features.blue_luminance <= 117 {
            if features.blue_chromaticity <= 0.281 {
            if features.blue_chromaticity <= 0.281 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.408 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.406 {
            if features.hue <= 52 {
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
            if features.blue_chromaticity <= 0.280 {
            if features.saturation <= 93 {
            if features.intensity <= 110 {
            if features.blue_chromaticity <= 0.259 {
            if features.luminance <= 114 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.328 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.value <= 144 {
            if features.red_chromaticity <= 0.324 {
            Intensity::High
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
            Intensity::High
            }
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.blue_luminance <= 123 {
            if features.green_chromaticity <= 0.404 {
            if features.red_luminance <= 146 {
            if features.red_chromaticity <= 0.314 {
            if features.value <= 172 {
            if features.red_luminance <= 133 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.316 {
            if features.intensity <= 142 {
            if features.green_chromaticity <= 0.402 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.green_luminance <= 173 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_chromaticity <= 0.279 {
            if features.intensity <= 145 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.280 {
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
            } else {
            if features.green_chromaticity <= 0.405 {
            if features.green_chromaticity <= 0.405 {
            if features.saturation <= 83 {
            if features.green_luminance <= 174 {
            Intensity::High
            } else {
            if features.hue <= 51 {
            Intensity::Low
            } else {
            Intensity::Low
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
            } else {
            if features.value <= 184 {
            if features.blue_chromaticity <= 0.284 {
            if features.red_luminance <= 142 {
            if features.blue_chromaticity <= 0.274 {
            Intensity::High
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
            if features.blue_chromaticity <= 0.284 {
            if features.intensity <= 142 {
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
            Intensity::High
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.401 {
            if features.hue <= 52 {
            if features.blue_chromaticity <= 0.283 {
            if features.red_chromaticity <= 0.318 {
            Intensity::Low
            } else {
            Intensity::Low
            }
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
            } else {
            if features.green_chromaticity <= 0.406 {
            if features.red_luminance <= 155 {
            if features.red_chromaticity <= 0.326 {
            if features.green_chromaticity <= 0.401 {
            if features.blue_luminance <= 124 {
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
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            } else {
            if features.red_luminance <= 147 {
            if features.red_chromaticity <= 0.304 {
            if features.red_luminance <= 138 {
            if features.red_chromaticity <= 0.304 {
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
            if features.blue_chromaticity <= 0.287 {
            if features.luminance <= 169 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.intensity <= 149 {
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
            if features.hue <= 39 {
            if features.blue_chromaticity <= 0.234 {
            Intensity::Low
            } else {
            if features.blue_chromaticity <= 0.236 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_luminance <= 83 {
            if features.red_chromaticity <= 0.350 {
            if features.green_chromaticity <= 0.405 {
            if features.blue_difference <= 108 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.403 {
            Intensity::Low
            } else {
            if features.blue_luminance <= 70 {
            if features.saturation <= 100 {
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
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.346 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.287 {
            if features.intensity <= 99 {
            if features.red_chromaticity <= 0.318 {
            Intensity::High
            } else {
            if features.blue_luminance <= 72 {
            if features.red_chromaticity <= 0.358 {
            if features.blue_chromaticity <= 0.237 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.404 {
            if features.green_chromaticity <= 0.404 {
            if features.red_luminance <= 76 {
            Intensity::Low
            } else {
            Intensity::Low
            }
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
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_luminance <= 76 {
            if features.red_difference <= 120 {
            Intensity::High
            } else {
            if features.value <= 118 {
            if features.value <= 113 {
            Intensity::Low
            } else {
            if features.saturation <= 89 {
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
            if features.red_chromaticity <= 0.321 {
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
            } else {
            if features.blue_difference <= 110 {
            if features.red_chromaticity <= 0.312 {
            if features.green_chromaticity <= 0.404 {
            if features.green_chromaticity <= 0.404 {
            if features.green_chromaticity <= 0.403 {
            if features.red_chromaticity <= 0.312 {
            if features.value <= 170 {
            Intensity::Low
            } else {
            Intensity::Low
            }
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
            if features.luminance <= 151 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_luminance <= 169 {
            if features.red_luminance <= 120 {
            if features.green_chromaticity <= 0.408 {
            if features.luminance <= 139 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.287 {
            if features.saturation <= 76 {
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
            if features.red_luminance <= 117 {
            if features.red_difference <= 120 {
            if features.red_chromaticity <= 0.325 {
            if features.green_chromaticity <= 0.406 {
            if features.intensity <= 111 {
            Intensity::High
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
            } else {
            if features.red_chromaticity <= 0.331 {
            if features.red_chromaticity <= 0.331 {
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
            if features.green_luminance <= 154 {
            if features.green_chromaticity <= 0.403 {
            if features.green_chromaticity <= 0.401 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.318 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.luminance <= 135 {
            Intensity::Low
            } else {
            if features.blue_chromaticity <= 0.279 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.402 {
            if features.red_luminance <= 123 {
            if features.intensity <= 128 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.intensity <= 130 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.intensity <= 134 {
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
            }
            } else {
            if features.saturation <= 78 {
            if features.red_chromaticity <= 0.319 {
            if features.intensity <= 107 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.281 {
            if features.luminance <= 120 {
            Intensity::Low
            } else {
            if features.blue_chromaticity <= 0.280 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.406 {
            if features.red_chromaticity <= 0.313 {
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
            if features.blue_chromaticity <= 0.278 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.409 {
            if features.red_difference <= 118 {
            if features.saturation <= 80 {
            if features.value <= 133 {
            if features.blue_chromaticity <= 0.277 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.intensity <= 111 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.saturation <= 84 {
            if features.saturation <= 81 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.value <= 128 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.value <= 121 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.322 {
            if features.green_chromaticity <= 0.404 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_luminance <= 86 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.red_chromaticity <= 0.316 {
            if features.red_difference <= 115 {
            if features.red_luminance <= 104 {
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
            if features.blue_chromaticity <= 0.290 {
            if features.luminance <= 145 {
            if features.blue_chromaticity <= 0.288 {
            if features.red_chromaticity <= 0.307 {
            if features.value <= 152 {
            if features.green_luminance <= 148 {
            if features.red_chromaticity <= 0.305 {
            if features.saturation <= 75 {
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
            if features.red_luminance <= 113 {
            if features.blue_chromaticity <= 0.287 {
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
            if features.red_luminance <= 114 {
            Intensity::Low
            } else {
            if features.blue_difference <= 111 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.saturation <= 73 {
            if features.red_chromaticity <= 0.309 {
            if features.luminance <= 133 {
            Intensity::Low
            } else {
            if features.green_luminance <= 150 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_luminance <= 112 {
            if features.luminance <= 133 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.luminance <= 144 {
            if features.blue_chromaticity <= 0.287 {
            if features.blue_difference <= 111 {
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
            if features.hue <= 54 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.406 {
            if features.red_luminance <= 113 {
            if features.red_difference <= 113 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.404 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_chromaticity <= 0.308 {
            if features.red_chromaticity <= 0.308 {
            if features.blue_chromaticity <= 0.289 {
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
            } else {
            if features.blue_chromaticity <= 0.289 {
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
            } else {
            if features.green_chromaticity <= 0.406 {
            if features.blue_luminance <= 109 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.408 {
            if features.blue_chromaticity <= 0.289 {
            if features.red_difference <= 112 {
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
            if features.saturation <= 74 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.404 {
            if features.blue_chromaticity <= 0.290 {
            if features.value <= 167 {
            if features.red_difference <= 113 {
            if features.green_chromaticity <= 0.403 {
            if features.red_chromaticity <= 0.310 {
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
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.402 {
            if features.red_luminance <= 136 {
            if features.red_chromaticity <= 0.309 {
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
            if features.intensity <= 144 {
            if features.red_difference <= 111 {
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
            }
            } else {
            if features.luminance <= 152 {
            Intensity::Low
            } else {
            if features.hue <= 55 {
            if features.blue_chromaticity <= 0.290 {
            Intensity::Low
            } else {
            if features.luminance <= 157 {
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
            if features.green_chromaticity <= 0.406 {
            if features.green_chromaticity <= 0.406 {
            if features.blue_chromaticity <= 0.289 {
            if features.value <= 171 {
            if features.blue_chromaticity <= 0.288 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.intensity <= 142 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.406 {
            if features.blue_chromaticity <= 0.290 {
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
            if features.blue_chromaticity <= 0.288 {
            if features.blue_luminance <= 121 {
            if features.luminance <= 152 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_luminance <= 124 {
            if features.green_chromaticity <= 0.408 {
            if features.blue_chromaticity <= 0.289 {
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
            if features.blue_luminance <= 126 {
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
            if features.green_chromaticity <= 0.407 {
            if features.blue_chromaticity <= 0.296 {
            if features.green_chromaticity <= 0.402 {
            if features.saturation <= 70 {
            if features.green_chromaticity <= 0.401 {
            Intensity::Low
            } else {
            if features.value <= 173 {
            if features.saturation <= 68 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_luminance <= 133 {
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
            if features.blue_luminance <= 124 {
            if features.blue_chromaticity <= 0.293 {
            if features.blue_chromaticity <= 0.293 {
            if features.red_chromaticity <= 0.303 {
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
            if features.red_chromaticity <= 0.302 {
            if features.blue_chromaticity <= 0.295 {
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
            }
            } else {
            if features.red_chromaticity <= 0.301 {
            if features.green_chromaticity <= 0.405 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.300 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_chromaticity <= 0.295 {
            if features.red_chromaticity <= 0.304 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.value <= 183 {
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
            if features.blue_chromaticity <= 0.296 {
            if features.green_chromaticity <= 0.406 {
            Intensity::Low
            } else {
            if features.red_difference <= 108 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.luminance <= 152 {
            if features.value <= 170 {
            if features.hue <= 59 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.402 {
            if features.blue_luminance <= 142 {
            Intensity::Low
            } else {
            Intensity::High
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
            if features.red_chromaticity <= 0.300 {
            if features.green_chromaticity <= 0.404 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.404 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_luminance <= 183 {
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
            if features.value <= 192 {
            if features.red_chromaticity <= 0.302 {
            if features.blue_chromaticity <= 0.291 {
            if features.blue_chromaticity <= 0.290 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.299 {
            if features.green_chromaticity <= 0.407 {
            if features.hue <= 60 {
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
            } else {
            if features.blue_chromaticity <= 0.293 {
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
            if features.blue_difference <= 109 {
            if features.red_difference <= 120 {
            if features.blue_difference <= 108 {
            if features.intensity <= 140 {
            if features.blue_difference <= 106 {
            if features.blue_difference <= 103 {
            if features.luminance <= 150 {
            if features.green_luminance <= 151 {
            if features.blue_chromaticity <= 0.251 {
            if features.red_chromaticity <= 0.331 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.intensity <= 119 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_luminance <= 152 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_chromaticity <= 0.421 {
            if features.hue <= 46 {
            if features.blue_chromaticity <= 0.256 {
            if features.blue_chromaticity <= 0.249 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.luminance <= 151 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_difference <= 112 {
            if features.red_luminance <= 132 {
            Intensity::High
            } else {
            Intensity::Low
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
            Intensity::Low
            }
            }
            } else {
            if features.red_luminance <= 134 {
            if features.red_chromaticity <= 0.310 {
            if features.red_luminance <= 128 {
            if features.value <= 165 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.309 {
            Intensity::Low
            } else {
            if features.green_luminance <= 175 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.420 {
            if features.red_chromaticity <= 0.311 {
            if features.luminance <= 152 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.luminance <= 122 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_chromaticity <= 0.316 {
            if features.red_chromaticity <= 0.311 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.318 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.410 {
            Intensity::High
            } else {
            if features.saturation <= 90 {
            if features.green_luminance <= 172 {
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
            if features.green_chromaticity <= 0.413 {
            if features.value <= 148 {
            if features.green_chromaticity <= 0.411 {
            if features.red_luminance <= 105 {
            if features.luminance <= 117 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.red_luminance <= 116 {
            if features.red_chromaticity <= 0.324 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.blue_luminance <= 95 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.411 {
            if features.blue_difference <= 107 {
            Intensity::High
            } else {
            if features.intensity <= 107 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.412 {
            if features.blue_chromaticity <= 0.267 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.412 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.luminance <= 143 {
            if features.red_chromaticity <= 0.321 {
            if features.red_chromaticity <= 0.320 {
            if features.red_luminance <= 122 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.320 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.410 {
            if features.red_chromaticity <= 0.322 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.saturation <= 89 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.red_luminance <= 122 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.279 {
            if features.blue_chromaticity <= 0.277 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.green_luminance <= 165 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            }
            } else {
            if features.value <= 164 {
            if features.green_chromaticity <= 0.419 {
            if features.blue_luminance <= 84 {
            if features.saturation <= 96 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.336 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_luminance <= 90 {
            if features.saturation <= 92 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_luminance <= 109 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.green_luminance <= 127 {
            if features.blue_chromaticity <= 0.250 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.328 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_luminance <= 101 {
            Intensity::High
            } else {
            if features.intensity <= 112 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.280 {
            if features.hue <= 53 {
            if features.blue_chromaticity <= 0.275 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.276 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_difference <= 108 {
            if features.green_chromaticity <= 0.420 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.417 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.281 {
            if features.blue_luminance <= 112 {
            if features.luminance <= 146 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.saturation <= 82 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_chromaticity <= 0.282 {
            if features.luminance <= 149 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.416 {
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
            if features.blue_chromaticity <= 0.267 {
            if features.red_chromaticity <= 0.320 {
            if features.green_chromaticity <= 0.416 {
            if features.blue_chromaticity <= 0.267 {
            if features.value <= 180 {
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
            if features.blue_difference <= 97 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_chromaticity <= 0.319 {
            if features.green_luminance <= 191 {
            if features.green_chromaticity <= 0.411 {
            if features.red_chromaticity <= 0.304 {
            Intensity::High
            } else {
            if features.red_luminance <= 141 {
            if features.green_chromaticity <= 0.409 {
            Intensity::Low
            } else {
            Intensity::Low
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
            if features.red_difference <= 108 {
            if features.red_chromaticity <= 0.305 {
            if features.saturation <= 87 {
            Intensity::Low
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
            } else {
            if features.red_luminance <= 138 {
            if features.blue_chromaticity <= 0.275 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.413 {
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
            if features.red_luminance <= 144 {
            if features.red_chromaticity <= 0.322 {
            if features.intensity <= 145 {
            if features.blue_difference <= 103 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.269 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_chromaticity <= 0.321 {
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
            Intensity::High
            }
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.285 {
            if features.green_chromaticity <= 0.416 {
            if features.green_chromaticity <= 0.409 {
            if features.red_chromaticity <= 0.321 {
            if features.value <= 170 {
            if features.red_chromaticity <= 0.315 {
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
            if features.blue_chromaticity <= 0.277 {
            if features.blue_chromaticity <= 0.274 {
            if features.value <= 145 {
            if features.red_chromaticity <= 0.316 {
            if features.saturation <= 89 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.325 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.luminance <= 130 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.415 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.hue <= 52 {
            if features.green_chromaticity <= 0.412 {
            if features.red_luminance <= 118 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.saturation <= 85 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.415 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.red_chromaticity <= 0.309 {
            if features.red_difference <= 110 {
            if features.red_chromaticity <= 0.305 {
            if features.intensity <= 134 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.411 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.luminance <= 141 {
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
            if features.green_luminance <= 160 {
            if features.value <= 159 {
            if features.green_luminance <= 158 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.luminance <= 142 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.410 {
            Intensity::Low
            } else {
            if features.blue_chromaticity <= 0.280 {
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
            if features.red_chromaticity <= 0.299 {
            if features.red_chromaticity <= 0.297 {
            if features.red_chromaticity <= 0.297 {
            if features.blue_chromaticity <= 0.283 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.284 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.saturation <= 82 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_chromaticity <= 0.300 {
            if features.luminance <= 144 {
            if features.blue_luminance <= 109 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.green_luminance <= 173 {
            if features.green_chromaticity <= 0.418 {
            if features.red_chromaticity <= 0.299 {
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
            if features.blue_chromaticity <= 0.256 {
            if features.saturation <= 100 {
            if features.red_chromaticity <= 0.326 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.303 {
            if features.red_chromaticity <= 0.303 {
            if features.red_chromaticity <= 0.300 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.value <= 160 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_chromaticity <= 0.304 {
            if features.green_chromaticity <= 0.418 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.307 {
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
            if features.saturation <= 79 {
            if features.red_chromaticity <= 0.295 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.302 {
            if features.green_chromaticity <= 0.413 {
            if features.blue_chromaticity <= 0.288 {
            if features.green_chromaticity <= 0.412 {
            if features.green_chromaticity <= 0.412 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.red_luminance <= 128 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.298 {
            if features.intensity <= 144 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.value <= 180 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.287 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.410 {
            if features.blue_luminance <= 127 {
            if features.value <= 180 {
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
            if features.red_luminance <= 126 {
            if features.value <= 175 {
            Intensity::Low
            } else {
            if features.red_luminance <= 125 {
            Intensity::High
            } else {
            if features.hue <= 57 {
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
            if features.hue <= 39 {
            if features.blue_chromaticity <= 0.217 {
            if features.intensity <= 44 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.hue <= 36 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.luminance <= 106 {
            if features.green_chromaticity <= 0.420 {
            if features.blue_difference <= 107 {
            if features.red_luminance <= 92 {
            Intensity::High
            } else {
            if features.saturation <= 106 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.419 {
            if features.value <= 117 {
            if features.red_difference <= 121 {
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
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            } else {
            if features.intensity <= 99 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.350 {
            if features.green_chromaticity <= 0.410 {
            Intensity::High
            } else {
            if features.red_luminance <= 104 {
            if features.green_chromaticity <= 0.414 {
            Intensity::Low
            } else {
            if features.blue_luminance <= 73 {
            if features.blue_chromaticity <= 0.239 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.intensity <= 115 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.229 {
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
            if features.saturation <= 79 {
            if features.blue_chromaticity <= 0.289 {
            if features.luminance <= 145 {
            if features.saturation <= 78 {
            if features.green_chromaticity <= 0.416 {
            if features.blue_chromaticity <= 0.287 {
            if features.red_chromaticity <= 0.303 {
            if features.red_chromaticity <= 0.302 {
            if features.red_difference <= 111 {
            if features.red_chromaticity <= 0.300 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.green_luminance <= 144 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.411 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_chromaticity <= 0.284 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.306 {
            if features.blue_chromaticity <= 0.287 {
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
            }
            }
            } else {
            if features.value <= 145 {
            if features.value <= 144 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.297 {
            if features.blue_chromaticity <= 0.289 {
            if features.intensity <= 120 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.289 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.410 {
            if features.luminance <= 143 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.412 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.416 {
            Intensity::Low
            } else {
            if features.value <= 155 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.luminance <= 122 {
            if features.red_chromaticity <= 0.305 {
            if features.red_difference <= 113 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            } else {
            if features.red_difference <= 109 {
            if features.blue_chromaticity <= 0.288 {
            Intensity::High
            } else {
            if features.luminance <= 142 {
            if features.green_chromaticity <= 0.417 {
            Intensity::Low
            } else {
            if features.green_luminance <= 151 {
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
            if features.blue_chromaticity <= 0.288 {
            if features.red_difference <= 111 {
            if features.green_chromaticity <= 0.411 {
            Intensity::Low
            } else {
            if features.blue_luminance <= 99 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.411 {
            if features.red_chromaticity <= 0.307 {
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
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.red_chromaticity <= 0.301 {
            if features.blue_chromaticity <= 0.289 {
            if features.red_chromaticity <= 0.297 {
            if features.luminance <= 151 {
            if features.value <= 171 {
            if features.red_difference <= 107 {
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
            if features.green_chromaticity <= 0.413 {
            if features.blue_chromaticity <= 0.288 {
            if features.value <= 171 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.red_luminance <= 127 {
            if features.blue_difference <= 110 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_chromaticity <= 0.287 {
            if features.red_luminance <= 121 {
            if features.blue_luminance <= 115 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_luminance <= 117 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_luminance <= 170 {
            if features.green_luminance <= 169 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.saturation <= 77 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            }
            } else {
            if features.green_luminance <= 174 {
            if features.blue_luminance <= 121 {
            if features.hue <= 58 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.luminance <= 155 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.red_chromaticity <= 0.304 {
            if features.red_luminance <= 124 {
            if features.green_chromaticity <= 0.412 {
            if features.green_chromaticity <= 0.412 {
            if features.green_chromaticity <= 0.411 {
            if features.blue_luminance <= 117 {
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
            Intensity::High
            }
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.305 {
            if features.green_chromaticity <= 0.409 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.305 {
            if features.intensity <= 135 {
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
            }
            } else {
            if features.blue_difference <= 112 {
            if features.blue_chromaticity <= 0.293 {
            if features.luminance <= 154 {
            if features.green_chromaticity <= 0.413 {
            if features.intensity <= 135 {
            if features.blue_chromaticity <= 0.291 {
            if features.blue_chromaticity <= 0.290 {
            if features.blue_chromaticity <= 0.290 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.luminance <= 147 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.value <= 163 {
            if features.green_luminance <= 161 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.412 {
            if features.green_chromaticity <= 0.411 {
            if features.blue_luminance <= 119 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            } else {
            if features.value <= 170 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.418 {
            if features.red_chromaticity <= 0.290 {
            Intensity::High
            } else {
            if features.intensity <= 133 {
            if features.red_chromaticity <= 0.291 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.296 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.saturation <= 78 {
            if features.blue_chromaticity <= 0.290 {
            if features.red_luminance <= 112 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.intensity <= 134 {
            if features.red_chromaticity <= 0.290 {
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
            if features.saturation <= 73 {
            Intensity::High
            } else {
            if features.saturation <= 76 {
            if features.green_chromaticity <= 0.411 {
            if features.blue_chromaticity <= 0.290 {
            if features.blue_chromaticity <= 0.290 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_luminance <= 131 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.414 {
            if features.value <= 179 {
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
            if features.red_chromaticity <= 0.295 {
            if features.green_chromaticity <= 0.409 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.292 {
            if features.blue_chromaticity <= 0.294 {
            if features.red_chromaticity <= 0.291 {
            Intensity::Low
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
            } else {
            if features.green_chromaticity <= 0.413 {
            if features.green_luminance <= 184 {
            if features.blue_chromaticity <= 0.296 {
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
            Intensity::Low
            }
            }
            } else {
            if features.saturation <= 73 {
            if features.red_luminance <= 114 {
            if features.luminance <= 140 {
            if features.blue_chromaticity <= 0.292 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.intensity <= 140 {
            if features.red_chromaticity <= 0.293 {
            if features.luminance <= 148 {
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
            if features.blue_chromaticity <= 0.297 {
            if features.luminance <= 146 {
            if features.red_chromaticity <= 0.291 {
            if features.red_chromaticity <= 0.291 {
            if features.green_chromaticity <= 0.416 {
            Intensity::Low
            } else {
            if features.luminance <= 139 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_luminance <= 114 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.415 {
            if features.green_chromaticity <= 0.414 {
            if features.luminance <= 131 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_difference <= 109 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.hue <= 60 {
            if features.value <= 154 {
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
            }
            }
            }
            } else {
            if features.green_luminance <= 119 {
            if features.green_luminance <= 107 {
            if features.blue_chromaticity <= 0.264 {
            if features.blue_luminance <= 67 {
            if features.green_chromaticity <= 0.418 {
            if features.red_luminance <= 71 {
            Intensity::Low
            } else {
            if features.luminance <= 79 {
            if features.red_chromaticity <= 0.340 {
            Intensity::High
            } else {
            if features.blue_luminance <= 48 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_chromaticity <= 0.326 {
            if features.red_luminance <= 79 {
            Intensity::High
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
            if features.green_chromaticity <= 0.418 {
            Intensity::High
            } else {
            if features.red_difference <= 123 {
            if features.blue_chromaticity <= 0.241 {
            if features.intensity <= 69 {
            Intensity::High
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
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.257 {
            Intensity::High
            } else {
            if features.blue_difference <= 110 {
            if features.green_chromaticity <= 0.420 {
            if features.green_chromaticity <= 0.414 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.421 {
            if features.green_chromaticity <= 0.412 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.324 {
            if features.green_chromaticity <= 0.412 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.317 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_luminance <= 73 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.saturation <= 92 {
            Intensity::Low
            } else {
            if features.red_luminance <= 86 {
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
            if features.saturation <= 84 {
            if features.red_chromaticity <= 0.288 {
            if features.red_difference <= 102 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.421 {
            if features.blue_difference <= 112 {
            if features.red_difference <= 105 {
            if features.green_chromaticity <= 0.417 {
            if features.red_chromaticity <= 0.286 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.red_luminance <= 112 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_luminance <= 108 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_luminance <= 103 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.286 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.414 {
            if features.green_luminance <= 150 {
            if features.green_luminance <= 127 {
            if features.green_chromaticity <= 0.409 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.311 {
            Intensity::Low
            } else {
            if features.red_luminance <= 95 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.red_difference <= 114 {
            if features.green_chromaticity <= 0.412 {
            if features.red_chromaticity <= 0.309 {
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
            if features.green_chromaticity <= 0.413 {
            if features.red_luminance <= 103 {
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
            if features.luminance <= 141 {
            if features.red_chromaticity <= 0.310 {
            if features.green_chromaticity <= 0.413 {
            if features.green_luminance <= 158 {
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
            } else {
            if features.green_chromaticity <= 0.409 {
            Intensity::Low
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
            } else {
            if features.blue_chromaticity <= 0.283 {
            if features.blue_luminance <= 97 {
            if features.blue_chromaticity <= 0.279 {
            if features.red_chromaticity <= 0.308 {
            if features.blue_luminance <= 87 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            } else {
            if features.intensity <= 103 {
            Intensity::High
            } else {
            if features.green_luminance <= 133 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.blue_luminance <= 103 {
            if features.red_chromaticity <= 0.306 {
            if features.red_chromaticity <= 0.299 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.value <= 148 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_chromaticity <= 0.301 {
            if features.red_luminance <= 112 {
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
            }
            }
            } else {
            if features.red_difference <= 109 {
            if features.red_chromaticity <= 0.293 {
            if features.red_chromaticity <= 0.289 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.420 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_chromaticity <= 0.287 {
            if features.blue_luminance <= 115 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_luminance <= 102 {
            if features.green_luminance <= 144 {
            if features.value <= 136 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            } else {
            if features.blue_luminance <= 111 {
            if features.blue_chromaticity <= 0.286 {
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
            if features.green_chromaticity <= 0.415 {
            if features.blue_chromaticity <= 0.266 {
            if features.blue_chromaticity <= 0.263 {
            if features.red_chromaticity <= 0.328 {
            if features.red_luminance <= 95 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.red_luminance <= 95 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_chromaticity <= 0.310 {
            if features.green_chromaticity <= 0.415 {
            if features.green_luminance <= 133 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_luminance <= 93 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_luminance <= 79 {
            if features.red_luminance <= 93 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_difference <= 110 {
            if features.red_chromaticity <= 0.315 {
            if features.green_chromaticity <= 0.413 {
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
            if features.saturation <= 87 {
            if features.luminance <= 115 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.268 {
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
            if features.blue_luminance <= 93 {
            if features.green_chromaticity <= 0.421 {
            if features.value <= 123 {
            if features.saturation <= 95 {
            if features.green_chromaticity <= 0.417 {
            if features.red_chromaticity <= 0.317 {
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
            Intensity::High
            }
            } else {
            if features.red_luminance <= 104 {
            if features.intensity <= 104 {
            if features.blue_chromaticity <= 0.278 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.268 {
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
            if features.green_chromaticity <= 0.421 {
            Intensity::High
            } else {
            if features.red_luminance <= 93 {
            Intensity::High
            } else {
            if features.red_difference <= 114 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.276 {
            if features.red_chromaticity <= 0.307 {
            if features.green_chromaticity <= 0.419 {
            Intensity::Low
            } else {
            if features.luminance <= 126 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_luminance <= 94 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.intensity <= 119 {
            if features.green_chromaticity <= 0.417 {
            if features.green_chromaticity <= 0.416 {
            Intensity::Low
            } else {
            if features.value <= 145 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_difference <= 112 {
            if features.red_luminance <= 107 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_chromaticity <= 0.289 {
            if features.red_chromaticity <= 0.299 {
            Intensity::Low
            } else {
            if features.blue_luminance <= 101 {
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
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.436 {
            if features.blue_difference <= 108 {
            if features.red_difference <= 121 {
            if features.blue_difference <= 106 {
            if features.luminance <= 154 {
            if features.green_chromaticity <= 0.429 {
            if features.red_luminance <= 113 {
            if features.blue_difference <= 104 {
            if features.luminance <= 110 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.429 {
            if features.luminance <= 122 {
            if features.red_luminance <= 108 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.318 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_difference <= 102 {
            Intensity::High
            } else {
            if features.red_luminance <= 101 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.427 {
            if features.red_chromaticity <= 0.334 {
            if features.red_chromaticity <= 0.313 {
            if features.intensity <= 120 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.value <= 152 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_luminance <= 121 {
            Intensity::High
            } else {
            if features.red_luminance <= 99 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.427 {
            if features.green_luminance <= 148 {
            if features.blue_chromaticity <= 0.253 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_difference <= 110 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_chromaticity <= 0.428 {
            if features.red_luminance <= 101 {
            Intensity::Low
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
            }
            }
            }
            } else {
            if features.blue_luminance <= 100 {
            if features.blue_difference <= 104 {
            if features.green_chromaticity <= 0.421 {
            if features.hue <= 46 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.424 {
            if features.luminance <= 136 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.429 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.saturation <= 97 {
            if features.saturation <= 94 {
            if features.green_chromaticity <= 0.422 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_luminance <= 118 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_luminance <= 115 {
            if features.red_chromaticity <= 0.308 {
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
            if features.red_chromaticity <= 0.310 {
            if features.blue_difference <= 105 {
            if features.blue_chromaticity <= 0.272 {
            if features.green_luminance <= 162 {
            Intensity::High
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
            } else {
            if features.green_luminance <= 173 {
            if features.red_chromaticity <= 0.308 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_luminance <= 125 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.424 {
            if features.red_luminance <= 120 {
            if features.value <= 162 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_luminance <= 128 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_chromaticity <= 0.426 {
            if features.blue_difference <= 103 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.261 {
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
            if features.red_chromaticity <= 0.297 {
            if features.green_chromaticity <= 0.435 {
            if features.green_chromaticity <= 0.435 {
            if features.red_luminance <= 114 {
            if features.red_chromaticity <= 0.294 {
            Intensity::High
            } else {
            if features.blue_luminance <= 105 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_chromaticity <= 0.271 {
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
            if features.red_luminance <= 105 {
            if features.red_chromaticity <= 0.341 {
            if features.saturation <= 106 {
            if features.green_chromaticity <= 0.436 {
            if features.red_difference <= 112 {
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
            if features.green_chromaticity <= 0.436 {
            if features.green_chromaticity <= 0.435 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_chromaticity <= 0.342 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.343 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.red_luminance <= 118 {
            if features.blue_difference <= 104 {
            if features.green_chromaticity <= 0.436 {
            if features.red_chromaticity <= 0.310 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_luminance <= 116 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.luminance <= 135 {
            if features.saturation <= 102 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.431 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.251 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.311 {
            if features.blue_chromaticity <= 0.265 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.saturation <= 104 {
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
            if features.red_chromaticity <= 0.305 {
            if features.blue_chromaticity <= 0.279 {
            if features.red_difference <= 103 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.272 {
            if features.blue_chromaticity <= 0.270 {
            if features.green_chromaticity <= 0.432 {
            if features.green_chromaticity <= 0.427 {
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
            } else {
            if features.luminance <= 156 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_luminance <= 178 {
            Intensity::High
            } else {
            if features.luminance <= 165 {
            if features.red_luminance <= 128 {
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
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.309 {
            Intensity::High
            } else {
            if features.red_luminance <= 136 {
            if features.red_chromaticity <= 0.309 {
            if features.red_luminance <= 130 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.428 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.428 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.value <= 179 {
            if features.red_luminance <= 133 {
            if features.red_chromaticity <= 0.314 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.422 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_chromaticity <= 0.257 {
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
            if features.saturation <= 92 {
            if features.intensity <= 122 {
            if features.red_chromaticity <= 0.308 {
            if features.green_chromaticity <= 0.431 {
            if features.blue_chromaticity <= 0.276 {
            if features.green_chromaticity <= 0.427 {
            if features.green_chromaticity <= 0.424 {
            if features.red_luminance <= 109 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.301 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.intensity <= 119 {
            if features.red_chromaticity <= 0.297 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.430 {
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
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.278 {
            if features.green_luminance <= 159 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_luminance <= 147 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.421 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.blue_luminance <= 116 {
            if features.red_chromaticity <= 0.298 {
            if features.red_luminance <= 112 {
            if features.saturation <= 90 {
            Intensity::High
            } else {
            if features.luminance <= 139 {
            if features.luminance <= 137 {
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
            }
            } else {
            if features.blue_chromaticity <= 0.278 {
            if features.blue_chromaticity <= 0.278 {
            if features.red_chromaticity <= 0.290 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.278 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_chromaticity <= 0.294 {
            if features.red_difference <= 105 {
            Intensity::High
            } else {
            Intensity::High
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
            if features.green_luminance <= 157 {
            if features.red_difference <= 109 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.306 {
            if features.red_chromaticity <= 0.304 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_chromaticity <= 0.421 {
            if features.green_chromaticity <= 0.421 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.276 {
            if features.blue_luminance <= 107 {
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
            if features.red_chromaticity <= 0.293 {
            if features.red_difference <= 105 {
            Intensity::Low
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
            } else {
            if features.blue_luminance <= 68 {
            if features.red_chromaticity <= 0.336 {
            if features.green_chromaticity <= 0.436 {
            if features.red_chromaticity <= 0.321 {
            if features.red_chromaticity <= 0.316 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.425 {
            if features.green_chromaticity <= 0.424 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_luminance <= 60 {
            if features.hue <= 44 {
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
            Intensity::High
            }
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.430 {
            if features.luminance <= 108 {
            if features.blue_luminance <= 71 {
            if features.red_chromaticity <= 0.321 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.intensity <= 95 {
            if features.saturation <= 104 {
            if features.red_luminance <= 92 {
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
            if features.value <= 151 {
            if features.red_luminance <= 106 {
            if features.blue_luminance <= 86 {
            if features.value <= 133 {
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
            if features.green_chromaticity <= 0.421 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.312 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.saturation <= 93 {
            if features.red_luminance <= 111 {
            if features.red_difference <= 109 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.309 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_chromaticity <= 0.303 {
            if features.green_chromaticity <= 0.429 {
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
            if features.blue_luminance <= 92 {
            if features.green_chromaticity <= 0.436 {
            if features.green_chromaticity <= 0.435 {
            if features.value <= 132 {
            if features.value <= 127 {
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
            if features.luminance <= 127 {
            if features.red_luminance <= 99 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_luminance <= 92 {
            Intensity::High
            } else {
            if features.value <= 136 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.268 {
            Intensity::High
            } else {
            if features.intensity <= 123 {
            if features.red_difference <= 107 {
            if features.blue_chromaticity <= 0.271 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.298 {
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
            } else {
            if features.red_difference <= 124 {
            if features.blue_luminance <= 62 {
            if features.blue_chromaticity <= 0.211 {
            Intensity::High
            } else {
            if features.red_luminance <= 87 {
            if features.value <= 92 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.213 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.355 {
            if features.red_chromaticity <= 0.345 {
            if features.intensity <= 80 {
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
            } else {
            if features.blue_luminance <= 59 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.222 {
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
            if features.hue <= 37 {
            Intensity::Low
            } else {
            if features.blue_difference <= 104 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.red_difference <= 117 {
            if features.green_chromaticity <= 0.427 {
            if features.blue_chromaticity <= 0.281 {
            if features.blue_chromaticity <= 0.271 {
            if features.green_luminance <= 121 {
            if features.blue_luminance <= 73 {
            if features.green_luminance <= 110 {
            if features.red_chromaticity <= 0.313 {
            if features.intensity <= 81 {
            Intensity::High
            } else {
            if features.luminance <= 95 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.value <= 107 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.hue <= 48 {
            if features.luminance <= 104 {
            Intensity::Low
            } else {
            if features.red_luminance <= 91 {
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
            if features.intensity <= 94 {
            if features.intensity <= 91 {
            Intensity::Low
            } else {
            if features.green_luminance <= 118 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.307 {
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
            if features.blue_chromaticity <= 0.270 {
            if features.green_chromaticity <= 0.425 {
            if features.blue_chromaticity <= 0.269 {
            if features.red_chromaticity <= 0.317 {
            if features.green_chromaticity <= 0.422 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.green_luminance <= 122 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_chromaticity <= 0.270 {
            if features.blue_chromaticity <= 0.269 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.308 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.saturation <= 96 {
            if features.red_luminance <= 92 {
            if features.green_luminance <= 127 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.304 {
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
            if features.green_chromaticity <= 0.424 {
            if features.red_luminance <= 101 {
            if features.luminance <= 118 {
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
            if features.green_chromaticity <= 0.423 {
            if features.red_chromaticity <= 0.298 {
            if features.red_difference <= 110 {
            if features.blue_chromaticity <= 0.279 {
            if features.green_chromaticity <= 0.423 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            } else {
            if features.blue_luminance <= 94 {
            if features.luminance <= 115 {
            if features.green_chromaticity <= 0.423 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.422 {
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
            if features.red_chromaticity <= 0.306 {
            if features.green_chromaticity <= 0.423 {
            if features.green_chromaticity <= 0.423 {
            if features.blue_chromaticity <= 0.275 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.276 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_difference <= 109 {
            Intensity::Low
            } else {
            if features.value <= 134 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.421 {
            if features.red_difference <= 112 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.luminance <= 105 {
            if features.blue_chromaticity <= 0.271 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.green_luminance <= 145 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.424 {
            if features.red_chromaticity <= 0.301 {
            if features.red_chromaticity <= 0.297 {
            if features.blue_chromaticity <= 0.280 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_difference <= 113 {
            if features.red_luminance <= 90 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_chromaticity <= 0.274 {
            if features.green_chromaticity <= 0.424 {
            if features.red_luminance <= 88 {
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
            if features.intensity <= 109 {
            if features.intensity <= 100 {
            if features.red_difference <= 113 {
            if features.green_chromaticity <= 0.427 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.302 {
            if features.red_chromaticity <= 0.297 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.426 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.luminance <= 123 {
            if features.saturation <= 87 {
            Intensity::High
            } else {
            if features.saturation <= 90 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.427 {
            if features.green_chromaticity <= 0.427 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.green_luminance <= 145 {
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
            if features.blue_chromaticity <= 0.286 {
            if features.luminance <= 125 {
            if features.red_chromaticity <= 0.290 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.424 {
            if features.intensity <= 110 {
            if features.red_chromaticity <= 0.291 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.285 {
            Intensity::Low
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
            if features.green_chromaticity <= 0.425 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.282 {
            if features.red_chromaticity <= 0.291 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.292 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            }
            }
            } else {
            if features.red_chromaticity <= 0.296 {
            if features.hue <= 58 {
            if features.green_chromaticity <= 0.422 {
            if features.intensity <= 127 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.293 {
            if features.blue_chromaticity <= 0.284 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.294 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.red_luminance <= 107 {
            if features.green_chromaticity <= 0.426 {
            if features.red_chromaticity <= 0.290 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.285 {
            Intensity::High
            } else {
            if features.value <= 168 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.422 {
            if features.red_chromaticity <= 0.297 {
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
            if features.blue_chromaticity <= 0.292 {
            if features.green_chromaticity <= 0.422 {
            if features.red_luminance <= 112 {
            if features.green_chromaticity <= 0.421 {
            Intensity::Low
            } else {
            if features.blue_chromaticity <= 0.288 {
            if features.red_luminance <= 109 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.289 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.290 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_chromaticity <= 0.427 {
            if features.red_luminance <= 95 {
            if features.blue_chromaticity <= 0.288 {
            if features.green_luminance <= 140 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.intensity <= 111 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.423 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.red_chromaticity <= 0.283 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.286 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.red_chromaticity <= 0.286 {
            if features.saturation <= 88 {
            if features.green_chromaticity <= 0.425 {
            if features.blue_chromaticity <= 0.293 {
            if features.value <= 158 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.value <= 158 {
            if features.red_chromaticity <= 0.280 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.intensity <= 124 {
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
            }
            }
            } else {
            if features.blue_luminance <= 97 {
            if features.blue_luminance <= 88 {
            if features.red_luminance <= 87 {
            if features.blue_difference <= 112 {
            if features.luminance <= 99 {
            if features.red_chromaticity <= 0.306 {
            if features.green_chromaticity <= 0.435 {
            if features.blue_chromaticity <= 0.269 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.value <= 111 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_chromaticity <= 0.312 {
            if features.value <= 113 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.318 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.434 {
            if features.red_luminance <= 83 {
            if features.green_chromaticity <= 0.433 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.red_luminance <= 86 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_chromaticity <= 0.436 {
            if features.intensity <= 98 {
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
            }
            }
            } else {
            if features.red_chromaticity <= 0.291 {
            if features.value <= 126 {
            Intensity::High
            } else {
            if features.saturation <= 90 {
            if features.red_difference <= 110 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.value <= 117 {
            if features.red_luminance <= 74 {
            if features.saturation <= 102 {
            Intensity::Low
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
            if features.saturation <= 92 {
            if features.red_luminance <= 83 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.green_luminance <= 120 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.435 {
            if features.green_luminance <= 128 {
            if features.blue_luminance <= 77 {
            if features.blue_chromaticity <= 0.258 {
            if features.red_chromaticity <= 0.314 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_luminance <= 90 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.271 {
            if features.blue_luminance <= 78 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_chromaticity <= 0.300 {
            if features.green_chromaticity <= 0.434 {
            if features.red_chromaticity <= 0.289 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.value <= 135 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_chromaticity <= 0.431 {
            if features.blue_luminance <= 80 {
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
            }
            } else {
            if features.saturation <= 98 {
            if features.blue_luminance <= 83 {
            if features.red_chromaticity <= 0.295 {
            if features.red_chromaticity <= 0.293 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.435 {
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
            if features.green_chromaticity <= 0.430 {
            if features.luminance <= 123 {
            if features.red_luminance <= 92 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.286 {
            if features.red_luminance <= 94 {
            if features.green_chromaticity <= 0.429 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.297 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.intensity <= 108 {
            Intensity::Low
            } else {
            if features.blue_luminance <= 95 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.blue_difference <= 109 {
            if features.green_chromaticity <= 0.428 {
            if features.red_difference <= 110 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.value <= 143 {
            Intensity::High
            } else {
            if features.green_luminance <= 150 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.value <= 146 {
            if features.saturation <= 85 {
            if features.red_chromaticity <= 0.286 {
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
            } else {
            if features.hue <= 58 {
            if features.green_chromaticity <= 0.429 {
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
            if features.red_chromaticity <= 0.290 {
            if features.saturation <= 88 {
            if features.saturation <= 86 {
            if features.red_difference <= 108 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.blue_luminance <= 91 {
            if features.blue_luminance <= 90 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.saturation <= 91 {
            if features.intensity <= 107 {
            if features.blue_chromaticity <= 0.286 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.red_luminance <= 94 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.value <= 149 {
            if features.blue_luminance <= 89 {
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
            if features.red_chromaticity <= 0.295 {
            if features.green_chromaticity <= 0.432 {
            if features.blue_difference <= 109 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.290 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.intensity <= 110 {
            if features.red_luminance <= 96 {
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
            if features.red_chromaticity <= 0.298 {
            if features.green_luminance <= 142 {
            if features.blue_chromaticity <= 0.274 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.271 {
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
            if features.blue_chromaticity <= 0.290 {
            if features.blue_chromaticity <= 0.277 {
            if features.red_difference <= 108 {
            if features.green_luminance <= 155 {
            if features.saturation <= 91 {
            if features.blue_luminance <= 99 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.433 {
            if features.blue_chromaticity <= 0.276 {
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
            Intensity::Low
            }
            } else {
            if features.saturation <= 89 {
            if features.blue_chromaticity <= 0.285 {
            if features.hue <= 57 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.285 {
            if features.red_chromaticity <= 0.287 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_difference <= 105 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.intensity <= 122 {
            if features.value <= 145 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_difference <= 105 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.blue_luminance <= 102 {
            if features.intensity <= 115 {
            if features.intensity <= 113 {
            if features.saturation <= 91 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            } else {
            if features.green_luminance <= 158 {
            if features.saturation <= 91 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_luminance <= 100 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.red_chromaticity <= 0.275 {
            if features.green_chromaticity <= 0.432 {
            Intensity::High
            } else {
            if features.saturation <= 95 {
            if features.blue_difference <= 112 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.293 {
            if features.green_chromaticity <= 0.428 {
            Intensity::High
            } else {
            if features.red_difference <= 105 {
            if features.luminance <= 134 {
            if features.intensity <= 113 {
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
            if features.green_chromaticity <= 0.432 {
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
            if features.value <= 157 {
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
            if features.blue_chromaticity <= 0.223 {
            if features.green_luminance <= 92 {
            if features.hue <= 39 {
            if features.green_chromaticity <= 0.435 {
            if features.green_chromaticity <= 0.433 {
            Intensity::Low
            } else {
            if features.blue_difference <= 109 {
            Intensity::High
            } else {
            if features.saturation <= 137 {
            if features.intensity <= 52 {
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
            if features.saturation <= 125 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.347 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.429 {
            Intensity::High
            } else {
            if features.luminance <= 62 {
            if features.red_luminance <= 56 {
            if features.blue_difference <= 112 {
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
            Intensity::High
            }
            } else {
            if features.blue_difference <= 111 {
            if features.green_chromaticity <= 0.433 {
            if features.luminance <= 104 {
            if features.saturation <= 115 {
            if features.blue_chromaticity <= 0.241 {
            if features.value <= 90 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.431 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.328 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.value <= 97 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.256 {
            if features.blue_chromaticity <= 0.252 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.blue_luminance <= 69 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.226 {
            if features.hue <= 41 {
            Intensity::Low
            } else {
            if features.red_luminance <= 66 {
            Intensity::Low
            } else {
            if features.luminance <= 75 {
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
            Intensity::High
            }
            } else {
            if features.luminance <= 89 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.322 {
            if features.green_chromaticity <= 0.434 {
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
            if features.blue_chromaticity <= 0.228 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.430 {
            if features.saturation <= 97 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.259 {
            if features.red_chromaticity <= 0.347 {
            if features.green_chromaticity <= 0.425 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.425 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.424 {
            Intensity::Low
            } else {
            if features.red_luminance <= 74 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.254 {
            if features.blue_chromaticity <= 0.249 {
            if features.red_chromaticity <= 0.325 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.430 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.328 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.253 {
            if features.red_chromaticity <= 0.317 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.luminance <= 84 {
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
            }
            } else {
            if features.red_difference <= 122 {
            if features.blue_chromaticity <= 0.265 {
            if features.green_chromaticity <= 0.456 {
            if features.blue_luminance <= 56 {
            if features.red_difference <= 120 {
            if features.luminance <= 75 {
            if features.blue_chromaticity <= 0.229 {
            if features.red_chromaticity <= 0.319 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.314 {
            if features.red_chromaticity <= 0.311 {
            if features.green_chromaticity <= 0.451 {
            if features.blue_chromaticity <= 0.240 {
            Intensity::Low
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
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.437 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.324 {
            if features.green_luminance <= 79 {
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
            if features.green_chromaticity <= 0.447 {
            if features.intensity <= 71 {
            if features.green_chromaticity <= 0.442 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.444 {
            if features.luminance <= 79 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.red_luminance <= 66 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.440 {
            if features.blue_chromaticity <= 0.237 {
            Intensity::High
            } else {
            if features.hue <= 50 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_luminance <= 78 {
            if features.blue_difference <= 108 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.328 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.red_chromaticity <= 0.291 {
            if features.red_chromaticity <= 0.290 {
            if features.luminance <= 81 {
            Intensity::Low
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
            if features.intensity <= 67 {
            if features.green_chromaticity <= 0.454 {
            if features.red_chromaticity <= 0.300 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            } else {
            if features.value <= 96 {
            Intensity::High
            } else {
            if features.blue_luminance <= 52 {
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
            if features.green_chromaticity <= 0.450 {
            if features.red_chromaticity <= 0.338 {
            if features.blue_chromaticity <= 0.231 {
            if features.green_chromaticity <= 0.440 {
            if features.red_chromaticity <= 0.332 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.226 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.intensity <= 60 {
            if features.green_chromaticity <= 0.442 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.442 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.234 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_chromaticity <= 0.347 {
            if features.blue_chromaticity <= 0.219 {
            if features.blue_difference <= 106 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.value <= 89 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.saturation <= 139 {
            if features.blue_chromaticity <= 0.210 {
            if features.blue_chromaticity <= 0.207 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.green_luminance <= 103 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_chromaticity <= 0.364 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.red_chromaticity <= 0.358 {
            if features.blue_luminance <= 38 {
            Intensity::High
            } else {
            if features.intensity <= 62 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.186 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.red_difference <= 102 {
            if features.hue <= 56 {
            if features.blue_chromaticity <= 0.259 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.287 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.value <= 171 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.luminance <= 100 {
            if features.green_chromaticity <= 0.440 {
            if features.blue_chromaticity <= 0.258 {
            if features.red_chromaticity <= 0.314 {
            if features.red_chromaticity <= 0.309 {
            if features.red_chromaticity <= 0.308 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            } else {
            if features.hue <= 48 {
            if features.intensity <= 79 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.438 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.259 {
            if features.intensity <= 86 {
            if features.hue <= 52 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_luminance <= 61 {
            if features.red_difference <= 115 {
            Intensity::Low
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
            if features.blue_chromaticity <= 0.260 {
            if features.blue_chromaticity <= 0.252 {
            if features.blue_chromaticity <= 0.252 {
            if features.red_chromaticity <= 0.320 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.green_luminance <= 117 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_luminance <= 78 {
            if features.saturation <= 106 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.green_luminance <= 116 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.green_luminance <= 112 {
            if features.red_chromaticity <= 0.294 {
            Intensity::High
            } else {
            if features.luminance <= 89 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.saturation <= 107 {
            if features.red_difference <= 111 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_difference <= 110 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            }
            } else {
            if features.saturation <= 109 {
            if features.luminance <= 106 {
            if features.luminance <= 104 {
            if features.red_luminance <= 76 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.291 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.253 {
            Intensity::Low
            } else {
            if features.luminance <= 105 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.red_luminance <= 106 {
            if features.green_chromaticity <= 0.450 {
            if features.red_chromaticity <= 0.294 {
            Intensity::High
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
            if features.hue <= 55 {
            if features.blue_luminance <= 88 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.saturation <= 103 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.red_chromaticity <= 0.342 {
            if features.saturation <= 112 {
            if features.intensity <= 106 {
            if features.blue_difference <= 105 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.248 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_chromaticity <= 0.456 {
            if features.red_luminance <= 94 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.blue_difference <= 103 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.213 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.344 {
            if features.blue_difference <= 99 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.intensity <= 90 {
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
            if features.red_difference <= 120 {
            if features.blue_chromaticity <= 0.249 {
            if features.blue_chromaticity <= 0.227 {
            if features.red_difference <= 118 {
            if features.intensity <= 109 {
            if features.blue_chromaticity <= 0.201 {
            if features.green_chromaticity <= 0.467 {
            if features.red_luminance <= 82 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.value <= 36 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_difference <= 99 {
            if features.red_luminance <= 49 {
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
            }
            } else {
            if features.red_chromaticity <= 0.325 {
            if features.red_chromaticity <= 0.299 {
            if features.red_chromaticity <= 0.295 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.326 {
            if features.saturation <= 134 {
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
            if features.blue_chromaticity <= 0.208 {
            if features.red_luminance <= 4 {
            if features.green_chromaticity <= 0.881 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.red_difference <= 119 {
            if features.green_chromaticity <= 0.493 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.338 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.saturation <= 142 {
            if features.blue_chromaticity <= 0.219 {
            if features.value <= 81 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.459 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_chromaticity <= 0.481 {
            if features.green_luminance <= 73 {
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
            if features.red_difference <= 97 {
            if features.intensity <= 76 {
            if features.blue_chromaticity <= 0.246 {
            if features.blue_chromaticity <= 0.241 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.241 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.246 {
            Intensity::Low
            } else {
            if features.saturation <= 154 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.239 {
            if features.blue_luminance <= 54 {
            if features.hue <= 60 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.green_luminance <= 128 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_difference <= 92 {
            if features.red_chromaticity <= 0.208 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.239 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.red_difference <= 117 {
            if features.red_difference <= 115 {
            if features.red_difference <= 99 {
            if features.saturation <= 137 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.luminance <= 128 {
            Intensity::High
            } else {
            Intensity::High
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
            if features.green_chromaticity <= 0.467 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.red_luminance <= 53 {
            if features.blue_luminance <= 39 {
            if features.luminance <= 65 {
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
            if features.red_difference <= 94 {
            if features.red_chromaticity <= 0.210 {
            if features.blue_chromaticity <= 0.263 {
            if features.blue_luminance <= 58 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.546 {
            if features.red_chromaticity <= 0.198 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.253 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.264 {
            if features.green_luminance <= 130 {
            if features.blue_chromaticity <= 0.264 {
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
            if features.blue_chromaticity <= 0.261 {
            if features.green_luminance <= 165 {
            if features.blue_chromaticity <= 0.249 {
            Intensity::High
            } else {
            if features.green_luminance <= 143 {
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
            if features.intensity <= 119 {
            if features.red_difference <= 100 {
            if features.luminance <= 112 {
            if features.blue_chromaticity <= 0.265 {
            if features.blue_difference <= 105 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.265 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_chromaticity <= 0.250 {
            if features.green_chromaticity <= 0.498 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.272 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.249 {
            if features.red_chromaticity <= 0.233 {
            Intensity::Low
            } else {
            if features.hue <= 59 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_luminance <= 91 {
            if features.blue_chromaticity <= 0.257 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.278 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.red_chromaticity <= 0.280 {
            if features.hue <= 58 {
            if features.red_chromaticity <= 0.274 {
            if features.saturation <= 112 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.luminance <= 142 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.saturation <= 112 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.150 {
            if features.hue <= 41 {
            if features.red_luminance <= 53 {
            if features.saturation <= 204 {
            if features.green_chromaticity <= 0.523 {
            if features.blue_chromaticity <= 0.131 {
            if features.saturation <= 194 {
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
            if features.blue_luminance <= 4 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_difference <= 121 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.value <= 37 {
            if features.red_luminance <= 18 {
            if features.blue_difference <= 108 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.658 {
            Intensity::High
            } else {
            if features.blue_difference <= 111 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.blue_difference <= 110 {
            Intensity::High
            } else {
            if features.red_luminance <= 31 {
            if features.green_chromaticity <= 0.529 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.intensity <= 30 {
            Intensity::Low
            } else {
            if features.green_luminance <= 53 {
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
            if features.green_luminance <= 55 {
            if features.hue <= 45 {
            if features.value <= 53 {
            if features.red_chromaticity <= 0.340 {
            if features.blue_luminance <= 16 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.value <= 54 {
            if features.blue_luminance <= 17 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_luminance <= 37 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.blue_difference <= 112 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.saturation <= 162 {
            if features.blue_chromaticity <= 0.179 {
            if features.green_chromaticity <= 0.466 {
            if features.saturation <= 158 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.green_luminance <= 73 {
            if features.saturation <= 161 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_luminance <= 33 {
            if features.green_chromaticity <= 0.460 {
            if features.green_chromaticity <= 0.458 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.saturation <= 146 {
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
            if features.red_chromaticity <= 0.339 {
            if features.green_chromaticity <= 0.516 {
            if features.red_chromaticity <= 0.339 {
            if features.value <= 60 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.red_luminance <= 36 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_difference <= 96 {
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
            if features.value <= 171 {
            if features.red_difference <= 98 {
            if features.green_luminance <= 126 {
            if features.red_difference <= 97 {
            if features.luminance <= 95 {
            if features.luminance <= 94 {
            if features.value <= 118 {
            if features.luminance <= 79 {
            Intensity::High
            } else {
            if features.red_luminance <= 40 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.271 {
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
            if features.red_difference <= 95 {
            if features.red_chromaticity <= 0.193 {
            Intensity::High
            } else {
            if features.red_difference <= 94 {
            if features.green_chromaticity <= 0.489 {
            Intensity::High
            } else {
            if features.blue_difference <= 109 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.198 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.268 {
            Intensity::High
            } else {
            if features.saturation <= 141 {
            if features.blue_difference <= 108 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.value <= 131 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.476 {
            if features.saturation <= 107 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.472 {
            if features.green_luminance <= 161 {
            Intensity::High
            } else {
            if features.saturation <= 117 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_luminance <= 69 {
            Intensity::High
            } else {
            if features.red_difference <= 97 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.red_chromaticity <= 0.242 {
            if features.saturation <= 133 {
            if features.red_difference <= 97 {
            if features.red_chromaticity <= 0.238 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.value <= 137 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.saturation <= 141 {
            if features.blue_luminance <= 77 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_luminance <= 69 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
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
            if features.blue_chromaticity <= 0.282 {
            if features.green_chromaticity <= 0.442 {
            if features.green_luminance <= 129 {
            if features.intensity <= 86 {
            if features.red_difference <= 113 {
            if features.green_chromaticity <= 0.440 {
            if features.blue_chromaticity <= 0.270 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.intensity <= 85 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_luminance <= 106 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_chromaticity <= 0.286 {
            if features.blue_chromaticity <= 0.281 {
            if features.blue_luminance <= 79 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_difference <= 108 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.268 {
            if features.green_chromaticity <= 0.437 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.436 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.270 {
            if features.intensity <= 117 {
            if features.blue_chromaticity <= 0.268 {
            if features.value <= 141 {
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
            Intensity::High
            }
            } else {
            if features.red_difference <= 103 {
            if features.luminance <= 142 {
            if features.green_chromaticity <= 0.442 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            } else {
            if features.red_luminance <= 85 {
            if features.green_chromaticity <= 0.438 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.438 {
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
            if features.blue_chromaticity <= 0.272 {
            if features.red_difference <= 111 {
            if features.red_difference <= 105 {
            if features.red_luminance <= 100 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.luminance <= 110 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_chromaticity <= 0.444 {
            Intensity::High
            } else {
            if features.luminance <= 93 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.red_chromaticity <= 0.276 {
            if features.red_difference <= 107 {
            if features.luminance <= 124 {
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
            } else {
            if features.red_chromaticity <= 0.278 {
            if features.blue_chromaticity <= 0.273 {
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
            if features.red_chromaticity <= 0.280 {
            if features.blue_chromaticity <= 0.278 {
            if features.hue <= 60 {
            if features.saturation <= 106 {
            Intensity::Low
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
            } else {
            if features.red_luminance <= 97 {
            if features.saturation <= 100 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.saturation <= 95 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.red_chromaticity <= 0.268 {
            if features.green_luminance <= 148 {
            if features.red_chromaticity <= 0.256 {
            if features.red_chromaticity <= 0.249 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.468 {
            if features.blue_chromaticity <= 0.284 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_luminance <= 75 {
            if features.red_luminance <= 74 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.284 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.luminance <= 107 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.287 {
            if features.green_chromaticity <= 0.451 {
            Intensity::High
            } else {
            if features.red_difference <= 99 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.262 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.red_chromaticity <= 0.252 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.red_chromaticity <= 0.271 {
            if features.red_difference <= 102 {
            if features.blue_chromaticity <= 0.286 {
            Intensity::High
            } else {
            if features.saturation <= 97 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.290 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.value <= 146 {
            if features.red_chromaticity <= 0.270 {
            if features.blue_chromaticity <= 0.282 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.saturation <= 99 {
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
            if features.red_difference <= 104 {
            Intensity::High
            } else {
            if features.green_luminance <= 142 {
            if features.green_chromaticity <= 0.443 {
            if features.green_chromaticity <= 0.438 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.saturation <= 99 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_chromaticity <= 0.439 {
            if features.blue_chromaticity <= 0.289 {
            Intensity::Low
            } else {
            Intensity::High
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
            }
            }
            } else {
            if features.red_difference <= 102 {
            if features.red_chromaticity <= 0.247 {
            Intensity::High
            } else {
            if features.red_difference <= 101 {
            if features.green_luminance <= 172 {
            if features.red_chromaticity <= 0.268 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.280 {
            if features.hue <= 58 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.281 {
            if features.blue_luminance <= 108 {
            if features.value <= 176 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            } else {
            if features.blue_difference <= 101 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.red_chromaticity <= 0.284 {
            if features.blue_difference <= 106 {
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
            if features.saturation <= 97 {
            if features.luminance <= 153 {
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
            if features.red_difference <= 123 {
            if features.red_chromaticity <= 0.354 {
            if features.green_chromaticity <= 0.453 {
            if features.red_chromaticity <= 0.347 {
            if features.luminance <= 57 {
            Intensity::High
            } else {
            if features.green_luminance <= 76 {
            if features.intensity <= 54 {
            if features.value <= 71 {
            if features.blue_chromaticity <= 0.209 {
            if features.red_luminance <= 51 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.value <= 70 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            Intensity::High
            }
            } else {
            if features.blue_difference <= 112 {
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
            if features.red_chromaticity <= 0.348 {
            Intensity::High
            } else {
            if features.saturation <= 143 {
            if features.luminance <= 77 {
            if features.green_chromaticity <= 0.444 {
            Intensity::High
            } else {
            if features.blue_difference <= 110 {
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
            if features.red_chromaticity <= 0.351 {
            if features.red_chromaticity <= 0.346 {
            if features.red_chromaticity <= 0.344 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.462 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.346 {
            if features.saturation <= 162 {
            if features.value <= 60 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.498 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.471 {
            if features.value <= 68 {
            if features.saturation <= 155 {
            if features.blue_luminance <= 27 {
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
            if features.luminance <= 30 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.luminance <= 43 {
            if features.green_chromaticity <= 0.589 {
            Intensity::Low
            } else {
            if features.red_luminance <= 21 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_chromaticity <= 0.353 {
            Intensity::High
            } else {
            if features.blue_luminance <= 17 {
            Intensity::High
            } else {
            if features.blue_difference <= 109 {
            Intensity::Low
            } else {
            if features.blue_difference <= 110 {
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
            if features.green_luminance <= 52 {
            if features.red_luminance <= 33 {
            if features.red_chromaticity <= 0.379 {
            if features.red_chromaticity <= 0.355 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.value <= 50 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.545 {
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
            if features.red_difference <= 125 {
            if features.blue_difference <= 112 {
            if features.saturation <= 191 {
            if features.red_chromaticity <= 0.367 {
            if features.blue_difference <= 108 {
            if features.hue <= 39 {
            if features.blue_luminance <= 47 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            } else {
            if features.green_luminance <= 53 {
            if features.blue_luminance <= 16 {
            if features.saturation <= 185 {
            if features.saturation <= 178 {
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
            if features.blue_chromaticity <= 0.169 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.205 {
            if features.green_chromaticity <= 0.450 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.value <= 77 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            }
            } else {
            if features.hue <= 40 {
            if features.blue_difference <= 108 {
            if features.intensity <= 74 {
            if features.intensity <= 48 {
            if features.green_luminance <= 65 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.green_luminance <= 69 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.value <= 51 {
            if features.blue_luminance <= 13 {
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
            if features.red_chromaticity <= 0.411 {
            if features.blue_difference <= 111 {
            Intensity::High
            } else {
            if features.red_luminance <= 26 {
            if features.red_chromaticity <= 0.363 {
            Intensity::High
            } else {
            if features.red_luminance <= 25 {
            if features.red_chromaticity <= 0.377 {
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
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.442 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.366 {
            if features.red_chromaticity <= 0.365 {
            if features.red_chromaticity <= 0.355 {
            if features.green_chromaticity <= 0.444 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.350 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.351 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.red_chromaticity <= 0.356 {
            Intensity::High
            } else {
            if features.intensity <= 30 {
            if features.red_chromaticity <= 0.360 {
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
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.red_difference <= 126 {
            if features.hue <= 36 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.386 {
            if features.red_luminance <= 47 {
            if features.red_luminance <= 45 {
            if features.luminance <= 45 {
            if features.red_chromaticity <= 0.384 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.456 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_luminance <= 21 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_luminance <= 29 {
            if features.hue <= 37 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_luminance <= 30 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.blue_difference <= 110 {
            if features.blue_difference <= 109 {
            if features.blue_luminance <= 6 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.487 {
            if features.red_luminance <= 61 {
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
            if features.blue_difference <= 112 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.red_luminance <= 26 {
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
            if features.green_chromaticity <= 0.502 {
            if features.green_chromaticity <= 0.462 {
            if features.green_chromaticity <= 0.438 {
            if features.green_chromaticity <= 0.410 {
            if features.green_chromaticity <= 0.400 {
            if features.green_chromaticity <= 0.390 {
            if features.blue_difference <= 133 {
            if features.green_chromaticity <= 0.381 {
            if features.red_difference <= 111 {
            if features.blue_difference <= 132 {
            if features.blue_chromaticity <= 0.342 {
            Intensity::Low
            } else {
            if features.blue_chromaticity <= 0.342 {
            if features.red_chromaticity <= 0.291 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.red_difference <= 110 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.red_difference <= 110 {
            if features.red_difference <= 109 {
            if features.luminance <= 112 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.saturation <= 53 {
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
            if features.blue_difference <= 116 {
            if features.blue_chromaticity <= 0.312 {
            if features.saturation <= 34 {
            if features.green_luminance <= 185 {
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
            if features.green_chromaticity <= 0.377 {
            if features.blue_difference <= 115 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_difference <= 129 {
            if features.green_chromaticity <= 0.373 {
            if features.green_luminance <= 189 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.373 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_difference <= 118 {
            if features.green_chromaticity <= 0.347 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.hue <= 89 {
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
            if features.red_difference <= 125 {
            if features.blue_luminance <= 117 {
            if features.green_chromaticity <= 0.386 {
            if features.intensity <= 32 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_luminance <= 120 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_chromaticity <= 0.307 {
            if features.saturation <= 60 {
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
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.381 {
            if features.blue_luminance <= 148 {
            if features.red_chromaticity <= 0.269 {
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
            if features.red_chromaticity <= 0.311 {
            if features.blue_difference <= 124 {
            if features.saturation <= 48 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_luminance <= 106 {
            Intensity::Low
            } else {
            Intensity::Low
            }
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
            }
            } else {
            if features.red_luminance <= 203 {
            if features.red_luminance <= 170 {
            if features.hue <= 92 {
            if features.red_chromaticity <= 0.230 {
            if features.red_difference <= 107 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.355 {
            if features.blue_chromaticity <= 0.355 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.red_difference <= 110 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.blue_luminance <= 157 {
            if features.blue_chromaticity <= 0.357 {
            if features.blue_chromaticity <= 0.357 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.red_difference <= 117 {
            if features.blue_luminance <= 165 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.342 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.red_chromaticity <= 0.307 {
            if features.luminance <= 184 {
            if features.blue_luminance <= 207 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.red_luminance <= 171 {
            if features.blue_chromaticity <= 0.362 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_luminance <= 172 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.saturation <= 36 {
            if features.red_chromaticity <= 0.323 {
            if features.red_difference <= 125 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.308 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.308 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.359 {
            if features.blue_chromaticity <= 0.353 {
            if features.blue_chromaticity <= 0.350 {
            if features.hue <= 93 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.350 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.319 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.red_luminance <= 218 {
            if features.red_luminance <= 206 {
            Intensity::High
            } else {
            if features.blue_difference <= 138 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_luminance <= 245 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.blue_difference <= 141 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.blue_difference <= 114 {
            if features.blue_chromaticity <= 0.293 {
            if features.red_chromaticity <= 0.328 {
            if features.red_chromaticity <= 0.328 {
            if features.saturation <= 73 {
            if features.red_chromaticity <= 0.311 {
            if features.green_chromaticity <= 0.400 {
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
            if features.luminance <= 115 {
            if features.blue_luminance <= 88 {
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
            } else {
            if features.green_chromaticity <= 0.393 {
            if features.green_chromaticity <= 0.393 {
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
            }
            } else {
            if features.red_chromaticity <= 0.305 {
            if features.blue_chromaticity <= 0.304 {
            if features.blue_chromaticity <= 0.304 {
            if features.value <= 185 {
            if features.red_luminance <= 130 {
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
            } else {
            if features.hue <= 61 {
            if features.blue_chromaticity <= 0.304 {
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
            if features.green_chromaticity <= 0.390 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.393 {
            if features.value <= 151 {
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
            if features.blue_difference <= 116 {
            if features.red_luminance <= 113 {
            if features.luminance <= 114 {
            if features.blue_difference <= 115 {
            if features.saturation <= 71 {
            if features.blue_luminance <= 84 {
            Intensity::High
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
            } else {
            Intensity::Low
            }
            } else {
            if features.intensity <= 110 {
            if features.blue_chromaticity <= 0.296 {
            if features.green_chromaticity <= 0.395 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.297 {
            if features.blue_chromaticity <= 0.292 {
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
            } else {
            if features.red_luminance <= 117 {
            if features.luminance <= 136 {
            if features.blue_chromaticity <= 0.296 {
            if features.red_luminance <= 115 {
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
            if features.red_difference <= 113 {
            if features.green_chromaticity <= 0.400 {
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
            } else {
            if features.blue_chromaticity <= 0.306 {
            if features.blue_chromaticity <= 0.305 {
            if features.blue_chromaticity <= 0.304 {
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
            if features.blue_chromaticity <= 0.310 {
            Intensity::Low
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
            if features.blue_luminance <= 36 {
            if features.green_chromaticity <= 0.390 {
            if features.blue_chromaticity <= 0.348 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.232 {
            if features.blue_luminance <= 21 {
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
            if features.green_luminance <= 144 {
            if features.blue_luminance <= 89 {
            if features.green_chromaticity <= 0.398 {
            if features.green_chromaticity <= 0.390 {
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
            } else {
            if features.value <= 104 {
            if features.saturation <= 90 {
            Intensity::High
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
            if features.red_chromaticity <= 0.302 {
            if features.green_chromaticity <= 0.396 {
            if features.intensity <= 122 {
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
            } else {
            if features.red_chromaticity <= 0.302 {
            if features.red_difference <= 112 {
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
            }
            }
            }
            } else {
            if features.intensity <= 92 {
            if features.blue_luminance <= 40 {
            if features.green_luminance <= 46 {
            if features.red_difference <= 123 {
            if features.green_chromaticity <= 0.408 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.408 {
            if features.hue <= 81 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.luminance <= 30 {
            Intensity::Low
            } else {
            if features.red_luminance <= 21 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.red_difference <= 124 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.409 {
            if features.luminance <= 42 {
            if features.red_chromaticity <= 0.307 {
            if features.red_chromaticity <= 0.260 {
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
            if features.green_chromaticity <= 0.408 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.408 {
            if features.red_chromaticity <= 0.328 {
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
            if features.green_chromaticity <= 0.409 {
            if features.saturation <= 89 {
            Intensity::Low
            } else {
            if features.red_difference <= 126 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.hue <= 67 {
            if features.red_luminance <= 50 {
            if features.blue_luminance <= 24 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.intensity <= 48 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_chromaticity <= 0.269 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            }
            } else {
            if features.value <= 42 {
            if features.green_chromaticity <= 0.405 {
            if features.luminance <= 29 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.332 {
            if features.green_chromaticity <= 0.407 {
            if features.saturation <= 69 {
            if features.green_chromaticity <= 0.406 {
            if features.intensity <= 46 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.340 {
            if features.red_luminance <= 79 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.340 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.blue_difference <= 119 {
            if features.green_chromaticity <= 0.407 {
            Intensity::High
            } else {
            if features.value <= 95 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_luminance <= 32 {
            if features.green_chromaticity <= 0.410 {
            Intensity::Low
            } else {
            Intensity::High
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
            if features.red_luminance <= 77 {
            if features.blue_chromaticity <= 0.264 {
            if features.green_luminance <= 78 {
            if features.intensity <= 53 {
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
            if features.luminance <= 64 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.332 {
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
            if features.intensity <= 80 {
            if features.red_luminance <= 78 {
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
            if features.blue_luminance <= 103 {
            if features.green_chromaticity <= 0.410 {
            if features.blue_chromaticity <= 0.289 {
            if features.blue_chromaticity <= 0.289 {
            if features.blue_luminance <= 81 {
            if features.green_chromaticity <= 0.409 {
            if features.luminance <= 101 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.luminance <= 104 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.279 {
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
            if features.green_chromaticity <= 0.407 {
            if features.red_chromaticity <= 0.310 {
            if features.red_luminance <= 92 {
            if features.red_luminance <= 74 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.saturation <= 74 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            Intensity::High
            }
            } else {
            if features.saturation <= 73 {
            if features.green_chromaticity <= 0.410 {
            if features.red_luminance <= 92 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            } else {
            if features.red_difference <= 109 {
            Intensity::High
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
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.299 {
            if features.red_luminance <= 111 {
            if features.value <= 153 {
            if features.red_chromaticity <= 0.295 {
            if features.green_chromaticity <= 0.409 {
            if features.saturation <= 71 {
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
            if features.green_chromaticity <= 0.410 {
            if features.red_luminance <= 104 {
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
            if features.red_chromaticity <= 0.303 {
            if features.blue_chromaticity <= 0.294 {
            if features.saturation <= 69 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.297 {
            if features.value <= 155 {
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
            }
            } else {
            if features.green_chromaticity <= 0.401 {
            if features.value <= 152 {
            Intensity::Low
            } else {
            if features.value <= 156 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_chromaticity <= 0.306 {
            if features.green_chromaticity <= 0.402 {
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
            if features.green_luminance <= 139 {
            if features.blue_chromaticity <= 0.306 {
            if features.red_difference <= 111 {
            Intensity::High
            } else {
            if features.saturation <= 68 {
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
            if features.green_chromaticity <= 0.407 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.407 {
            Intensity::High
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
            if features.blue_luminance <= 146 {
            if features.blue_chromaticity <= 0.307 {
            if features.saturation <= 75 {
            if features.blue_luminance <= 128 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.288 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.value <= 173 {
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
            if features.value <= 104 {
            if features.green_luminance <= 60 {
            if features.green_luminance <= 48 {
            if features.blue_luminance <= 24 {
            if features.blue_luminance <= 20 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.271 {
            if features.red_luminance <= 20 {
            if features.hue <= 66 {
            if features.red_chromaticity <= 0.269 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.intensity <= 22 {
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
            if features.blue_difference <= 125 {
            if features.blue_chromaticity <= 0.318 {
            if features.green_chromaticity <= 0.423 {
            if features.blue_chromaticity <= 0.308 {
            if features.intensity <= 34 {
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
            if features.blue_chromaticity <= 0.315 {
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
            if features.intensity <= 35 {
            if features.blue_chromaticity <= 0.321 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.321 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.blue_difference <= 133 {
            if features.blue_luminance <= 32 {
            if features.green_chromaticity <= 0.415 {
            if features.green_chromaticity <= 0.414 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_luminance <= 35 {
            if features.saturation <= 124 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_chromaticity <= 0.129 {
            if features.red_difference <= 108 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.value <= 42 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.429 {
            if features.green_chromaticity <= 0.425 {
            if features.red_difference <= 119 {
            if features.red_chromaticity <= 0.274 {
            if features.blue_luminance <= 39 {
            if features.blue_chromaticity <= 0.328 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.261 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.red_luminance <= 45 {
            if features.blue_chromaticity <= 0.317 {
            if features.red_chromaticity <= 0.335 {
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
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.saturation <= 107 {
            if features.green_chromaticity <= 0.428 {
            if features.blue_chromaticity <= 0.285 {
            if features.red_chromaticity <= 0.291 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_luminance <= 30 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.hue <= 71 {
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
            if features.luminance <= 48 {
            if features.green_chromaticity <= 0.430 {
            if features.blue_luminance <= 38 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.saturation <= 95 {
            if features.blue_chromaticity <= 0.293 {
            if features.saturation <= 94 {
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
            if features.red_luminance <= 24 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.437 {
            if features.blue_chromaticity <= 0.291 {
            if features.green_chromaticity <= 0.431 {
            if features.value <= 58 {
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
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_luminance <= 34 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.233 {
            Intensity::Low
            } else {
            if features.saturation <= 110 {
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
            if features.green_chromaticity <= 0.431 {
            if features.green_chromaticity <= 0.426 {
            if features.green_chromaticity <= 0.416 {
            if features.red_chromaticity <= 0.306 {
            if features.saturation <= 76 {
            if features.red_chromaticity <= 0.290 {
            if features.red_luminance <= 60 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.saturation <= 74 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_luminance <= 75 {
            if features.red_chromaticity <= 0.279 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_luminance <= 76 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.green_luminance <= 81 {
            if features.green_chromaticity <= 0.416 {
            if features.red_luminance <= 46 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_luminance <= 41 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.saturation <= 99 {
            if features.blue_chromaticity <= 0.281 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.416 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.310 {
            if features.blue_chromaticity <= 0.310 {
            if features.blue_luminance <= 75 {
            if features.red_chromaticity <= 0.347 {
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
            if features.green_chromaticity <= 0.420 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.420 {
            Intensity::Low
            } else {
            if features.blue_chromaticity <= 0.324 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            }
            } else {
            if features.red_chromaticity <= 0.348 {
            if features.red_luminance <= 57 {
            if features.blue_luminance <= 59 {
            if features.green_luminance <= 86 {
            if features.green_chromaticity <= 0.427 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            } else {
            if features.red_luminance <= 52 {
            if features.luminance <= 53 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.green_luminance <= 89 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.red_chromaticity <= 0.303 {
            if features.red_chromaticity <= 0.276 {
            if features.green_luminance <= 97 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_difference <= 112 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_chromaticity <= 0.304 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.428 {
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
            if features.blue_difference <= 118 {
            if features.saturation <= 113 {
            if features.blue_chromaticity <= 0.242 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.437 {
            if features.red_difference <= 112 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.433 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.437 {
            if features.hue <= 51 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.saturation <= 95 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.value <= 77 {
            if features.blue_difference <= 114 {
            if features.green_chromaticity <= 0.437 {
            Intensity::Low
            } else {
            if features.red_difference <= 122 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_chromaticity <= 0.228 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.437 {
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
            if features.red_luminance <= 62 {
            if features.green_chromaticity <= 0.431 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.434 {
            if features.intensity <= 51 {
            if features.saturation <= 97 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_luminance <= 51 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.434 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.279 {
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
            if features.blue_chromaticity <= 0.296 {
            if features.green_chromaticity <= 0.425 {
            if features.green_chromaticity <= 0.417 {
            if features.intensity <= 115 {
            if features.red_luminance <= 87 {
            if features.blue_luminance <= 69 {
            Intensity::High
            } else {
            if features.blue_luminance <= 86 {
            if features.red_chromaticity <= 0.311 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_luminance <= 88 {
            Intensity::High
            } else {
            if features.value <= 142 {
            if features.saturation <= 78 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.hue <= 59 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.296 {
            if features.green_chromaticity <= 0.411 {
            if features.hue <= 59 {
            if features.blue_chromaticity <= 0.293 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.412 {
            if features.saturation <= 73 {
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
            if features.red_chromaticity <= 0.289 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.red_luminance <= 80 {
            if features.saturation <= 77 {
            Intensity::High
            } else {
            if features.luminance <= 91 {
            Intensity::High
            } else {
            if features.value <= 109 {
            if features.green_chromaticity <= 0.420 {
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
            }
            }
            } else {
            if features.green_luminance <= 150 {
            if features.red_chromaticity <= 0.290 {
            if features.red_difference <= 111 {
            if features.red_chromaticity <= 0.282 {
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
            if features.green_chromaticity <= 0.419 {
            if features.blue_luminance <= 93 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.282 {
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
            if features.green_chromaticity <= 0.430 {
            if features.green_chromaticity <= 0.425 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.276 {
            Intensity::High
            } else {
            if features.red_luminance <= 73 {
            if features.red_chromaticity <= 0.282 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.278 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_luminance <= 74 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.429 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            }
            } else {
            if features.green_luminance <= 139 {
            if features.red_difference <= 108 {
            if features.green_chromaticity <= 0.437 {
            if features.saturation <= 92 {
            if features.green_chromaticity <= 0.430 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.293 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_chromaticity <= 0.437 {
            if features.luminance <= 107 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.luminance <= 104 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.291 {
            if features.blue_chromaticity <= 0.285 {
            if features.red_difference <= 113 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.435 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_chromaticity <= 0.430 {
            if features.green_chromaticity <= 0.430 {
            Intensity::Low
            } else {
            Intensity::High
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
            } else {
            if features.blue_chromaticity <= 0.292 {
            Intensity::High
            } else {
            if features.blue_luminance <= 96 {
            if features.green_chromaticity <= 0.436 {
            if features.red_luminance <= 89 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.saturation <= 92 {
            if features.hue <= 63 {
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
            if features.blue_luminance <= 104 {
            if features.green_chromaticity <= 0.422 {
            if features.value <= 145 {
            if features.red_luminance <= 79 {
            if features.saturation <= 75 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.417 {
            if features.blue_luminance <= 76 {
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
            }
            } else {
            if features.blue_difference <= 121 {
            if features.red_chromaticity <= 0.293 {
            if features.green_chromaticity <= 0.416 {
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
            Intensity::High
            }
            } else {
            if features.blue_difference <= 117 {
            if features.green_chromaticity <= 0.436 {
            if features.blue_chromaticity <= 0.297 {
            Intensity::High
            } else {
            if features.red_luminance <= 91 {
            if features.red_chromaticity <= 0.272 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_difference <= 103 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.red_luminance <= 73 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_chromaticity <= 0.424 {
            if features.red_chromaticity <= 0.263 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.422 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_chromaticity <= 0.437 {
            if features.saturation <= 102 {
            if features.red_difference <= 106 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_luminance <= 79 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_difference <= 105 {
            Intensity::Low
            } else {
            if features.saturation <= 106 {
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
            if features.blue_chromaticity <= 0.307 {
            if features.green_chromaticity <= 0.434 {
            if features.red_luminance <= 126 {
            if features.green_chromaticity <= 0.418 {
            if features.luminance <= 140 {
            if features.green_luminance <= 160 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.green_luminance <= 160 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_chromaticity <= 0.298 {
            Intensity::High
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
            } else {
            Intensity::High
            }
            } else {
            if features.green_luminance <= 181 {
            if features.blue_difference <= 117 {
            if features.red_difference <= 102 {
            Intensity::High
            } else {
            if features.value <= 154 {
            if features.green_chromaticity <= 0.414 {
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
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.316 {
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
            if features.luminance <= 53 {
            if features.value <= 53 {
            if features.value <= 43 {
            if features.green_luminance <= 34 {
            if features.red_difference <= 117 {
            if features.blue_chromaticity <= 0.409 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.414 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.438 {
            if features.saturation <= 112 {
            Intensity::Low
            } else {
            if features.red_luminance <= 14 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.luminance <= 28 {
            if features.blue_luminance <= 22 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.455 {
            Intensity::Low
            } else {
            if features.blue_difference <= 127 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.235 {
            if features.red_luminance <= 24 {
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
            }
            }
            }
            } else {
            if features.red_luminance <= 29 {
            if features.value <= 42 {
            if features.red_chromaticity <= 0.293 {
            if features.green_chromaticity <= 0.456 {
            if features.red_luminance <= 25 {
            if features.luminance <= 30 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.267 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_luminance <= 24 {
            if features.saturation <= 107 {
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
            if features.red_chromaticity <= 0.305 {
            if features.red_chromaticity <= 0.303 {
            if features.blue_difference <= 121 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_luminance <= 22 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_chromaticity <= 0.333 {
            if features.blue_chromaticity <= 0.239 {
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
            }
            }
            } else {
            if features.red_chromaticity <= 0.229 {
            if features.blue_luminance <= 35 {
            if features.green_chromaticity <= 0.450 {
            Intensity::High
            } else {
            if features.blue_difference <= 125 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.455 {
            if features.red_luminance <= 28 {
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
            if features.green_chromaticity <= 0.456 {
            if features.blue_chromaticity <= 0.128 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.207 {
            if features.blue_chromaticity <= 0.117 {
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
            if features.red_difference <= 122 {
            if features.red_chromaticity <= 0.319 {
            if features.green_chromaticity <= 0.456 {
            if features.green_chromaticity <= 0.441 {
            if features.luminance <= 42 {
            Intensity::Low
            } else {
            if features.red_luminance <= 33 {
            if features.blue_luminance <= 37 {
            Intensity::Low
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
            if features.blue_chromaticity <= 0.344 {
            if features.saturation <= 134 {
            if features.luminance <= 36 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.204 {
            if features.blue_chromaticity <= 0.346 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.green_luminance <= 48 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.value <= 48 {
            if features.red_chromaticity <= 0.233 {
            if features.luminance <= 37 {
            Intensity::Low
            } else {
            if features.luminance <= 38 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_luminance <= 31 {
            if features.green_chromaticity <= 0.457 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_luminance <= 29 {
            if features.red_chromaticity <= 0.214 {
            if features.value <= 50 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_luminance <= 27 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.luminance <= 41 {
            Intensity::High
            } else {
            if features.green_luminance <= 51 {
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
            if features.red_chromaticity <= 0.346 {
            if features.green_chromaticity <= 0.446 {
            if features.hue <= 44 {
            Intensity::Low
            } else {
            if features.saturation <= 115 {
            if features.saturation <= 114 {
            if features.blue_chromaticity <= 0.246 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.327 {
            if features.blue_luminance <= 23 {
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
            }
            }
            } else {
            if features.green_chromaticity <= 0.447 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.456 {
            if features.blue_chromaticity <= 0.212 {
            if features.blue_chromaticity <= 0.210 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.455 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.saturation <= 145 {
            if features.red_chromaticity <= 0.338 {
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
            if features.green_chromaticity <= 0.438 {
            if features.green_luminance <= 49 {
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
            if features.green_chromaticity <= 0.452 {
            if features.red_luminance <= 33 {
            if features.intensity <= 40 {
            if features.hue <= 65 {
            if features.hue <= 63 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_difference <= 123 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.saturation <= 123 {
            if features.green_luminance <= 60 {
            if features.blue_chromaticity <= 0.315 {
            Intensity::Low
            } else {
            if features.blue_luminance <= 42 {
            if features.saturation <= 118 {
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
            if features.red_luminance <= 22 {
            if features.value <= 56 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.172 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.saturation <= 115 {
            if features.green_chromaticity <= 0.438 {
            if features.red_chromaticity <= 0.274 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.443 {
            if features.blue_chromaticity <= 0.251 {
            Intensity::High
            } else {
            if features.saturation <= 94 {
            Intensity::Low
            } else {
            if features.saturation <= 99 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.298 {
            if features.green_chromaticity <= 0.444 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.280 {
            Intensity::Low
            } else {
            Intensity::Low
            }
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
            if features.blue_luminance <= 45 {
            if features.blue_chromaticity <= 0.208 {
            if features.blue_chromaticity <= 0.206 {
            if features.intensity <= 40 {
            Intensity::Low
            } else {
            if features.blue_chromaticity <= 0.203 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_difference <= 114 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_chromaticity <= 0.442 {
            if features.red_luminance <= 46 {
            if features.red_chromaticity <= 0.324 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.442 {
            Intensity::High
            } else {
            if features.red_luminance <= 45 {
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
            }
            }
            } else {
            if features.blue_chromaticity <= 0.331 {
            if features.green_luminance <= 57 {
            if features.hue <= 73 {
            if features.green_chromaticity <= 0.461 {
            if features.red_difference <= 117 {
            if features.saturation <= 113 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.230 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_chromaticity <= 0.273 {
            if features.green_chromaticity <= 0.459 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.value <= 56 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.hue <= 56 {
            if features.saturation <= 127 {
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
            if features.saturation <= 106 {
            if features.red_chromaticity <= 0.272 {
            if features.value <= 58 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.269 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.454 {
            if features.red_chromaticity <= 0.242 {
            Intensity::High
            } else {
            if features.red_difference <= 120 {
            if features.red_difference <= 116 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.intensity <= 43 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.hue <= 63 {
            if features.blue_difference <= 119 {
            if features.red_chromaticity <= 0.341 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            } else {
            if features.blue_luminance <= 39 {
            if features.luminance <= 50 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.luminance <= 51 {
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
            if features.green_chromaticity <= 0.455 {
            Intensity::High
            } else {
            if features.value <= 65 {
            if features.luminance <= 47 {
            if features.green_luminance <= 57 {
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
            } else {
            if features.blue_difference <= 119 {
            if features.green_chromaticity <= 0.449 {
            if features.luminance <= 78 {
            if features.green_chromaticity <= 0.442 {
            if features.red_chromaticity <= 0.306 {
            if features.blue_difference <= 115 {
            if features.blue_chromaticity <= 0.256 {
            if features.blue_chromaticity <= 0.256 {
            if features.luminance <= 74 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.red_luminance <= 58 {
            if features.saturation <= 105 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_luminance <= 58 {
            if features.blue_luminance <= 52 {
            if features.red_luminance <= 55 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.saturation <= 92 {
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
            if features.value <= 63 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.441 {
            if features.red_difference <= 122 {
            if features.green_chromaticity <= 0.440 {
            Intensity::Low
            } else {
            Intensity::Low
            }
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
            }
            } else {
            if features.blue_luminance <= 43 {
            if features.value <= 77 {
            if features.green_chromaticity <= 0.442 {
            Intensity::High
            } else {
            if features.intensity <= 47 {
            if features.red_chromaticity <= 0.343 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_luminance <= 33 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.intensity <= 58 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_chromaticity <= 0.445 {
            if features.luminance <= 69 {
            if features.blue_chromaticity <= 0.272 {
            if features.blue_chromaticity <= 0.268 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.442 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_chromaticity <= 0.442 {
            Intensity::High
            } else {
            if features.red_luminance <= 59 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.252 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.446 {
            if features.red_chromaticity <= 0.301 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.292 {
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
            if features.blue_difference <= 114 {
            if features.green_chromaticity <= 0.445 {
            if features.red_chromaticity <= 0.270 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.287 {
            if features.saturation <= 98 {
            if features.saturation <= 97 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.value <= 125 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.saturation <= 102 {
            Intensity::High
            } else {
            if features.red_difference <= 115 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.red_difference <= 102 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.294 {
            if features.green_chromaticity <= 0.447 {
            if features.green_chromaticity <= 0.447 {
            if features.value <= 120 {
            if features.green_chromaticity <= 0.444 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.red_luminance <= 75 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_difference <= 116 {
            if features.blue_difference <= 115 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_difference <= 110 {
            Intensity::Low
            } else {
            if features.green_luminance <= 95 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.439 {
            if features.red_difference <= 111 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.443 {
            if features.green_chromaticity <= 0.442 {
            if features.red_chromaticity <= 0.265 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            } else {
            if features.intensity <= 72 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.300 {
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
            if features.red_chromaticity <= 0.251 {
            if features.red_chromaticity <= 0.251 {
            if features.green_chromaticity <= 0.454 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.459 {
            if features.red_chromaticity <= 0.238 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.455 {
            if features.green_luminance <= 124 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.saturation <= 116 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.461 {
            if features.red_luminance <= 50 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.green_luminance <= 121 {
            Intensity::High
            } else {
            if features.luminance <= 106 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.298 {
            if features.hue <= 66 {
            if features.blue_difference <= 115 {
            if features.green_luminance <= 127 {
            Intensity::Low
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
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.251 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.red_luminance <= 50 {
            if features.green_chromaticity <= 0.454 {
            if features.green_chromaticity <= 0.453 {
            if features.green_chromaticity <= 0.451 {
            if features.blue_luminance <= 31 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.314 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_luminance <= 47 {
            if features.hue <= 55 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.281 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.red_luminance <= 46 {
            if features.green_chromaticity <= 0.453 {
            if features.blue_chromaticity <= 0.270 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_luminance <= 43 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_difference <= 113 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.453 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.461 {
            if features.green_luminance <= 81 {
            if features.green_chromaticity <= 0.456 {
            if features.red_difference <= 116 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.value <= 68 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.saturation <= 104 {
            if features.intensity <= 60 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.269 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.hue <= 48 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.458 {
            if features.blue_chromaticity <= 0.274 {
            if features.red_chromaticity <= 0.285 {
            if features.intensity <= 70 {
            if features.saturation <= 100 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.luminance <= 81 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.saturation <= 119 {
            if features.blue_luminance <= 42 {
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
            }
            } else {
            if features.green_chromaticity <= 0.457 {
            if features.blue_luminance <= 52 {
            if features.saturation <= 102 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.291 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_difference <= 114 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.458 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.blue_luminance <= 58 {
            if features.red_chromaticity <= 0.292 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.245 {
            if features.hue <= 51 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.280 {
            if features.blue_chromaticity <= 0.279 {
            if features.saturation <= 105 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.blue_difference <= 114 {
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
            } else {
            if features.blue_chromaticity <= 0.312 {
            if features.green_chromaticity <= 0.443 {
            if features.red_chromaticity <= 0.251 {
            Intensity::High
            } else {
            if features.blue_luminance <= 56 {
            if features.blue_difference <= 120 {
            if features.green_chromaticity <= 0.439 {
            if features.red_chromaticity <= 0.275 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.440 {
            Intensity::High
            } else {
            if features.intensity <= 52 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.441 {
            if features.hue <= 65 {
            if features.intensity <= 48 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.253 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.442 {
            Intensity::High
            } else {
            if features.blue_luminance <= 48 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.300 {
            Intensity::High
            } else {
            if features.intensity <= 76 {
            if features.red_luminance <= 47 {
            Intensity::High
            } else {
            if features.red_difference <= 110 {
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
            if features.value <= 69 {
            if features.red_chromaticity <= 0.259 {
            if features.red_luminance <= 37 {
            if features.blue_difference <= 121 {
            Intensity::High
            } else {
            if features.intensity <= 49 {
            if features.red_difference <= 115 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_luminance <= 68 {
            if features.blue_chromaticity <= 0.295 {
            if features.red_difference <= 115 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.saturation <= 109 {
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
            if features.red_chromaticity <= 0.261 {
            Intensity::High
            } else {
            if features.intensity <= 50 {
            if features.blue_chromaticity <= 0.283 {
            if features.red_luminance <= 39 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.saturation <= 106 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_luminance <= 68 {
            if features.blue_difference <= 120 {
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
            if features.red_chromaticity <= 0.252 {
            if features.blue_difference <= 121 {
            if features.green_chromaticity <= 0.460 {
            if features.green_chromaticity <= 0.446 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.301 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            Intensity::High
            }
            } else {
            if features.luminance <= 60 {
            if features.blue_chromaticity <= 0.304 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.239 {
            Intensity::Low
            } else {
            Intensity::Low
            }
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
            if features.red_chromaticity <= 0.263 {
            if features.green_luminance <= 70 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.intensity <= 54 {
            if features.green_luminance <= 70 {
            Intensity::High
            } else {
            if features.red_difference <= 115 {
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
            if features.red_difference <= 102 {
            if features.red_chromaticity <= 0.231 {
            if features.blue_luminance <= 73 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.green_luminance <= 124 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.438 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.323 {
            if features.green_chromaticity <= 0.445 {
            if features.blue_chromaticity <= 0.322 {
            if features.value <= 68 {
            Intensity::Low
            } else {
            if features.blue_chromaticity <= 0.314 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.237 {
            if features.green_chromaticity <= 0.461 {
            if features.red_chromaticity <= 0.220 {
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
            } else {
            if features.blue_difference <= 122 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.363 {
            if features.green_chromaticity <= 0.441 {
            if features.blue_chromaticity <= 0.345 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.234 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_chromaticity <= 0.175 {
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
            if features.value <= 51 {
            if features.green_luminance <= 44 {
            if features.value <= 39 {
            if features.value <= 32 {
            if features.green_luminance <= 29 {
            if features.intensity <= 17 {
            if features.red_difference <= 122 {
            if features.blue_chromaticity <= 0.297 {
            if features.saturation <= 147 {
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
            if features.red_chromaticity <= 0.345 {
            if features.green_chromaticity <= 0.474 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.482 {
            if features.red_chromaticity <= 0.283 {
            if features.red_luminance <= 10 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.496 {
            Intensity::Low
            } else {
            if features.green_luminance <= 28 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.hue <= 42 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.492 {
            if features.green_chromaticity <= 0.484 {
            if features.luminance <= 25 {
            Intensity::Low
            } else {
            if features.red_luminance <= 21 {
            if features.green_chromaticity <= 0.470 {
            if features.green_chromaticity <= 0.463 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.hue <= 62 {
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
            if features.saturation <= 133 {
            if features.red_luminance <= 16 {
            if features.red_chromaticity <= 0.248 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.saturation <= 138 {
            if features.luminance <= 25 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.saturation <= 141 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.blue_luminance <= 14 {
            if features.value <= 30 {
            if features.luminance <= 24 {
            Intensity::Low
            } else {
            if features.red_luminance <= 24 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_difference <= 119 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.luminance <= 24 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.496 {
            if features.red_luminance <= 15 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.203 {
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
            if features.red_difference <= 122 {
            if features.green_chromaticity <= 0.479 {
            if features.red_chromaticity <= 0.186 {
            if features.luminance <= 29 {
            if features.blue_chromaticity <= 0.347 {
            if features.red_luminance <= 13 {
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
            } else {
            Intensity::High
            }
            } else {
            if features.saturation <= 138 {
            if features.red_luminance <= 21 {
            if features.green_chromaticity <= 0.470 {
            if features.blue_luminance <= 23 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.475 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_luminance <= 37 {
            if features.red_chromaticity <= 0.287 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.value <= 38 {
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
            if features.red_luminance <= 12 {
            if features.green_chromaticity <= 0.486 {
            if features.red_luminance <= 7 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.blue_luminance <= 22 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.saturation <= 166 {
            if features.green_chromaticity <= 0.494 {
            if features.blue_difference <= 121 {
            if features.saturation <= 136 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.481 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_luminance <= 13 {
            Intensity::Low
            } else {
            if features.intensity <= 22 {
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
            if features.blue_chromaticity <= 0.237 {
            if features.green_chromaticity <= 0.493 {
            if features.red_chromaticity <= 0.291 {
            Intensity::Low
            } else {
            if features.value <= 33 {
            if features.green_chromaticity <= 0.475 {
            if features.hue <= 38 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.294 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.blue_luminance <= 14 {
            if features.intensity <= 25 {
            if features.blue_luminance <= 13 {
            Intensity::Low
            } else {
            if features.value <= 35 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.497 {
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
            if features.blue_chromaticity <= 0.239 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.478 {
            if features.green_chromaticity <= 0.472 {
            if features.red_chromaticity <= 0.305 {
            if features.red_chromaticity <= 0.295 {
            if features.blue_luminance <= 29 {
            if features.red_chromaticity <= 0.277 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.279 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_luminance <= 30 {
            if features.blue_chromaticity <= 0.323 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.469 {
            if features.value <= 42 {
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
            if features.saturation <= 147 {
            if features.blue_luminance <= 25 {
            if features.green_chromaticity <= 0.473 {
            if features.luminance <= 35 {
            if features.red_difference <= 121 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.476 {
            if features.saturation <= 127 {
            if features.green_chromaticity <= 0.475 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.red_luminance <= 22 {
            Intensity::High
            } else {
            if features.saturation <= 127 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.red_luminance <= 21 {
            if features.blue_difference <= 125 {
            if features.red_luminance <= 20 {
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
            Intensity::Low
            }
            }
            } else {
            if features.red_chromaticity <= 0.308 {
            if features.green_chromaticity <= 0.494 {
            if features.red_chromaticity <= 0.305 {
            if features.saturation <= 121 {
            if features.green_chromaticity <= 0.483 {
            if features.value <= 42 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.485 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.478 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.483 {
            if features.green_chromaticity <= 0.480 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_luminance <= 13 {
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
            } else {
            if features.red_luminance <= 19 {
            if features.blue_chromaticity <= 0.326 {
            if features.green_luminance <= 43 {
            if features.luminance <= 31 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.176 {
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
            if features.blue_luminance <= 22 {
            if features.value <= 42 {
            if features.luminance <= 32 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.hue <= 52 {
            Intensity::Low
            } else {
            if features.hue <= 57 {
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
            } else {
            if features.green_chromaticity <= 0.480 {
            Intensity::High
            } else {
            if features.saturation <= 173 {
            if features.blue_difference <= 116 {
            if features.hue <= 45 {
            if features.green_chromaticity <= 0.486 {
            if features.green_luminance <= 42 {
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
            if features.intensity <= 28 {
            if features.green_chromaticity <= 0.497 {
            if features.green_chromaticity <= 0.491 {
            Intensity::Low
            } else {
            Intensity::Low
            }
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
            } else {
            if features.green_chromaticity <= 0.480 {
            if features.red_chromaticity <= 0.306 {
            if features.luminance <= 39 {
            if features.green_chromaticity <= 0.474 {
            if features.saturation <= 113 {
            if features.saturation <= 112 {
            if features.blue_chromaticity <= 0.265 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.intensity <= 34 {
            if features.saturation <= 120 {
            if features.green_chromaticity <= 0.470 {
            if features.intensity <= 32 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.465 {
            if features.luminance <= 36 {
            Intensity::Low
            } else {
            Intensity::High
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
            if features.blue_luminance <= 35 {
            Intensity::High
            } else {
            if features.red_luminance <= 18 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.saturation <= 131 {
            if features.red_chromaticity <= 0.236 {
            Intensity::High
            } else {
            if features.saturation <= 118 {
            Intensity::High
            } else {
            if features.luminance <= 37 {
            if features.saturation <= 126 {
            Intensity::Low
            } else {
            Intensity::Low
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
            } else {
            if features.value <= 47 {
            Intensity::High
            } else {
            if features.blue_luminance <= 31 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.475 {
            Intensity::High
            } else {
            if features.red_luminance <= 16 {
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
            if features.red_chromaticity <= 0.263 {
            if features.green_chromaticity <= 0.463 {
            if features.blue_chromaticity <= 0.313 {
            Intensity::Low
            } else {
            if features.red_luminance <= 21 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_chromaticity <= 0.465 {
            Intensity::Low
            } else {
            if features.blue_chromaticity <= 0.293 {
            if features.green_chromaticity <= 0.474 {
            if features.red_luminance <= 25 {
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
            if features.red_chromaticity <= 0.231 {
            if features.green_luminance <= 50 {
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
            if features.blue_chromaticity <= 0.247 {
            if features.red_chromaticity <= 0.301 {
            if features.value <= 50 {
            if features.value <= 48 {
            if features.saturation <= 133 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.239 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
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
            if features.intensity <= 35 {
            if features.green_luminance <= 48 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.hue <= 60 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            }
            }
            } else {
            if features.saturation <= 172 {
            if features.green_luminance <= 49 {
            if features.luminance <= 38 {
            if features.blue_difference <= 116 {
            Intensity::High
            } else {
            if features.red_luminance <= 30 {
            if features.red_chromaticity <= 0.311 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.333 {
            if features.green_chromaticity <= 0.474 {
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
            if features.blue_chromaticity <= 0.210 {
            if features.blue_chromaticity <= 0.177 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.340 {
            if features.blue_difference <= 116 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.186 {
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
            } else {
            if features.green_chromaticity <= 0.472 {
            if features.hue <= 46 {
            if features.luminance <= 43 {
            if features.green_chromaticity <= 0.468 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.465 {
            Intensity::Low
            } else {
            if features.saturation <= 141 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.red_difference <= 122 {
            if features.red_luminance <= 34 {
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
            if features.red_chromaticity <= 0.194 {
            if features.red_chromaticity <= 0.168 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.179 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.490 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.498 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.495 {
            if features.blue_chromaticity <= 0.209 {
            if features.green_chromaticity <= 0.481 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.489 {
            if features.red_chromaticity <= 0.348 {
            if features.value <= 47 {
            if features.red_chromaticity <= 0.330 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.175 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.490 {
            Intensity::High
            } else {
            if features.red_difference <= 121 {
            if features.green_chromaticity <= 0.492 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.saturation <= 160 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            }
            }
            } else {
            if features.luminance <= 38 {
            if features.red_chromaticity <= 0.212 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.495 {
            if features.luminance <= 36 {
            if features.intensity <= 30 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.blue_luminance <= 28 {
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
            if features.blue_luminance <= 28 {
            if features.blue_chromaticity <= 0.268 {
            if features.red_luminance <= 30 {
            if features.green_luminance <= 48 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.488 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_luminance <= 27 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_luminance <= 30 {
            if features.blue_difference <= 121 {
            Intensity::Low
            } else {
            if features.green_luminance <= 49 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_luminance <= 22 {
            if features.red_chromaticity <= 0.210 {
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
            if features.value <= 46 {
            if features.red_difference <= 118 {
            if features.red_luminance <= 21 {
            if features.blue_chromaticity <= 0.283 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.267 {
            Intensity::Low
            } else {
            if features.red_luminance <= 27 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.283 {
            if features.luminance <= 40 {
            if features.saturation <= 129 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.495 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_luminance <= 17 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.310 {
            if features.blue_luminance <= 20 {
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
            if features.luminance <= 39 {
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
            if features.blue_chromaticity <= 0.298 {
            if features.value <= 58 {
            if features.green_chromaticity <= 0.487 {
            if features.green_chromaticity <= 0.474 {
            if features.green_chromaticity <= 0.466 {
            if features.blue_chromaticity <= 0.219 {
            if features.green_chromaticity <= 0.462 {
            Intensity::High
            } else {
            if features.value <= 56 {
            if features.green_chromaticity <= 0.465 {
            Intensity::Low
            } else {
            if features.hue <= 44 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_difference <= 114 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.saturation <= 128 {
            if features.red_chromaticity <= 0.260 {
            if features.intensity <= 38 {
            if features.saturation <= 115 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_luminance <= 30 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.235 {
            if features.blue_luminance <= 27 {
            Intensity::Low
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
            }
            } else {
            if features.saturation <= 131 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.saturation <= 150 {
            if features.blue_chromaticity <= 0.200 {
            Intensity::High
            } else {
            if features.saturation <= 112 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.243 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.302 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.473 {
            if features.blue_luminance <= 20 {
            Intensity::Low
            } else {
            if features.saturation <= 154 {
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
            if features.blue_luminance <= 24 {
            if features.green_chromaticity <= 0.477 {
            if features.luminance <= 46 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.green_luminance <= 55 {
            if features.red_luminance <= 32 {
            Intensity::High
            } else {
            if features.red_luminance <= 33 {
            if features.green_chromaticity <= 0.484 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.hue <= 49 {
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
            if features.red_luminance <= 31 {
            if features.blue_chromaticity <= 0.249 {
            if features.hue <= 56 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.269 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.272 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.value <= 54 {
            Intensity::High
            } else {
            if features.green_luminance <= 57 {
            if features.saturation <= 119 {
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
            if features.green_chromaticity <= 0.479 {
            if features.blue_difference <= 115 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.220 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.saturation <= 143 {
            if features.green_luminance <= 55 {
            Intensity::High
            } else {
            if features.luminance <= 47 {
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
            } else {
            if features.red_chromaticity <= 0.235 {
            if features.red_chromaticity <= 0.227 {
            if features.red_luminance <= 23 {
            if features.red_difference <= 114 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.489 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_chromaticity <= 0.231 {
            if features.red_difference <= 115 {
            if features.blue_chromaticity <= 0.281 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.green_luminance <= 54 {
            Intensity::High
            } else {
            if features.blue_luminance <= 30 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.257 {
            if features.blue_luminance <= 22 {
            Intensity::High
            } else {
            if features.saturation <= 145 {
            if features.blue_chromaticity <= 0.221 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.487 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.258 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.saturation <= 148 {
            if features.hue <= 49 {
            Intensity::Low
            } else {
            if features.luminance <= 44 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.198 {
            Intensity::High
            } else {
            if features.blue_luminance <= 23 {
            Intensity::High
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
            if features.blue_difference <= 117 {
            if features.green_chromaticity <= 0.483 {
            if features.green_luminance <= 68 {
            if features.green_chromaticity <= 0.464 {
            if features.blue_luminance <= 30 {
            Intensity::High
            } else {
            if features.hue <= 53 {
            if features.green_chromaticity <= 0.462 {
            Intensity::Low
            } else {
            if features.red_difference <= 118 {
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
            if features.red_chromaticity <= 0.320 {
            if features.blue_luminance <= 35 {
            if features.intensity <= 46 {
            if features.green_chromaticity <= 0.466 {
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
            } else {
            if features.intensity <= 47 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.saturation <= 140 {
            Intensity::High
            } else {
            if features.saturation <= 146 {
            if features.red_chromaticity <= 0.327 {
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
            if features.red_chromaticity <= 0.222 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.468 {
            if features.green_chromaticity <= 0.468 {
            if features.red_luminance <= 56 {
            if features.luminance <= 58 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.luminance <= 82 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.saturation <= 114 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.279 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.blue_difference <= 115 {
            if features.red_chromaticity <= 0.300 {
            if features.red_chromaticity <= 0.276 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_difference <= 118 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.265 {
            if features.red_luminance <= 39 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.value <= 81 {
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
            if features.blue_chromaticity <= 0.284 {
            if features.red_chromaticity <= 0.218 {
            if features.blue_difference <= 116 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.195 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.487 {
            if features.red_chromaticity <= 0.247 {
            if features.red_difference <= 108 {
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
            if features.green_chromaticity <= 0.495 {
            if features.saturation <= 144 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.495 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            }
            }
            } else {
            if features.blue_luminance <= 59 {
            Intensity::High
            } else {
            if features.red_difference <= 97 {
            if features.saturation <= 138 {
            Intensity::Low
            } else {
            if features.blue_luminance <= 72 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_luminance <= 64 {
            if features.red_chromaticity <= 0.219 {
            if features.blue_chromaticity <= 0.293 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.290 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.hue <= 67 {
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
            if features.blue_chromaticity <= 0.290 {
            if features.green_chromaticity <= 0.475 {
            if features.green_luminance <= 67 {
            if features.intensity <= 47 {
            if features.saturation <= 118 {
            if features.red_chromaticity <= 0.251 {
            Intensity::Low
            } else {
            if features.blue_chromaticity <= 0.270 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_luminance <= 31 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.473 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.blue_difference <= 119 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_chromaticity <= 0.274 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.283 {
            if features.red_chromaticity <= 0.246 {
            if features.red_difference <= 111 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.252 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.473 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.239 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.259 {
            if features.blue_chromaticity <= 0.253 {
            if features.green_chromaticity <= 0.498 {
            if features.green_chromaticity <= 0.478 {
            Intensity::High
            } else {
            if features.red_difference <= 117 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_difference <= 115 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_luminance <= 30 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_chromaticity <= 0.221 {
            Intensity::High
            } else {
            if features.hue <= 67 {
            if features.red_difference <= 111 {
            if features.blue_chromaticity <= 0.272 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.224 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_luminance <= 37 {
            if features.intensity <= 47 {
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
            if features.intensity <= 63 {
            if features.green_chromaticity <= 0.464 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.477 {
            if features.red_chromaticity <= 0.231 {
            if features.green_chromaticity <= 0.473 {
            Intensity::High
            } else {
            if features.blue_luminance <= 49 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.469 {
            if features.intensity <= 57 {
            Intensity::Low
            } else {
            Intensity::Low
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
            if features.green_chromaticity <= 0.485 {
            if features.saturation <= 135 {
            if features.saturation <= 134 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            } else {
            if features.red_difference <= 105 {
            Intensity::High
            } else {
            if features.green_luminance <= 86 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            }
            } else {
            if features.intensity <= 67 {
            if features.luminance <= 73 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.intensity <= 68 {
            if features.saturation <= 143 {
            if features.red_difference <= 106 {
            if features.green_luminance <= 99 {
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
            if features.green_luminance <= 97 {
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
            if features.blue_chromaticity <= 0.315 {
            if features.red_luminance <= 25 {
            if features.blue_difference <= 122 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.469 {
            Intensity::Low
            } else {
            if features.red_luminance <= 21 {
            if features.red_chromaticity <= 0.196 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.red_luminance <= 23 {
            if features.intensity <= 37 {
            if features.luminance <= 41 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.green_luminance <= 58 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.saturation <= 134 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.314 {
            if features.intensity <= 88 {
            if features.blue_chromaticity <= 0.311 {
            if features.green_chromaticity <= 0.472 {
            if features.green_chromaticity <= 0.470 {
            if features.saturation <= 131 {
            if features.blue_difference <= 119 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.302 {
            if features.intensity <= 52 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.471 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.value <= 62 {
            Intensity::High
            } else {
            if features.red_luminance <= 28 {
            if features.red_chromaticity <= 0.197 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.476 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.value <= 65 {
            if features.green_chromaticity <= 0.469 {
            if features.red_difference <= 114 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            } else {
            if features.intensity <= 56 {
            if features.blue_chromaticity <= 0.312 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.220 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_luminance <= 105 {
            if features.blue_chromaticity <= 0.311 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.value <= 123 {
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
            Intensity::High
            }
            }
            } else {
            if features.red_difference <= 111 {
            if features.blue_chromaticity <= 0.327 {
            if features.value <= 66 {
            if features.blue_chromaticity <= 0.325 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.194 {
            if features.green_luminance <= 63 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.326 {
            if features.red_difference <= 100 {
            Intensity::Low
            } else {
            if features.blue_luminance <= 61 {
            if features.blue_luminance <= 54 {
            if features.saturation <= 162 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.blue_luminance <= 56 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_chromaticity <= 0.317 {
            if features.saturation <= 135 {
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
            if features.green_chromaticity <= 0.468 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.blue_luminance <= 48 {
            if features.green_chromaticity <= 0.463 {
            Intensity::High
            } else {
            if features.saturation <= 146 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.328 {
            Intensity::High
            } else {
            if features.luminance <= 54 {
            if features.red_chromaticity <= 0.180 {
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
            if features.red_difference <= 109 {
            if features.green_chromaticity <= 0.499 {
            if features.saturation <= 157 {
            if features.red_chromaticity <= 0.181 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.481 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.intensity <= 40 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_difference <= 124 {
            Intensity::Low
            } else {
            if features.blue_difference <= 125 {
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
            if features.blue_chromaticity <= 0.337 {
            if features.blue_chromaticity <= 0.319 {
            if features.red_chromaticity <= 0.194 {
            if features.saturation <= 157 {
            Intensity::High
            } else {
            if features.red_difference <= 112 {
            Intensity::Low
            } else {
            if features.red_luminance <= 19 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.484 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.luminance <= 45 {
            if features.saturation <= 155 {
            Intensity::High
            } else {
            if features.green_luminance <= 53 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.472 {
            if features.red_luminance <= 27 {
            if features.saturation <= 137 {
            Intensity::Low
            } else {
            if features.saturation <= 147 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_chromaticity <= 0.213 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.320 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.hue <= 77 {
            Intensity::Low
            } else {
            if features.value <= 53 {
            Intensity::Low
            } else {
            if features.blue_chromaticity <= 0.350 {
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
            if features.value <= 33 {
            if features.red_difference <= 117 {
            if features.blue_difference <= 121 {
            if features.value <= 28 {
            if features.blue_difference <= 117 {
            Intensity::High
            } else {
            if features.red_difference <= 114 {
            if features.blue_difference <= 120 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.047 {
            if features.luminance <= 14 {
            if features.blue_difference <= 120 {
            if features.luminance <= 12 {
            Intensity::Low
            } else {
            if features.red_difference <= 115 {
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
            if features.blue_chromaticity <= 0.155 {
            if features.blue_chromaticity <= 0.090 {
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
            if features.saturation <= 213 {
            if features.value <= 30 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.149 {
            Intensity::High
            } else {
            if features.red_luminance <= 7 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.hue <= 65 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.164 {
            Intensity::Low
            } else {
            if features.hue <= 66 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            }
            }
            } else {
            if features.value <= 29 {
            if features.red_difference <= 116 {
            if features.green_chromaticity <= 0.591 {
            Intensity::Low
            } else {
            if features.luminance <= 13 {
            Intensity::Low
            } else {
            if features.red_difference <= 115 {
            Intensity::High
            } else {
            if features.green_luminance <= 26 {
            Intensity::High
            } else {
            if features.green_luminance <= 28 {
            if features.hue <= 70 {
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
            if features.blue_difference <= 123 {
            if features.green_chromaticity <= 0.704 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.value <= 20 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.blue_luminance <= 16 {
            if features.luminance <= 18 {
            if features.blue_luminance <= 8 {
            Intensity::High
            } else {
            if features.blue_luminance <= 11 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.hue <= 65 {
            if features.saturation <= 196 {
            Intensity::High
            } else {
            if features.blue_luminance <= 10 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.hue <= 74 {
            if features.luminance <= 20 {
            Intensity::High
            } else {
            if features.intensity <= 15 {
            if features.hue <= 67 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.258 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.saturation <= 192 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.326 {
            Intensity::Low
            } else {
            if features.red_difference <= 112 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            }
            }
            } else {
            if features.saturation <= 239 {
            if features.luminance <= 22 {
            if features.green_luminance <= 30 {
            if features.green_chromaticity <= 0.541 {
            if features.red_luminance <= 6 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.076 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_luminance <= 31 {
            Intensity::Low
            } else {
            if features.saturation <= 219 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.saturation <= 215 {
            if features.green_chromaticity <= 0.537 {
            Intensity::Low
            } else {
            if features.intensity <= 19 {
            if features.blue_chromaticity <= 0.328 {
            if features.hue <= 73 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.blue_luminance <= 20 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_difference <= 116 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.584 {
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
            if features.value <= 30 {
            if features.value <= 27 {
            if features.blue_difference <= 114 {
            Intensity::High
            } else {
            if features.blue_difference <= 117 {
            if features.red_difference <= 120 {
            if features.luminance <= 15 {
            Intensity::High
            } else {
            if features.red_luminance <= 4 {
            if features.value <= 26 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_difference <= 121 {
            if features.hue <= 53 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.green_luminance <= 25 {
            Intensity::Low
            } else {
            if features.hue <= 41 {
            Intensity::Low
            } else {
            if features.saturation <= 250 {
            Intensity::High
            } else {
            if features.blue_difference <= 116 {
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
            if features.green_luminance <= 25 {
            if features.blue_difference <= 120 {
            if features.red_difference <= 121 {
            if features.red_chromaticity <= 0.206 {
            if features.hue <= 59 {
            if features.green_chromaticity <= 0.772 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.value <= 19 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.754 {
            if features.saturation <= 212 {
            if features.saturation <= 210 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.hue <= 50 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.saturation <= 135 {
            Intensity::Low
            } else {
            if features.value <= 23 {
            if features.green_chromaticity <= 0.513 {
            if features.red_luminance <= 8 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.685 {
            if features.green_chromaticity <= 0.636 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.700 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.884 {
            if features.green_chromaticity <= 0.815 {
            if features.green_chromaticity <= 0.646 {
            if features.green_chromaticity <= 0.570 {
            if features.saturation <= 149 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.saturation <= 187 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_chromaticity <= 0.215 {
            if features.saturation <= 212 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.654 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.842 {
            Intensity::High
            } else {
            if features.value <= 26 {
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
            if features.green_chromaticity <= 0.728 {
            if features.green_chromaticity <= 0.586 {
            if features.green_chromaticity <= 0.548 {
            if features.red_chromaticity <= 0.368 {
            if features.red_chromaticity <= 0.157 {
            if features.blue_difference <= 125 {
            if features.luminance <= 21 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.saturation <= 154 {
            if features.green_chromaticity <= 0.522 {
            if features.red_difference <= 122 {
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
            Intensity::Low
            }
            }
            } else {
            if features.value <= 28 {
            Intensity::High
            } else {
            if features.saturation <= 199 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.saturation <= 192 {
            if features.luminance <= 22 {
            if features.hue <= 64 {
            if features.green_chromaticity <= 0.570 {
            if features.saturation <= 183 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_luminance <= 14 {
            if features.green_chromaticity <= 0.573 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_luminance <= 10 {
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
            if features.saturation <= 171 {
            Intensity::High
            } else {
            if features.saturation <= 189 {
            if features.blue_chromaticity <= 0.183 {
            Intensity::High
            } else {
            if features.green_luminance <= 29 {
            if features.luminance <= 20 {
            Intensity::Low
            } else {
            if features.blue_chromaticity <= 0.229 {
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
            if features.green_chromaticity <= 0.678 {
            if features.red_chromaticity <= 0.114 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.152 {
            if features.blue_chromaticity <= 0.138 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.644 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.saturation <= 242 {
            if features.green_chromaticity <= 0.682 {
            Intensity::High
            } else {
            if features.saturation <= 203 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_chromaticity <= 0.012 {
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
            if features.hue <= 61 {
            if features.blue_luminance <= 0 {
            if features.value <= 28 {
            Intensity::High
            } else {
            if features.blue_difference <= 116 {
            if features.blue_difference <= 115 {
            if features.hue <= 55 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            } else {
            if features.value <= 29 {
            if features.red_luminance <= 9 {
            if features.green_chromaticity <= 0.821 {
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
            Intensity::High
            }
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.630 {
            if features.red_luminance <= 12 {
            if features.green_chromaticity <= 0.578 {
            if features.green_luminance <= 31 {
            if features.green_chromaticity <= 0.569 {
            if features.red_difference <= 119 {
            if features.red_chromaticity <= 0.178 {
            if features.blue_luminance <= 17 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_difference <= 124 {
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
            if features.green_chromaticity <= 0.537 {
            if features.green_chromaticity <= 0.512 {
            Intensity::High
            } else {
            if features.red_luminance <= 11 {
            if features.saturation <= 172 {
            if features.luminance <= 24 {
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
            if features.green_chromaticity <= 0.551 {
            Intensity::High
            } else {
            if features.saturation <= 172 {
            if features.green_chromaticity <= 0.565 {
            if features.luminance <= 24 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.saturation <= 180 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            }
            }
            } else {
            if features.saturation <= 179 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.157 {
            Intensity::Low
            } else {
            if features.blue_chromaticity <= 0.223 {
            if features.value <= 32 {
            if features.hue <= 57 {
            if features.red_chromaticity <= 0.233 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.hue <= 62 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_difference <= 122 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.547 {
            if features.red_difference <= 122 {
            if features.blue_chromaticity <= 0.207 {
            if features.green_chromaticity <= 0.525 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.542 {
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
            Intensity::Low
            }
            }
            } else {
            if features.blue_chromaticity <= 0.219 {
            Intensity::Low
            } else {
            if features.intensity <= 20 {
            if features.value <= 32 {
            if features.red_chromaticity <= 0.230 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.252 {
            if features.blue_chromaticity <= 0.244 {
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
            if features.saturation <= 162 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.529 {
            Intensity::Low
            } else {
            if features.saturation <= 194 {
            if features.red_luminance <= 18 {
            Intensity::Low
            } else {
            if features.hue <= 46 {
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
            if features.hue <= 50 {
            if features.blue_luminance <= 0 {
            Intensity::High
            } else {
            if features.red_difference <= 122 {
            if features.green_luminance <= 31 {
            if features.green_chromaticity <= 0.586 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.312 {
            if features.red_luminance <= 16 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.intensity <= 19 {
            if features.blue_chromaticity <= 0.090 {
            if features.green_chromaticity <= 0.553 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.097 {
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
            if features.green_luminance <= 32 {
            if features.blue_difference <= 121 {
            if features.intensity <= 17 {
            if features.red_chromaticity <= 0.250 {
            Intensity::High
            } else {
            if features.hue <= 52 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.577 {
            if features.red_chromaticity <= 0.239 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.saturation <= 183 {
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
            if features.green_chromaticity <= 0.606 {
            if features.blue_difference <= 120 {
            if features.green_chromaticity <= 0.564 {
            Intensity::High
            } else {
            if features.red_difference <= 120 {
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
            }
            }
            } else {
            if features.red_chromaticity <= 0.209 {
            if features.red_chromaticity <= 0.177 {
            if features.blue_difference <= 116 {
            Intensity::High
            } else {
            if features.red_luminance <= 6 {
            Intensity::Low
            } else {
            if features.intensity <= 15 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.188 {
            if features.luminance <= 22 {
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
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.272 {
            if features.blue_chromaticity <= 0.094 {
            if features.green_chromaticity <= 0.696 {
            if features.red_chromaticity <= 0.269 {
            if features.green_chromaticity <= 0.681 {
            if features.green_luminance <= 32 {
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
            if features.green_chromaticity <= 0.765 {
            if features.blue_luminance <= 0 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.blue_difference <= 114 {
            Intensity::High
            } else {
            if features.hue <= 52 {
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
            } else {
            if features.blue_chromaticity <= 0.283 {
            if features.red_difference <= 119 {
            if features.blue_difference <= 118 {
            if features.blue_chromaticity <= 0.276 {
            if features.red_difference <= 117 {
            if features.blue_chromaticity <= 0.249 {
            if features.blue_chromaticity <= 0.234 {
            if features.red_chromaticity <= 0.253 {
            if features.green_luminance <= 35 {
            if features.red_chromaticity <= 0.128 {
            if features.blue_difference <= 114 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.831 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_chromaticity <= 0.643 {
            if features.red_difference <= 102 {
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
            } else {
            if features.red_chromaticity <= 0.253 {
            if features.blue_luminance <= 19 {
            if features.luminance <= 39 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            } else {
            if features.saturation <= 204 {
            if features.green_chromaticity <= 0.504 {
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
            if features.saturation <= 133 {
            if features.red_chromaticity <= 0.247 {
            if features.blue_chromaticity <= 0.245 {
            Intensity::High
            } else {
            if features.red_luminance <= 30 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.245 {
            if features.luminance <= 56 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.value <= 58 {
            if features.red_chromaticity <= 0.246 {
            if features.blue_chromaticity <= 0.244 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.intensity <= 36 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_chromaticity <= 0.206 {
            if features.blue_chromaticity <= 0.248 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.hue <= 59 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            }
            }
            } else {
            if features.hue <= 65 {
            if features.value <= 77 {
            if features.blue_chromaticity <= 0.268 {
            if features.red_luminance <= 27 {
            if features.red_chromaticity <= 0.187 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_luminance <= 30 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_luminance <= 73 {
            if features.blue_chromaticity <= 0.269 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.value <= 89 {
            if features.blue_chromaticity <= 0.251 {
            if features.red_luminance <= 33 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.231 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.hue <= 64 {
            if features.saturation <= 149 {
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
            if features.value <= 93 {
            if features.value <= 80 {
            if features.hue <= 67 {
            if features.green_chromaticity <= 0.613 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.155 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_chromaticity <= 0.186 {
            if features.luminance <= 55 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.186 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.luminance <= 75 {
            if features.green_chromaticity <= 0.516 {
            if features.blue_chromaticity <= 0.273 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.275 {
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
            if features.blue_chromaticity <= 0.193 {
            if features.red_difference <= 118 {
            if features.red_chromaticity <= 0.240 {
            if features.red_chromaticity <= 0.238 {
            if features.green_chromaticity <= 0.650 {
            if features.saturation <= 196 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            } else {
            if features.intensity <= 21 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.593 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.162 {
            if features.green_chromaticity <= 0.571 {
            if features.blue_chromaticity <= 0.158 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.255 {
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
            if features.blue_chromaticity <= 0.171 {
            if features.hue <= 54 {
            if features.blue_difference <= 115 {
            if features.blue_chromaticity <= 0.082 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            } else {
            if features.blue_luminance <= 9 {
            if features.green_luminance <= 34 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.value <= 52 {
            if features.red_chromaticity <= 0.260 {
            if features.red_luminance <= 18 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.185 {
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
            if features.blue_difference <= 117 {
            if features.saturation <= 158 {
            if features.green_chromaticity <= 0.513 {
            if features.red_chromaticity <= 0.274 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.212 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.205 {
            if features.blue_chromaticity <= 0.204 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_chromaticity <= 0.266 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.536 {
            if features.blue_chromaticity <= 0.198 {
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
            if features.red_chromaticity <= 0.263 {
            if features.green_luminance <= 44 {
            Intensity::High
            } else {
            if features.saturation <= 155 {
            if features.saturation <= 149 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.saturation <= 162 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.red_luminance <= 27 {
            if features.green_luminance <= 46 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.268 {
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
            } else {
            if features.red_chromaticity <= 0.190 {
            if features.red_luminance <= 28 {
            if features.red_luminance <= 18 {
            if features.blue_luminance <= 37 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.280 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.117 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.green_luminance <= 76 {
            if features.saturation <= 189 {
            if features.red_chromaticity <= 0.167 {
            Intensity::Low
            } else {
            Intensity::Low
            }
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
            } else {
            if features.blue_chromaticity <= 0.282 {
            if features.red_luminance <= 35 {
            if features.green_chromaticity <= 0.548 {
            if features.green_chromaticity <= 0.537 {
            if features.red_chromaticity <= 0.186 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.279 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.saturation <= 179 {
            if features.blue_luminance <= 50 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_chromaticity <= 0.182 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.luminance <= 70 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.red_chromaticity <= 0.207 {
            if features.blue_chromaticity <= 0.281 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.281 {
            if features.value <= 89 {
            Intensity::Low
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
            } else {
            if features.red_chromaticity <= 0.208 {
            Intensity::Low
            } else {
            if features.blue_chromaticity <= 0.282 {
            if features.blue_chromaticity <= 0.277 {
            if features.red_luminance <= 38 {
            if features.red_luminance <= 32 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.511 {
            if features.red_luminance <= 42 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_difference <= 103 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.283 {
            if features.luminance <= 73 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.212 {
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
            if features.red_difference <= 117 {
            if features.green_chromaticity <= 0.594 {
            if features.blue_chromaticity <= 0.262 {
            if features.intensity <= 19 {
            if features.green_luminance <= 34 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_difference <= 120 {
            if features.red_chromaticity <= 0.233 {
            if features.red_difference <= 109 {
            Intensity::High
            } else {
            if features.green_luminance <= 51 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.saturation <= 137 {
            if features.value <= 54 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.saturation <= 148 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.257 {
            if features.red_luminance <= 14 {
            if features.green_chromaticity <= 0.562 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_luminance <= 16 {
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
            if features.value <= 68 {
            if features.value <= 47 {
            if features.green_chromaticity <= 0.592 {
            if features.blue_chromaticity <= 0.280 {
            if features.saturation <= 171 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.saturation <= 181 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_chromaticity <= 0.134 {
            Intensity::High
            } else {
            if features.value <= 40 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.red_difference <= 108 {
            if features.red_difference <= 107 {
            if features.red_chromaticity <= 0.142 {
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
            if features.red_luminance <= 23 {
            if features.value <= 51 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.509 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.282 {
            if features.intensity <= 40 {
            if features.green_chromaticity <= 0.577 {
            if features.value <= 69 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            } else {
            if features.luminance <= 53 {
            if features.luminance <= 52 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_luminance <= 25 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
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
            } else {
            if features.blue_chromaticity <= 0.092 {
            if features.red_difference <= 111 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.red_difference <= 114 {
            if features.red_luminance <= 12 {
            if features.red_chromaticity <= 0.162 {
            if features.saturation <= 218 {
            if features.saturation <= 194 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.633 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.149 {
            if features.blue_chromaticity <= 0.259 {
            if features.red_chromaticity <= 0.141 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            } else {
            if features.saturation <= 182 {
            if features.red_difference <= 113 {
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
            if features.blue_chromaticity <= 0.228 {
            if features.red_chromaticity <= 0.100 {
            if features.red_chromaticity <= 0.089 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.730 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_luminance <= 11 {
            if features.blue_chromaticity <= 0.196 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.628 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.242 {
            if features.blue_chromaticity <= 0.239 {
            if features.red_difference <= 115 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.241 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_difference <= 115 {
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
            if features.blue_chromaticity <= 0.249 {
            if features.blue_difference <= 119 {
            if features.red_chromaticity <= 0.232 {
            if features.green_chromaticity <= 0.589 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.171 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.179 {
            if features.red_difference <= 118 {
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
            }
            }
            } else {
            if features.luminance <= 30 {
            if features.red_chromaticity <= 0.237 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.value <= 47 {
            if features.blue_chromaticity <= 0.228 {
            if features.green_chromaticity <= 0.526 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_difference <= 118 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.intensity <= 31 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.red_chromaticity <= 0.229 {
            if features.green_chromaticity <= 0.537 {
            if features.blue_chromaticity <= 0.243 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_luminance <= 13 {
            if features.saturation <= 171 {
            if features.red_luminance <= 13 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.182 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_chromaticity <= 0.567 {
            if features.blue_luminance <= 17 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.intensity <= 19 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.value <= 42 {
            if features.green_chromaticity <= 0.540 {
            if features.saturation <= 141 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.value <= 38 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_luminance <= 43 {
            if features.green_chromaticity <= 0.521 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.246 {
            if features.green_chromaticity <= 0.514 {
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
            if features.saturation <= 145 {
            if features.value <= 42 {
            if features.blue_chromaticity <= 0.255 {
            if features.green_chromaticity <= 0.509 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.231 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_chromaticity <= 0.227 {
            if features.hue <= 65 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.value <= 38 {
            Intensity::Low
            } else {
            if features.red_luminance <= 18 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.blue_luminance <= 21 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.257 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.green_luminance <= 34 {
            if features.hue <= 66 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_luminance <= 18 {
            if features.green_luminance <= 35 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.536 {
            if features.blue_luminance <= 17 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.259 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.277 {
            if features.red_luminance <= 14 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.526 {
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
            if features.red_difference <= 122 {
            if features.blue_difference <= 118 {
            if features.blue_difference <= 114 {
            if features.value <= 38 {
            if features.saturation <= 231 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.664 {
            if features.intensity <= 18 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.value <= 53 {
            Intensity::High
            } else {
            if features.saturation <= 168 {
            if features.green_luminance <= 56 {
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
            if features.red_difference <= 120 {
            if features.saturation <= 155 {
            if features.saturation <= 153 {
            if features.value <= 45 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.green_luminance <= 49 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.hue <= 50 {
            if features.blue_luminance <= 11 {
            if features.red_chromaticity <= 0.273 {
            if features.green_chromaticity <= 0.635 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.190 {
            if features.blue_chromaticity <= 0.176 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.luminance <= 41 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.intensity <= 17 {
            Intensity::Low
            } else {
            if features.intensity <= 20 {
            if features.blue_chromaticity <= 0.123 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_luminance <= 17 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.142 {
            if features.red_luminance <= 19 {
            if features.saturation <= 210 {
            if features.saturation <= 204 {
            if features.blue_chromaticity <= 0.132 {
            Intensity::High
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
            if features.blue_chromaticity <= 0.086 {
            if features.red_luminance <= 18 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.hue <= 47 {
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
            if features.blue_chromaticity <= 0.147 {
            if features.green_luminance <= 39 {
            if features.green_chromaticity <= 0.549 {
            Intensity::High
            } else {
            if features.green_luminance <= 37 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.hue <= 46 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_chromaticity <= 0.152 {
            Intensity::High
            } else {
            if features.saturation <= 165 {
            if features.green_chromaticity <= 0.505 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.554 {
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
            if features.green_chromaticity <= 0.541 {
            if features.green_luminance <= 39 {
            if features.blue_difference <= 121 {
            if features.saturation <= 142 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.528 {
            if features.red_luminance <= 19 {
            if features.hue <= 55 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_luminance <= 21 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.537 {
            if features.blue_difference <= 119 {
            Intensity::High
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
            }
            } else {
            if features.green_chromaticity <= 0.511 {
            if features.saturation <= 133 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.saturation <= 140 {
            if features.luminance <= 27 {
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
            if features.blue_chromaticity <= 0.212 {
            if features.red_chromaticity <= 0.277 {
            if features.green_chromaticity <= 0.529 {
            if features.green_chromaticity <= 0.522 {
            Intensity::Low
            } else {
            Intensity::High
            }
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
            } else {
            if features.intensity <= 27 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.blue_luminance <= 12 {
            if features.red_luminance <= 15 {
            if features.saturation <= 185 {
            if features.blue_difference <= 119 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.178 {
            if features.green_luminance <= 35 {
            if features.luminance <= 26 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.265 {
            if features.saturation <= 168 {
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
            }
            } else {
            if features.value <= 42 {
            if features.red_difference <= 123 {
            if features.green_chromaticity <= 0.507 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.522 {
            if features.saturation <= 179 {
            if features.luminance <= 33 {
            if features.saturation <= 165 {
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
            if features.luminance <= 30 {
            if features.value <= 35 {
            if features.luminance <= 26 {
            Intensity::Low
            } else {
            if features.intensity <= 20 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.hue <= 45 {
            Intensity::Low
            } else {
            if features.value <= 37 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.red_chromaticity <= 0.338 {
            if features.green_chromaticity <= 0.529 {
            if features.red_chromaticity <= 0.333 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.533 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_luminance <= 40 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            }
            }
            } else {
            if features.green_luminance <= 36 {
            if features.luminance <= 29 {
            if features.green_chromaticity <= 0.597 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.558 {
            if features.blue_difference <= 114 {
            Intensity::Low
            } else {
            if features.saturation <= 186 {
            if features.saturation <= 180 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_luminance <= 24 {
            if features.hue <= 43 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.121 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.red_chromaticity <= 0.359 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.blue_luminance <= 12 {
            Intensity::High
            } else {
            if features.intensity <= 30 {
            if features.hue <= 43 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.341 {
            if features.red_chromaticity <= 0.333 {
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
            }
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.566 {
            if features.blue_chromaticity <= 0.303 {
            if features.saturation <= 152 {
            if features.green_chromaticity <= 0.508 {
            if features.green_luminance <= 59 {
            if features.green_luminance <= 56 {
            if features.green_chromaticity <= 0.505 {
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
            if features.green_chromaticity <= 0.509 {
            if features.value <= 79 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.value <= 63 {
            if features.red_difference <= 117 {
            if features.hue <= 70 {
            if features.green_chromaticity <= 0.514 {
            if features.green_chromaticity <= 0.510 {
            if features.luminance <= 43 {
            if features.red_luminance <= 21 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.red_luminance <= 22 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            Intensity::High
            }
            } else {
            if features.saturation <= 160 {
            if features.blue_chromaticity <= 0.286 {
            if features.red_difference <= 113 {
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
            if features.red_luminance <= 20 {
            if features.green_chromaticity <= 0.539 {
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
            if features.green_chromaticity <= 0.564 {
            if features.green_chromaticity <= 0.521 {
            if features.saturation <= 165 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.red_luminance <= 10 {
            if features.red_difference <= 115 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.luminance <= 36 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.saturation <= 165 {
            if features.green_luminance <= 35 {
            if features.red_chromaticity <= 0.200 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.saturation <= 161 {
            if features.luminance <= 28 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.saturation <= 170 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.hue <= 69 {
            if features.red_difference <= 99 {
            if features.blue_chromaticity <= 0.289 {
            if features.green_luminance <= 97 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.204 {
            if features.value <= 110 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.luminance <= 89 {
            Intensity::High
            } else {
            if features.value <= 121 {
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
            if features.red_luminance <= 22 {
            Intensity::High
            } else {
            if features.red_luminance <= 25 {
            if features.value <= 65 {
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
            } else {
            if features.intensity <= 38 {
            Intensity::High
            } else {
            if features.hue <= 70 {
            if features.red_difference <= 103 {
            if features.blue_chromaticity <= 0.289 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.300 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.302 {
            if features.blue_luminance <= 41 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.302 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.297 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.302 {
            if features.red_luminance <= 26 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.saturation <= 191 {
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
            if features.blue_luminance <= 31 {
            if features.blue_chromaticity <= 0.345 {
            if features.value <= 37 {
            if features.green_chromaticity <= 0.550 {
            if features.saturation <= 194 {
            if features.red_chromaticity <= 0.155 {
            if features.red_chromaticity <= 0.131 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.511 {
            if features.intensity <= 23 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.518 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.luminance <= 25 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_luminance <= 7 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_chromaticity <= 0.554 {
            if features.blue_chromaticity <= 0.303 {
            if features.value <= 51 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.luminance <= 35 {
            if features.value <= 46 {
            if features.saturation <= 205 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.123 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_chromaticity <= 0.316 {
            if features.green_chromaticity <= 0.508 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.318 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.564 {
            if features.intensity <= 27 {
            if features.luminance <= 30 {
            if features.luminance <= 28 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.322 {
            if features.green_luminance <= 50 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.saturation <= 200 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.luminance <= 35 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.377 {
            if features.red_luminance <= 5 {
            if features.green_luminance <= 39 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.510 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.516 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.357 {
            Intensity::Low
            } else {
            if features.saturation <= 196 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            }
            }
            } else {
            if features.blue_luminance <= 30 {
            if features.saturation <= 208 {
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
            if features.red_difference <= 104 {
            if features.blue_chromaticity <= 0.312 {
            if features.blue_chromaticity <= 0.312 {
            if features.value <= 74 {
            if features.blue_chromaticity <= 0.309 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.311 {
            if features.green_chromaticity <= 0.556 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_chromaticity <= 0.189 {
            if features.red_luminance <= 32 {
            if features.red_chromaticity <= 0.179 {
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
            } else {
            if features.red_chromaticity <= 0.181 {
            if features.red_chromaticity <= 0.158 {
            if features.saturation <= 205 {
            if features.green_chromaticity <= 0.526 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.527 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.159 {
            Intensity::High
            } else {
            if features.red_difference <= 100 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.503 {
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
            if features.blue_chromaticity <= 0.327 {
            if features.green_luminance <= 54 {
            if features.luminance <= 39 {
            if features.hue <= 73 {
            if features.green_chromaticity <= 0.510 {
            Intensity::High
            } else {
            if features.saturation <= 177 {
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
            if features.blue_chromaticity <= 0.327 {
            if features.blue_chromaticity <= 0.307 {
            if features.blue_difference <= 121 {
            if features.green_chromaticity <= 0.517 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.184 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.saturation <= 162 {
            if features.blue_chromaticity <= 0.310 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.503 {
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
            if features.green_chromaticity <= 0.558 {
            if features.luminance <= 53 {
            if features.green_chromaticity <= 0.523 {
            if features.saturation <= 172 {
            if features.blue_luminance <= 42 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.328 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_luminance <= 42 {
            if features.blue_luminance <= 32 {
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
            Intensity::High
            }
            }
            }
            }
            }
            } else {
            if features.red_luminance <= 6 {
            if features.red_chromaticity <= 0.019 {
            if features.blue_luminance <= 25 {
            if features.red_chromaticity <= 0.018 {
            if features.red_chromaticity <= 0.015 {
            Intensity::High
            } else {
            if features.green_luminance <= 44 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.green_luminance <= 58 {
            if features.blue_difference <= 126 {
            if features.hue <= 75 {
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
            } else {
            if features.intensity <= 18 {
            if features.luminance <= 23 {
            Intensity::High
            } else {
            if features.red_luminance <= 3 {
            if features.red_chromaticity <= 0.045 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_difference <= 120 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.316 {
            if features.red_chromaticity <= 0.134 {
            if features.blue_chromaticity <= 0.297 {
            if features.blue_chromaticity <= 0.286 {
            if features.red_luminance <= 9 {
            Intensity::High
            } else {
            if features.red_luminance <= 10 {
            if features.value <= 54 {
            if features.saturation <= 203 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.red_difference <= 99 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.value <= 34 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.303 {
            if features.green_luminance <= 64 {
            if features.value <= 47 {
            if features.value <= 46 {
            if features.luminance <= 31 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.saturation <= 211 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.saturation <= 195 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.300 {
            Intensity::High
            } else {
            if features.red_luminance <= 16 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.red_chromaticity <= 0.104 {
            if features.green_chromaticity <= 0.587 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.604 {
            Intensity::High
            } else {
            if features.red_difference <= 108 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.green_luminance <= 38 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.289 {
            if features.red_difference <= 103 {
            Intensity::High
            } else {
            if features.green_luminance <= 67 {
            if features.red_difference <= 106 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.286 {
            if features.green_luminance <= 59 {
            Intensity::High
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
            if features.green_chromaticity <= 0.574 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.red_chromaticity <= 0.135 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.saturation <= 207 {
            Intensity::High
            } else {
            if features.hue <= 74 {
            if features.red_chromaticity <= 0.072 {
            Intensity::High
            } else {
            if features.red_luminance <= 10 {
            if features.green_chromaticity <= 0.588 {
            if features.red_chromaticity <= 0.095 {
            Intensity::High
            } else {
            if features.luminance <= 34 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.red_difference <= 103 {
            Intensity::Low
            } else {
            if features.intensity <= 36 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.red_luminance <= 9 {
            Intensity::High
            } else {
            if features.value <= 62 {
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
            }
}