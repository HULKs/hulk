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
    if features.center <= 0.41372549019607846 {
        if features.center <= 0.3901960784313726 {
            if features.left <= 0.3823529411764706 {
                if features.right <= 0.3745098039215687 {
                    if features.center <= 0.2843137254901961 {
                        return 0.1292841244201008;
                    } else {
                        return 0.12117833665873456;
                    }
                } else {
                    if features.center <= 0.30000000000000004 {
                        return 0.17509623975033564;
                    } else {
                        return 0.13276107079437602;
                    }
                }
            } else {
                if features.right <= 0.3823529411764706 {
                    if features.center <= 0.30000000000000004 {
                        return 0.19102302545523586;
                    } else {
                        return 0.13303941000096445;
                    }
                } else {
                    if features.bottom <= 0.3823529411764706 {
                        return 0.1562415780537144;
                    } else {
                        return 0.24852136511754866;
                    }
                }
            }
        } else {
            if features.right <= 0.38627450980392164 {
                if features.left <= 0.3823529411764706 {
                    return 0.13535755860508342;
                } else {
                    if features.bottom <= 0.3823529411764706 {
                        return 0.16752100045520762;
                    } else {
                        return 0.2334803651980021;
                    }
                }
            } else {
                if features.left <= 0.38627450980392164 {
                    if features.top <= 0.3823529411764706 {
                        return 0.1667505797468238;
                    } else {
                        return 0.2410149915030246;
                    }
                } else {
                    if features.top <= 0.48431372549019613 {
                        return 0.6008475803213328;
                    } else {
                        return 0.2926986608658493;
                    }
                }
            }
        }
    } else {
        if features.right <= 0.41372549019607846 {
            if features.left <= 0.40196078431372556 {
                if features.bottom <= 0.3941176470588236 {
                    if features.center <= 0.4960784313725491 {
                        return 0.1607225962004354;
                    } else {
                        return 0.24111378456612334;
                    }
                } else {
                    if features.right <= 0.39803921568627454 {
                        return 0.24471007479868095;
                    } else {
                        return 0.4014911518349719;
                    }
                }
            } else {
                if features.bottom <= 0.40196078431372556 {
                    if features.top <= 0.40196078431372556 {
                        return 0.25138046962559557;
                    } else {
                        return 0.5314763955912074;
                    }
                } else {
                    if features.center <= 0.48039215686274517 {
                        return 0.5999534639906836;
                    } else {
                        return 0.7332743790430671;
                    }
                }
            }
        } else {
            if features.left <= 0.41372549019607846 {
                if features.bottom <= 0.4058823529411765 {
                    if features.top <= 0.4058823529411765 {
                        return 0.2632509258740385;
                    } else {
                        return 0.5776376594205969;
                    }
                } else {
                    if features.center <= 0.47647058823529415 {
                        return 0.6268537752855378;
                    } else {
                        return 0.7387958815517713;
                    }
                }
            } else {
                if features.bottom <= 0.41372549019607846 {
                    if features.top <= 0.4058823529411765 {
                        return 0.5122246686049644;
                    } else {
                        return 0.778991955609075;
                    }
                } else {
                    if features.top <= 0.41372549019607846 {
                        return 0.8034348682086462;
                    } else {
                        return 0.8688164070171662;
                    }
                }
            }
        }
    }
}
