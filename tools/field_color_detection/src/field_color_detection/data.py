import io
import os
import random
from pathlib import Path

import cv2
import h5py
import matplotlib.pyplot as plt
import numpy as np
import pandas as pd
from numpy.typing import NDArray
from PIL import Image

from .settings import (
    HEIGHT,
    WIDTH,
    Classes,
    ColorValues,
    FeatureIndices,
)


def create_sampling_masks(y_train, y_test, ratio_ones):
    random_generator = np.random.default_rng(seed=42)
    samples_count_train = np.count_nonzero(y_train != Classes.UNKNOWN.value)
    train_mask = random_generator.choice(
        [0, 1], size=samples_count_train, p=[1 - ratio_ones, ratio_ones]
    )
    samples_count_test = np.count_nonzero(y_test != Classes.UNKNOWN.value)
    test_mask = random_generator.choice(
        [0, 1], size=samples_count_test, p=[1 - ratio_ones, ratio_ones]
    )

    np.savez("masks.npz", train_mask=train_mask, test_mask=test_mask)


def load_sampling_masks(mask_file_path):
    data = np.load(mask_file_path)

    return data["train_mask"].astype(np.uint8), data["test_mask"].astype(
        np.uint8
    )


def check_memory(min_available_gb: float = 1.0):
    import optuna
    import psutil

    mem = psutil.virtual_memory()
    if mem.available < min_available_gb * (1024**3):
        print("--> ERROR: No RAM!!!")
        raise optuna.exceptions.TrialPruned()


def load_file(filepath: Path) -> tuple[NDArray[np.uint8], NDArray[np.uint8]]:
    float_features = [
        FeatureIndices.b.value,
        FeatureIndices.g.value,
        FeatureIndices.r.value,
    ]
    with h5py.File(filepath, "r") as db:
        X = np.array(db["data"], dtype=np.float32)
        y = np.array(db["labels"], dtype=np.uint8)
        X[:, float_features] = np.floor(X[:, float_features] * 255)
        return X.astype(np.uint8), y


def get_data_from_hdf5(
    *filepaths: Path,
) -> tuple[NDArray[np.uint8], NDArray[np.uint8]]:
    data = [load_file(path) for path in filepaths]
    X_all, y_all = zip(*data)
    return np.concatenate(X_all, axis=0), np.concatenate(y_all, axis=0)


def read_YCrCb_image(filepath: str) -> NDArray:
    image_CrCbY = cv2.imread(filepath)
    return image_CrCbY[..., [2, 0, 1]]


def get_random_png_files(root_dir: str, num_files: int = 5000) -> list:
    png_files = []

    for dirpath, _, filenames in os.walk(root_dir):
        for filename in filenames:
            if filename.lower().endswith(".png"):
                png_files.append(os.path.join(dirpath, filename))

    return random.sample(png_files, min(len(png_files), num_files))


def determine_loss_from_JPEG_conversion(
    png_files: list, quality_levels: list[int], *, show_outliers: bool = False
) -> NDArray[np.float64]:
    quality_losses = {q: np.array([]) for q in quality_levels}
    for file_path in png_files:
        image_YCrCb = read_YCrCb_image(file_path)

        for quality in quality_levels:
            _, encoded_image = cv2.imencode(
                ".jpg", image_YCrCb, [int(cv2.IMWRITE_JPEG_QUALITY), quality]
            )
            image_YCrCb_JPEG = cv2.imdecode(encoded_image, cv2.IMREAD_COLOR)

            norm = (
                np.abs(
                    image_YCrCb.astype(np.int16)
                    - image_YCrCb_JPEG.astype(np.int16)
                )
                .reshape(HEIGHT * WIDTH * 3)
                .astype(np.uint8)
            )
            # if show_outliers and 50 in norm:
            #     cv2.imshow("original YCrCb", image_YCrCb)
            #     cv2.imshow("JPEG image", image_YCrCb_JPEG)
            #     cv2.imshow(
            #         "absolute difference",
            #         np.abs(
            #             image_YCrCb.astype(np.int16)
            #             - image_YCrCb_JPEG.astype(np.int16)
            #         ).astype(np.uint8),
            #     )
            #     cv2.waitKey()
            #     cv2.destroyAllWindows()
            quality_losses[quality] = np.append(quality_losses[quality], norm)

    plt.figure(figsize=(10, 6))
    plt.boxplot(
        [quality_losses[q] for q in quality_levels],
        patch_artist=True,
        labels=[str(q) for q in quality_levels],
    )
    plt.xlabel("JPEG Quality Level")
    plt.ylabel("Relative L1 Loss per Pixel")
    plt.title("Loss from JPEG Compression at Different Quality Levels")
    plt.show()

    return np.array([np.mean(quality_losses[q]) for q in quality_levels])


