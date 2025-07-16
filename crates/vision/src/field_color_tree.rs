/*
    *********************************** GENERATED CODE ***********************************

    This code was generated from a decision tree model in Python.

    The tool to generate this Rust code can be found here:
        /tools/field_color_detection/tree_to_rust.py
    The input is a joblib file of a LGBMClassifier model and the output is this Rust file.

    **************************************************************************************
*/

pub struct Features {
    pub center: f32,
    pub right: f32,
    pub top: f32,
    pub left: f32,
    pub bottom: f32,
}

#[allow(
    clippy::collapsible_else_if,
    clippy::excessive_precision,
    clippy::needless_return
)]
pub fn predict(features: &Features) -> f32 {
    if features.center <= 0.4098039215686275 {
        if features.center <= 0.3823529411764706 {
            if features.left <= 0.37843137254901965 {
                if features.right <= 0.3705882352941177 {
                    if features.bottom <= 0.37843137254901965 {
                        return 0.12148769817473835;
                    } else {
                        return 0.128887070836163;
                    }
                } else {
                    if features.bottom <= 0.3705882352941177 {
                        return 0.12939208777185368;
                    } else {
                        return 0.14793623920423282;
                    }
                }
            } else {
                if features.right <= 0.3823529411764706 {
                    if features.center <= 0.303921568627451 {
                        return 0.17916841091363248;
                    } else {
                        return 0.13503361413912668;
                    }
                } else {
                    if features.bottom <= 0.3823529411764706 {
                        return 0.1598091074339025;
                    } else {
                        return 0.2765066367769039;
                    }
                }
            }
        } else {
            if features.right <= 0.3823529411764706 {
                if features.left <= 0.3745098039215687 {
                    return 0.13719453249773836;
                } else {
                    if features.right <= 0.3745098039215687 {
                        return 0.19261669577087856;
                    } else {
                        return 0.2976731560291228;
                    }
                }
            } else {
                if features.left <= 0.3823529411764706 {
                    if features.left <= 0.3745098039215687 {
                        return 0.19949540508295557;
                    } else {
                        return 0.3279878964553507;
                    }
                } else {
                    if features.bottom <= 0.37843137254901965 {
                        return 0.29953583550196106;
                    } else {
                        return 0.6226755910629058;
                    }
                }
            }
        }
    } else {
        if features.right <= 0.4098039215686275 {
            if features.left <= 0.40196078431372556 {
                if features.bottom <= 0.3901960784313726 {
                    if features.top <= 0.3901960784313726 {
                        return 0.15597483302612666;
                    } else {
                        return 0.2217616367624327;
                    }
                } else {
                    if features.right <= 0.3901960784313726 {
                        return 0.2649962620002478;
                    } else {
                        return 0.4449760983242563;
                    }
                }
            } else {
                if features.bottom <= 0.40196078431372556 {
                    if features.top <= 0.39803921568627454 {
                        return 0.270653012555593;
                    } else {
                        return 0.5605207815361912;
                    }
                } else {
                    if features.center <= 0.4725490196078432 {
                        return 0.6264774434735823;
                    } else {
                        return 0.7418375324729622;
                    }
                }
            }
        } else {
            if features.left <= 0.4098039215686275 {
                if features.bottom <= 0.40196078431372556 {
                    if features.top <= 0.40196078431372556 {
                        return 0.276078603725037;
                    } else {
                        return 0.5848154058265783;
                    }
                } else {
                    if features.top <= 0.4058823529411765 {
                        return 0.613597034082506;
                    } else {
                        return 0.7301814435355048;
                    }
                }
            } else {
                if features.bottom <= 0.4098039215686275 {
                    if features.top <= 0.40196078431372556 {
                        return 0.5098471451142847;
                    } else {
                        return 0.785632622495393;
                    }
                } else {
                    if features.top <= 0.41372549019607846 {
                        return 0.8035584485944034;
                    } else {
                        return 0.869368887015289;
                    }
                }
            }
        }
    }
}
