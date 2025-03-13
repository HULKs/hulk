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
if features.blue_chromaticity <= 0.266 {
            if features.red_difference <= 121 {
            if features.green_chromaticity <= 0.431 {
            if features.green_chromaticity <= 0.421 {
            if features.saturation <= 91 {
            if features.blue_chromaticity <= 0.266 {
            if features.red_luminance <= 122 {
            if features.blue_luminance <= 97 {
            if features.red_difference <= 120 {
            if features.green_luminance <= 135 {
            if features.red_luminance <= 103 {
            if features.saturation <= 90 {
            Intensity::High
            } else {
            if features.red_difference <= 118 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_chromaticity <= 0.327 {
            if features.value <= 134 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.411 {
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
            if features.saturation <= 87 {
            Intensity::High
            } else {
            if features.intensity <= 118 {
            if features.blue_chromaticity <= 0.265 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.327 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.262 {
            Intensity::High
            } else {
            if features.saturation <= 89 {
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
            Intensity::High
            } else {
            Intensity::High
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
            if features.blue_chromaticity <= 0.264 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.324 {
            if features.green_chromaticity <= 0.410 {
            Intensity::High
            } else {
            if features.blue_difference <= 105 {
            if features.green_luminance <= 168 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.luminance <= 140 {
            if features.red_luminance <= 124 {
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
            if features.blue_difference <= 108 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_chromaticity <= 0.314 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.262 {
            if features.value <= 116 {
            if features.green_chromaticity <= 0.419 {
            if features.blue_luminance <= 71 {
            if features.green_chromaticity <= 0.419 {
            if features.green_luminance <= 112 {
            if features.blue_chromaticity <= 0.256 {
            Intensity::High
            } else {
            if features.intensity <= 88 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
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
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            } else {
            if features.red_luminance <= 101 {
            if features.blue_chromaticity <= 0.259 {
            if features.green_chromaticity <= 0.421 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_luminance <= 81 {
            if features.red_luminance <= 98 {
            if features.luminance <= 107 {
            Intensity::High
            } else {
            if features.green_luminance <= 119 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_luminance <= 99 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.415 {
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
            if features.value <= 127 {
            if features.red_difference <= 120 {
            if features.red_luminance <= 102 {
            Intensity::High
            } else {
            if features.saturation <= 92 {
            Intensity::Low
            } else {
            Intensity::Low
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
            if features.luminance <= 133 {
            if features.red_chromaticity <= 0.327 {
            if features.blue_difference <= 105 {
            if features.green_chromaticity <= 0.418 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.420 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_chromaticity <= 0.419 {
            if features.red_chromaticity <= 0.329 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.value <= 154 {
            if features.green_chromaticity <= 0.409 {
            Intensity::High
            } else {
            if features.saturation <= 104 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.luminance <= 140 {
            Intensity::High
            } else {
            if features.saturation <= 95 {
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
            if features.blue_difference <= 105 {
            if features.green_chromaticity <= 0.420 {
            if features.red_chromaticity <= 0.316 {
            if features.red_chromaticity <= 0.316 {
            if features.saturation <= 94 {
            if features.red_chromaticity <= 0.315 {
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
            } else {
            Intensity::High
            }
            } else {
            if features.red_difference <= 112 {
            Intensity::Low
            } else {
            if features.value <= 170 {
            if features.blue_difference <= 103 {
            Intensity::High
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
            }
            }
            } else {
            Intensity::High
            }
            } else {
            if features.green_luminance <= 82 {
            Intensity::Low
            } else {
            if features.luminance <= 104 {
            Intensity::High
            } else {
            if features.red_difference <= 118 {
            if features.intensity <= 122 {
            if features.green_luminance <= 121 {
            if features.red_difference <= 117 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.value <= 129 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_chromaticity <= 0.265 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.265 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.red_chromaticity <= 0.325 {
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
            if features.green_chromaticity <= 0.429 {
            if features.value <= 166 {
            if features.blue_difference <= 104 {
            if features.saturation <= 97 {
            if features.intensity <= 129 {
            if features.red_chromaticity <= 0.315 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.323 {
            if features.green_chromaticity <= 0.428 {
            if features.red_chromaticity <= 0.316 {
            if features.red_chromaticity <= 0.316 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            } else {
            if features.luminance <= 134 {
            if features.red_difference <= 113 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.hue <= 47 {
            if features.red_luminance <= 107 {
            if features.luminance <= 110 {
            Intensity::Low
            } else {
            if features.blue_chromaticity <= 0.239 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.330 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.423 {
            if features.green_chromaticity <= 0.422 {
            Intensity::High
            } else {
            if features.hue <= 45 {
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
            if features.green_chromaticity <= 0.422 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.422 {
            Intensity::Low
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
            }
            } else {
            if features.value <= 109 {
            if features.blue_chromaticity <= 0.256 {
            if features.green_chromaticity <= 0.426 {
            if features.blue_difference <= 111 {
            Intensity::High
            } else {
            if features.value <= 85 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_difference <= 118 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_chromaticity <= 0.307 {
            Intensity::High
            } else {
            if features.red_difference <= 117 {
            if features.blue_difference <= 113 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.311 {
            Intensity::High
            } else {
            if features.intensity <= 55 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.321 {
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
            if features.green_chromaticity <= 0.426 {
            if features.value <= 151 {
            if features.luminance <= 117 {
            if features.blue_luminance <= 77 {
            if features.saturation <= 108 {
            if features.value <= 130 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.261 {
            if features.red_difference <= 117 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.saturation <= 96 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.249 {
            Intensity::High
            } else {
            if features.green_luminance <= 136 {
            if features.green_luminance <= 134 {
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
            }
            }
            } else {
            if features.luminance <= 140 {
            if features.green_chromaticity <= 0.424 {
            if features.blue_chromaticity <= 0.265 {
            if features.red_chromaticity <= 0.318 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_luminance <= 115 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.luminance <= 136 {
            if features.blue_luminance <= 94 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_chromaticity <= 0.314 {
            if features.red_chromaticity <= 0.310 {
            if features.green_chromaticity <= 0.425 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.422 {
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
            if features.blue_luminance <= 97 {
            if features.green_luminance <= 152 {
            if features.red_chromaticity <= 0.308 {
            if features.blue_chromaticity <= 0.266 {
            if features.value <= 126 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.266 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_luminance <= 73 {
            if features.red_chromaticity <= 0.316 {
            Intensity::High
            } else {
            Intensity::Low
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
            if features.saturation <= 99 {
            if features.red_chromaticity <= 0.313 {
            if features.green_chromaticity <= 0.428 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.259 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.264 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.307 {
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
            if features.red_chromaticity <= 0.308 {
            Intensity::High
            } else {
            if features.saturation <= 95 {
            Intensity::High
            } else {
            if features.red_luminance <= 128 {
            if features.green_chromaticity <= 0.423 {
            Intensity::High
            } else {
            if features.value <= 172 {
            if features.red_chromaticity <= 0.310 {
            Intensity::High
            } else {
            if features.red_luminance <= 126 {
            if features.red_chromaticity <= 0.310 {
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
            } else {
            if features.green_chromaticity <= 0.424 {
            if features.red_difference <= 111 {
            Intensity::High
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
            }
            }
            } else {
            if features.value <= 168 {
            if features.luminance <= 133 {
            if features.saturation <= 106 {
            if features.green_luminance <= 145 {
            if features.blue_chromaticity <= 0.263 {
            if features.blue_chromaticity <= 0.261 {
            if features.red_chromaticity <= 0.311 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.257 {
            if features.red_chromaticity <= 0.316 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.258 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.value <= 130 {
            if features.luminance <= 106 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.307 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.429 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.red_chromaticity <= 0.303 {
            Intensity::Low
            } else {
            if features.intensity <= 104 {
            Intensity::High
            } else {
            if features.intensity <= 111 {
            if features.green_chromaticity <= 0.430 {
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
            if features.red_chromaticity <= 0.319 {
            if features.blue_luminance <= 88 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.254 {
            Intensity::High
            } else {
            if features.red_luminance <= 108 {
            if features.saturation <= 98 {
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
            }
            }
            } else {
            if features.intensity <= 117 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.blue_luminance <= 70 {
            if features.green_chromaticity <= 0.431 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.239 {
            if features.red_luminance <= 99 {
            if features.intensity <= 96 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.248 {
            if features.green_luminance <= 78 {
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
            Intensity::High
            }
            }
            } else {
            if features.green_chromaticity <= 0.429 {
            if features.luminance <= 138 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.blue_luminance <= 97 {
            Intensity::High
            } else {
            if features.intensity <= 124 {
            Intensity::Low
            } else {
            if features.blue_difference <= 102 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.430 {
            Intensity::High
            } else {
            if features.red_difference <= 107 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.311 {
            if features.green_chromaticity <= 0.431 {
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
            if features.value <= 28 {
            if features.red_difference <= 119 {
            if features.red_chromaticity <= 0.153 {
            Intensity::High
            } else {
            if features.saturation <= 200 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_difference <= 118 {
            Intensity::High
            } else {
            if features.hue <= 53 {
            Intensity::High
            } else {
            if features.blue_difference <= 121 {
            if features.saturation <= 235 {
            if features.red_chromaticity <= 0.212 {
            if features.red_chromaticity <= 0.159 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.129 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.172 {
            if features.green_luminance <= 27 {
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
            if features.value <= 26 {
            Intensity::Low
            } else {
            if features.blue_difference <= 122 {
            Intensity::Low
            } else {
            if features.luminance <= 20 {
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
            if features.green_chromaticity <= 0.456 {
            if features.saturation <= 106 {
            if features.red_difference <= 108 {
            if features.intensity <= 138 {
            if features.blue_difference <= 109 {
            if features.red_difference <= 106 {
            if features.blue_chromaticity <= 0.265 {
            if features.luminance <= 134 {
            if features.red_luminance <= 95 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.448 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
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
            } else {
            if features.saturation <= 98 {
            if features.red_difference <= 107 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.294 {
            if features.green_chromaticity <= 0.444 {
            if features.blue_chromaticity <= 0.266 {
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
            } else {
            if features.blue_chromaticity <= 0.259 {
            if features.red_chromaticity <= 0.298 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.saturation <= 99 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            }
            }
            } else {
            if features.saturation <= 105 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_difference <= 100 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.433 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.intensity <= 94 {
            if features.red_difference <= 120 {
            if features.red_chromaticity <= 0.310 {
            if features.blue_chromaticity <= 0.261 {
            if features.hue <= 52 {
            if features.red_chromaticity <= 0.305 {
            if features.red_chromaticity <= 0.305 {
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
            if features.red_chromaticity <= 0.302 {
            if features.green_chromaticity <= 0.445 {
            if features.blue_chromaticity <= 0.261 {
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
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.254 {
            if features.blue_chromaticity <= 0.252 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.432 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.313 {
            if features.blue_chromaticity <= 0.256 {
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
            if features.green_chromaticity <= 0.436 {
            if features.hue <= 51 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.saturation <= 104 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.448 {
            if features.red_chromaticity <= 0.299 {
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
            if features.red_luminance <= 94 {
            if features.red_chromaticity <= 0.294 {
            if features.red_chromaticity <= 0.291 {
            if features.blue_difference <= 109 {
            if features.blue_luminance <= 76 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.448 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.293 {
            if features.blue_chromaticity <= 0.261 {
            if features.green_chromaticity <= 0.446 {
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
            if features.red_luminance <= 85 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.256 {
            if features.red_luminance <= 93 {
            if features.hue <= 51 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.intensity <= 100 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_difference <= 110 {
            if features.blue_chromaticity <= 0.260 {
            if features.red_chromaticity <= 0.298 {
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
            if features.red_difference <= 111 {
            if features.green_chromaticity <= 0.443 {
            Intensity::High
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
            }
            }
            }
            } else {
            if features.red_luminance <= 106 {
            if features.green_chromaticity <= 0.437 {
            if features.green_chromaticity <= 0.436 {
            if features.value <= 144 {
            if features.red_chromaticity <= 0.300 {
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
            if features.green_chromaticity <= 0.436 {
            if features.blue_difference <= 106 {
            Intensity::High
            } else {
            Intensity::Low
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
            if features.green_chromaticity <= 0.444 {
            if features.red_luminance <= 103 {
            if features.blue_chromaticity <= 0.260 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.438 {
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
            if features.blue_difference <= 102 {
            if features.green_luminance <= 167 {
            if features.blue_chromaticity <= 0.254 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.311 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.305 {
            if features.blue_difference <= 105 {
            Intensity::High
            } else {
            if features.red_difference <= 109 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.255 {
            Intensity::High
            } else {
            if features.blue_luminance <= 94 {
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
            if features.luminance <= 100 {
            if features.green_chromaticity <= 0.439 {
            if features.red_chromaticity <= 0.328 {
            if features.red_difference <= 116 {
            Intensity::High
            } else {
            if features.red_luminance <= 78 {
            if features.blue_difference <= 111 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.253 {
            if features.red_chromaticity <= 0.314 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.244 {
            if features.luminance <= 96 {
            if features.blue_difference <= 108 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.432 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.434 {
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
            if features.blue_difference <= 116 {
            if features.blue_luminance <= 65 {
            if features.blue_chromaticity <= 0.250 {
            if features.red_chromaticity <= 0.297 {
            if features.red_luminance <= 68 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.296 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_luminance <= 61 {
            if features.value <= 110 {
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
            if features.intensity <= 72 {
            if features.luminance <= 82 {
            if features.blue_chromaticity <= 0.256 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.blue_difference <= 112 {
            Intensity::High
            } else {
            Intensity::Low
            }
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
            if features.red_chromaticity <= 0.283 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.297 {
            if features.intensity <= 85 {
            if features.blue_chromaticity <= 0.260 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            } else {
            if features.green_luminance <= 116 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.303 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.248 {
            if features.green_chromaticity <= 0.453 {
            if features.blue_chromaticity <= 0.247 {
            if features.green_chromaticity <= 0.446 {
            Intensity::High
            } else {
            if features.intensity <= 46 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_chromaticity <= 0.306 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.256 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.293 {
            if features.red_chromaticity <= 0.282 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.294 {
            if features.red_luminance <= 45 {
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
            if features.saturation <= 112 {
            if features.red_luminance <= 102 {
            if features.red_luminance <= 91 {
            if features.green_luminance <= 126 {
            if features.blue_chromaticity <= 0.255 {
            if features.red_luminance <= 84 {
            Intensity::High
            } else {
            if features.green_luminance <= 118 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_chromaticity <= 0.455 {
            if features.intensity <= 89 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_luminance <= 77 {
            if features.saturation <= 111 {
            if features.blue_luminance <= 75 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            } else {
            if features.saturation <= 108 {
            if features.value <= 142 {
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
            }
            } else {
            if features.intensity <= 105 {
            if features.blue_difference <= 105 {
            if features.green_chromaticity <= 0.434 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.248 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.luminance <= 110 {
            if features.red_chromaticity <= 0.316 {
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
            } else {
            if features.green_luminance <= 139 {
            if features.saturation <= 110 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.253 {
            if features.red_chromaticity <= 0.311 {
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
            }
            } else {
            if features.red_luminance <= 118 {
            if features.saturation <= 111 {
            if features.blue_chromaticity <= 0.255 {
            if features.green_chromaticity <= 0.432 {
            if features.green_chromaticity <= 0.432 {
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
            if features.green_luminance <= 155 {
            if features.value <= 153 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.intensity <= 109 {
            if features.intensity <= 107 {
            Intensity::High
            } else {
            if features.intensity <= 108 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_luminance <= 151 {
            Intensity::High
            } else {
            if features.luminance <= 132 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.251 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.252 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.441 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            }
            }
            } else {
            if features.red_luminance <= 93 {
            if features.green_chromaticity <= 0.456 {
            if features.blue_chromaticity <= 0.242 {
            if features.intensity <= 88 {
            if features.red_chromaticity <= 0.316 {
            if features.red_chromaticity <= 0.316 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            } else {
            if features.saturation <= 116 {
            if features.green_chromaticity <= 0.442 {
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
            }
            } else {
            if features.red_difference <= 110 {
            if features.green_chromaticity <= 0.451 {
            if features.blue_luminance <= 75 {
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
            if features.green_chromaticity <= 0.452 {
            if features.blue_chromaticity <= 0.242 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_luminance <= 80 {
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
            if features.blue_luminance <= 80 {
            if features.red_chromaticity <= 0.296 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.445 {
            if features.green_chromaticity <= 0.445 {
            if features.blue_luminance <= 73 {
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
            } else {
            if features.red_chromaticity <= 0.329 {
            if features.red_chromaticity <= 0.316 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.331 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.435 {
            if features.green_chromaticity <= 0.434 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.305 {
            if features.red_chromaticity <= 0.304 {
            if features.red_chromaticity <= 0.300 {
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
            }
            }
            } else {
            if features.saturation <= 125 {
            if features.blue_difference <= 121 {
            if features.saturation <= 118 {
            if features.red_luminance <= 80 {
            if features.red_luminance <= 67 {
            if features.green_chromaticity <= 0.457 {
            if features.green_chromaticity <= 0.457 {
            if features.blue_chromaticity <= 0.264 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.265 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.251 {
            if features.blue_difference <= 117 {
            if features.red_luminance <= 63 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.value <= 58 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_chromaticity <= 0.263 {
            if features.red_difference <= 114 {
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
            }
            }
            } else {
            if features.green_luminance <= 121 {
            if features.green_chromaticity <= 0.475 {
            if features.saturation <= 114 {
            if features.blue_chromaticity <= 0.254 {
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
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.481 {
            if features.blue_luminance <= 75 {
            if features.red_chromaticity <= 0.267 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.luminance <= 112 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_luminance <= 137 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.266 {
            if features.red_chromaticity <= 0.298 {
            if features.blue_difference <= 105 {
            if features.red_luminance <= 88 {
            if features.intensity <= 99 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.280 {
            if features.red_chromaticity <= 0.272 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_difference <= 105 {
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
            Intensity::High
            }
            }
            } else {
            if features.red_chromaticity <= 0.274 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.blue_luminance <= 25 {
            if features.green_chromaticity <= 0.466 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.492 {
            if features.saturation <= 120 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_chromaticity <= 0.312 {
            if features.red_luminance <= 62 {
            if features.blue_chromaticity <= 0.266 {
            if features.red_difference <= 118 {
            if features.green_chromaticity <= 0.496 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.saturation <= 121 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_chromaticity <= 0.252 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_luminance <= 96 {
            if features.green_chromaticity <= 0.460 {
            if features.blue_difference <= 110 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.494 {
            if features.value <= 122 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_difference <= 102 {
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
            if features.red_difference <= 120 {
            if features.red_chromaticity <= 0.263 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.saturation <= 114 {
            Intensity::Low
            } else {
            if features.blue_chromaticity <= 0.252 {
            Intensity::Low
            } else {
            if features.hue <= 60 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            }
            } else {
            if features.value <= 33 {
            if features.saturation <= 177 {
            if features.red_chromaticity <= 0.206 {
            Intensity::High
            } else {
            if features.red_difference <= 120 {
            if features.saturation <= 157 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.572 {
            if features.red_chromaticity <= 0.235 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.green_luminance <= 30 {
            if features.green_chromaticity <= 0.596 {
            Intensity::High
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
            }
            } else {
            if features.green_chromaticity <= 0.557 {
            if features.blue_luminance <= 13 {
            if features.blue_chromaticity <= 0.207 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.504 {
            Intensity::Low
            } else {
            if features.intensity <= 19 {
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
            if features.red_difference <= 117 {
            if features.blue_difference <= 110 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.836 {
            if features.red_luminance <= 9 {
            if features.red_chromaticity <= 0.212 {
            if features.green_luminance <= 30 {
            if features.intensity <= 14 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            } else {
            if features.blue_difference <= 114 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.785 {
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
            if features.blue_difference <= 116 {
            if features.red_chromaticity <= 0.104 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.blue_difference <= 117 {
            if features.luminance <= 19 {
            if features.value <= 29 {
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
            if features.red_difference <= 120 {
            if features.intensity <= 109 {
            if features.blue_luminance <= 53 {
            if features.blue_difference <= 119 {
            if features.saturation <= 127 {
            if features.red_difference <= 119 {
            if features.red_difference <= 115 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.233 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_luminance <= 47 {
            if features.green_chromaticity <= 0.485 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.543 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.red_difference <= 118 {
            if features.red_chromaticity <= 0.170 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.172 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_chromaticity <= 0.207 {
            if features.red_chromaticity <= 0.203 {
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
            }
            }
            } else {
            if features.saturation <= 138 {
            if features.value <= 127 {
            if features.blue_luminance <= 57 {
            if features.red_chromaticity <= 0.276 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_luminance <= 65 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.214 {
            if features.blue_chromaticity <= 0.214 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.green_luminance <= 147 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.blue_luminance <= 69 {
            if features.value <= 129 {
            if features.intensity <= 91 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.intensity <= 100 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_difference <= 105 {
            Intensity::High
            } else {
            if features.luminance <= 128 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            }
            }
            } else {
            if features.red_chromaticity <= 0.325 {
            if features.blue_chromaticity <= 0.239 {
            if features.green_chromaticity <= 0.464 {
            if features.red_chromaticity <= 0.306 {
            if features.blue_chromaticity <= 0.233 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.hue <= 47 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.240 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_chromaticity <= 0.326 {
            if features.intensity <= 110 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.325 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_luminance <= 154 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.blue_difference <= 118 {
            if features.blue_chromaticity <= 0.006 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.280 {
            if features.green_chromaticity <= 0.529 {
            if features.green_chromaticity <= 0.497 {
            if features.saturation <= 127 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.493 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_luminance <= 20 {
            if features.red_chromaticity <= 0.255 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.178 {
            if features.blue_chromaticity <= 0.171 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_chromaticity <= 0.288 {
            if features.blue_chromaticity <= 0.203 {
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
            }
            }
            }
            }
            }
            }
            }
            } else {
            if features.blue_difference <= 116 {
            if features.green_chromaticity <= 0.425 {
            if features.blue_difference <= 107 {
            if features.green_luminance <= 117 {
            if features.green_chromaticity <= 0.413 {
            Intensity::High
            } else {
            if features.blue_difference <= 106 {
            if features.red_luminance <= 95 {
            if features.red_luminance <= 91 {
            if features.value <= 104 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.222 {
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
            if features.value <= 151 {
            if features.green_chromaticity <= 0.385 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.391 {
            Intensity::High
            } else {
            if features.hue <= 40 {
            if features.green_luminance <= 165 {
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
            if features.green_chromaticity <= 0.395 {
            if features.hue <= 39 {
            if features.red_chromaticity <= 0.350 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.262 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.397 {
            Intensity::High
            } else {
            if features.luminance <= 111 {
            if features.green_chromaticity <= 0.420 {
            if features.blue_difference <= 115 {
            if features.red_difference <= 123 {
            if features.blue_luminance <= 52 {
            Intensity::High
            } else {
            if features.saturation <= 106 {
            if features.blue_chromaticity <= 0.257 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_difference <= 114 {
            if features.red_luminance <= 93 {
            if features.blue_luminance <= 46 {
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
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.420 {
            Intensity::High
            } else {
            if features.luminance <= 59 {
            Intensity::High
            } else {
            if features.blue_difference <= 110 {
            Intensity::Low
            } else {
            if features.blue_difference <= 112 {
            Intensity::High
            } else {
            if features.saturation <= 109 {
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
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.red_difference <= 125 {
            if features.blue_luminance <= 38 {
            if features.blue_difference <= 115 {
            if features.green_luminance <= 38 {
            if features.green_chromaticity <= 0.590 {
            if features.red_chromaticity <= 0.351 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.value <= 30 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.170 {
            if features.red_difference <= 122 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.170 {
            Intensity::Low
            } else {
            if features.value <= 70 {
            Intensity::High
            } else {
            if features.red_luminance <= 53 {
            if features.blue_difference <= 109 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.344 {
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
            if features.blue_chromaticity <= 0.087 {
            if features.intensity <= 16 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.value <= 62 {
            if features.green_chromaticity <= 0.534 {
            if features.red_chromaticity <= 0.323 {
            if features.green_chromaticity <= 0.497 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.saturation <= 116 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_chromaticity <= 0.328 {
            if features.red_luminance <= 18 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.saturation <= 202 {
            if features.green_chromaticity <= 0.539 {
            Intensity::Low
            } else {
            if features.blue_luminance <= 8 {
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
            if features.intensity <= 49 {
            Intensity::Low
            } else {
            if features.blue_luminance <= 36 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.435 {
            if features.saturation <= 111 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.blue_difference <= 106 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.346 {
            Intensity::High
            } else {
            if features.red_luminance <= 63 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.354 {
            if features.value <= 84 {
            if features.saturation <= 130 {
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
            }
            }
            } else {
            if features.blue_difference <= 112 {
            if features.hue <= 35 {
            Intensity::Low
            } else {
            Intensity::High
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
            if features.blue_difference <= 119 {
            if features.hue <= 46 {
            if features.green_chromaticity <= 0.575 {
            if features.red_difference <= 123 {
            if features.green_chromaticity <= 0.435 {
            if features.blue_chromaticity <= 0.244 {
            Intensity::Low
            } else {
            if features.value <= 69 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.saturation <= 144 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.330 {
            if features.saturation <= 203 {
            if features.green_chromaticity <= 0.500 {
            if features.saturation <= 158 {
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
            if features.green_chromaticity <= 0.472 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.418 {
            if features.green_chromaticity <= 0.407 {
            Intensity::Low
            } else {
            if features.value <= 52 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.412 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.green_luminance <= 60 {
            if features.green_luminance <= 43 {
            if features.luminance <= 26 {
            if features.green_chromaticity <= 0.528 {
            if features.hue <= 36 {
            Intensity::Low
            } else {
            if features.blue_chromaticity <= 0.156 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            Intensity::High
            }
            } else {
            if features.value <= 40 {
            if features.hue <= 41 {
            if features.red_chromaticity <= 0.365 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.intensity <= 28 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.326 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.value <= 45 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.361 {
            if features.red_chromaticity <= 0.327 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.242 {
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
            Intensity::High
            }
            }
            }
            } else {
            if features.green_luminance <= 26 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_chromaticity <= 0.429 {
            if features.red_chromaticity <= 0.320 {
            Intensity::High
            } else {
            if features.saturation <= 106 {
            if features.blue_chromaticity <= 0.258 {
            if features.hue <= 47 {
            Intensity::High
            } else {
            if features.blue_luminance <= 36 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_chromaticity <= 0.326 {
            if features.green_chromaticity <= 0.417 {
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
            if features.luminance <= 35 {
            if features.hue <= 47 {
            if features.green_chromaticity <= 0.663 {
            if features.green_chromaticity <= 0.491 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.green_luminance <= 28 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_chromaticity <= 0.309 {
            if features.green_chromaticity <= 0.539 {
            if features.red_chromaticity <= 0.298 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.302 {
            if features.luminance <= 26 {
            if features.hue <= 48 {
            Intensity::High
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
            } else {
            Intensity::High
            }
            }
            } else {
            if features.intensity <= 28 {
            if features.luminance <= 33 {
            if features.saturation <= 156 {
            if features.intensity <= 26 {
            Intensity::High
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
            if features.blue_difference <= 118 {
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
            if features.green_luminance <= 28 {
            if features.blue_difference <= 121 {
            if features.hue <= 52 {
            if features.red_chromaticity <= 0.334 {
            if features.hue <= 46 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_luminance <= 15 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.346 {
            if features.red_luminance <= 20 {
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
            if features.green_chromaticity <= 0.567 {
            Intensity::High
            } else {
            if features.luminance <= 18 {
            Intensity::Low
            } else {
            if features.luminance <= 19 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.green_luminance <= 22 {
            Intensity::Low
            } else {
            if features.hue <= 57 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.243 {
            Intensity::Low
            } else {
            if features.hue <= 59 {
            Intensity::High
            } else {
            if features.blue_luminance <= 11 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.257 {
            Intensity::Low
            } else {
            if features.saturation <= 115 {
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
            if features.hue <= 51 {
            if features.red_chromaticity <= 0.305 {
            if features.saturation <= 182 {
            if features.hue <= 50 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.luminance <= 43 {
            if features.green_chromaticity <= 0.446 {
            if features.red_chromaticity <= 0.315 {
            if features.red_difference <= 122 {
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
            Intensity::High
            }
            }
            } else {
            if features.red_chromaticity <= 0.302 {
            if features.saturation <= 119 {
            if features.red_chromaticity <= 0.299 {
            if features.red_chromaticity <= 0.298 {
            if features.saturation <= 101 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.443 {
            Intensity::Low
            } else {
            if features.intensity <= 21 {
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
            if features.red_chromaticity <= 0.301 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_chromaticity <= 0.246 {
            if features.red_chromaticity <= 0.299 {
            if features.saturation <= 137 {
            if features.value <= 33 {
            if features.green_luminance <= 29 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.287 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_chromaticity <= 0.212 {
            if features.green_luminance <= 30 {
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
            Intensity::High
            }
            }
            } else {
            if features.saturation <= 103 {
            if features.red_chromaticity <= 0.309 {
            if features.saturation <= 98 {
            Intensity::High
            } else {
            if features.red_luminance <= 29 {
            Intensity::Low
            } else {
            if features.saturation <= 100 {
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
            Intensity::High
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.442 {
            if features.value <= 48 {
            if features.red_chromaticity <= 0.317 {
            if features.green_chromaticity <= 0.426 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.intensity <= 39 {
            Intensity::High
            } else {
            if features.saturation <= 94 {
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
            if features.red_chromaticity <= 0.333 {
            if features.green_chromaticity <= 0.446 {
            if features.red_chromaticity <= 0.314 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.309 {
            if features.red_chromaticity <= 0.306 {
            Intensity::Low
            } else {
            if features.blue_luminance <= 14 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.242 {
            if features.blue_difference <= 121 {
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
            if features.green_chromaticity <= 0.444 {
            if features.blue_chromaticity <= 0.218 {
            Intensity::Low
            } else {
            Intensity::Low
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
            }
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.282 {
            if features.red_difference <= 111 {
            if features.saturation <= 93 {
            if features.green_chromaticity <= 0.424 {
            if features.green_chromaticity <= 0.420 {
            if features.intensity <= 138 {
            if features.blue_difference <= 108 {
            if features.value <= 172 {
            if features.blue_chromaticity <= 0.280 {
            if features.red_difference <= 110 {
            if features.red_chromaticity <= 0.305 {
            if features.blue_chromaticity <= 0.278 {
            if features.red_luminance <= 120 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.304 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.413 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.419 {
            if features.green_chromaticity <= 0.415 {
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
            } else {
            if features.blue_chromaticity <= 0.281 {
            if features.green_chromaticity <= 0.412 {
            if features.green_chromaticity <= 0.411 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.281 {
            if features.green_chromaticity <= 0.413 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.intensity <= 133 {
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
            if features.green_luminance <= 173 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_chromaticity <= 0.417 {
            if features.red_difference <= 110 {
            if features.blue_chromaticity <= 0.282 {
            if features.value <= 162 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.415 {
            Intensity::Low
            } else {
            if features.saturation <= 82 {
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
            if features.saturation <= 82 {
            if features.green_chromaticity <= 0.412 {
            Intensity::Low
            } else {
            if features.blue_chromaticity <= 0.281 {
            if features.intensity <= 132 {
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
            if features.green_chromaticity <= 0.415 {
            Intensity::Low
            } else {
            if features.blue_chromaticity <= 0.279 {
            if features.red_luminance <= 117 {
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
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.417 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.304 {
            if features.red_luminance <= 112 {
            if features.luminance <= 127 {
            Intensity::Low
            } else {
            if features.hue <= 54 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.418 {
            if features.blue_chromaticity <= 0.281 {
            Intensity::High
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
            if features.value <= 158 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            }
            }
            } else {
            if features.green_luminance <= 192 {
            if features.intensity <= 141 {
            if features.green_chromaticity <= 0.419 {
            if features.red_chromaticity <= 0.307 {
            Intensity::High
            } else {
            if features.value <= 173 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.310 {
            if features.red_chromaticity <= 0.310 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.red_luminance <= 131 {
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
            if features.blue_difference <= 103 {
            if features.red_difference <= 110 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.red_luminance <= 132 {
            if features.blue_luminance <= 119 {
            if features.saturation <= 82 {
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
            } else {
            if features.red_difference <= 108 {
            if features.red_chromaticity <= 0.305 {
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
            Intensity::High
            }
            }
            } else {
            if features.luminance <= 136 {
            if features.blue_chromaticity <= 0.280 {
            if features.red_luminance <= 105 {
            if features.red_difference <= 110 {
            Intensity::High
            } else {
            if features.green_luminance <= 142 {
            if features.red_chromaticity <= 0.297 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_luminance <= 95 {
            if features.hue <= 55 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.luminance <= 128 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.red_luminance <= 108 {
            if features.red_chromaticity <= 0.300 {
            if features.saturation <= 86 {
            if features.red_luminance <= 107 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.saturation <= 89 {
            if features.red_difference <= 110 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.421 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.421 {
            if features.red_chromaticity <= 0.303 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.value <= 153 {
            if features.saturation <= 89 {
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
            if features.red_difference <= 110 {
            if features.luminance <= 135 {
            if features.value <= 146 {
            Intensity::Low
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
            Intensity::Low
            }
            }
            } else {
            if features.blue_luminance <= 113 {
            if features.green_chromaticity <= 0.422 {
            if features.red_luminance <= 120 {
            if features.blue_luminance <= 102 {
            if features.luminance <= 138 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.420 {
            Intensity::Low
            } else {
            if features.blue_chromaticity <= 0.281 {
            if features.red_luminance <= 117 {
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
            if features.red_chromaticity <= 0.311 {
            if features.green_chromaticity <= 0.421 {
            Intensity::High
            } else {
            Intensity::High
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
            if features.value <= 160 {
            if features.blue_chromaticity <= 0.275 {
            if features.red_chromaticity <= 0.306 {
            if features.value <= 158 {
            if features.blue_chromaticity <= 0.273 {
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
            if features.red_chromaticity <= 0.298 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.302 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.saturation <= 85 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.423 {
            Intensity::High
            } else {
            if features.blue_difference <= 106 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.279 {
            if features.blue_luminance <= 114 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_luminance <= 116 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            }
            }
            } else {
            if features.value <= 138 {
            if features.blue_difference <= 112 {
            if features.green_chromaticity <= 0.428 {
            if features.red_luminance <= 94 {
            if features.green_chromaticity <= 0.425 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.293 {
            if features.intensity <= 104 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.281 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.hue <= 56 {
            if features.blue_chromaticity <= 0.278 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_chromaticity <= 0.283 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.284 {
            if features.blue_luminance <= 83 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.436 {
            if features.value <= 132 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.277 {
            if features.green_chromaticity <= 0.428 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.276 {
            if features.value <= 131 {
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
            } else {
            if features.red_chromaticity <= 0.290 {
            if features.red_chromaticity <= 0.288 {
            if features.red_chromaticity <= 0.288 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.431 {
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
            if features.blue_chromaticity <= 0.280 {
            if features.green_chromaticity <= 0.436 {
            if features.luminance <= 104 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.278 {
            Intensity::High
            } else {
            if features.saturation <= 92 {
            Intensity::High
            } else {
            if features.red_luminance <= 77 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.value <= 130 {
            if features.luminance <= 110 {
            if features.luminance <= 109 {
            if features.blue_chromaticity <= 0.281 {
            if features.red_chromaticity <= 0.288 {
            if features.intensity <= 90 {
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
            } else {
            if features.red_chromaticity <= 0.291 {
            if features.blue_chromaticity <= 0.281 {
            if features.green_chromaticity <= 0.430 {
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
            if features.saturation <= 86 {
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
            if features.saturation <= 88 {
            if features.green_chromaticity <= 0.430 {
            if features.blue_chromaticity <= 0.277 {
            Intensity::High
            } else {
            if features.value <= 151 {
            if features.blue_chromaticity <= 0.281 {
            if features.intensity <= 116 {
            if features.green_chromaticity <= 0.425 {
            if features.red_luminance <= 101 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_difference <= 110 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_chromaticity <= 0.293 {
            Intensity::Low
            } else {
            if features.luminance <= 130 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.282 {
            if features.blue_chromaticity <= 0.281 {
            if features.blue_luminance <= 94 {
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
            if features.blue_chromaticity <= 0.280 {
            if features.green_chromaticity <= 0.427 {
            if features.blue_chromaticity <= 0.280 {
            if features.luminance <= 147 {
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
            if features.blue_chromaticity <= 0.282 {
            if features.blue_chromaticity <= 0.281 {
            if features.green_luminance <= 168 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.292 {
            if features.blue_difference <= 109 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.425 {
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
            if features.blue_chromaticity <= 0.281 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_luminance <= 151 {
            if features.green_chromaticity <= 0.430 {
            if features.blue_chromaticity <= 0.271 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.274 {
            if features.blue_difference <= 108 {
            if features.blue_chromaticity <= 0.272 {
            if features.hue <= 54 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.425 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_chromaticity <= 0.301 {
            if features.red_chromaticity <= 0.299 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.luminance <= 128 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.275 {
            if features.red_luminance <= 106 {
            if features.luminance <= 126 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            } else {
            if features.value <= 142 {
            if features.green_chromaticity <= 0.428 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.luminance <= 125 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.279 {
            if features.blue_chromaticity <= 0.278 {
            if features.blue_chromaticity <= 0.277 {
            if features.red_chromaticity <= 0.295 {
            if features.red_chromaticity <= 0.293 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.value <= 150 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_luminance <= 145 {
            Intensity::Low
            } else {
            if features.blue_chromaticity <= 0.278 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.intensity <= 112 {
            if features.value <= 146 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_difference <= 107 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.430 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.282 {
            if features.blue_chromaticity <= 0.280 {
            if features.green_chromaticity <= 0.438 {
            Intensity::Low
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
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.426 {
            if features.intensity <= 122 {
            if features.blue_chromaticity <= 0.274 {
            if features.blue_chromaticity <= 0.270 {
            Intensity::Low
            } else {
            if features.blue_chromaticity <= 0.272 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_luminance <= 100 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_luminance <= 115 {
            if features.intensity <= 123 {
            if features.hue <= 55 {
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
            } else {
            if features.blue_luminance <= 108 {
            if features.blue_chromaticity <= 0.271 {
            if features.intensity <= 126 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.424 {
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
            if features.green_luminance <= 173 {
            if features.red_difference <= 104 {
            Intensity::High
            } else {
            if features.red_luminance <= 108 {
            if features.blue_chromaticity <= 0.277 {
            if features.red_chromaticity <= 0.294 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.luminance <= 139 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.luminance <= 140 {
            if features.luminance <= 138 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.red_luminance <= 112 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.red_chromaticity <= 0.290 {
            if features.blue_difference <= 106 {
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
            }
            }
            } else {
            if features.green_chromaticity <= 0.458 {
            if features.red_luminance <= 98 {
            if features.saturation <= 97 {
            if features.blue_difference <= 108 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.444 {
            if features.red_chromaticity <= 0.287 {
            if features.value <= 129 {
            if features.value <= 114 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.274 {
            Intensity::High
            } else {
            if features.green_luminance <= 122 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.red_luminance <= 85 {
            if features.green_chromaticity <= 0.444 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.444 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_chromaticity <= 0.444 {
            if features.red_chromaticity <= 0.285 {
            Intensity::High
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
            }
            }
            } else {
            if features.green_chromaticity <= 0.434 {
            if features.luminance <= 122 {
            if features.blue_chromaticity <= 0.270 {
            if features.intensity <= 103 {
            Intensity::Low
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
            if features.red_chromaticity <= 0.296 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.saturation <= 96 {
            if features.red_luminance <= 86 {
            Intensity::High
            } else {
            if features.blue_difference <= 110 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_chromaticity <= 0.292 {
            if features.green_chromaticity <= 0.438 {
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
            }
            }
            }
            } else {
            if features.red_chromaticity <= 0.278 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.279 {
            if features.blue_luminance <= 90 {
            if features.blue_difference <= 111 {
            Intensity::High
            } else {
            if features.hue <= 59 {
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
            }
            } else {
            if features.value <= 131 {
            if features.green_luminance <= 114 {
            if features.blue_chromaticity <= 0.267 {
            if features.blue_chromaticity <= 0.267 {
            if features.saturation <= 104 {
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
            if features.red_chromaticity <= 0.276 {
            if features.blue_luminance <= 80 {
            if features.red_chromaticity <= 0.267 {
            if features.red_chromaticity <= 0.267 {
            if features.green_luminance <= 120 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.intensity <= 90 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.intensity <= 93 {
            if features.red_difference <= 107 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_luminance <= 78 {
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
            if features.green_chromaticity <= 0.440 {
            if features.saturation <= 98 {
            if features.red_luminance <= 85 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.blue_luminance <= 78 {
            Intensity::High
            } else {
            if features.blue_luminance <= 79 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.267 {
            if features.red_luminance <= 77 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.290 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.268 {
            Intensity::High
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
            if features.red_luminance <= 87 {
            if features.saturation <= 102 {
            if features.blue_chromaticity <= 0.274 {
            if features.blue_chromaticity <= 0.274 {
            if features.intensity <= 100 {
            if features.red_chromaticity <= 0.286 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.450 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.hue <= 59 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_chromaticity <= 0.451 {
            if features.value <= 133 {
            Intensity::Low
            } else {
            if features.blue_difference <= 110 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.value <= 143 {
            if features.green_chromaticity <= 0.454 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.456 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_chromaticity <= 0.272 {
            Intensity::High
            } else {
            if features.blue_luminance <= 85 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.270 {
            if features.blue_luminance <= 86 {
            if features.green_chromaticity <= 0.447 {
            if features.red_chromaticity <= 0.287 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.292 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.269 {
            if features.red_chromaticity <= 0.285 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_chromaticity <= 0.299 {
            if features.blue_chromaticity <= 0.270 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.289 {
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
            if features.red_chromaticity <= 0.286 {
            if features.red_luminance <= 93 {
            if features.green_chromaticity <= 0.458 {
            if features.red_chromaticity <= 0.273 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.270 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_luminance <= 146 {
            if features.saturation <= 98 {
            if features.red_chromaticity <= 0.287 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.440 {
            Intensity::High
            } else {
            if features.red_luminance <= 96 {
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
            if features.saturation <= 96 {
            if features.luminance <= 154 {
            if features.red_chromaticity <= 0.301 {
            if features.green_chromaticity <= 0.431 {
            if features.luminance <= 123 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.271 {
            if features.red_difference <= 108 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.430 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.430 {
            Intensity::High
            } else {
            if features.value <= 156 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.value <= 156 {
            if features.red_chromaticity <= 0.286 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.275 {
            if features.value <= 155 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.288 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.intensity <= 120 {
            if features.green_chromaticity <= 0.440 {
            if features.blue_chromaticity <= 0.277 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.294 {
            Intensity::High
            } else {
            if features.red_difference <= 107 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            }
            }
            } else {
            if features.value <= 151 {
            if features.intensity <= 111 {
            if features.green_chromaticity <= 0.430 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_luminance <= 106 {
            if features.luminance <= 127 {
            if features.value <= 145 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.268 {
            Intensity::High
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
            }
            } else {
            if features.blue_chromaticity <= 0.269 {
            if features.green_chromaticity <= 0.425 {
            if features.intensity <= 131 {
            if features.blue_chromaticity <= 0.267 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            } else {
            if features.green_luminance <= 158 {
            if features.blue_chromaticity <= 0.267 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.luminance <= 134 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.value <= 181 {
            if features.blue_luminance <= 111 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.305 {
            Intensity::Low
            } else {
            if features.red_luminance <= 128 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.green_luminance <= 185 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.green_luminance <= 157 {
            if features.red_chromaticity <= 0.300 {
            if features.saturation <= 101 {
            if features.red_difference <= 108 {
            if features.blue_chromaticity <= 0.271 {
            if features.blue_chromaticity <= 0.270 {
            if features.green_luminance <= 155 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            } else {
            if features.red_luminance <= 101 {
            if features.value <= 155 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_chromaticity <= 0.269 {
            if features.blue_luminance <= 92 {
            if features.luminance <= 128 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.luminance <= 130 {
            Intensity::Low
            } else {
            Intensity::Low
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
            } else {
            Intensity::Low
            }
            } else {
            if features.value <= 145 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.270 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.270 {
            if features.red_chromaticity <= 0.291 {
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
            if features.red_luminance <= 61 {
            if features.green_chromaticity <= 0.462 {
            if features.green_chromaticity <= 0.462 {
            Intensity::High
            } else {
            if features.blue_difference <= 114 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_difference <= 110 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.245 {
            Intensity::High
            } else {
            if features.luminance <= 65 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.intensity <= 88 {
            if features.blue_chromaticity <= 0.275 {
            if features.red_chromaticity <= 0.272 {
            if features.intensity <= 85 {
            if features.blue_luminance <= 63 {
            if features.blue_difference <= 112 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.intensity <= 81 {
            if features.luminance <= 93 {
            Intensity::High
            } else {
            if features.red_luminance <= 64 {
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
            if features.blue_luminance <= 69 {
            if features.luminance <= 100 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.489 {
            if features.blue_chromaticity <= 0.270 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.272 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.269 {
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
            } else {
            if features.red_chromaticity <= 0.259 {
            if features.blue_chromaticity <= 0.279 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.469 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.247 {
            if features.green_chromaticity <= 0.474 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.luminance <= 100 {
            if features.saturation <= 120 {
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
            if features.green_chromaticity <= 0.460 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.276 {
            if features.green_chromaticity <= 0.465 {
            if features.red_chromaticity <= 0.262 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.value <= 115 {
            Intensity::Low
            } else {
            if features.green_luminance <= 119 {
            Intensity::Low
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
            }
            }
            } else {
            if features.red_luminance <= 74 {
            if features.red_difference <= 100 {
            if features.red_chromaticity <= 0.248 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.271 {
            if features.value <= 136 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.474 {
            Intensity::High
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
            } else {
            if features.green_luminance <= 129 {
            if features.blue_luminance <= 76 {
            if features.green_chromaticity <= 0.460 {
            Intensity::High
            } else {
            if features.intensity <= 92 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.268 {
            if features.red_chromaticity <= 0.268 {
            if features.blue_chromaticity <= 0.271 {
            Intensity::High
            } else {
            if features.green_luminance <= 136 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.275 {
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
            if features.saturation <= 92 {
            if features.green_chromaticity <= 0.416 {
            if features.green_chromaticity <= 0.409 {
            if features.saturation <= 87 {
            if features.red_luminance <= 134 {
            if features.red_chromaticity <= 0.323 {
            if features.luminance <= 128 {
            if features.red_chromaticity <= 0.312 {
            if features.red_chromaticity <= 0.311 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.314 {
            if features.green_chromaticity <= 0.406 {
            Intensity::High
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
            }
            } else {
            if features.hue <= 49 {
            if features.value <= 152 {
            if features.saturation <= 84 {
            Intensity::High
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
            } else {
            if features.blue_difference <= 106 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.407 {
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
            if features.red_chromaticity <= 0.313 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.315 {
            if features.red_chromaticity <= 0.315 {
            if features.saturation <= 77 {
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
            Intensity::Low
            }
            }
            }
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.414 {
            if features.red_difference <= 114 {
            if features.luminance <= 149 {
            if features.red_chromaticity <= 0.317 {
            if features.green_chromaticity <= 0.409 {
            Intensity::High
            } else {
            if features.red_difference <= 112 {
            if features.red_luminance <= 118 {
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
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.413 {
            if features.red_luminance <= 134 {
            if features.value <= 173 {
            if features.blue_chromaticity <= 0.273 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_luminance <= 115 {
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
            }
            } else {
            if features.hue <= 48 {
            Intensity::High
            } else {
            if features.luminance <= 133 {
            if features.blue_difference <= 108 {
            if features.red_chromaticity <= 0.317 {
            Intensity::High
            } else {
            if features.saturation <= 86 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.value <= 127 {
            if features.luminance <= 107 {
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
            }
            } else {
            if features.saturation <= 88 {
            if features.red_chromaticity <= 0.317 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.411 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.411 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.321 {
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
            if features.green_chromaticity <= 0.414 {
            if features.blue_luminance <= 90 {
            Intensity::High
            } else {
            if features.red_luminance <= 110 {
            if features.green_luminance <= 140 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.value <= 153 {
            if features.blue_luminance <= 99 {
            if features.blue_chromaticity <= 0.267 {
            Intensity::Low
            } else {
            if features.green_luminance <= 149 {
            if features.red_luminance <= 108 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_luminance <= 98 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.red_chromaticity <= 0.306 {
            if features.green_chromaticity <= 0.415 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.intensity <= 120 {
            Intensity::Low
            } else {
            if features.saturation <= 86 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.intensity <= 123 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.317 {
            if features.value <= 170 {
            if features.blue_chromaticity <= 0.275 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            } else {
            if features.blue_luminance <= 112 {
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
            if features.green_chromaticity <= 0.423 {
            if features.green_chromaticity <= 0.419 {
            if features.green_luminance <= 129 {
            if features.red_difference <= 115 {
            if features.intensity <= 101 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.417 {
            Intensity::High
            } else {
            if features.saturation <= 89 {
            if features.hue <= 53 {
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
            if features.green_luminance <= 111 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.313 {
            if features.saturation <= 90 {
            if features.blue_difference <= 111 {
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
            if features.blue_chromaticity <= 0.275 {
            if features.saturation <= 87 {
            if features.green_chromaticity <= 0.416 {
            if features.green_chromaticity <= 0.416 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.309 {
            if features.intensity <= 122 {
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
            }
            } else {
            if features.red_chromaticity <= 0.313 {
            if features.intensity <= 104 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.311 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_chromaticity <= 0.268 {
            if features.blue_chromaticity <= 0.268 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.417 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.red_chromaticity <= 0.303 {
            if features.blue_chromaticity <= 0.282 {
            if features.blue_luminance <= 88 {
            if features.green_chromaticity <= 0.419 {
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
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.305 {
            if features.blue_chromaticity <= 0.279 {
            if features.intensity <= 110 {
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
            if features.green_luminance <= 150 {
            if features.red_chromaticity <= 0.307 {
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
            }
            }
            }
            }
            } else {
            if features.value <= 147 {
            if features.blue_chromaticity <= 0.278 {
            if features.blue_difference <= 111 {
            if features.red_chromaticity <= 0.302 {
            if features.saturation <= 87 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.301 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_chromaticity <= 0.306 {
            if features.value <= 140 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.intensity <= 102 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.saturation <= 91 {
            if features.red_difference <= 115 {
            if features.intensity <= 99 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.saturation <= 90 {
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
            if features.red_chromaticity <= 0.300 {
            if features.blue_chromaticity <= 0.279 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.296 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.281 {
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
            if features.green_chromaticity <= 0.421 {
            if features.green_chromaticity <= 0.420 {
            if features.blue_chromaticity <= 0.270 {
            if features.intensity <= 124 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.blue_difference <= 107 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.saturation <= 90 {
            if features.red_chromaticity <= 0.308 {
            if features.blue_luminance <= 96 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.saturation <= 91 {
            if features.red_luminance <= 113 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_luminance <= 98 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.270 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.433 {
            if features.red_chromaticity <= 0.294 {
            if features.saturation <= 86 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.281 {
            if features.blue_chromaticity <= 0.280 {
            if features.green_chromaticity <= 0.432 {
            if features.green_chromaticity <= 0.427 {
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
            if features.blue_chromaticity <= 0.281 {
            if features.red_luminance <= 74 {
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
            if features.red_difference <= 113 {
            if features.value <= 138 {
            if features.green_luminance <= 132 {
            if features.green_chromaticity <= 0.424 {
            if features.green_luminance <= 125 {
            Intensity::High
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
            if features.hue <= 54 {
            if features.green_chromaticity <= 0.424 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_chromaticity <= 0.424 {
            Intensity::High
            } else {
            if features.blue_luminance <= 89 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.279 {
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
            }
            } else {
            if features.green_chromaticity <= 0.442 {
            if features.green_chromaticity <= 0.427 {
            if features.red_chromaticity <= 0.308 {
            if features.luminance <= 102 {
            if features.red_chromaticity <= 0.305 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.426 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.423 {
            if features.green_luminance <= 143 {
            if features.green_luminance <= 136 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.424 {
            if features.saturation <= 93 {
            if features.red_difference <= 113 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.270 {
            if features.blue_chromaticity <= 0.270 {
            if features.blue_chromaticity <= 0.270 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.value <= 134 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.value <= 132 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.421 {
            if features.blue_luminance <= 86 {
            Intensity::High
            } else {
            if features.blue_difference <= 105 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.421 {
            if features.green_luminance <= 150 {
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
            if features.saturation <= 93 {
            if features.red_chromaticity <= 0.309 {
            if features.blue_chromaticity <= 0.269 {
            if features.intensity <= 105 {
            Intensity::High
            } else {
            if features.red_difference <= 112 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.intensity <= 119 {
            if features.blue_chromaticity <= 0.268 {
            if features.value <= 146 {
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
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.268 {
            if features.blue_luminance <= 87 {
            if features.intensity <= 101 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.310 {
            if features.blue_chromaticity <= 0.267 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.hue <= 51 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.green_luminance <= 129 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            }
            }
            } else {
            if features.blue_luminance <= 80 {
            if features.green_chromaticity <= 0.429 {
            if features.red_difference <= 115 {
            if features.blue_chromaticity <= 0.271 {
            if features.green_chromaticity <= 0.428 {
            Intensity::High
            } else {
            if features.intensity <= 96 {
            if features.value <= 118 {
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
            if features.intensity <= 78 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_luminance <= 83 {
            if features.luminance <= 93 {
            if features.value <= 101 {
            if features.value <= 98 {
            if features.green_chromaticity <= 0.440 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_luminance <= 63 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.272 {
            if features.blue_luminance <= 70 {
            if features.value <= 111 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.300 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_difference <= 113 {
            if features.blue_chromaticity <= 0.276 {
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
            if features.red_chromaticity <= 0.299 {
            Intensity::High
            } else {
            if features.intensity <= 94 {
            Intensity::High
            } else {
            if features.saturation <= 95 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.432 {
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
            if features.intensity <= 103 {
            if features.saturation <= 93 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.saturation <= 94 {
            Intensity::High
            } else {
            if features.blue_difference <= 108 {
            Intensity::High
            } else {
            if features.blue_difference <= 109 {
            if features.red_luminance <= 98 {
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
            if features.green_chromaticity <= 0.454 {
            if features.green_chromaticity <= 0.453 {
            if features.red_chromaticity <= 0.268 {
            if features.red_chromaticity <= 0.268 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.saturation <= 96 {
            if features.luminance <= 80 {
            if features.red_luminance <= 49 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.luminance <= 83 {
            Intensity::Low
            } else {
            if features.blue_chromaticity <= 0.279 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.blue_luminance <= 48 {
            if features.red_luminance <= 50 {
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
            Intensity::Low
            }
            } else {
            if features.hue <= 64 {
            if features.red_luminance <= 27 {
            if features.green_luminance <= 55 {
            Intensity::High
            } else {
            if features.blue_luminance <= 30 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.274 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.274 {
            if features.red_chromaticity <= 0.259 {
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
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.red_difference <= 120 {
            if features.red_chromaticity <= 0.298 {
            if features.value <= 29 {
            if features.red_difference <= 119 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.hue <= 63 {
            if features.value <= 38 {
            if features.luminance <= 29 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.278 {
            if features.red_difference <= 117 {
            if features.blue_chromaticity <= 0.268 {
            if features.blue_chromaticity <= 0.267 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.saturation <= 99 {
            if features.green_chromaticity <= 0.445 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.274 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.275 {
            if features.red_chromaticity <= 0.261 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_luminance <= 25 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.279 {
            Intensity::Low
            } else {
            if features.red_luminance <= 41 {
            if features.green_chromaticity <= 0.431 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.438 {
            Intensity::High
            } else {
            if features.red_difference <= 117 {
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
            if features.value <= 30 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.404 {
            if features.saturation <= 79 {
            if features.green_chromaticity <= 0.404 {
            if features.green_chromaticity <= 0.401 {
            if features.red_chromaticity <= 0.319 {
            if features.green_chromaticity <= 0.401 {
            if features.green_chromaticity <= 0.400 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_luminance <= 90 {
            if features.luminance <= 117 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.325 {
            if features.red_chromaticity <= 0.325 {
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
            }
            }
            } else {
            if features.red_chromaticity <= 0.322 {
            if features.green_luminance <= 135 {
            if features.green_chromaticity <= 0.403 {
            if features.red_chromaticity <= 0.317 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.403 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.luminance <= 128 {
            if features.saturation <= 78 {
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
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.320 {
            Intensity::High
            } else {
            if features.blue_difference <= 107 {
            if features.red_chromaticity <= 0.328 {
            if features.green_chromaticity <= 0.403 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.267 {
            Intensity::High
            } else {
            if features.intensity <= 118 {
            if features.intensity <= 117 {
            if features.blue_chromaticity <= 0.274 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.saturation <= 83 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_luminance <= 124 {
            if features.blue_chromaticity <= 0.271 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.399 {
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
            if features.saturation <= 96 {
            if features.saturation <= 88 {
            if features.red_chromaticity <= 0.328 {
            if features.blue_chromaticity <= 0.281 {
            if features.green_chromaticity <= 0.407 {
            if features.saturation <= 87 {
            if features.blue_chromaticity <= 0.278 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            } else {
            if features.value <= 145 {
            if features.blue_luminance <= 78 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.intensity <= 119 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.saturation <= 79 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.green_luminance <= 148 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_difference <= 118 {
            if features.value <= 130 {
            if features.blue_chromaticity <= 0.271 {
            if features.red_chromaticity <= 0.300 {
            Intensity::Low
            } else {
            if features.blue_chromaticity <= 0.268 {
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
            if features.blue_difference <= 118 {
            if features.saturation <= 95 {
            if features.red_difference <= 119 {
            if features.green_chromaticity <= 0.424 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.green_luminance <= 73 {
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
            if features.blue_difference <= 117 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.red_luminance <= 155 {
            if features.red_difference <= 121 {
            if features.blue_chromaticity <= 0.282 {
            if features.red_luminance <= 45 {
            if features.green_luminance <= 55 {
            if features.blue_chromaticity <= 0.281 {
            if features.saturation <= 146 {
            if features.blue_chromaticity <= 0.275 {
            if features.intensity <= 35 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.280 {
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
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.335 {
            if features.blue_chromaticity <= 0.281 {
            if features.red_chromaticity <= 0.317 {
            if features.green_chromaticity <= 0.407 {
            Intensity::High
            } else {
            if features.value <= 77 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.luminance <= 73 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.luminance <= 127 {
            if features.saturation <= 74 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_chromaticity <= 0.337 {
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
            if features.red_difference <= 122 {
            if features.saturation <= 97 {
            if features.green_chromaticity <= 0.409 {
            Intensity::Low
            } else {
            if features.blue_luminance <= 37 {
            if features.red_luminance <= 33 {
            if features.saturation <= 90 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.276 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_chromaticity <= 0.275 {
            if features.value <= 50 {
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
            if features.blue_chromaticity <= 0.267 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_difference <= 107 {
            Intensity::High
            } else {
            if features.luminance <= 138 {
            if features.blue_chromaticity <= 0.282 {
            if features.green_luminance <= 43 {
            if features.red_chromaticity <= 0.045 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.hue <= 53 {
            if features.saturation <= 87 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.282 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_luminance <= 110 {
            Intensity::High
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
            }
            } else {
            if features.red_difference <= 104 {
            if features.green_chromaticity <= 0.437 {
            if features.red_difference <= 99 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.296 {
            if features.green_chromaticity <= 0.432 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.295 {
            if features.blue_chromaticity <= 0.290 {
            if features.green_chromaticity <= 0.433 {
            Intensity::Low
            } else {
            if features.hue <= 62 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.292 {
            if features.red_difference <= 103 {
            Intensity::High
            } else {
            if features.luminance <= 133 {
            Intensity::Low
            } else {
            if features.value <= 157 {
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
            Intensity::High
            }
            }
            } else {
            if features.green_chromaticity <= 0.430 {
            if features.red_difference <= 102 {
            if features.blue_chromaticity <= 0.302 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.312 {
            if features.blue_chromaticity <= 0.305 {
            Intensity::Low
            } else {
            if features.intensity <= 138 {
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
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.432 {
            Intensity::High
            } else {
            if features.saturation <= 103 {
            if features.luminance <= 118 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.261 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.436 {
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
            } else {
            if features.blue_luminance <= 78 {
            if features.value <= 51 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.287 {
            if features.green_luminance <= 131 {
            if features.green_chromaticity <= 0.459 {
            if features.blue_difference <= 113 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.253 {
            Intensity::High
            } else {
            if features.luminance <= 107 {
            if features.green_chromaticity <= 0.462 {
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
            if features.green_chromaticity <= 0.445 {
            if features.green_chromaticity <= 0.445 {
            if features.green_luminance <= 145 {
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
            } else {
            if features.red_chromaticity <= 0.249 {
            if features.red_chromaticity <= 0.226 {
            if features.red_chromaticity <= 0.224 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.luminance <= 96 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.hue <= 65 {
            if features.red_chromaticity <= 0.268 {
            Intensity::High
            } else {
            if features.blue_difference <= 112 {
            if features.blue_difference <= 111 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.blue_luminance <= 95 {
            if features.green_chromaticity <= 0.442 {
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
            if features.blue_luminance <= 87 {
            if features.blue_luminance <= 85 {
            if features.blue_luminance <= 82 {
            if features.blue_chromaticity <= 0.293 {
            if features.red_chromaticity <= 0.251 {
            if features.saturation <= 115 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.454 {
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
            if features.green_luminance <= 128 {
            Intensity::High
            } else {
            if features.red_luminance <= 75 {
            if features.blue_luminance <= 86 {
            if features.red_difference <= 103 {
            Intensity::Low
            } else {
            Intensity::Low
            }
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
            }
            } else {
            if features.blue_chromaticity <= 0.291 {
            if features.blue_difference <= 112 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.261 {
            if features.intensity <= 95 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.red_luminance <= 82 {
            Intensity::Low
            } else {
            if features.blue_luminance <= 96 {
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
            if features.green_chromaticity <= 0.417 {
            if features.green_chromaticity <= 0.409 {
            if features.green_chromaticity <= 0.404 {
            if features.blue_difference <= 140 {
            if features.blue_chromaticity <= 0.290 {
            if features.intensity <= 177 {
            if features.saturation <= 72 {
            if features.red_luminance <= 174 {
            if features.red_chromaticity <= 0.313 {
            if features.intensity <= 120 {
            if features.green_chromaticity <= 0.402 {
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
            if features.red_luminance <= 168 {
            if features.green_chromaticity <= 0.397 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.intensity <= 165 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.290 {
            if features.red_luminance <= 175 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.363 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.value <= 131 {
            if features.blue_difference <= 115 {
            if features.red_chromaticity <= 0.311 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.283 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_luminance <= 71 {
            if features.red_luminance <= 16 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.luminance <= 86 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.value <= 154 {
            if features.blue_chromaticity <= 0.282 {
            if features.red_difference <= 115 {
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
            } else {
            if features.intensity <= 143 {
            if features.luminance <= 155 {
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
            Intensity::High
            }
            } else {
            if features.blue_difference <= 107 {
            Intensity::High
            } else {
            if features.blue_difference <= 137 {
            if features.blue_difference <= 133 {
            if features.green_chromaticity <= 0.394 {
            if features.red_chromaticity <= 0.349 {
            if features.blue_luminance <= 117 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_difference <= 112 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_luminance <= 109 {
            if features.red_difference <= 108 {
            Intensity::High
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
            }
            } else {
            if features.hue <= 92 {
            Intensity::High
            } else {
            if features.green_luminance <= 183 {
            if features.green_luminance <= 150 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.311 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.green_luminance <= 153 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.332 {
            if features.green_chromaticity <= 0.323 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.332 {
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
            if features.saturation <= 71 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_chromaticity <= 0.287 {
            if features.green_chromaticity <= 0.404 {
            Intensity::High
            } else {
            if features.intensity <= 110 {
            if features.blue_chromaticity <= 0.287 {
            if features.green_luminance <= 112 {
            if features.red_difference <= 123 {
            Intensity::Low
            } else {
            if features.saturation <= 76 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_difference <= 114 {
            if features.green_chromaticity <= 0.405 {
            Intensity::High
            } else {
            if features.green_luminance <= 127 {
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
            if features.red_chromaticity <= 0.304 {
            if features.blue_difference <= 111 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.luminance <= 154 {
            if features.saturation <= 77 {
            if features.green_chromaticity <= 0.404 {
            if features.red_chromaticity <= 0.310 {
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
            if features.luminance <= 143 {
            if features.green_chromaticity <= 0.409 {
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
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.300 {
            if features.blue_chromaticity <= 0.300 {
            if features.red_luminance <= 100 {
            if features.red_chromaticity <= 0.293 {
            Intensity::High
            } else {
            if features.blue_difference <= 114 {
            Intensity::High
            } else {
            if features.red_luminance <= 92 {
            if features.green_chromaticity <= 0.408 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.saturation <= 67 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.luminance <= 130 {
            if features.blue_luminance <= 104 {
            if features.green_chromaticity <= 0.406 {
            if features.blue_chromaticity <= 0.290 {
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
            if features.green_chromaticity <= 0.406 {
            if features.luminance <= 127 {
            Intensity::Low
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
            if features.hue <= 56 {
            if features.red_chromaticity <= 0.304 {
            if features.green_chromaticity <= 0.407 {
            Intensity::High
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
            if features.red_luminance <= 106 {
            Intensity::Low
            } else {
            if features.blue_luminance <= 117 {
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
            if features.blue_difference <= 133 {
            if features.red_luminance <= 19 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.265 {
            if features.blue_difference <= 124 {
            Intensity::High
            } else {
            if features.green_luminance <= 59 {
            if features.blue_luminance <= 27 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.251 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.red_difference <= 110 {
            if features.red_chromaticity <= 0.286 {
            if features.hue <= 65 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.409 {
            if features.green_chromaticity <= 0.404 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.409 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.500 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.289 {
            if features.red_luminance <= 104 {
            if features.value <= 105 {
            if features.red_chromaticity <= 0.298 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.green_luminance <= 129 {
            if features.green_chromaticity <= 0.416 {
            if features.red_luminance <= 93 {
            if features.blue_chromaticity <= 0.284 {
            if features.green_chromaticity <= 0.412 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.intensity <= 86 {
            Intensity::High
            } else {
            if features.saturation <= 75 {
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
            } else {
            if features.green_chromaticity <= 0.416 {
            if features.blue_chromaticity <= 0.284 {
            if features.red_luminance <= 99 {
            if features.green_luminance <= 134 {
            if features.green_chromaticity <= 0.413 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            } else {
            if features.red_chromaticity <= 0.304 {
            if features.green_chromaticity <= 0.414 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.saturation <= 79 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.411 {
            if features.saturation <= 75 {
            Intensity::Low
            } else {
            if features.green_luminance <= 137 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.411 {
            Intensity::High
            } else {
            if features.saturation <= 76 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.red_difference <= 112 {
            if features.luminance <= 119 {
            Intensity::Low
            } else {
            if features.red_difference <= 110 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.417 {
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
            if features.saturation <= 79 {
            if features.red_chromaticity <= 0.296 {
            if features.green_chromaticity <= 0.416 {
            Intensity::High
            } else {
            if features.green_luminance <= 165 {
            if features.red_chromaticity <= 0.296 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.luminance <= 153 {
            if features.red_luminance <= 123 {
            if features.red_chromaticity <= 0.302 {
            if features.green_luminance <= 144 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.414 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.412 {
            if features.green_chromaticity <= 0.411 {
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
            } else {
            if features.luminance <= 151 {
            if features.red_chromaticity <= 0.303 {
            if features.red_difference <= 109 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.410 {
            Intensity::Low
            } else {
            if features.intensity <= 139 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.285 {
            if features.red_chromaticity <= 0.303 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.284 {
            if features.green_luminance <= 175 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_luminance <= 181 {
            Intensity::Low
            } else {
            if features.luminance <= 161 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            }
            }
            } else {
            if features.luminance <= 132 {
            if features.blue_chromaticity <= 0.282 {
            Intensity::Low
            } else {
            if features.blue_luminance <= 100 {
            Intensity::Low
            } else {
            if features.saturation <= 81 {
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
            } else {
            if features.green_luminance <= 160 {
            if features.red_difference <= 110 {
            if features.blue_chromaticity <= 0.286 {
            if features.green_chromaticity <= 0.416 {
            if features.green_luminance <= 158 {
            Intensity::Low
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
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.283 {
            if features.red_luminance <= 116 {
            if features.red_difference <= 112 {
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
            } else {
            if features.blue_chromaticity <= 0.284 {
            if features.green_chromaticity <= 0.416 {
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
            if features.green_chromaticity <= 0.412 {
            if features.blue_chromaticity <= 0.283 {
            if features.luminance <= 154 {
            if features.blue_chromaticity <= 0.282 {
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
            if features.red_luminance <= 128 {
            if features.blue_difference <= 108 {
            if features.green_chromaticity <= 0.415 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.red_luminance <= 123 {
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
            if features.blue_chromaticity <= 0.296 {
            if features.red_luminance <= 102 {
            if features.green_chromaticity <= 0.410 {
            if features.blue_chromaticity <= 0.293 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.290 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.416 {
            if features.blue_chromaticity <= 0.291 {
            if features.red_chromaticity <= 0.297 {
            if features.red_difference <= 112 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_chromaticity <= 0.291 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.294 {
            if features.blue_chromaticity <= 0.294 {
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
            } else {
            if features.green_chromaticity <= 0.416 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.416 {
            Intensity::Low
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
            }
            } else {
            if features.blue_luminance <= 114 {
            if features.blue_chromaticity <= 0.295 {
            if features.green_luminance <= 157 {
            if features.green_chromaticity <= 0.410 {
            if features.red_luminance <= 109 {
            Intensity::Low
            } else {
            if features.red_luminance <= 112 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_chromaticity <= 0.298 {
            if features.luminance <= 127 {
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
            } else {
            if features.green_chromaticity <= 0.416 {
            if features.intensity <= 127 {
            if features.green_chromaticity <= 0.414 {
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
            if features.saturation <= 77 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.295 {
            if features.green_luminance <= 151 {
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
            if features.blue_chromaticity <= 0.293 {
            if features.blue_chromaticity <= 0.293 {
            if features.green_chromaticity <= 0.412 {
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
            if features.hue <= 59 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_chromaticity <= 0.292 {
            if features.red_chromaticity <= 0.297 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.291 {
            if features.value <= 178 {
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
            }
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.value <= 38 {
            if features.red_luminance <= 17 {
            Intensity::Low
            } else {
            if features.value <= 29 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_difference <= 130 {
            if features.luminance <= 119 {
            if features.red_luminance <= 70 {
            if features.red_difference <= 107 {
            Intensity::High
            } else {
            if features.blue_difference <= 124 {
            if features.blue_chromaticity <= 0.315 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_difference <= 125 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.luminance <= 93 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.415 {
            if features.saturation <= 75 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_luminance <= 84 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.417 {
            if features.green_chromaticity <= 0.412 {
            if features.green_luminance <= 142 {
            if features.red_luminance <= 98 {
            Intensity::Low
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
            if features.intensity <= 126 {
            if features.intensity <= 117 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_luminance <= 101 {
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
            if features.red_difference <= 117 {
            if features.green_chromaticity <= 0.450 {
            if features.green_chromaticity <= 0.429 {
            if features.blue_luminance <= 99 {
            if features.green_chromaticity <= 0.425 {
            if features.blue_chromaticity <= 0.285 {
            if features.green_luminance <= 114 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.295 {
            if features.green_chromaticity <= 0.423 {
            if features.green_chromaticity <= 0.422 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.283 {
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
            } else {
            if features.value <= 145 {
            if features.intensity <= 109 {
            if features.red_luminance <= 63 {
            if features.green_chromaticity <= 0.420 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_luminance <= 68 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.intensity <= 110 {
            if features.value <= 138 {
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
            }
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_luminance <= 98 {
            if features.red_chromaticity <= 0.279 {
            if features.green_chromaticity <= 0.425 {
            if features.red_luminance <= 86 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.425 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.saturation <= 106 {
            if features.blue_luminance <= 97 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.332 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.293 {
            if features.green_chromaticity <= 0.428 {
            if features.red_chromaticity <= 0.287 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.blue_difference <= 113 {
            Intensity::High
            } else {
            Intensity::Low
            }
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
            if features.value <= 142 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.287 {
            if features.hue <= 59 {
            if features.green_chromaticity <= 0.417 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.420 {
            if features.green_chromaticity <= 0.417 {
            if features.intensity <= 129 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.295 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.420 {
            Intensity::High
            } else {
            if features.blue_luminance <= 106 {
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
            if features.red_luminance <= 100 {
            if features.blue_chromaticity <= 0.288 {
            Intensity::High
            } else {
            if features.red_difference <= 109 {
            if features.blue_chromaticity <= 0.297 {
            if features.blue_chromaticity <= 0.295 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.blue_luminance <= 103 {
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
            if features.blue_chromaticity <= 0.303 {
            if features.saturation <= 88 {
            if features.blue_chromaticity <= 0.289 {
            if features.green_luminance <= 149 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.luminance <= 134 {
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
            }
            } else {
            if features.blue_chromaticity <= 0.354 {
            if features.blue_luminance <= 96 {
            if features.red_luminance <= 38 {
            if features.saturation <= 124 {
            Intensity::High
            } else {
            if features.blue_difference <= 125 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.287 {
            if features.red_luminance <= 73 {
            if features.red_chromaticity <= 0.267 {
            if features.luminance <= 95 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.saturation <= 87 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            } else {
            if features.blue_chromaticity <= 0.285 {
            if features.red_chromaticity <= 0.284 {
            Intensity::Low
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
            }
            } else {
            if features.red_luminance <= 76 {
            if features.green_chromaticity <= 0.440 {
            if features.blue_difference <= 123 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.blue_difference <= 116 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_chromaticity <= 0.269 {
            Intensity::High
            } else {
            if features.green_luminance <= 122 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            }
            }
            } else {
            if features.blue_luminance <= 101 {
            if features.saturation <= 88 {
            if features.red_luminance <= 99 {
            if features.blue_chromaticity <= 0.286 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.saturation <= 87 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_difference <= 116 {
            if features.saturation <= 94 {
            if features.red_luminance <= 91 {
            Intensity::High
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
            Intensity::High
            }
            }
            } else {
            if features.red_chromaticity <= 0.286 {
            if features.red_chromaticity <= 0.279 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            if features.green_chromaticity <= 0.430 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            }
            }
            } else {
            if features.intensity <= 38 {
            if features.green_chromaticity <= 0.446 {
            Intensity::Low
            } else {
            if features.green_chromaticity <= 0.448 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_luminance <= 53 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.blue_chromaticity <= 0.351 {
            if features.red_difference <= 115 {
            if features.green_luminance <= 113 {
            if features.blue_difference <= 124 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.326 {
            if features.saturation <= 181 {
            if features.blue_chromaticity <= 0.326 {
            if features.green_luminance <= 40 {
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
            }
            } else {
            if features.blue_chromaticity <= 0.285 {
            Intensity::High
            } else {
            if features.red_difference <= 105 {
            if features.red_luminance <= 73 {
            if features.red_chromaticity <= 0.250 {
            if features.red_chromaticity <= 0.248 {
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
            } else {
            Intensity::High
            }
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.506 {
            if features.red_chromaticity <= 0.221 {
            if features.blue_luminance <= 28 {
            if features.saturation <= 151 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.491 {
            Intensity::High
            } else {
            if features.intensity <= 29 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.hue <= 74 {
            if features.red_chromaticity <= 0.210 {
            if features.value <= 42 {
            Intensity::Low
            } else {
            Intensity::High
            }
            } else {
            if features.blue_luminance <= 35 {
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
            } else {
            Intensity::High
            }
            }
            } else {
            if features.red_difference <= 112 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.554 {
            if features.blue_difference <= 127 {
            if features.blue_chromaticity <= 0.356 {
            if features.red_difference <= 115 {
            if features.red_difference <= 114 {
            if features.value <= 50 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.116 {
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
            if features.green_chromaticity <= 0.540 {
            if features.saturation <= 182 {
            if features.blue_luminance <= 26 {
            Intensity::High
            } else {
            if features.red_chromaticity <= 0.138 {
            Intensity::High
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.red_difference <= 114 {
            if features.blue_difference <= 128 {
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
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.value <= 43 {
            if features.value <= 31 {
            if features.green_luminance <= 29 {
            if features.green_luminance <= 27 {
            if features.blue_difference <= 126 {
            if features.hue <= 68 {
            Intensity::Low
            } else {
            if features.red_chromaticity <= 0.194 {
            Intensity::Low
            } else {
            if features.value <= 22 {
            if features.green_luminance <= 21 {
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
            if features.blue_luminance <= 16 {
            if features.green_chromaticity <= 0.548 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.intensity <= 19 {
            if features.green_chromaticity <= 0.479 {
            if features.red_luminance <= 10 {
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
            if features.blue_luminance <= 17 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.500 {
            if features.blue_luminance <= 23 {
            if features.blue_luminance <= 22 {
            if features.saturation <= 100 {
            if features.green_chromaticity <= 0.430 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            Intensity::Low
            }
            } else {
            if features.red_difference <= 120 {
            Intensity::High
            } else {
            if features.green_luminance <= 30 {
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
            if features.saturation <= 167 {
            Intensity::High
            } else {
            if features.value <= 30 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.508 {
            if features.green_chromaticity <= 0.471 {
            if features.blue_difference <= 125 {
            if features.blue_chromaticity <= 0.319 {
            if features.blue_luminance <= 31 {
            if features.blue_chromaticity <= 0.312 {
            if features.luminance <= 27 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.value <= 39 {
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
            if features.blue_luminance <= 34 {
            if features.blue_chromaticity <= 0.341 {
            Intensity::Low
            } else {
            if features.blue_chromaticity <= 0.343 {
            Intensity::High
            } else {
            if features.green_luminance <= 38 {
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
            if features.red_luminance <= 19 {
            if features.intensity <= 21 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.488 {
            if features.red_chromaticity <= 0.231 {
            if features.green_chromaticity <= 0.472 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            if features.red_chromaticity <= 0.235 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.green_chromaticity <= 0.490 {
            Intensity::High
            } else {
            if features.green_chromaticity <= 0.494 {
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
            if features.luminance <= 24 {
            Intensity::High
            } else {
            Intensity::High
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.440 {
            if features.saturation <= 80 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.282 {
            Intensity::High
            } else {
            if features.blue_chromaticity <= 0.322 {
            if features.red_chromaticity <= 0.276 {
            if features.intensity <= 37 {
            if features.blue_luminance <= 35 {
            if features.red_luminance <= 25 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            } else {
            if features.red_luminance <= 41 {
            if features.blue_difference <= 121 {
            Intensity::High
            } else {
            Intensity::High
            }
            } else {
            Intensity::Low
            }
            }
            } else {
            if features.blue_chromaticity <= 0.285 {
            if features.value <= 59 {
            if features.value <= 56 {
            Intensity::High
            } else {
            Intensity::Low
            }
            } else {
            Intensity::High
            }
            } else {
            if features.blue_chromaticity <= 0.296 {
            if features.saturation <= 81 {
            Intensity::Low
            } else {
            Intensity::Low
            }
            } else {
            if features.green_chromaticity <= 0.420 {
            Intensity::Low
            } else {
            Intensity::High
            }
            }
            }
            }
            } else {
            if features.green_chromaticity <= 0.422 {
            if features.value <= 50 {
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
            if features.red_difference <= 119 {
            Intensity::High
            } else {
            if features.value <= 44 {
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