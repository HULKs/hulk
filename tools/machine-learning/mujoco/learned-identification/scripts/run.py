import multiprocessing

import click
from learned_identification.run_optimizer import run_optimization


@click.command()
@click.option(
    "--spec",
    "spec_path",
    required=True,
    help="Path to the model specification file",
)
@click.option(
    "--recording",
    "recording_path",
    required=True,
    help="Path to the mcap recording file",
)
@click.option("--study", help="Name of the study", required=True)
@click.option(
    "--storage",
    help="Path to the optuna database",
    default="sqlite:///optuna.db",
)
@click.option("--jobs", help="Number of jobs to run", default=1, type=int)
def run_many(
    spec_path: str,
    recording_path: str,
    study: str,
    storage: str,
    jobs: int,
) -> None:
    if jobs == 1:
        run_optimization(spec_path, recording_path, study, storage)
        return

    processes = []
    for _ in range(jobs):
        p = multiprocessing.Process(
            target=run_optimization,
            args=(spec_path, recording_path, study, storage),
        )
        p.start()
        print(f"Started process {p.pid}")
        processes.append(p)

    for p in processes:
        p.join()


if __name__ == "__main__":
    run_many()
