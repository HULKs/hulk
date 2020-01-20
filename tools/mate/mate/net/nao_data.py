import typing as ty


class Data():
    def __init__(self, key: str, data):
        self.key = key
        self.data = data


class DebugValue(Data):
    def __init__(self, key: str, timestamp: int, data):
        super(DebugValue, self).__init__(key, data)
        self.isImage = False
        self.timestamp = timestamp


class DebugImage(Data):
    def __init__(self, key: str, width: int, height: int, data: bytes, timestamp: int = 0):
        super(DebugImage, self).__init__(key, data)
        self.timestamp = timestamp
        self.isImage = True
        self.width = width
        self.height = height


class ConfigMount(Data):
    def __init__(self,
                 key: str,
                 filename: str,
                 data: ty.Dict[str, str]):
        super(ConfigMount, self).__init__(key, data)
        self.filename = filename
