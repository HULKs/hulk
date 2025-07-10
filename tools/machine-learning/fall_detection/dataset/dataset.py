from typing import Iterable
import tensorflow as tf
import numpy as np
import polars as pl
import pandas as pd
from polars._typing import IntoExpr

from .pseudo_labels import PseudoLabeller


class FallenDataset:
    dataframe: pl.DataFrame
    element_spec: list[tf.TensorSpec]
    input_data: pl.DataFrame
    labels: pl.DataFrame
    groups: pl.DataFrame
    features: Iterable[IntoExpr] | IntoExpr

    def __init__(
        self,
        dataframe: pl.DataFrame,
        *,
        group_keys: list[str],
        features: Iterable[IntoExpr] | IntoExpr,
    ) -> None:
        self.dataframe = dataframe.drop_nulls()[:10000]

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
        window_stride: float = 0.2,
        label_shift: int = 0,
    ) -> None:
        samples_per_window = int(window_size * control_frequency)
        self.samples_per_window = samples_per_window
        samples_between_windows = int(window_stride * control_frequency)

        windowed_features = pl.concat(
            [
                self.dataframe.select(
                    generate_lags(feature, samples_per_window, "group"),
                )[
                    samples_per_window : -label_shift
                    or None : samples_between_windows
                ]
                for feature in self.features
            ],
            how="horizontal",
        )

        shifted_labels = self.dataframe.select(
            pl.col("labels").shift(-label_shift).over("group")
        )[samples_per_window : -label_shift or None : samples_between_windows]

        windowed_dataframe = windowed_features.hstack(shifted_labels)

        n_minority = (
            windowed_dataframe.get_column("labels")
            .value_counts()
            .min()
            .select(pl.col("count"))
            .item()
        )

        balanced_df = windowed_dataframe.group_by(
            "labels", maintain_order=True
        ).map_groups(lambda group: group.sample(n=n_minority, seed=1))
        balanced_df = windowed_dataframe.sample(
            fraction=1, shuffle=True, seed=1
        )

        self.input_data = balanced_df.drop("labels")
        self.labels = balanced_df.select(pl.col("labels"))

    def __len__(self) -> int:
        return self.groups.unique().numel()

    def n_features(self) -> int:
        return self.input_data.size(1)

    def n_classes(self) -> int:
        return len(self.labeller.label_type)

    def __getitem__(self, index: int) -> tuple[tf.Tensor, tf.Tensor]:
        mask = self.groups == index
        return self.input_data[mask], self.labels[mask]

    def get_input_tensor(self) -> tf.Tensor:
        num_windows = len(self.input_data)
        window_length = self.samples_per_window
        num_features = len(self.features)
        return tf.convert_to_tensor(
            np.stack([self.input_data.to_numpy()]).reshape(
                (num_windows, window_length, num_features)
            ),
            dtype=tf.float32,
        )

    def get_labels_tensor(self) -> tf.Tensor:
        return tf.convert_to_tensor(self.labels.to_numpy(), dtype=tf.float32)

    def get_windows_input_tensor(self) -> list[tf.Tensor]:
        windowed_input_tensor = []
        for input_window in self.input_data.iter():
            windowed_input_tensor.append(
                tf.convert_to_tensor(input_window.to_pandas())
            )
        return windowed_input_tensor


def generate_lags(feature: pl.Expr, lags: int, group: str) -> list[pl.Expr]:
    return [
        feature.shift(i).over(group).name.suffix(str(i)) for i in range(lags)
    ]
