#!/usr/bin/env python3

import os
import sys
import argparse
import setproctitle
import signal
import PyQt5.QtCore as qtc
from mate.app import App
from mate.ui.main.main_controller import Main


def parse_arguments():
    parser = argparse.ArgumentParser()
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
        "/mate/settings/")

    return parser.parse_args()


if __name__ == "__main__":
    setproctitle.setproctitle("mate ({})".format(sys.argv[0]))
    args = parse_arguments()

    app = App(sys.argv)
    main_controller = Main(args.config)
    app.aboutToQuit.connect(main_controller.exit)
    signal.signal(signal.SIGINT, lambda *a: app.exit())
    main_controller.run()

    timer = qtc.QTimer()
    timer.start(500)
    timer.timeout.connect(lambda: None)

    sys.exit(app.exec_())
