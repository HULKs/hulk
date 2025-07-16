import os
import time
import warnings
from pathlib import Path
from typing import get_args

import matplotlib.pyplot as plt
import numpy as np
import optuna
from lightgbm import LGBMClassifier
from numpy.typing import NDArray
from sklearn import metrics, svm
from sklearn.base import ClassifierMixin
from sklearn.neural_network import MLPClassifier
from sklearn.pipeline import make_pipeline
from sklearn.preprocessing import StandardScaler

from .gabor_filters import GaborFilter
from .histogram_of_oriented_gradients import HoGFilter
from .local_binary_pattern import LBPFilter
from .neighboring_pixels import NeighboringPixels
from .nystroem import NystroemApprox
from .settings import (
    HEIGHT,
    MAX_N_JOBS,
    SHOW_PREDICTED_IMGS,
    WIDTH,
    Classes,
    Classifiers,
    FeatureIndices,
    TextureMethods,
)


def print_duration(start: float, end: float, process: str = "") -> None:
    elapsed = end - start
    minutes = int(elapsed // 60)
    seconds = int(elapsed % 60)

    print(f"---------> {process} took: {minutes}m {seconds}s")


def f2_score(y_true: NDArray[np.int8], y_pred: NDArray[np.int8]) -> float:
    return metrics.fbeta_score(y_true, y_pred, beta=2)


class Objective:
    def __init__(
        self,
        train_data: tuple[NDArray[np.float32], NDArray[np.float32]],
        val_data: tuple[NDArray[np.float32], NDArray[np.float32]],
        classifier_name: Classifiers,
        train_mask: NDArray[np.uint8] = None,
        val_mask: NDArray[np.uint8] = None,
    ) -> None:
        self.classifier_name = classifier_name
        self.train_data = train_data
        self.val_data = val_data
        self.train_mask = train_mask
        self.val_mask = val_mask

    def __call__(self, trial: optuna.Trial) -> float:
        X_train, y_train = self.train_data
        X_val, y_val = self.val_data

        indices = [
            feature.value
            for feature in FeatureIndices
            if trial.suggest_categorical(f"color_{feature.name}", [True, False])
        ]

        if len(indices) == 0:
            return 0

        start = time.time()
        X_train, y_train_binary = self.prepocess_data(X_train, y_train, indices)
        X_val, y_val_binary = self.prepocess_data(X_val, y_val, indices)
        end = time.time()
        print_duration(start, end, "Data Pre-Processing")

        selected_color_channel = trial.suggest_int(
            "selected_channel_index", 0, len(indices) - 1
        )
        selected_texture_method = trial.suggest_categorical(
            "texture_method", get_args(TextureMethods)
        )

        start = time.time()
        texture_features_train, texture_features_val = (
            self.get_texture_features(
                np.reshape(X_train, (-1, HEIGHT, WIDTH, len(indices))),
                np.reshape(X_val, (-1, HEIGHT, WIDTH, len(indices))),
                selected_texture_method,
                selected_color_channel,
                trial=trial,
            )
        )
        end = time.time()
        print_duration(start, end, "Texture Feature Extraction")

        X_train = np.concatenate(
            (X_train, texture_features_train), axis=-1
        ).astype(np.float32)
        X_val = np.concatenate((X_val, texture_features_val), axis=-1).astype(
            np.float32
        )
        X_train_binary = X_train[y_train != Classes.UNKNOWN.value]
        X_val_binary = X_val[y_val != Classes.UNKNOWN.value]
        if self.train_mask is not None and self.val_mask is not None:
            X_train_binary = X_train_binary[self.train_mask == 1]
            y_train_binary = y_train_binary[self.train_mask == 1]

            X_val_binary = X_val_binary[self.val_mask == 1]
            y_val_binary = y_val_binary[self.val_mask == 1]

        classifier = self.get_classifier()
        start = time.time()
        model = classifier.fit(X_train_binary, y_train_binary)
        end = time.time()
        print_duration(start, end, "Model Training")

        start = time.time()

        if self.classifier_name == "DecisionTree":
            with warnings.catch_warnings():
                warnings.simplefilter("ignore")
                prediction = model.predict(X_val_binary)
        else:
            prediction = model.predict(X_val_binary)
        end = time.time()
        print_duration(start, end, "Inference")

        if SHOW_PREDICTED_IMGS:
            output_directory = Path("predictions")
            output_directory.mkdir(exist_ok=True)

            if self.classifier_name == "DecisionTree":
                with warnings.catch_warnings():
                    warnings.simplefilter("ignore")
                    prediction_imgs = model.predict(X_val)
            else:
                prediction_imgs = model.predict(X_val)

            prediction_imgs = np.reshape(prediction_imgs, (-1, HEIGHT, WIDTH))
            y_val_imgs = np.reshape(y_val, (-1, HEIGHT, WIDTH))

            prediction_imgs = prediction_imgs[:12]
            y_val_imgs = y_val_imgs[:12]

            visualizations = []
            for pred, true in zip(prediction_imgs, y_val_imgs, strict=False):
                visualization = np.zeros((HEIGHT, WIDTH, 3), dtype=np.uint8)

                tp = (pred == Classes.FIELD.value) & (
                    (true == Classes.FIELD.value)
                    | (true == Classes.UNKNOWN.value)
                )
                tn = (pred == Classes.NOT_FIELD.value) & (
                    (true == Classes.NOT_FIELD.value)
                    | (true == Classes.UNKNOWN.value)
                )
                fn = (pred == Classes.NOT_FIELD.value) & (
                    true == Classes.FIELD.value
                )
                fp = (pred == Classes.FIELD.value) & (
                    true == Classes.NOT_FIELD.value
                )

                visualization[tp] = [255, 255, 255]  # White
                visualization[tn] = [0, 0, 0]  # Black
                visualization[fn] = [0, 0, 255]  # Blue
                visualization[fp] = [255, 0, 0]  # Red

                visualizations.append(visualization)

            # Plot the visualizations
            fig, axes = plt.subplots(3, 4, figsize=(12, 9))
            for i, ax in enumerate(axes.flat):
                ax.imshow(visualizations[i])
                ax.axis("off")

            plt.tight_layout()
            new_img_name = (
                f"predictions_{trial.study.study_name}_{trial.number}.png"
            )
            new_img_path = os.path.join(output_directory, new_img_name)
            plt.savefig(new_img_path, bbox_inches="tight")
            plt.close(fig)

        return f2_score(y_val_binary, prediction), len(indices)

    def get_texture_features(
        self,
        images_train: NDArray[np.uint8],
        images_val: NDArray[np.uint8],
        selected_texture_feature: TextureMethods,
        selected_channel_index: int,
        trial: optuna.Trial = None,
        params: dict | None = None,
    ) -> tuple[NDArray[np.uint8], NDArray[np.uint8]]:
        def get_param(
            name: str, method: str, *args: int | float
        ) -> int | float:
            if trial:
                suggest_fn = getattr(trial, method)
                return suggest_fn(name, *args)
            if params and name in params:
                return params[name]
            msg = f"Missing parameter: {name}"
            raise ValueError(msg)

        match selected_texture_feature:
            case "HoG":
                orientations = get_param(
                    "HoG_orientations", "suggest_int", 4, 8
                )
                cells_per_block = get_param(
                    "HoG_cells_per_block", "suggest_int", 2, 4
                )
                extractor = HoGFilter(
                    orientations,
                    (8, 8),
                    (cells_per_block, cells_per_block),
                )
            case "Neighbors":
                radius = get_param("Neighbors_radius", "suggest_int", 1, 31)
                orientations = get_param(
                    "Neighbors_orientations", "suggest_int", 4, 8
                )
                extractor = NeighboringPixels(radius, orientations)
            case "NeighborsDifference":
                radius = get_param("Neighbors_radius", "suggest_int", 1, 31)
                orientations = get_param(
                    "Neighbors_orientations", "suggest_int", 4, 8
                )
                extractor = NeighboringPixels(
                    radius, orientations, "difference"
                )
            case "LBP":
                radius = get_param("LBP_radius", "suggest_int", 1, 31)
                extractor = LBPFilter(radius)
            case "GaborFilters":
                ksize = 2 * get_param("Gabor_ksize", "suggest_int", 1, 50) + 1
                sigma = get_param("Gabor_sigma", "suggest_float", 1, 10)
                lambd = get_param("Gabor_lambda", "suggest_float", 0.05, 0.5)
                gamma = get_param("Gabor_gamma", "suggest_float", 0.3, 1)
                phi = get_param("Gabor_phi", "suggest_float", 0, 2 * np.pi)
                orientations = get_param(
                    "Gabor_orientations", "suggest_int", 4, 8
                )
                extractor = GaborFilter(
                    ksize, sigma, lambd, gamma, phi, orientations
                )
            case _:
                msg = f"Unknown texture feature: {selected_texture_feature}"
                raise ValueError(msg)

        features_train = extractor.get_features(
            images_train, selected_channel_index
        )
        features_val = extractor.get_features(
            images_val, selected_channel_index
        )
        return features_train.astype(np.float32), features_val.astype(
            np.float32
        )

    def get_classifier(self) -> ClassifierMixin:
        classifiers = {
            "LinearSVM": svm.LinearSVC(class_weight="balanced"),
            # "LinearSVM": make_pipeline(StandardScaler(), LinearSVM()),
            "MLPClassifier": make_pipeline(
                StandardScaler(),
                MLPClassifier(
                    hidden_layer_sizes=(128, 64),
                    activation="relu",
                    solver="adam",
                ),
            ),
            "DecisionTree": LGBMClassifier(
                objective="binary",
                n_estimators=1,
                n_jobs=MAX_N_JOBS,
                learning_rate=1.0,
                class_weight="balanced",
                verbose=-1,
            ),
            "NystroemRBF": NystroemApprox(
                kernel="rbf",
                gamma=0.01,
                n_components=50,
                n_jobs=1,
                class_weight="balanced",
                batch_size=1000,
                sample_size=100000,
            ),
            "NystroemPolynomial": NystroemApprox(
                kernel="polynomial",
                gamma=0.01,
                n_components=50,
                n_jobs=1,
                class_weight="balanced",
                batch_size=1000,
                sample_size=100000,
                degree=3,
            ),
            # "RBFSampler": make_pipeline(
            #     RBFSampler(gamma=0.01, random_state=1),
            #     svm.LinearSVC(class_weight="balanced"),
            # ),
            # "PolynomialCountSketch": make_pipeline(
            #     PolynomialCountSketch(degree=2),
            #     svm.LinearSVC(class_weight="balanced"),
            # ),
        }
        return classifiers[self.classifier_name]

    def get_param_grid(self) -> dict:
        params = {
            "LinearSVM": {
                "C": [1e-4, 1e-3, 1e-2, 1e-1, 1e0, 1e1, 1e2],
            },
            "MLPClassifier": {
                "alpha": [1e-4, 1e-3, 1e-2, 1e-1, 1e0, 1e1],
            },
            "DecisionTree": {
                "max_depth": [2, 3, 5, 10, 20],
                "min_child_samples": [5, 10, 20, 50, 100],
            },
            "NystroemRBF": {
                "alpha": [1e-4, 1e-3, 1e-2, 1e0, 1e1],
                "gamma": [1e-5, 1e-4, 1e-3, 1e-2, 1e-1],
                "n_components": [50, 75, 100],
            },
            "NystroemPolynomial": {
                "alpha": [1e-4, 1e-3, 1e-2, 1e0, 1e1],
                "gamma": [1e-5, 1e-4, 1e-3, 1e-2, 1e-1],
                "n_components": [50, 75, 100],
            },
            # "Nystroem": {
            #     "C": [0.01, 0.1, 1, 10, 100],
            #     "gamma": [0.001, 0.0001],
            # },
            # "RBFSampler": {
            #     "C": [0.01, 0.1, 1, 10, 100],
            #     "gamma": [0.001, 0.0001],
            # },
            # "PolynomialCountSketch": {
            #     "C": [0.01, 0.1, 1, 10, 100],
            #     "degree": [1, 2, 3, 4, 5],
            # },
        }
        return params[self.classifier_name]

    def prepocess_data(
        self,
        X: NDArray[np.float32],
        y: NDArray[np.float32],
        indices: NDArray[np.uint8],
    ) -> tuple[NDArray[np.uint8], NDArray[np.uint8]]:
        X = X[:, indices].astype(np.uint8)
        # images = np.reshape(X, (-1, HEIGHT, WIDTH, len(indices)))
        y_binary = y[y != Classes.UNKNOWN.value].astype(np.uint8)
        return X, y_binary
