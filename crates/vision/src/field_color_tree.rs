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
        if features.center <= 0.3901960784313726 {
            if features.left <= 0.37843137254901965 {
                if features.right <= 0.3745098039215687 {
                    if features.center <= 0.2843137254901961 {
                        return 0.12670732446944305;
                    } else {
                        return 0.12085729565017765;
                    }
                } else {
                    if features.bottom <= 0.3745098039215687 {
                        return 0.12606780109742372;
                    } else {
                        return 0.1411418190098626;
                    }
                }
            } else {
                if features.right <= 0.3823529411764706 {
                    if features.center <= 0.3745098039215687 {
                        return 0.1291684731164999;
                    } else {
                        return 0.1521390237672899;
                    }
                } else {
                    if features.bottom <= 0.37843137254901965 {
                        return 0.1463740021113685;
                    } else {
                        return 0.22955991873174758;
                    }
                }
            }
        } else {
            if features.left <= 0.38627450980392164 {
                if features.right <= 0.3823529411764706 {
                    return 0.1320429616950684;
                } else {
                    if features.bottom <= 0.37843137254901965 {
                        return 0.16395749729151257;
                    } else {
                        return 0.23737927392298988;
                    }
                }
            } else {
                if features.right <= 0.38627450980392164 {
                    if features.bottom <= 0.3823529411764706 {
                        return 0.16650039388812346;
                    } else {
                        return 0.23987460675760636;
                    }
                } else {
                    if features.top <= 0.47647058823529415 {
                        return 0.5903058660876411;
                    } else {
                        return 0.2954125383839439;
                    }
                }
            }
        }
    } else {
        if features.left <= 0.41372549019607846 {
            if features.right <= 0.4098039215686275 {
                if features.bottom <= 0.3941176470588236 {
                    if features.top <= 0.39803921568627454 {
                        return 0.14296937360215986;
                    } else {
                        return 0.1946692992398949;
                    }
                } else {
                    if features.left <= 0.3941176470588236 {
                        return 0.2320232441026442;
                    } else {
                        return 0.4117564768796564;
                    }
                }
            } else {
                if features.bottom <= 0.4058823529411765 {
                    if features.top <= 0.4058823529411765 {
                        return 0.24049803718562876;
                    } else {
                        return 0.5409598525038413;
                    }
                } else {
                    if features.center <= 0.47647058823529415 {
                        return 0.6186181061526359;
                    } else {
                        return 0.7395206825626245;
                    }
                }
            }
        } else {
            if features.right <= 0.4176470588235295 {
                if features.bottom <= 0.4058823529411765 {
                    if features.top <= 0.4058823529411765 {
                        return 0.247792355640504;
                    } else {
                        return 0.5523424285235968;
                    }
                } else {
                    if features.top <= 0.4098039215686275 {
                        return 0.5850320006502542;
                    } else {
                        return 0.7364418654908175;
                    }
                }
            } else {
                if features.bottom <= 0.4098039215686275 {
                    if features.top <= 0.4098039215686275 {
                        return 0.5090412848567579;
                    } else {
                        return 0.7864760831905675;
                    }
                } else {
                    if features.top <= 0.4176470588235295 {
                        return 0.8112097262720579;
                    } else {
                        return 0.8705955721367864;
                    }
                }
            }
        }
    }
}
