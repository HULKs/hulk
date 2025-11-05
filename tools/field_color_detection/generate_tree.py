from enum import Enum
from typing import Literal

import cv2
import numpy as np
from numpy.typing import NDArray
from sklearn.tree import DecisionTreeClassifier, export_graphviz

color_channels = Enum(
    "color_channel",
    [
        ("B", "blue_luminance"),
        ("G", "green_luminance"),
        ("R", "red_luminance"),
        ("Y", "luminance"),
        ("CR", "red_difference"),
        ("CB", "blue_difference"),
        ("B_C", "blue_chromaticity"),
        ("G_C", "green_chromaticity"),
        ("R_C", "red_chromaticity"),
        ("I", "intensity"),
        ("H", "hue"),
        ("S", "saturation"),
        ("V", "value"),
    ],
)

data_types = Literal["u8", "f32", "u16"]


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
        Feature(color_channels.B.value, "u8"),
        Feature(color_channels.G.value, "u8"),
        Feature(color_channels.R.value, "u8"),
        Feature(color_channels.Y.value, "u8"),
        Feature(color_channels.CR.value, "u8"),
        Feature(color_channels.CB.value, "u8"),
        Feature(color_channels.B_C.value, "f32"),
        Feature(color_channels.G_C.value, "f32"),
        Feature(color_channels.R_C.value, "f32"),
        Feature(color_channels.I.value, "u8"),
        Feature(color_channels.H.value, "u16"),
        Feature(color_channels.S.value, "u8"),
        Feature(color_channels.V.value, "u8"),
    ]
    labels = ["Intensity::Low", "Intensity::High"]
    rust_expression = tree_to_rust_code(model, features, labels)
    with open(f"field_color_tree.rs", "w") as text_file:
        text_file.write(rust_expression)

    print(f"\n\n*** camera ***\n")
    print(rust_expression)
    print("*** END ***")
    return model


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
