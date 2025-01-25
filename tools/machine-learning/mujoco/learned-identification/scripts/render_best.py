import json

import click
import mujoco as mj
import optuna
from learned_identification.optimization import objective
from learned_identification.recording import (
    load_recorded_actuator_positions,
    load_recorded_sensors,
)

SENSORS = [
    "head.yaw",
    "head.pitch",
    "left_leg.hip_yaw_pitch",
    "left_leg.hip_roll",
    "left_leg.hip_pitch",
    "left_leg.knee_pitch",
    "left_leg.ankle_pitch",
    "left_leg.ankle_roll",
    "right_leg.hip_roll",
    "right_leg.hip_pitch",
    "right_leg.knee_pitch",
    "right_leg.ankle_pitch",
    "right_leg.ankle_roll",
    "left_arm.shoulder_pitch",
    "left_arm.shoulder_roll",
    "left_arm.elbow_yaw",
    "left_arm.elbow_roll",
    "left_arm.wrist_yaw",
    "right_arm.shoulder_pitch",
    "right_arm.shoulder_roll",
    "right_arm.elbow_yaw",
    "right_arm.elbow_roll",
    "right_arm.wrist_yaw",
]


def render_trial(
    spec_path: str,
    recording_path: str,
    study_name: str,
    trial_number: str,
    storage: str,
    video_path: str,
) -> None:
    spec = mj.MjSpec.from_file(spec_path)

    recorded_actuator_positions = load_recorded_actuator_positions(
        spec,
        recording_path,
    )
    recorded_sensors = load_recorded_sensors(
        spec,
        recording_path,
    )

    study = optuna.load_study(
        study_name=study_name,
        storage=storage,
    )

    trial = (
        study.best_trial
        if trial_number == "best"
        else study.trials[int(trial_number)]
    )

    print(f"Trial Number: {trial.number}")
    print("Parameters:")
    print(json.dumps(trial.params, indent=2))
    print(f"Stored Value: {trial.value}")

    value = objective(
        trial,
        spec,
        recorded_actuator_positions,
        recorded_sensors,
        sensors=SENSORS,
        video_path=video_path,
    )
    print(f"Computed Value: {value}")


@click.command()
@click.option("--spec", help="Path to the model specification file")
@click.option("--recording", help="Path to the mcap recording file")
@click.option("--study_name", help="Name of the study")
@click.option("--trial", help="Which trial (number or 'best')", default="best")
@click.option(
    "--storage",
    help="Path to the optuna database",
    default="sqlite:///optuna.db",
)
@click.option(
    "--video_path",
    help="Path to save the video",
    default="video.mp4",
)
def run(
    spec: str,
    recording: str,
    study_name: str,
    trial: str,
    storage: str,
    video_path: str,
) -> None:
    render_trial(
        spec,
        recording,
        study_name,
        trial,
        storage,
        video_path,
    )


if __name__ == "__main__":
    run()
