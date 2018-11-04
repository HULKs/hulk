import asyncio as a
import copy
import json
import typing as ty

import mate.net.utils as netutils
from mate.net.nao_data import DebugImage
from mate.net.nao_protocol import NaoProtocol


class NaoDebugProtocol(NaoProtocol):
    def __init__(self, loop: a.BaseEventLoop):
        super(NaoDebugProtocol, self).__init__(loop)

    def connection_made(self, transport):
        super(NaoDebugProtocol, self).connection_made(transport)
        self.send_debug_msg(netutils.DebugMsgType.request_list)
        # initialize buffers for data_received
        self.header_buffer = b''
        self.body_buffer = b''
        self.read_header = True
        self.receive_length = 16

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
            msg_head, msg_version, msg_type, msg_size = netutils.DebugMessage.header_from_bytes(
                self.header_buffer)
            if msg_head != b'DMSG':
                print("Received invalid debug header: {}".format(msg_head))
                self.transport.close()
                return

            self.receive_length = msg_size
            self.msg_type = msg_type
            self.msg_version = msg_version

        if self.receive_length == 0 and not self.read_header:
            self.handle_message(
                netutils.DebugMessage(self.msg_type, self.body_buffer,
                                      self.receive_length, self.msg_version))
            self.read_header = True
            self.header_buffer = b''
            self.receive_length = 16

        data = data[length_to_parse:]
        if len(data):
            self.data_received(data)

    def handle_message(self, message: netutils.DebugMessage):
        if message.type == netutils.DebugMsgType.list:
            data = json.loads(message.body)
            for d in data["keys"]:
                parsed = netutils.DebugMessage.parse_data(d)
                self.data[parsed.key] = parsed
            if self.msg_type_subscribors.get(netutils.DebugMsgType.list):
                for callback in self.msg_type_subscribors[
                    netutils.DebugMsgType.list].values():
                    callback()

        if message.type == netutils.DebugMsgType.update:
            data = json.loads(message.body)
            for d in data:
                parsed = netutils.DebugMessage.parse_data(d)
                self.data[parsed.key] = parsed
                parsed_copy = copy.copy(parsed)
                for callback in self.subscribors.get(parsed.key, {}).values():
                    callback(parsed_copy)

        if message.type == netutils.DebugMsgType.image:
            key, width, height, data = netutils.DebugMessage.get_image(
                message.body)
            parsed = DebugImage(key, width, height, data)
            self.data[key] = parsed
            parsed_copy = copy.copy(parsed)
            for callback in self.subscribors.get(key, {}).values():
                callback(parsed_copy)

    def subscribe_queued(self):
        filtered, self.subscribors_queue = netutils.split(
            lambda k: k in self.data, self.subscribors_queue)
        for key in filtered:
            self.send_debug_msg(netutils.DebugMsgType.subscribe, key)
            for subscribor in filtered[key]:
                if key not in self.subscribors:
                    self.subscribors[key] = {}
                self.subscribors[key][subscribor] = filtered[key][subscribor]

    def send_debug_msg(self, msg_type: netutils.DebugMsgType, body: str = ""):
        self.send(netutils.DebugMessage(msg_type, body).toBytes())

    def subscribe(self, key: str, subscribor: str, callback: ty.Callable):
        if super(NaoDebugProtocol, self).subscribe(key, subscribor, callback):
            self.send_debug_msg(netutils.DebugMsgType.subscribe, key)

    def unsubscribe(self, key: str, subscribor: str):
        if super(NaoDebugProtocol, self).unsubscribe(key, subscribor):
            self.send_debug_msg(netutils.DebugMsgType.unsubscribe, key)
