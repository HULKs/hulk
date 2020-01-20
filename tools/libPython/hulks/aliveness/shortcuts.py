import yaml

from datetime import timedelta, datetime

from hulks.constants import ALIVENESS_PATH

from .errors import MissingCacheError
from .errors import MoldyCacheError


def read_aliveness_cache():
    aliveness_info = None

    if not ALIVENESS_PATH.exists():
        raise MissingCacheError(
            "{} does not exist".format(ALIVENESS_PATH))

    with ALIVENESS_PATH.open('r') as aliveness_file:
        while aliveness_info is None:
            # TODO: investigate why yaml.full_load yields
            #       None sometimes.
            # (Does this happen when the file is being written
            #  at the same time?)
            aliveness_info = yaml.full_load(aliveness_file)

    if datetime.now() - aliveness_info["timestamp"] > timedelta(seconds=20):
        raise MoldyCacheError(
            "{} is too old".format(ALIVENESS_PATH))

    return aliveness_info
