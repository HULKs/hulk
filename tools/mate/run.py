#!/usr/bin/env python3

import os
import sys
import argparse
import setproctitle
import signal
import time
import logging

import PyQt5.QtCore as qtc

from mate.app import App
from mate.ui.main.main_window import MainWindow
from mate.debug.colorlog import ColorLog


def parse_arguments():
    parser = argparse.ArgumentParser(description="MATE - Debug tools")
    parser.add_argument(
        "-v",
        "--verbose",
        help="Explain what is done",
        action="store_true",
        default=False)

    parser.add_argument(
        "-c",
        "--config",
        help="Location of settings directory",
        default=os.path.realpath(os.path.dirname(__file__)) +
        "/mate/layouts/")

    parser.add_argument(
        "--panel-directory",
        help="Location of panels",
        default=os.path.realpath(os.path.dirname(__file__)) +
        "/mate/ui/panels/")

    parser.add_argument(
        "-t",
        "--timeout",
        help="Connection timeout in seconds",
        default=None)

    parser.add_argument(
        "--default-config-dir",
        help="Path to nao/home/configuration/",
        default=os.path.realpath(os.path.dirname(__file__)) +
        "/../../home/configuration/"
    )

    return parser.parse_args()


if __name__ == "__main__":
    initTime = time.time()
    logger = ColorLog()
    setproctitle.setproctitle("mate ({})".format(sys.argv[0]))
    args = parse_arguments()
    settings = qtc.QSettings(args.config + "main.config",
                             qtc.QSettings.NativeFormat)
    if args.verbose:
        logger.setLevel(logging.DEBUG)
    elif settings.value("logLevel"):
        logger.setLevel(int(settings.value("logLevel")))
    else:
        logger.setLevel(logging.INFO)
    logger.info(__name__ + ": Initializing Mate")

    app = App(sys.argv)
    main_window = MainWindow(args.config,
                             args.panel_directory,
                             args.verbose,
                             args.timeout,
                             args.default_config_dir)
    app.aboutToQuit.connect(main_window.exit)
    signal.signal(signal.SIGINT, lambda *a: app.exit())
    main_window.show()

    timer = qtc.QTimer()
    timer.start(500)
    timer.timeout.connect(lambda: None)

    logger.info(__name__ + ": Initializing Mate took: " +
                logger.timerLogStr(initTime))

    sys.exit(app.exec_())
