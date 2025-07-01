from typing import Iterable
import tensorflow as tf
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
        self.dataframe = dataframe

        self.labeller = PseudoLabeller()
        self.features = features

        number_of_nulls = (
            dataframe.select(features)
            .select(pl.sum_horizontal(pl.all().is_null().sum()))
            .item()
        )
        if number_of_nulls > 0:
            raise Exception(
                f"Null values found in features: {number_of_nulls} null values"
            )

        self.dataframe = self.dataframe.hstack(
            [self.labeller.generate_labels(self.dataframe)]
        )
        self.input_data = self.dataframe.select(features)
        self.labels = self.dataframe.select("labels")
        self.element_spec = tf.TensorSpec(shape=self.input_data.shape[1])
        self.groups = self.dataframe.select(
            pl.struct(group_keys).rank("dense") - 1
        ).to_series()

    def to_windowed(
        self,
        stride: int = 1,
        control_frequency: float = 83,
        window_size: int = 1500,
        window_stride: int = 200,
        is_state_prediction: bool = False,
    ) -> None:
        number_of_samples = int((window_size / 1000) * control_frequency)
        windowed_dataframe = self.dataframe[::stride].group_by_dynamic(
            index_column="time",
            every=f"{window_stride}ms",
            period=f"{window_size}ms",
        )
        self.input_data = windowed_dataframe.agg(self.features).drop("time")
        for col in self.input_data.iter_columns():
            for elem in col:
                print(elem.shape)
        windowed_labels = windowed_dataframe.agg(pl.col("labels")).drop("time")
        self.labels = pl.Series(
            [
                row[-1] if is_state_prediction else row[0]
                # print(row)
                for row in windowed_labels.get_column("labels")
            ]
        )

    def __len__(self) -> int:
        return self.groups.unique().numel()

    def n_features(self) -> int:
        return self.input_data.size(1)

    def __getitem__(self, index: int) -> tuple[tf.Tensor, tf.Tensor]:
        mask = self.groups == index
        return self.input_data[mask], self.labels[mask]

    def get_input_tensor(self) -> tf.Tensor:
        pd_df = self.input_data.to_pandas().values
        # print(pd_df)
        # print(type(pd_df))
        tf.convert_to_tensor(pd_df, dtype=tf.float64)

    def get_labels_tensor(self) -> tf.Tensor:
        tf.convert_to_tensor(self.labels.to_numpy())

    def get_windows_input_tensor(self) -> list[tf.Tensor]:
        windowed_input_tensor = []
        for input_window in self.input_data.iter():
            windowed_input_tensor.append(
                tf.convert_to_tensor(input_window.to_pandas())
            )
        return windowed_input_tensor

    # def element_spec()
