import copy

import plotly.io as pio
import polars as pl
from data_loading import load
from dataset import FallenDataset


def main():
    pio.renderers.default = "browser"

    df = load("go-2025-data.parquet")
    dataset = FallenDataset(
        df,
        group_keys=[
            pl.col("robot_identifier"),
            pl.col("game_phase_identifier"),
            pl.col("match_identifier"),
        ],
        features=[
            pl.col(
                "Control.main_outputs.sensor_data.inertial_measurement_unit.linear_acceleration.x"
            ),
            pl.col(
                "Control.main_outputs.sensor_data.inertial_measurement_unit.linear_acceleration.y"
            ),
            pl.col(
                "Control.main_outputs.sensor_data.inertial_measurement_unit.linear_acceleration.z"
            ),
            pl.col(
                "Control.main_outputs.sensor_data.inertial_measurement_unit.roll_pitch.x"
            ),
            pl.col(
                "Control.main_outputs.sensor_data.inertial_measurement_unit.roll_pitch.y"
            ),
            pl.col("Control.main_outputs.has_ground_contact"),
        ],
    )
    dataset_copy = copy.deepcopy(dataset)
    dataset.to_windowed(window_size=30 / 83, label_shift=0)
    print(dataset.get_input_tensor().shape)
    print(dataset.get_labels_tensor().shape)
    dataset_copy.to_windowed(
        window_stride=1 / 83, window_size=2 / 83, label_shift=20
    )
    print(dataset_copy.get_input_tensor().shape)
    print(dataset_copy.get_labels_tensor().shape)


if __name__ == "__main__":
    main()
