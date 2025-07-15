import click
from data_loading import convert_mcaps
from pathlib import Path


@click.command()
@click.argument("mcaps", nargs=-1)
def main(mcaps: list[Path]):
    paths = []
    for path in mcaps:
        path = Path(path)
        if path.is_file():
            paths.append(path)
        else:
            paths.extend(path.glob("**/*.mcap"))

    dataframe = convert_mcaps(paths)
    dataframe.write_parquet("data.parquet")


if __name__ == "__main__":
    # Add the parent directory to sys.path

    main()
