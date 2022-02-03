import logging


class ColoredFormatter(logging.Formatter):
    BLACK = 30
    RED = 31
    GREEN = 32
    YELLOW = 33
    BLUE = 34
    MAGENTA = 35
    CYAN = 36
    WHITE = 37
    BG_RED = 41
    BG_GREY = 100
    RESET_SEQ = "\033[0m"
    COLOR_SEQ = "\033[1;{}m"
    BOLD_SEQ = "\033[1m"
    COLORS = {
        'INFO': WHITE,
        'WARNING': YELLOW,
        'ERROR': RED,
        'CRITICAL': BG_RED,
        'DEBUG': CYAN,
        'SUCCESS': GREEN
    }

    def __init__(self, msg):
        logging.Formatter.__init__(self, msg)

    def format(self, record):
        levelname = record.levelname
        if levelname in self.COLORS:
            record.levelname = self.COLOR_SEQ.format(
                self.COLORS[levelname]) + levelname + self.RESET_SEQ
        return logging.Formatter.format(self, record)


logger = logging.getLogger()
handler = logging.StreamHandler()
formatter = ColoredFormatter('[%(levelname)-19s] %(message)s')
handler.setFormatter(formatter)
logger.addHandler(handler)

# set success level
logging.SUCCESS = 25  # between WARNING and INFO
logging.addLevelName(logging.SUCCESS, 'SUCCESS')
setattr(logger, 'success',
        lambda message, *args: logger._log(logging.SUCCESS, message, args))


def set_level(level):
    logger.setLevel(level)
