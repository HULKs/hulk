from dataclasses import dataclass
from typing import Literal

import cv2
import numpy as np
from numpy.typing import NDArray
from sklearn.tree import DecisionTreeClassifier, export_graphviz


@dataclass
class Feature:
    identifier: str
    data_type: str


def convert_pixels_BGR2bgrI(
    pixel_BGR: NDArray[np.integer],
) -> NDArray[np.floating]:
    pixel_sums = np.sum(pixel_BGR, axis=-1, keepdims=True)
    pixel_sums[pixel_sums == 0] = 1
    chromaticity = pixel_BGR / pixel_sums
    intensity = np.array(pixel_sums / 3).astype(np.uint8)
    return np.concatenate((chromaticity, intensity), axis=-1)


def optimize_thresholds(
    pixels_BGR: NDArray[np.integer],
    pixels_YCrCb: NDArray[np.integer],
    y: NDArray[np.integer],
    camera: Literal["top", "bottom"],
) -> DecisionTreeClassifier:
    pixels_bgrI = convert_pixels_BGR2bgrI(pixels_BGR)
    pixels_HSV = cv2.cvtColor(
        pixels_BGR.reshape(1, -1, 3), cv2.COLOR_BGR2HSV
    ).squeeze(axis=0)

    X = np.concatenate(
        (pixels_BGR, pixels_YCrCb, pixels_bgrI, pixels_HSV), axis=1
    )

    classifier = DecisionTreeClassifier(class_weight="balanced", max_depth=14)
    model = classifier.fit(X, y)
    features = [
        Feature("blue_luminance", "u8"),
        Feature("green_luminance", "u8"),
        Feature("red_luminance", "u8"),
        Feature("luminance", "u8"),
        Feature("red_difference", "u8"),
        Feature("blue_difference", "u8"),
        Feature("blue_chromaticity", "f32"),
        Feature("green_chromaticity", "f32"),
        Feature("red_chromaticity", "f32"),
        Feature("intensity", "u8"),
        Feature("hue", "u16"),
        Feature("saturation", "u8"),
        Feature("value", "u8"),
    ]
    labels = ["Intensity::Low", "Intensity::High"]
    rust_expression = tree_to_rust_code(model, features, labels)
    with open(f"{camera}_field_color_tree.rs", "w") as text_file:
        text_file.write(rust_expression)

    print(f"\n\n*** {camera} camera ***\n")
    print(rust_expression)
    print("*** END ***")
    return model


def tree_to_rust_code(
    clf: DecisionTreeClassifier, features: list[Feature], labels: list[str]
) -> str:
    tree = clf.tree_

    def recurse(node: int) -> str:
        if tree.children_left[node] == -1 and tree.children_right[node] == -1:
            predicted_class = int(np.argmax(tree.value[node][0]))
            return f"{labels[predicted_class]}"

        feature_index = tree.feature[node]
        threshold = tree.threshold[node]
        left_branch = recurse(tree.children_left[node])
        right_branch = recurse(tree.children_right[node])
        if features[feature_index].data_type in ["u8", "u16"]:
            threshold_str = f"{np.floor(threshold):.0f}"
        else:
            threshold_str = f"{threshold:.3f}"
        return f"""if features.{features[feature_index].identifier} <= {threshold_str} {{
            {left_branch}
            }} else {{
            {right_branch}
            }}"""

    out = "use types::color::Intensity;\n\n"
    out += "pub struct Features {\n"
    for feature in features:
        out += f"pub {feature.identifier}: {feature.data_type},\n"
    out += "}\n\n"
    out += "#[allow(clippy::collapsible_else_if)]\n"
    out += "pub fn predict(features: &Features) -> Intensity {\n"
    out += recurse(0)
    out += "\n}"
    return out


def save_tree_as_dot_file(model: DecisionTreeClassifier, filename: str) -> None:
    feature_names = [
        "B",
        "G",
        "R",
        "Y",
        "Cr",
        "Cb",
        "b",
        "g",
        "r",
        "I",
        "H",
        "S",
        "V",
    ]
    export_graphviz(model, f"{filename}.dot", feature_names=feature_names)


def predict_image(
    image_CrCbY: NDArray[np.integer], model: DecisionTreeClassifier
) -> None:
    M, N, _ = image_CrCbY.shape
    image_YCrCb = image_CrCbY[..., [2, 0, 1]]
    image_BGR = cv2.cvtColor(image_YCrCb, cv2.COLOR_YCrCb2BGR)
    image_bgrI = convert_pixels_BGR2bgrI(image_BGR.reshape((M * N, -1)))
    image_HSV = cv2.cvtColor(image_BGR, cv2.COLOR_BGR2HSV)

    X = np.concatenate(
        (
            image_BGR.reshape((M * N, -1)),
            image_YCrCb.reshape((M * N, -1)),
            image_bgrI,
            image_HSV.reshape((M * N, -1)),
        ),
        axis=1,
    )
    pred = model.predict(X)
    pred = (
        cv2.cvtColor(
            (np.resize(pred, (M, N))).astype(np.uint8), cv2.COLOR_GRAY2BGR
        )
        * 255
    )
    cv2.imshow("Prediction", pred)
    cv2.waitKey()
    cv2.destroyAllWindows()
