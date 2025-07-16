import argparse
import os
import time
from pathlib import Path

import joblib
import numpy as np
from field_color_detection import (
    HEIGHT,
    WIDTH,
    Classes,
    FeatureIndices,
    get_data_from_hdf5,
)
from field_color_detection.neighboring_pixels import NeighboringPixels
from lightgbm import LGBMClassifier

RADIUS = 28
ORIENTATIONS = 4
MAX_DEPTH = 5
MIN_CHILD_SAMPLES = 5


# uv run train_decision_tree.py \
# --train /home/franziska-sophie/image-segmentation/dataset/train_split.hdf5 \
#         /home/franziska-sophie/image-segmentation/dataset/dataset_fieldCenter.hdf5 \
#         /home/franziska-sophie/image-segmentation/dataset/dataset_fieldCenter.hdf5 \
#         /home/franziska-sophie/image-segmentation/dataset/dataset_fieldCenter.hdf5 \
#         /home/franziska-sophie/image-segmentation/dataset/dataset_fieldCenter.hdf5 \
#         /home/franziska-sophie/image-segmentation/dataset/dataset_fieldCenter.hdf5 \
#         /home/franziska-sophie/image-segmentation/dataset/dataset_fieldCenter.hdf5 \
#         /home/franziska-sophie/image-segmentation/dataset/dataset_fieldCenter.hdf5 \
#         /home/franziska-sophie/image-segmentation/dataset/dataset_fieldCenter.hdf5 \
#         /home/franziska-sophie/image-segmentation/dataset/dataset_fieldCenter.hdf5 \
#         /home/franziska-sophie/image-segmentation/dataset/dataset_fieldCenter.hdf5 \
#         /home/franziska-sophie/image-segmentation/dataset/dataset_fieldCenter.hdf5 \
#         /home/franziska-sophie/image-segmentation/dataset/dataset_fieldCenter.hdf5 \
#         /home/franziska-sophie/image-segmentation/dataset/dataset_fieldCenter.hdf5 \
# --model-file-name "new_DT"


def print_duration(start: float, end: float, process: str = "") -> None:
    elapsed = end - start
    minutes = int(elapsed // 60)
    seconds = int(elapsed % 60)

    print(f"---------> {process} took: {minutes}m {seconds}s")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--train",
        type=Path,
        nargs="+",
        required=True,
        help="Paths to training HDF5 file",
    )
    parser.add_argument(
        "--model-file-name",
        type=str,
        required=True,
        help="Name of the joblib file",
    )

    args = parser.parse_args()

    start = time.time()
    X, y = get_data_from_hdf5(*args.train)
    end = time.time()
    print_duration(start, end, "Reading hdf5 files")

    X = X[:, FeatureIndices.g.value]
    start = time.time()
    extractor = NeighboringPixels(radius=RADIUS, orientations=ORIENTATIONS)
    texture_features = extractor.get_features(
        np.reshape(X, (-1, HEIGHT, WIDTH, 1)), 0
    )
    end = time.time()
    print_duration(start, end, "Texture Feature Extraction")

    X = np.concatenate((X[:, np.newaxis], texture_features), axis=-1)
    X_binary = X[y != Classes.UNKNOWN.value]
    y_binary = y[y != Classes.UNKNOWN.value]

    classifier = LGBMClassifier(
        objective="binary",
        n_estimators=1,
        n_jobs=1,
        learning_rate=1.0,
        class_weight="balanced",
        verbose=-1,
        max_depth=MAX_DEPTH,
        min_child_samples=MIN_CHILD_SAMPLES,
    )

    start = time.time()
    model = classifier.fit(X_binary, y_binary)
    end = time.time()
    print_duration(start, end, "Model Training")

    os.makedirs("best_models", exist_ok=True)
    joblib.dump(
        model,
        f"best_models/{args.model_file_name}.joblib",
    )
