from typing import Iterable
import torch
from torch.utils.data import Dataset
import polars as pl
import polars.selectors as ps
from polars._typing import IntoExpr

from .pseudo_labels import PseudoLabeller


class FallenDataset(Dataset):
    groups: list[pl.DataFrame]

    def __init__(
        self,
        dataframe: pl.DataFrame,
        *,
        group_keys: list[str],
        features: Iterable[IntoExpr] | IntoExpr,
    ):
        self.labeller = PseudoLabeller()
        self.features = features

        number_of_nulls = (
            dataframe.select(features)
            .select(pl.sum_horizontal(pl.all().is_null().sum()))
            .item()
        )
        if number_of_nulls > 0:
            print(f"Null values found in features: {number_of_nulls} null values")

        self.X = (
            dataframe.select(features)
            .select(pl.all().cast(pl.Float32).fill_null(0.0))
            .to_torch()
            .contiguous()
        )
        targets = self.labeller.generate_labels(dataframe)
        self.y = targets.rank("dense").to_torch().contiguous() - 1
        self.groups = (
            dataframe.select(pl.struct(group_keys).rank("dense") - 1)
            .to_series()
            .to_torch()
        ).contiguous()

    def __len__(self):
        return self.groups.unique().numel()

    def n_features(self) -> int:
        return self.X.size(1)

    def __getitem__(self, index: int) -> tuple[torch.Tensor, torch.Tensor]:
        mask = self.groups == index
        return self.X[mask], self.y[mask]
