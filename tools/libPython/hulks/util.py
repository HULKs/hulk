import asyncio
import git
from pathlib import Path
import os
import errno


def get_project_root() -> Path:
    """get_project_root returns the absolute path to the git repository root"""
    repo = git.Repo(__file__, search_parent_directories=True)
    return Path(repo.git.rev_parse('--show-toplevel'))


PROJECT_ROOT = get_project_root()


def force_symlink(src, dst, target_is_directory=False):
    try:
        os.symlink(src, dst, target_is_directory=target_is_directory)
    except OSError as e:
        if e.errno == errno.EEXIST:
            os.remove(dst)
            os.symlink(src, dst, target_is_directory=target_is_directory)


def make_thread_target(coroutine):
    """make_thread_target takes a coroutine and
    returns a thread target with the coroutine
    executed inside an event loop that is
    explicitly closed when complete
    """
    def target():
        # logger.debug(__name__ +
        #              ": Executing " + str(coroutine) +
        #              " in a new event loop.")
        loop = asyncio.new_event_loop()
        loop.run_until_complete(coroutine)
        loop.close()

    return target
