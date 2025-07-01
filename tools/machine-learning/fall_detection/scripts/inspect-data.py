import polars as pl
import plotly.express as px
import plotly.io as pio
from data_loading import load


def main():
    pio.renderers.default = "browser"
    data = load("data.parquet")
    # px.scatter(
    #     data, x="time", y="Control.main_outputs.fall_state", color="robot_identifier"
    # ).show()
    # px.scatter(
    #     data.filter(pl.col("robot_identifier") == "10.1.24.33"),
    #     x="time",
    #     y="Control.main_outputs.robot_orientation.pitch",
    #     color="Control.main_outputs.fall_state",
    # ).show()


if __name__ == "__main__":
    main()
