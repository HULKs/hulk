from dataclasses import dataclass
from enum import Enum

import polars as pl


@dataclass
class PseudoLabelParameters:
    unstable_threshold: float = 0.6


class Label(int, Enum):
    Stable = 0
    SoonToBeUnstable = 1


class PseudoLabeller:
    label_type = Label

    def __init__(self, parameters: PseudoLabelParameters | None = None):
        self.parameters = parameters or PseudoLabelParameters()

    def generate_labels(self, data: pl.DataFrame) -> pl.Series:
        has_ground_contact = pl.col("Control.main_outputs.has_ground_contact")
        pitch = pl.col(
            "Control.main_outputs.sensor_data.inertial_measurement_unit.roll_pitch.y"
        )
        return (
            data.select(
                pl.when(has_ground_contact)
                .then(
                    pl.when(pitch.abs() > self.parameters.unstable_threshold)
                    .then(pl.lit(Label.SoonToBeUnstable))
                    .otherwise(pl.lit(Label.Stable))
                )
                .otherwise(pl.lit(Label.SoonToBeUnstable))
            )
            .to_series()
            .alias("labels")
        )
