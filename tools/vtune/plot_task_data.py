#!/usr/bin/env python3
from collections import defaultdict
import sys, os
import argparse, textwrap

parser = argparse.ArgumentParser(
    description="Create output from VTune Amplifier profiler runs",
    formatter_class=argparse.RawTextHelpFormatter,
)
parser.add_argument(
    "--supress-wait-modules",
    help="Supress the ImageReceiver and SensorDataProvider in the plot output",
    action="store_true",
)
parser.add_argument(
    "project_path",
    help=textwrap.dedent(
        """\
  The path to the VTune Amplifier capture.
  If $VTUNE_HOME is set the path will be relative to $VTUNE_HOME\\..\\amplxe\\projects.
  If a project_dir file exists the path will be relative to the path specified in this file.
  """
    ),
)
parser.add_argument(
    "output", help="Decide whether to output text or plot", choices=["plot", "text"]
)

base_project_path = ""

if os.environ.get("VTUNE_HOME"):
    base_project_path = os.path.join(
        os.environ["VTUNE_HOME"], "..", "amplxe", "projects"
    )

script_path = os.path.abspath(os.path.dirname(sys.argv[0]))
if os.path.isfile(os.path.join(script_path, "project_dir")):
    base_project_path = open(os.path.join(script_path, "project_dir")).read()

args = parser.parse_args()
absolute_project_path = os.path.join(base_project_path, args.project_path)
sql_database_file = os.path.join(absolute_project_path, "sqlite-db", "dicer.db")
project_name = os.path.splitext(
    [f for f in os.listdir(absolute_project_path) if "amplxe" in f][0]
)[0]

# Load the libraries later to get faster help messages
import sqlite3
import matplotlib.pyplot as plt
import pandas


def make_boxplot(number, data, title, ylabel):
    plt.figure(number)
    figure = data.boxplot()  # seaborn.boxplot(data)
    loc, labels = plt.xticks()
    figure.set_xticklabels(labels, rotation=90)
    figure.set_title(title)
    figure.set_ylabel(ylabel)
    plt.gcf().subplots_adjust(bottom=0.5)


if args.output == "text":
    # Only output average data
    conn = sqlite3.connect(sql_database_file)
    c = conn.cursor()
    c.execute(
        """
    select
      cast(min(task_data.end_tsc - task_data.start_tsc) as float) / 10000000 as "min in ms",
      cast(max(task_data.end_tsc - task_data.start_tsc) as float) / 10000000 as "max in ms",
      cast(avg(task_data.end_tsc - task_data.start_tsc) as float) / 10000000 as "avg in ms",
      cast(sum(task_data.end_tsc - task_data.start_tsc) as float) / 10000000 as "sum in ms",
      count(*),
      dd_task_type.name as "Modulename",
      dd_domain.name as "Threadname"
    from task_data
    left join dd_task_type on task_data.attr=dd_task_type.rowid
    left join dd_domain on dd_task_type.domain=dd_domain.rowid
    group by task_data.attr;
    """
    )

    results = c.fetchall()
    pandas.set_option("display.width", None)
    pandas.set_option("display.max_rows", None)
    print(
        pandas.DataFrame(
            results,
            columns=["min", "max", "avg", "sum", "count", "Modulename", "Threadname"],
        )
    )

if args.output == "plot":
    conn = sqlite3.connect(sql_database_file)
    c = conn.cursor()
    c.execute(
        """
    select
      cast(task_data.end_tsc - task_data.start_tsc as float) / 10000000 as "min in ms",
      dd_task_type.name as "Modulename",
      dd_domain.name as "Threadname"
    from task_data
    left join dd_task_type on task_data.attr=dd_task_type.rowid
    left join dd_domain on dd_task_type.domain=dd_domain.rowid;
    """
    )

    results = c.fetchall()
    conn.close()

    brain_top_data = defaultdict(list)
    brain_bot_data = defaultdict(list)
    motion_data = defaultdict(list)
    for result in results:
        if result[2] == "BrainTop":
            if args.supress_wait_modules and result[1] == "ImageReceiver":
                continue
            brain_top_data[result[1]].append(result[0])
        if result[2] == "BrainBottom":
            if args.supress_wait_modules and result[1] == "ImageReceiver":
                continue
            brain_bot_data[result[1]].append(result[0])
        if result[2] == "Motion":
            if args.supress_wait_modules and result[1] == "SensorDataProvider":
                continue
            motion_data[result[1]].append(result[0])

    pandas_brain_top_data = pandas.DataFrame(
        dict([(k, pandas.Series(v)) for k, v in brain_top_data.items()])
    )
    pandas_brain_bot_data = pandas.DataFrame(
        dict([(k, pandas.Series(v)) for k, v in brain_bot_data.items()])
    )
    pandas_motion_data = pandas.DataFrame(
        dict([(k, pandas.Series(v)) for k, v in motion_data.items()])
    )

    # Plot Brain Top Image Data
    make_boxplot(
        1, pandas_brain_top_data, "Brain Top Image Modules\n" + project_name, "ms/cycle"
    )

    # Plot Brain Bottom Image Data
    make_boxplot(
        2,
        pandas_brain_bot_data,
        "Brain Bottom Image Modules\n" + project_name,
        "ms/cycle",
    )

    # Plot Motion Image Data
    make_boxplot(3, pandas_motion_data, "Motion Module\n" + project_name, "ms/cycle")

    plt.show()
