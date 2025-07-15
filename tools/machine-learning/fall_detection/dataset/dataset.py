from collections.abc import Iterable
from pathlib import Path

import numpy as np
import polars as pl
import polars.selectors as cs
import tensorflow as tf
from polars._typing import IntoExpr

from .pseudo_labels import Label, PseudoLabeller


class FallenDataset:
    dataframe: pl.DataFrame
    element_spec: list[tf.TensorSpec]
    input_data: pl.DataFrame
    labels: pl.DataFrame
    groups: pl.DataFrame
    features: Iterable[IntoExpr]

    def __init__(
        self,
        dataframe: pl.DataFrame,
        *,
        group_keys: list[pl.Expr],
        features: Iterable[IntoExpr],
    ) -> None:
        self.dataframe = dataframe.drop_nulls()

        self.labeller = PseudoLabeller()
        self.features = features

        number_of_nulls = (
            self.dataframe.select(features)
            .select(pl.sum_horizontal(pl.all().is_null().sum()))
            .item()
        )
        if number_of_nulls > 0:
            raise Exception(
                f"Null values found in features: {number_of_nulls} null values"
            )
        groups = (
            self.dataframe.select(pl.struct(group_keys).rank("dense") - 1)
            .to_series()
            .rename("group")
        )

        self.dataframe = self.dataframe.hstack(
            [self.labeller.generate_labels(self.dataframe), groups]
        )
        self.input_data = self.dataframe.select(features)
        self.labels = self.dataframe.select("labels")
        self.element_spec = tf.TensorSpec(shape=self.input_data.shape[1])

    def to_windowed(
        self,
        control_frequency: float = 83,
        window_size: float = 1.5,
        window_stride: float = 1 / 83,
        label_shift: int = 0,
    ) -> None:
        samples_per_window = int(window_size * control_frequency)
        self.samples_per_window = samples_per_window
        samples_between_windows = int(window_stride * control_frequency)

        windowed_features = pl.concat(
            [
                self.dataframe.select(
                    generate_lags(feature, samples_per_window, "group"),
                )[::samples_between_windows]
                for feature in self.features
            ],
            how="horizontal",
        )
        shifted_labels = pl.concat(
            [
                self.dataframe.select(
                    generate_shifts(
                        pl.col("labels"), label_shift, "group", "labels"
                    ),
                )[::samples_between_windows]
            ],
            how="horizontal",
        )

        windowed_filtered_dataframe = (
            windowed_features.hstack(shifted_labels)
            .drop_nulls()
            .sample(fraction=1, shuffle=True, seed=1)
        )

        train_test_ratio = 0.8
        number_of_windows = len(windowed_filtered_dataframe)
        split_index = int(number_of_windows * train_test_ratio)
        self.train_windowed_filtered_dataframe = windowed_filtered_dataframe[
            :split_index:
        ]

        self.test_windowed_filtered_dataframe = windowed_filtered_dataframe[
            split_index + 1 :
        ]

        windowed_dataframe = filter_with_labels_predecessor(
            self.train_windowed_filtered_dataframe, label_shift
        )

        balanced_dataframe = do_class_balancing(
            windowed_dataframe, label_shift - 1
        )

        self.input_data = balanced_dataframe.select(
            generate_selector_up_to_index(self.features, label_shift)
        )

        self.labels = balanced_dataframe.select(
            cs.starts_with("labels") & cs.ends_with("_" + str(label_shift))
        )

        cache_path = Path("./.cache")
        cache_path.mkdir(exist_ok=True)
        self.train_windowed_filtered_dataframe.write_parquet(
            cache_path.joinpath("/train_windowed_filtered_dataframe.parquet")
        )
        self.test_windowed_filtered_dataframe.write_parquet(
            cache_path.joinpath("/test_windowed_filtered_dataframe.parquet")
        )

    def __len__(self) -> int:
        return self.groups.unique().numel()

    def n_features(self) -> int:
        return self.input_data.size(1)

    def n_classes(self) -> int:
        return len(self.labeller.label_type)

    def __getitem__(self, index: int) -> tuple[tf.Tensor, tf.Tensor]:
        mask = self.groups == index
        return self.input_data[mask], self.labels[mask]


def get_input_tensor(
    input_data: pl.DataFrame, samples_per_window: int, features: list[pl.Expr]
) -> tf.Tensor:
    num_windows = len(input_data)
    window_length = samples_per_window
    num_features = len(features)
    return tf.convert_to_tensor(
        np.stack([input_data.to_numpy()]).reshape(
            (num_windows, window_length, num_features)
        ),
        dtype=tf.float32,
    )


def get_input_tensor_up_to_shift(
    windowed_filtered_dataframe: pl.DataFrame,
    features: list[pl.Expr],
    samples_per_window: int,
    label_shift: int,
) -> tf.Tensor:
    filtered_windowed_dataframe = filter_with_labels_predecessor(
        windowed_filtered_dataframe, label_shift
    )

    shuffled_balanced_df = do_class_balancing(
        filtered_windowed_dataframe, label_shift
    )

    input_data = shuffled_balanced_df.select(
        generate_selector_up_to_index(features, samples_per_window)
    )

    num_windows = len(input_data)
    window_length = samples_per_window
    num_features = len(features)
    return tf.convert_to_tensor(
        np.stack([input_data.to_numpy()]).reshape(
            (num_windows, window_length, num_features)
        ),
        dtype=tf.float32,
    )


def get_labels_tensor(labels: pl.DataFrame) -> tf.Tensor:
    return tf.convert_to_tensor(labels.to_numpy(), dtype=tf.float32)


def get_labels_tensor_up_to_shift(
    windowed_filtered_dataframe: pl.DataFrame,
    label_shift: int,
) -> tf.Tensor:
    filtered_windowed_dataframe = filter_with_labels_predecessor(
        windowed_filtered_dataframe, label_shift
    )

    shuffled_balanced_df = do_class_balancing(
        filtered_windowed_dataframe, label_shift
    )

    labels_at_shift_index = shuffled_balanced_df.select(
        cs.starts_with("labels") & cs.ends_with("_" + str(label_shift))
    )
    return tf.convert_to_tensor(
        labels_at_shift_index.to_numpy(), dtype=tf.float32
    )


def generate_lags(feature: pl.Expr, lags: int, group: str) -> list[pl.Expr]:
    return [
        feature.shift(i).over(group).name.suffix("_" + str(i))
        for i in range(lags)
    ]


def generate_shifts(
    feature: pl.Expr, shifts: int, group: str, alias: str
) -> list[pl.Expr]:
    return [
        feature.shift(-i).over(group).alias(alias + "_" + str(i))
        for i in range(shifts)
    ]


def filter_with_labels_predecessor(
    windowed_dataframe: pl.DataFrame, label_shift: int
) -> pl.DataFrame:
    return windowed_dataframe.filter(
        pl.col("labels" + "_" + str(label_shift - 1)) == Label.Stable
    )


def do_class_balancing(
    filtered_windowed_dataframe: pl.DataFrame, index: int
) -> pl.DataFrame:
    n_minority = (
        filtered_windowed_dataframe.get_column("labels" + "_" + str(index))
        .value_counts()
        .min()
        .select(pl.col("count"))
        .item()
    )
    return filtered_windowed_dataframe.group_by(
        "labels" + "_" + str(index), maintain_order=True
    ).map_groups(lambda group: group.sample(n=n_minority, seed=1))


def generate_selector_up_to_index(features: list[pl.Expr], index: int) -> list:
    return [
        cs.starts_with(feature.meta.output_name()) & cs.ends_with("_" + str(i))
        for feature in features
        for i in range(index)
    ]
