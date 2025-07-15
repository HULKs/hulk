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

#[allow(clippy::collapsible_else_if)]
pub fn predict(features: &Features) -> f32 {
    if features.center <= 0.4058823529411765 {
        if features.center <= 0.3823529411764706 {
            if features.left <= 0.37843137254901965 {
                if features.right <= 0.3705882352941177 {
                    if features.bottom <= 0.37843137254901965 {
                        return 0.12151206420015728;
                    } else {
                        return 0.12911391606990508;
                    }
                } else {
                    if features.bottom <= 0.3705882352941177 {
                        return 0.12960328016008227;
                    } else {
                        return 0.14884866002749184;
                    }
                }
            } else {
                if features.right <= 0.3823529411764706 {
                    if features.center <= 0.303921568627451 {
                        return 0.18049986721198988;
                    } else {
                        return 0.1354889695301491;
                    }
                } else {
                    if features.bottom <= 0.3823529411764706 {
                        return 0.16114637057624703;
                    } else {
                        return 0.2825189618887777;
                    }
                }
            }
        } else {
            if features.right <= 0.3823529411764706 {
                if features.left <= 0.3745098039215687 {
                    return 0.1372970959505043;
                } else {
                    if features.right <= 0.3745098039215687 {
                        return 0.19121866131104442;
                    } else {
                        return 0.310507378686262;
                    }
                }
            } else {
                if features.left <= 0.37843137254901965 {
                    if features.bottom <= 0.3745098039215687 {
                        return 0.17237134521306802;
                    } else {
                        return 0.23899852192474358;
                    }
                } else {
                    if features.bottom <= 0.37843137254901965 {
                        return 0.2927700756538762;
                    } else {
                        return 0.6074003608102483;
                    }
                }
            }
        }
    } else {
        if features.right <= 0.4098039215686275 {
            if features.left <= 0.3941176470588236 {
                if features.bottom <= 0.3901960784313726 {
                    if features.top <= 0.3901960784313726 {
                        return 0.15452893377239668;
                    } else {
                        return 0.2157351413037524;
                    }
                } else {
                    if features.right <= 0.3901960784313726 {
                        return 0.25799813590404475;
                    } else {
                        return 0.3929344234773921;
                    }
                }
            } else {
                if features.bottom <= 0.3941176470588236 {
                    if features.top <= 0.3941176470588236 {
                        return 0.2523341789785405;
                    } else {
                        return 0.5399912804486535;
                    }
                } else {
                    if features.top <= 0.39803921568627454 {
                        return 0.5581550841324332;
                    } else {
                        return 0.7118083511490878;
                    }
                }
            }
        } else {
            if features.left <= 0.4098039215686275 {
                if features.bottom <= 0.40196078431372556 {
                    if features.top <= 0.40196078431372556 {
                        return 0.2793070258778932;
                    } else {
                        return 0.5828927484546683;
                    }
                } else {
                    if features.top <= 0.4058823529411765 {
                        return 0.6111915778071642;
                    } else {
                        return 0.7312411697996828;
                    }
                }
            } else {
                if features.bottom <= 0.4098039215686275 {
                    if features.top <= 0.40196078431372556 {
                        return 0.5079792674717701;
                    } else {
                        return 0.7851108922964548;
                    }
                } else {
                    if features.top <= 0.41372549019607846 {
                        return 0.8022759686527685;
                    } else {
                        return 0.8693202689152983;
                    }
                }
            }
        }
    }
}
