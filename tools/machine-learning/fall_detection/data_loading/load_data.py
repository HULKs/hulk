from pathlib import Path
from typing import Any
from mcap.reader import make_reader, McapReader
import polars as pl
from msgpack import unpackb
from datetime import datetime
from itertools import batched
from tqdm import tqdm
from scipy.spatial.transform import Rotation
from .unnest_structs import unnest_column


class Unwrapper:
    def unwrap(self, data: Any) -> Any:
        return data


class VectorUnwrapper(Unwrapper):
    def unwrap(self, data: list[float]) -> dict[str, float]:
        coordinate_names = ["x", "y", "z"]
        return dict(zip(coordinate_names, data))


class QuaternionUnwrapper(Unwrapper):
    def unwrap(self, data: list[float]) -> dict[str, float]:
        [x, y, z, w] = data
        rotation = Rotation.from_quat([x, y, z, w])
        [roll, pitch, yaw] = rotation.as_euler("xyz")
        return {
            "roll": roll,
            "pitch": pitch,
            "yaw": yaw,
        }


class SensorDataUnwrapper(Unwrapper):
    pass


class RobotOrientationUnwrapper(QuaternionUnwrapper):
    pass


class CenterOfMassUnwrapper(VectorUnwrapper):
    pass


class ZeroMomentPointUnwrapper(VectorUnwrapper):
    pass


class FallStateUnwrapper(Unwrapper):
    def unwrap(self, data: dict | str) -> str:
        if isinstance(data, str):
            return data
        if isinstance(data, dict):
            return next(iter(data.keys()))
        raise ValueError(f"did not expect fall state {data}")


OUTPUTS = {
    "Control.main_outputs.sensor_data": SensorDataUnwrapper(),
    "Control.main_outputs.robot_orientation": RobotOrientationUnwrapper(),
    "Control.main_outputs.center_of_mass": CenterOfMassUnwrapper(),
    "Control.main_outputs.zero_moment_point": ZeroMomentPointUnwrapper(),
    "Control.main_outputs.fall_state": FallStateUnwrapper(),
}


def iter_mcap(reader: McapReader, topics: list[str]):
    summary = reader.get_summary()
    channels = [
        channel for channel in summary.channels.values() if channel.topic in topics
    ]
    message_counts = sum(
        summary.statistics.channel_message_counts[channel.id] for channel in channels
    )
    number_of_steps = message_counts // len(topics)

    for outputs in tqdm(
        batched(
            reader.iter_messages(topics=OUTPUTS.keys()), n=len(topics), strict=True
        ),
        total=number_of_steps,
    ):
        channels = [channel for _, channel, _ in outputs]
        messages = [message for _, _, message in outputs]

        log_times = [
            datetime.fromtimestamp(message.log_time / 1e9) for message in messages
        ]
        assert all(log_times[0] == time for time in log_times)
        data = {
            channel.topic: OUTPUTS[channel.topic].unwrap(unpackb(message.data))
            for channel, message in zip(channels, messages)
        }
        yield {"time": log_times[0], **data}


def read_mcap(mcap_path: Path) -> pl.DataFrame:
    robot_identifier = mcap_path.parts[-3]
    match_identifier = mcap_path.parts[-5]

    with open(mcap_path, "rb") as mcap_data:
        reader = make_reader(mcap_data)
        dataframe = pl.from_dicts(iter_mcap(reader, OUTPUTS)).with_columns(
            pl.lit(robot_identifier).alias("robot_identifier"),
            pl.lit(match_identifier).alias("match_identifier"),
        )
    return dataframe


def convert_mcaps(mcaps: list[str]) -> pl.DataFrame:
    return pl.concat((read_mcap(Path(mcap)) for mcap in tqdm(mcaps)), how="diagonal")


def load(path: str):
    df = pl.read_parquet(path).with_columns(
        (pl.col("time") - pl.col("time").min())
        .over("robot_identifier", "match_identifier")
        .dt.total_seconds()
        .alias("time_in_game"),
    )
    struct_columns = [col for col, schema in df.schema.items() if schema == pl.Struct]
    for column in struct_columns:
        df.hstack(unnest_column(df[column]), in_place=True)
        df.drop_in_place(column)
    return df.rechunk()
