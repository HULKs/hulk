# Since it is not possible / intended to listen for aliveness on
# multiple connections, we need to make sure that only one process can
# listen for aliveness at any given time.

# This is achieved by using create_listen_lock() and
# remove_listen_lock() when a receiving process is starting or ending,
# respectively.

# listen_lock() provides a convenient way to achieve the same thing:
# with listen_lock("my_receiver"):
#     ...

import contextlib

from hulks.constants import ALIVENESS_LOCK_PATH
from hulks.aliveness.errors import LockError


def read_listen_lock():
    return ALIVENESS_LOCK_PATH.read_text()


# used manually in MATE
def create_listen_lock(tag: str):
    if ALIVENESS_LOCK_PATH.exists():
        raise LockError("Aliveness lockfile exists, is "
                        "another receiver running? Hint: {}"
                        .format(read_listen_lock()))
    else:
        ALIVENESS_LOCK_PATH.parent.mkdir(parents=True, exist_ok=True)
        ALIVENESS_LOCK_PATH.write_text(tag)


# used manually in MATE
def remove_listen_lock(tag: str):
    if tag == read_listen_lock():
        ALIVENESS_LOCK_PATH.unlink()


# used in alivenessReceiver.py
@contextlib.contextmanager
def listen_lock(tag: str):
    """listen_lock() should be used to ensure that an aliveness.lock
    is created/deleted every time a listener is started
    """

    create_listen_lock(tag)

    try:
        yield
    finally:
        remove_listen_lock(tag)
