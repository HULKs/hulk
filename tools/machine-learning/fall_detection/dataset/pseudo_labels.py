from enum import Enum

import polars as pl

from dataclasses import dataclass


@dataclass
class PseudoLabelParameters:
    falling_threshold: float = 0.6
    fallen_threshold: float = 1.3


class Label(int, Enum):
    Upright = 0
    Falling = 1
    Fallen = 2


class PseudoLabeller:
    label_type = Label

    def __init__(self, parameters: PseudoLabelParameters | None = None):
        self.parameters = parameters or PseudoLabelParameters()

    def generate_labels(self, data: pl.DataFrame) -> pl.Series:
        has_ground_contact = pl.col("Control.main_outputs.has_ground_contact")
        pitch = pl.col("Control.main_outputs.robot_orientation.pitch")
        # print(data.select(primary_state))
        return (
            data.select(
                pl.when(has_ground_contact)
                .then(
                    pl.when(pitch.abs() > self.parameters.falling_threshold)
                    .then(pl.lit(Label.Falling))
                    .otherwise(pl.lit(Label.Upright))
                )
                .otherwise(
                    pl.when(pitch.abs() > self.parameters.fallen_threshold)
                    .then(pl.lit(Label.Fallen))
                    .otherwise(pl.lit(Label.Falling))
                )
            )
            .to_series()
            .alias("labels")
        )
