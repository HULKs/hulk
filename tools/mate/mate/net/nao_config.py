import json
import typing as ty
import asyncio as a

import mate.net.utils as netutils
from mate.net.nao_data import ConfigMount
from mate.net.nao_protocol import NaoProtocol
from mate.debug.colorlog import ColorLog

logger = ColorLog()


class NaoConfigProtocol(NaoProtocol):
    def __init__(self, loop: a.BaseEventLoop):
        super(NaoConfigProtocol, self).__init__(loop)

    def connection_made(self, transport):
        super(NaoConfigProtocol, self).connection_made(transport)
        self.send_config_msg(netutils.ConfigMsgType.get_mounts)
        # initialize buffers for data_received
        self.header_buffer = b''
        self.body_buffer = b''
        self.read_header = True
        self.receive_length = 12

    def data_received(self, data):
        length_to_parse = min(self.receive_length, len(data))
        if self.read_header:
            self.header_buffer = self.header_buffer + data[0:length_to_parse]
        else:
            self.body_buffer = self.body_buffer + data[0:length_to_parse]

        self.receive_length -= length_to_parse

        if self.receive_length == 0 and self.read_header:
            self.read_header = False
            self.body_buffer = b''
            msg_head, msg_version, msg_type, msg_size = netutils.ConfigMessage.header_from_bytes(
                self.header_buffer)
            if msg_head != b'CONF':
                logger.warning(__name__ + ": Received invalid config header" +
                               ": {}".format(msg_head))
                self.transport.close()
                return

            self.receive_length = msg_size
            self.msg_type = msg_type
            self.msg_version = msg_version

        if self.receive_length == 0 and not self.read_header:
            self.handle_message(
                netutils.ConfigMessage(self.msg_type, self.body_buffer,
                                       self.receive_length, self.msg_version))
            self.read_header = True
            self.header_buffer = b''
            self.receive_length = 12

        data = data[length_to_parse:]
        if len(data):
            self.data_received(data)

    def handle_message(self, message):
        if message.type == netutils.ConfigMsgType.send_mounts:
            data = json.loads(message.body)
            for d in data["keys"]:
                self.data[d["key"]] = ConfigMount(d["key"], d["filename"], {})

        if message.type == netutils.ConfigMsgType.send_keys:
            data = json.loads(message.body)
            mount_name = data["mountPoint"]
            if mount_name not in self.data:
                self.data[mount_name] = ConfigMount(mount_name, "n/a", {})
            for d in data["keys"]:
                if not d["key"].startswith('//'):
                    self.data[mount_name].data[d["key"]] = d["value"]
            try:
                for callback in self.subscribors.get(mount_name, {}).values():
                    callback(self.data[mount_name])
            except RuntimeError as e:
                logger.warning(__name__ +
                               ": Exception in handle_message: " +
                               str(e))

    def send_config_msg(self, msg_type: netutils.ConfigMsgType,
                        body: str = ""):
        self.send(netutils.ConfigMessage(msg_type, body).toBytes())

    def set(self, mount_name: str, key: str, value: str):
        body = json.dumps([{"mp": mount_name, "key": key, "value": value}])
        self.send_config_msg(netutils.ConfigMsgType.set, body)

    def save(self):
        self.send_config_msg(netutils.ConfigMsgType.save)

    def request_keys(self, mount):
        self.send_config_msg(netutils.ConfigMsgType.get_keys, mount)

    def subscribe(self, mount: str, subscribor: str, callback: ty.Callable):
        if super(NaoConfigProtocol, self).subscribe(mount, subscribor,
                                                    callback):
            self.request_keys(mount)