def determine_loss_from_BGR_conversion(
    png_files: list, *, show_outliers: bool = False
) -> float:
    l1_norms = np.array([])
    for file_path in png_files:
        image_YCrCb = read_YCrCb_image(file_path)
        if image_YCrCb.shape != (HEIGHT, WIDTH, 3):
            ValueError("wrong dimensions: " + str(image_YCrCb.shape))
            continue
        image_BGR = cv2.cvtColor(image_YCrCb, cv2.COLOR_YCrCb2BGR)
        image_YCrCb_2 = cv2.cvtColor(image_BGR, cv2.COLOR_BGR2YCrCb)
        norm = (
            np.abs(
                image_YCrCb.astype(np.int16) - image_YCrCb_2.astype(np.int16)
            )
            .reshape(HEIGHT * WIDTH * 3)
            .astype(np.uint8)
        )
        # if show_outliers and 50 in norm:
        #     cv2.imshow("original YCrCb", image_YCrCb)
        #     cv2.imshow("transformed BGR", image_BGR)
        #     cv2.imshow("transformed YCrCb", image_YCrCb_2)
        #     cv2.imshow(
        #         "absolute difference",
        #         np.abs(
        #             image_YCrCb.astype(np.int16)
        #             - image_YCrCb_2.astype(np.int16)
        #         ).astype(np.int8),
        #     )
        #     cv2.waitKey()
        #     cv2.destroyAllWindows()
        l1_norms = np.append(l1_norms, norm)
    plt.figure(figsize=(10, 6))
    plt.hist(
        l1_norms,
        weights=np.zeros_like(l1_norms) + 1.0 / l1_norms.size,
        color="blue",
        bins=int(np.max(l1_norms)),
        alpha=0.7,
        edgecolor="black",
        log=True,
    )
    plt.xlabel("L1 Loss per Pixel")
    plt.ylabel("Relative Frequency")

    plt.tight_layout()
    plt.show()
    return sum(l1_norms) / len(l1_norms)


def convert_YCrCb_to_BGR(image: NDArray) -> NDArray:
    return cv2.cvtColor(image, cv2.COLOR_YCrCb2BGR)


def convert_BGR_to_bgrI(image: NDArray) -> NDArray:
    pixel_sums = np.sum(image, axis=-1, keepdims=True)
    pixel_sums[pixel_sums == 0] = 1
    chromaticity = image / pixel_sums
    intensity = np.array(pixel_sums / 3).astype(np.uint8)
    return np.concatenate((chromaticity, intensity), axis=-1)


def convert_BGR_to_YCrCb(image: NDArray) -> NDArray:
    return cv2.cvtColor(image, cv2.COLOR_BGR2YCR_CB)


def convert_BGR_to_Lab(image: NDArray) -> NDArray:
    return cv2.cvtColor(image, cv2.COLOR_BGR2Lab)


def convert_BGR_to_HSV(image: NDArray) -> NDArray:
    return cv2.cvtColor(image, cv2.COLOR_BGR2HSV)


def create_random_vector(
    length: int, ones_percentage: float
) -> NDArray[np.int8]:
    num_ones = int(ones_percentage * length)
    num_zeros = length - num_ones
    vector = [0] * num_zeros + [1] * num_ones
    np.random.shuffle(vector)
    return np.array(vector)


