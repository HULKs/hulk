import json

import click
import mujoco as mj
import optuna
from optimization import objective
from recording import load_recorded_actuators, load_recorded_sensors


def render_trial(
    spec_path: str,
    recording_path: str,
    study_name: str,
    trial_number: str,
    storage: str,
    video_path: str,
) -> None:
    spec = mj.MjSpec.from_file(spec_path)

    recorded_actuators = load_recorded_actuators(
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
        recorded_actuators,
        recorded_sensors,
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
