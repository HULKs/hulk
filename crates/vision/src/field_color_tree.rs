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
    if features.center <= 0.4176470588235295 {
        if features.center <= 0.3941176470588236 {
            if features.right <= 0.3823529411764706 {
                if features.left <= 0.37843137254901965 {
                    if features.left <= 0.2843137254901961 {
                        return 0.12886987279155843;
                    } else {
                        return 0.1212221082469262;
                    }
                } else {
                    if features.center <= 0.2960784313725491 {
                        return 0.18658499285585473;
                    } else {
                        return 0.13228514319471357;
                    }
                }
            } else {
                if features.left <= 0.3823529411764706 {
                    if features.center <= 0.303921568627451 {
                        return 0.17641368593053208;
                    } else {
                        return 0.13276288403061828;
                    }
                } else {
                    if features.bottom <= 0.3823529411764706 {
                        return 0.15470478884094854;
                    } else {
                        return 0.23830679645549777;
                    }
                }
            }
        } else {
            if features.right <= 0.3901960784313726 {
                if features.left <= 0.38627450980392164 {
                    return 0.13683437281921776;
                } else {
                    if features.bottom <= 0.3823529411764706 {
                        return 0.1703009738805902;
                    } else {
                        return 0.24299087359451027;
                    }
                }
            } else {
                if features.left <= 0.3901960784313726 {
                    if features.left <= 0.3823529411764706 {
                        return 0.20103568926900833;
                    } else {
                        return 0.32017111860092673;
                    }
                } else {
                    if features.top <= 0.48431372549019613 {
                        return 0.631942558231881;
                    } else {
                        return 0.2928143765274028;
                    }
                }
            }
        }
    } else {
        if features.right <= 0.41372549019607846 {
            if features.left <= 0.4058823529411765 {
                if features.bottom <= 0.39803921568627454 {
                    if features.center <= 0.4960784313725491 {
                        return 0.15842406703637488;
                    } else {
                        return 0.23096003183492772;
                    }
                } else {
                    if features.right <= 0.39803921568627454 {
                        return 0.23908277395246438;
                    } else {
                        return 0.3866008442937028;
                    }
                }
            } else {
                if features.bottom <= 0.4058823529411765 {
                    if features.top <= 0.4058823529411765 {
                        return 0.2511718586608732;
                    } else {
                        return 0.5251074813968122;
                    }
                } else {
                    if features.center <= 0.48039215686274517 {
                        return 0.5726155285940048;
                    } else {
                        return 0.7269162443845337;
                    }
                }
            }
        } else {
            if features.left <= 0.41372549019607846 {
                if features.bottom <= 0.4058823529411765 {
                    if features.top <= 0.4098039215686275 {
                        return 0.25604398337606676;
                    } else {
                        return 0.5681447797785855;
                    }
                } else {
                    if features.center <= 0.48039215686274517 {
                        return 0.6016854390884664;
                    } else {
                        return 0.7299388976259975;
                    }
                }
            } else {
                if features.bottom <= 0.41372549019607846 {
                    if features.top <= 0.4098039215686275 {
                        return 0.508852100462204;
                    } else {
                        return 0.77431898518854;
                    }
                } else {
                    if features.top <= 0.4176470588235295 {
                        return 0.8063226295654063;
                    } else {
                        return 0.8686042704408621;
                    }
                }
            }
        }
    }
}
