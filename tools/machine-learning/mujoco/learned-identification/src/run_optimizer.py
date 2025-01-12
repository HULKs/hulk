import click
import mujoco as mj
import optuna
from optimization import objective
from recording import load_recorded_actuators, load_recorded_sensors


def run_optimizer(
    spec_path: str,
    recording_path: str,
    study_name: str,
    storage: str,
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

    study = optuna.create_study(
        study_name=study_name,
        storage=storage,
        load_if_exists=True,
        sampler=optuna.samplers.CmaEsSampler(),
    )

    study.enqueue_trial(
        {
            "head.yaw_kp": 21.1,
            "head.yaw_kv": 0.0,
            "head.pitch_kp": 21.1,
            "head.pitch_kv": 0.0,
            "left_leg.hip_yaw_pitch_kp": 21.1,
            "left_leg.hip_yaw_pitch_kv": 0.0,
            "left_leg.hip_roll_kp": 21.1,
            "left_leg.hip_roll_kv": 0.0,
            "left_leg.hip_pitch_kp": 21.1,
            "left_leg.hip_pitch_kv": 0.0,
            "left_leg.knee_pitch_kp": 21.1,
            "left_leg.knee_pitch_kv": 0.0,
            "left_leg.ankle_pitch_kp": 21.1,
            "left_leg.ankle_pitch_kv": 0.0,
            "left_leg.ankle_roll_kp": 21.1,
            "left_leg.ankle_roll_kv": 0.0,
            "right_leg.hip_roll_kp": 21.1,
            "right_leg.hip_roll_kv": 0.0,
            "right_leg.hip_pitch_kp": 21.1,
            "right_leg.hip_pitch_kv": 0.0,
            "right_leg.knee_pitch_kp": 21.1,
            "right_leg.knee_pitch_kv": 0.0,
            "right_leg.ankle_pitch_kp": 21.1,
            "right_leg.ankle_pitch_kv": 0.0,
            "right_leg.ankle_roll_kp": 21.1,
            "right_leg.ankle_roll_kv": 0.0,
            "left_arm.shoulder_pitch_kp": 21.1,
            "left_arm.shoulder_pitch_kv": 0.0,
            "left_arm.shoulder_roll_kp": 21.1,
            "left_arm.shoulder_roll_kv": 0.0,
            "left_arm.elbow_yaw_kp": 21.1,
            "left_arm.elbow_yaw_kv": 0.0,
            "left_arm.elbow_roll_kp": 21.1,
            "left_arm.elbow_roll_kv": 0.0,
            "left_arm.wrist_yaw_kp": 21.1,
            "left_arm.wrist_yaw_kv": 0.0,
            "right_arm.shoulder_pitch_kp": 21.1,
            "right_arm.shoulder_pitch_kv": 0.0,
            "right_arm.shoulder_roll_kp": 21.1,
            "right_arm.shoulder_roll_kv": 0.0,
            "right_arm.elbow_yaw_kp": 21.1,
            "right_arm.elbow_yaw_kv": 0.0,
            "right_arm.elbow_roll_kp": 21.1,
            "right_arm.elbow_roll_kv": 0.0,
            "right_arm.wrist_yaw_kp": 21.1,
            "right_arm.wrist_yaw_kv": 0.0,
        },
    )

    study.optimize(
        lambda trial: objective(
            trial,
            spec,
            recorded_actuators,
            recorded_sensors,
        ),
        n_jobs=1,
        # n_trials=100,
    )


@click.command()
@click.option(
    "--spec",
    "spec_path",
    help="Path to the model specification file",
)
@click.option(
    "--recording",
    "recording_path",
    help="Path to the mcap recording file",
)
@click.option("--study", "study_name", help="Name of the study")
@click.option(
    "--storage",
    help="Path to the optuna database",
    default="sqlite:///optuna.db",
)
def main(
    spec_path: str,
    recording_path: str,
    study_name: str,
    storage: str,
) -> None:
    run_optimizer(
        spec_path,
        recording_path,
        study_name,
        storage,
    )


if __name__ == "__main__":
    main()
