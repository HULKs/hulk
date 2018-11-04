import asyncio as a
import typing as ty

import mate.net.utils as netutils
from mate.net.nao_data import Data


class NaoProtocol(a.Protocol):
    def __init__(self, loop: a.AbstractEventLoop):
        super(NaoProtocol, self).__init__()
        self.loop = loop
        self.on_con_lost = self.loop.create_future()
        self.transport = None

        self.data = {}
        self.message = netutils.Message(None)
        self.subscribors = {}
        self.subscribors_queue = {}
        self.msg_type_subscribors = {}
        self.status_subscribors = {}

    def flush_all(self):
        self.flush_subscribors()
        self.flush_data()

    def flush_subscribors(self):
        self.subscribors = {}
        self.subscribors_queue = {}
        self.msg_type_subscribors = {}
        self.status_subscribors = {}

    def flush_data(self):
        self.data = {}

    def connection_made(self, transport):
        self.transport = transport
        if netutils.ConnectionStatusType.connection_lost in self.status_subscribors:
            for callback in self.status_subscribors[netutils.ConnectionStatusType.connection_made].values():
                callback()

    def data_received(self, data):
        ...

    def send(self, data):
        self.transport.write(data)

    def connection_lost(self, exc):
        if netutils.ConnectionStatusType.connection_lost in self.status_subscribors:
            for callback in self.status_subscribors[
                    netutils.ConnectionStatusType.connection_lost].values():
                callback()
        self.on_con_lost.set_result(True)

    def subscribe(self, key: str, subscribor: str,
                  callback: ty.Callable[[Data], None]) -> bool:
        if key not in self.data:
            if key not in self.subscribors_queue:
                self.subscribors_queue[key] = {}
            self.subscribors_queue[key][subscribor] = callback
            return False
        else:
            if key not in self.subscribors:
                self.subscribors[key] = {}
            self.subscribors[key][subscribor] = callback
            return True

    def subscribe_msg_type(self, msg_type, subscribor: str,
                           callback: ty.Callable[[], None]):
        if msg_type not in self.msg_type_subscribors:
            self.msg_type_subscribors[msg_type] = {}
        self.msg_type_subscribors[msg_type][subscribor] = callback

    def subscribe_status(self, status_type: netutils.ConnectionStatusType,
                         subscribor: str, callback: ty.Callable):
        if status_type not in self.status_subscribors:
            self.status_subscribors[status_type] = {}
        self.status_subscribors[status_type][subscribor] = callback

    def unsubscribe(self, key: str, subscribor: str) -> bool:
        if key in self.subscribors_queue and subscribor in \
                self.subscribors_queue[key]:
            self.subscribors_queue[key].pop(subscribor)
        if key in self.subscribors and subscribor in self.subscribors[key]:
            self.subscribors[key].pop(subscribor)
            if not self.subscribors[key]:
                self.subscribors.pop(key)
                return True
        return False

    def unsubscribe_msg_type(self, msg_type, subscribor: str):
        if msg_type in self.msg_type_subscribors and subscribor in \
                self.msg_type_subscribors[msg_type]:
            self.msg_type_subscribors[msg_type].pop(subscribor)

    def unsubscribe_status(self, status_type: netutils.ConnectionStatusType,
                           subscribor: str):
        if status_type in self.status_subscribors and subscribor in \
                self.status_subscribors[status_type]:
            self.status_subscribors[status_type].pop(subscribor)