def generate_hdf5_from_images(
    path_to_masks: str,
    path_to_YCrCb_images: str,
    target_path: str,
    path_to_RGB_images: str = "",
    *,
    split: bool = False,
) -> None:
    def append_to_dataset(dataset, data, start_idx):
        end_idx = start_idx + len(data)
        dataset.resize(end_idx, axis=0)
        dataset[start_idx:end_idx] = data
        return end_idx

    number_samples = len(os.listdir(path_to_masks))
    train_test_split = (
        np.zeros(number_samples)
        if not split
        else create_random_vector(number_samples, 0.2)
    )

    file_train = h5py.File(os.path.join(target_path, "dataset_train.hdf5"), "w")
    file_test = (
        h5py.File(os.path.join(target_path, "dataset_test.hdf5"), "w")
        if split
        else None
    )

    data_shape = (0, 16)
    label_shape = (0,)
    maxshape = (None, 16)
    maxshape_label = (None,)

    dset_train_data = file_train.create_dataset(
        "data", shape=data_shape, maxshape=maxshape, chunks=True
    )
    dset_train_labels = file_train.create_dataset(
        "labels", shape=label_shape, maxshape=maxshape_label, chunks=True
    )

    if split:
        dset_test_data = file_test.create_dataset(
            "data", shape=data_shape, maxshape=maxshape, chunks=True
        )
        dset_test_labels = file_test.create_dataset(
            "labels", shape=label_shape, maxshape=maxshape_label, chunks=True
        )

    batch_data_train, batch_labels_train = [], []
    batch_data_test, batch_labels_test = [], []

    idx_train_data = idx_train_labels = 0
    idx_test_data = idx_test_labels = 0
    batch_size = 50

    file_list = os.listdir(path_to_masks)
    for index, filename in enumerate(file_list):
        image_YCrCb = read_YCrCb_image(
            os.path.join(path_to_YCrCb_images, filename)
        )
        rows, cols, _ = image_YCrCb.shape
        mask_BGR = cv2.imread(os.path.join(path_to_masks, filename))

        mask = np.zeros((rows, cols))
        mask += np.where(
            np.all(mask_BGR == ColorValues.FIELD_COLOR.value, axis=-1),
            Classes.FIELD.value,
            0,
        )
        mask += np.where(
            np.all(mask_BGR == ColorValues.UNKNOWN.value, axis=-1),
            Classes.UNKNOWN.value,
            0,
        )

        image_BGR = (
            cv2.imread(os.path.join(path_to_RGB_images, filename))
            if path_to_RGB_images
            else convert_YCrCb_to_BGR(image_YCrCb)
        )
        image_bgrI = convert_BGR_to_bgrI(image_BGR)
        image_HSV = convert_BGR_to_HSV(image_BGR)
        image_Lab = convert_BGR_to_Lab(image_BGR)
        image_combined = np.concatenate(
            (image_YCrCb, image_BGR, image_bgrI, image_Lab, image_HSV), axis=-1
        )

        reshaped_img = np.reshape(image_combined, (rows * cols, -1))
        reshaped_mask = np.reshape(mask, (rows * cols,))

        if train_test_split[index] == 0:
            batch_data_train.append(reshaped_img)
            batch_labels_train.append(reshaped_mask)
        else:
            batch_data_test.append(reshaped_img)
            batch_labels_test.append(reshaped_mask)

        def flush_batches(
            batch_data,
            batch_labels,
            dset_data,
            dset_labels,
            idx_data,
            idx_labels,
        ):
            stacked_data = np.vstack(batch_data)
            stacked_labels = np.hstack(batch_labels)
            idx_data = append_to_dataset(dset_data, stacked_data, idx_data)
            idx_labels = append_to_dataset(
                dset_labels, stacked_labels, idx_labels
            )
            return [], [], idx_data, idx_labels

        if len(batch_data_train) == batch_size:
            (
                batch_data_train,
                batch_labels_train,
                idx_train_data,
                idx_train_labels,
            ) = flush_batches(
                batch_data_train,
                batch_labels_train,
                dset_train_data,
                dset_train_labels,
                idx_train_data,
                idx_train_labels,
            )

        if split and len(batch_data_test) == batch_size:
            (
                batch_data_test,
                batch_labels_test,
                idx_test_data,
                idx_test_labels,
            ) = flush_batches(
                batch_data_test,
                batch_labels_test,
                dset_test_data,
                dset_test_labels,
                idx_test_data,
                idx_test_labels,
            )

    # Flush remaining
    if batch_data_train:
        (
            batch_data_train,
            batch_labels_train,
            idx_train_data,
            idx_train_labels,
        ) = flush_batches(
            batch_data_train,
            batch_labels_train,
            dset_train_data,
            dset_train_labels,
            idx_train_data,
            idx_train_labels,
        )

    if split and batch_data_test:
        batch_data_test, batch_labels_test, idx_test_data, idx_test_labels = (
            flush_batches(
                batch_data_test,
                batch_labels_test,
                dset_test_data,
                dset_test_labels,
                idx_test_data,
                idx_test_labels,
            )
        )

    file_train.close()
    if split:
        file_test.close()


def get_data_from_parquet(image_directory: str, parquet_file: str) -> None:
    df = pd.read_parquet(parquet_file)
    images = df["image"]
    for image in images:
        jpeg_bytes = image["bytes"]
        name = image["path"]
        reconstruct_jpeg(jpeg_bytes, os.path.join(image_directory, name))


def reconstruct_jpeg(jpeg_bytes: bytes, path: str) -> None:
    image = Image.open(io.BytesIO(jpeg_bytes))
    image.save(path, format="JPEG")


if __name__ == "__main__":
    # generate_hdf5_from_images(
    #     "/home/franziska-sophie/Documents/research_project/test_data/test"
    # )
    # generate_hdf5_from_images(
    #     "/home/franziska-sophie/Documents/research_project/test_data/train"
    # )

    # images_path = "/home/franziska-sophie/Documents/Datasets/[028]-[049]"
    # png_files = get_random_png_files(images_path, num_files=100)
    # print(
    #     determine_loss_from_JPEG_conversion(
    #         png_files, [10, 20, 30, 40, 50, 60, 70, 80, 90, 100]
    #     )
    # )
    # print(determine_loss_from_BGR_conversion(png_files))

    generate_hdf5_from_images(
        "/home/franziska-sophie/image-segmentation/dataset/labels_binary",
        "/home/franziska-sophie/image-segmentation/dataset/YCbCr",
        "/home/franziska-sophie/image-segmentation/dataset",
        "/home/franziska-sophie/image-segmentation/dataset/RGB",
        split=True,
    )
