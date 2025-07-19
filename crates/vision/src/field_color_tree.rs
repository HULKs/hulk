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
if features.center <= 0.38627450980392164 {
if features.left <= 0.37843137254901965 {
if features.right <= 0.3705882352941177 {
if features.bottom <= 0.39803921568627454 {
return 0.12116682623258168;
} else {
return 0.12784465347975446;
}
} else {
if features.center <= 0.3705882352941177 {
return 0.12973564792927733;
} else {
return 0.15302289428583216;
}
}
} else {
if features.right <= 0.37843137254901965 {
if features.center <= 0.303921568627451 {
return 0.17272543082255798;
} else {
return 0.13206022094846728;
}
} else {
if features.bottom <= 0.37843137254901965 {
return 0.15212753723088504;
} else {
return 0.24999857056862332;
}
}
}
} else {
if features.right <= 0.3823529411764706 {
if features.left <= 0.37843137254901965 {
return 0.13442821190419835;
} else {
if features.bottom <= 0.37843137254901965 {
return 0.16967578449554185;
} else {
return 0.23894101889265834;
}
}
} else {
if features.left <= 0.3823529411764706 {
if features.bottom <= 0.37843137254901965 {
return 0.17284697746801755;
} else {
return 0.2457767941966458;
}
} else {
if features.bottom <= 0.37843137254901965 {
return 0.3012658492199801;
} else {
return 0.5964499205008726;
}
}
}
}
} else {
if features.left <= 0.41372549019607846 {
if features.right <= 0.40196078431372556 {
if features.bottom <= 0.3941176470588236 {
if features.top <= 0.3941176470588236 {
return 0.15010415460259663;
} else {
return 0.21040891818393506;
}
} else {
if features.left <= 0.3941176470588236 {
return 0.2533435647807847;
} else {
return 0.416101069835509;
}
}
} else {
if features.bottom <= 0.40196078431372556 {
if features.top <= 0.40196078431372556 {
return 0.2557250840428251;
} else {
return 0.5632677671511223;
}
} else {
if features.top <= 0.4058823529411765 {
return 0.5919476612870447;
} else {
return 0.7223066256709855;
}
}
}
} else {
if features.right <= 0.41372549019607846 {
if features.bottom <= 0.4058823529411765 {
if features.top <= 0.4058823529411765 {
return 0.2766886127748117;
} else {
return 0.5776273836602992;
}
} else {
if features.center <= 0.4607843137254902 {
return 0.6322608028508981;
} else {
return 0.7447113217587108;
}
}
} else {
if features.bottom <= 0.4098039215686275 {
if features.top <= 0.4058823529411765 {
return 0.5156280479884553;
} else {
return 0.785876202251901;
}
} else {
if features.top <= 0.41372549019607846 {
return 0.8042369938047145;
} else {
return 0.8694056947714173;
}
}
}
}
}
}