import polars as pl
import plotly.express as px
import plotly.io as pio
from data_loading import load
from dataset import FallenDataset


def main():
    pio.renderers.default = "browser"

    df = load("data.parquet")
    dataset = FallenDataset(
        df,
        group_keys=["robot_identifier", "match_identifier"],
        features=[
            pl.col("Control.main_outputs.robot_orientation.pitch"),
            pl.col("Control.main_outputs.robot_orientation.roll"),
            pl.col("Control.main_outputs.robot_orientation.yaw"),
            pl.col("Control.main_outputs.has_ground_contact"),
        ],
    )
    dataset.to_windowed(window_stride=1 / 83)

    df = (
        dataset.input_data.hstack(dataset.labels)
        .with_row_index()
        .hstack(
            dataset.input_data.select(
                pl.col(
                    "Control.main_outputs.robot_orientation.pitch"
                ).list.last()
            ).rename({"Control.main_outputs.robot_orientation.pitch": "pitch"})
        )
    )
    print(df)

    px.scatter(
        df,
        x="index",
        y="pitch",
        color="labels",
    ).show()


if __name__ == "__main__":
    main()
