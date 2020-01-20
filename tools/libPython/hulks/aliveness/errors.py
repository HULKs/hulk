class LockError(RuntimeError):
    pass


class MoldyCacheError(RuntimeError):
    pass


class MissingCacheError(OSError):
    pass
