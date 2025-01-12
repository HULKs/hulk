import logging
from collections.abc import Iterable
from itertools import islice
from pathlib import Path

import msgpack
import mujoco as mj
import numpy as np
import numpy.typing as npt
import tqdm
from mcap.reader import make_reader

logger = logging.getLogger(__name__)


def collect_values_from_mcap(
    path: Path | str,
    topic: str,
    joint_names: Iterable[str],
    max_num_messages: int | None = None,
) -> npt.NDArray[np.float64]:
    """Collect joint values from an MCAP file.

    Args:
        path: The path to the MCAP file.
        topic: The topic to collect messages from.
        joint_names: The order of joint names to collect.
        max_num_messages: The maximum number of messages to collect.

    Returns:
        A 2D array of joint values.

    """
    path = Path(path)
    collected = []
    with path.open("rb") as file:
        reader = make_reader(file)
        messages = islice(
            reader.iter_messages(topics=[topic]),
            max_num_messages,
        )
        for _, _, message in tqdm.tqdm(messages, desc=f"collecting {topic}"):
            data = msgpack.unpackb(message.data)
            if data is None:
                logger.warning(
                    "skipping empty message with sequence %s",
                    message.sequence,
                )
                continue

            positions = data["positions"]
            flattened = {
                f"{parent}.{joint}": value
                for parent, child in positions.items()
                for joint, value in child.items()
            }
            restructured = np.array(
                [flattened[name] for name in joint_names],
                dtype=np.float64,
            )
            collected.append(restructured)
    return np.vstack(collected)


def load_recorded_sensors(
    spec: mj.MjSpec,
    recording: str,
    *,
    max_num_messages: int | None = None,
) -> npt.NDArray[np.float64]:
    sensor_names = [
        sensor.name
        for sensor in spec.sensors
        if sensor.type == mj.mjtSensor.mjSENS_JOINTPOS
    ]
    return collect_values_from_mcap(
        recording,
        "Control.main_outputs.sensor_data",
        sensor_names,
        max_num_messages,
    )


def load_recorded_actuators(
    spec: mj.MjSpec,
    recording: str,
    *,
    max_num_messages: int | None = None,
) -> npt.NDArray[np.float64]:
    actuator_names = [actuator.name for actuator in spec.actuators]
    return collect_values_from_mcap(
        recording,
        "Control.main_outputs.actuated_motor_commands",
        actuator_names,
        max_num_messages,
    )
