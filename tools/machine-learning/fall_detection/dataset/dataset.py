from typing import Iterable
import tensorflow as tf
import numpy as np
from tensorflow.data import Dataset
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
        stride: int = 1,
        control_frequency: float = 83,
        window_size: float = 1.5,
        window_stride: float = 0.2,
        is_state_prediction: bool = False,
    ) -> None:
        samples_per_window = int(window_size * control_frequency)
        samples_between_windows = int(window_stride * control_frequency)
        windows = (
            self.dataframe[::stride]
            .group_by("group")
            .map_groups(
                lambda group: group.with_row_index()
                .cast({"index": pl.Int32})
                .group_by_dynamic(
                    index_column="index",
                    every=f"{samples_between_windows}i",
                    period=f"{samples_per_window}i",
                )
                .agg([*self.features, pl.col("labels")])
                .drop("index")
                .filter(self.features[0].list.len() == samples_per_window)
                .select(*self.features, pl.col("labels").list.last())
            )
        )
        lowest_label_count = windows.select(
            pl.min_horizontal(
                pl.col("labels")
                .filter(pl.col("labels") == 0)
                .len()
                .alias("num_upright"),
                pl.col("labels")
                .filter(pl.col("labels") == 1)
                .len()
                .alias("num_falling"),
                pl.col("labels")
                .filter(pl.col("labels") == 2)
                .len()
                .alias("num_fallen"),
            )
        ).item()
        balanced_windows = pl.concat(
            (
                windows.filter(pl.col("labels") == 0).sample(
                    n=lowest_label_count
                ),
                windows.filter(pl.col("labels") == 1).sample(
                    n=lowest_label_count
                ),
                windows.filter(pl.col("labels") == 2).sample(
                    n=lowest_label_count
                ),
            )
        )
        balanced_windows = balanced_windows.sample(fraction=1, shuffle=True)
        print(balanced_windows)
        self.input_data = balanced_windows.select(self.features)
        self.labels = balanced_windows.select(pl.col("labels"))

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
        pd_df = self.input_data.to_pandas()
        return tf.convert_to_tensor(
            np.array(pd_df.values.tolist()), dtype=tf.float32
        )

    def get_labels_tensor(self) -> tf.Tensor:
        return tf.convert_to_tensor(self.labels.to_numpy())

    def get_windows_input_tensor(self) -> list[tf.Tensor]:
        windowed_input_tensor = []
        for input_window in self.input_data.iter():
            windowed_input_tensor.append(
                tf.convert_to_tensor(input_window.to_pandas())
            )
        return windowed_input_tensor

    # def element_spec()
