import plotly.express as px
import plotly.io as pio
import polars as pl
from data_loading import load
from dataset import FallenDataset
import copy


def main():
    pio.renderers.default = "browser"

    df = load("data.parquet")
    dataset = FallenDataset(
        df,
        group_keys=[
            pl.col("robot_identifier"),
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
    dataset.to_windowed(window_stride=1 / 83, window_size=2 / 83, label_shift=0)
    # df = (
    #     dataset.input_data.hstack(dataset.labels)
    #     .with_row_index()
    #     .hstack(
    #         dataset.input_data.select(
    #             pl.col(
    #                 "Control.main_outputs.sensor_data.inertial_measurement_unit.roll_pitch.y"
    #             )  # .list.last()
    #         ).rename(
    #             {
    #                 "Control.main_outputs.sensor_data.inertial_measurement_unit.roll_pitch.y": "pitch"
    #             }
    #         )
    #     )
    # )
    # print(dataset.input_data[0:10])
    # print(dataset.input_data.columns)
    print(dataset.get_input_tensor().shape)
    print(dataset.get_labels_tensor().shape)
    dataset_copy.to_windowed(
        window_stride=1 / 83, window_size=2 / 83, label_shift=20
    )
    print(dataset_copy.get_input_tensor().shape)
    print(dataset_copy.get_labels_tensor().shape)
    # print(dataset_copy.input_data[0:10])

    # df_copy = (
    #     dataset_copy.input_data.hstack(dataset_copy.labels)
    #     .with_row_index()
    #     .hstack(
    #         dataset_copy.input_data.select(
    #             pl.col(
    #                 "Control.main_outputs.sensor_data.inertial_measurement_unit.roll_pitch.y"
    #             )  # .list.last()
    #         ).rename(
    #             {
    #                 "Control.main_outputs.sensor_data.inertial_measurement_unit.roll_pitch.y": "pitch"
    #             }
    #         )
    #     )
    # )

    # px.scatter(
    #     df,
    #     x="index",
    #     y="pitch",
    #     color="labels",
    # ).show()


if __name__ == "__main__":
    main()
