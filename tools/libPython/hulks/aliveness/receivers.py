import sys
import yaml

from datetime import datetime

from . import models, listeners
from hulks.constants import ALIVENESS_PATH


class AlivenessReceiver:
    """AlivenessReceiver is a high-level class for fetching data and
    writing it into a file
    """

    def __init__(self, verbose=False, up_callback=None):
        self.verbose = verbose

        self.is_up = False
        self.up_callback = up_callback

        self.__setup()

    @staticmethod
    def __setup():
        ALIVENESS_PATH.parent.mkdir(parents=True, exist_ok=True)

    def __handle_results(self, robots: models.RobotList):
        aliveness_info = {
            "robots":    robots,
            "timestamp": datetime.now()
        }

        if self.verbose:
            print(aliveness_info, file=sys.stderr)

        ALIVENESS_PATH.write_text(yaml.dump(aliveness_info))

        if not self.is_up and self.up_callback is not None:
            self.is_up = True
            self.up_callback()

    def start_receiving(self):
        listener = listeners.AlivenessListener(
            results_callback=self.__handle_results)
        listener.start_listening()
