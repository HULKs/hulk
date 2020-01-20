import struct
from enum import Enum
import typing as ty
from mate.net.nao_data import Data, DebugValue, DebugImage

NO_SUBSCRIBE_KEY = "none"
K = ty.TypeVar('K')


def split(predicate: ty.Callable[[K], bool], dictionary: ty.Dict[K, dict]):
    dict1 = {}
    dict2 = {}
    for key in dictionary:
        if predicate(key):
            dict1[key] = dictionary[key]
        else:
            dict2[key] = dictionary[key]
    return dict1, dict2


class DebugMsgType(Enum):
    subscribe = 0
    unsubscribe = 1
    update = 2
    request_list = 3
    list = 4
    subscribe_bulk = 5
    image = 6


class ConfigMsgType(Enum):
    set = 0             # Sets a given key to a given value (at runtime)
    get_mounts = 1      # ask for send_mounts, containing all mounts
    get_keys = 2        # ask for send_keys of a given key
    save = 3            # saves the current config
    send_keys = 4       # containing key, value
    send_mounts = 5     # containing filename, key


class ConnectionStatusType(Enum):
    disconnected = 0
    connected = 1
    connection_lost = 2
    connection_refused = 3


class Message:
    def __init__(self,
                 type: DebugMsgType,
                 body: str = "",
                 length: int = None,
                 version: int = 1):
        self.type = type
        self.body = body
        self.length = length if length is not None else max(0, len(body))
        self.version = version

    def __str__(self):
        return "{}|v{}|{}|{}|{}".format(
            type(self).__name__, self.version, self.type.name, self.length,
            self.body)


class ConfigMessage(Message):
    def __init__(self,
                 type: DebugMsgType,
                 body: str = "",
                 length: int = None,
                 version: int = 2):
        super(ConfigMessage, self).__init__(type, body, length, version)

    @staticmethod
    def header_from_bytes(msg):
        if len(msg) >= 12:
            fmt = "<4sBBxxI"
            (msg_head, raw_version, raw_type, msg_size) = struct.unpack(
                fmt, msg[:12])
            return msg_head, raw_version, ConfigMsgType(raw_type), msg_size

    def toBytes(self):
        msg_format = "<4sBBxxI{}s".format(len(self.body))
        return struct.pack(msg_format, b'CONF', self.version, self.type.value,
                           self.length, self.body.encode())


class DebugMessage(Message):
    def __init__(self,
                 type: DebugMsgType,
                 body: str = "",
                 length: int = None,
                 version: int = 1):
        super(DebugMessage, self).__init__(type, body, length, version)

    def toBytes(self):
        fmt = "<4sbbxxIxxxx{}s".format(self.length)
        return struct.pack(fmt, b'DMSG', self.version, self.type.value,
                           self.length, self.body.encode())

    @staticmethod
    def header_from_bytes(msg):
        if len(msg) >= 16:
            fmt = "<4sbbxxIxxxx"
            (msg_head, raw_version, raw_type, msg_size) = struct.unpack(
                fmt, msg[:16])
            return (msg_head, raw_version, DebugMsgType(raw_type), msg_size)

    @staticmethod
    def to_body(type, msg):
        if type == DebugMsgType.image:
            return msg
        else:
            return msg.decode(errors='ignore')

    @staticmethod
    def get_image(body):
        fmt = "<QHHH"
        (timestamp, width, height, key_length) = struct.unpack(fmt, body[:14])
        return body[14:14 + key_length].decode(), timestamp, width, height, body[
            14 + key_length:]

    @staticmethod
    def parse_data(d: dict) -> Data:
        if d.get("isImage", False):
            return DebugImage(
                d["key"],
                d.get("timestamp", 0),
                d.get("width", 0),
                d.get("height", 0),
                d.get("value", b'')
            )
        else:
            return DebugValue(
                d["key"],
                d.get("timestamp", 0),
                d.get("value", 0)
            )
