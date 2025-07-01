import click
from data_loading import convert_mcaps


@click.command()
@click.argument("mcaps", nargs=-1)
def main(mcaps: list[str]):
    dataframe = convert_mcaps(mcaps)
    dataframe.write_parquet("data.parquet")


if __name__ == "__main__":
    # Add the parent directory to sys.path

    main()
