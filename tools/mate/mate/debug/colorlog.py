import logging
import time
from termcolor import colored


class Singleton(type):
    _instances = {}

    def __call__(cls, *args, **kwargs):
        if cls not in cls._instances:
            cls._instances[cls] = super(Singleton, cls).__call__(*args,
                                                                 **kwargs)
        return cls._instances[cls]


class ColorLog(metaclass=Singleton):

    colormap = {"debug": {"color": 'grey', "attrs": ['bold']},
                "info": {"color": 'white'},
                "warning": {"color": 'yellow'},
                "error": {"color": 'red'},
                "critical": {"color": 'red', "attrs": ['bold']}}

    def __init__(self):
        self._log = logging.getLogger(" ")
        formatter = logging.Formatter("[%(levelname)-8s] %(message)s")
        handler = logging.StreamHandler()
        handler.setFormatter(formatter)
        self._log.addHandler(handler)

    def __getattr__(self, level: str):
        if level in ['debug', 'info', 'warning', 'error', 'critical']:
            return lambda s, *args: getattr(
                self._log, level)(colored(s,
                                          **self.colormap[level]),
                                  *args)

        return getattr(self._log, level)

    @staticmethod
    def timerLogStr(startTime: float):
        return str((time.time() - startTime) * 1000)[0:5] + " ms"
