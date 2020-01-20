import asyncio as a
import typing as ty

import mate.net.utils as netutils
from mate.net.nao_data import Data
from mate.debug.colorlog import ColorLog

logger = ColorLog()


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
            logger.debug(__name__ + ": Subscribe queue: " + str(key) +
                         " for " + str(subscribor))
            if key not in self.subscribors_queue:
                self.subscribors_queue[key] = {}
            if subscribor in self.subscribors_queue[key]:
                logger.error(__name__ + ": " + subscribor +
                             " is already subscribed to " + key)
            else:
                self.subscribors_queue[key][subscribor] = callback
            return False
        else:
            logger.debug(__name__ + ": Subscribe: " + str(key) +
                         " for " + str(subscribor))
            if key not in self.subscribors:
                self.subscribors[key] = {}
            if subscribor in self.subscribors[key]:
                logger.error(__name__ + ": " + subscribor +
                             " is already subscribed to " + key)
            else:
                self.subscribors[key][subscribor] = callback
            return True

    def subscribe_msg_type(self, msg_type, subscribor: str,
                           callback: ty.Callable[[], None]):
        logger.debug(__name__ + ": Subscribe msg type: " + str(msg_type) +
                     " for " + str(subscribor))
        if msg_type not in self.msg_type_subscribors:
            self.msg_type_subscribors[msg_type] = {}
        self.msg_type_subscribors[msg_type][subscribor] = callback

    def subscribe_status(self, status_type: netutils.ConnectionStatusType,
                         subscribor: str, callback: ty.Callable):
        logger.debug(__name__ + ": Subscribe status: " + str(status_type) +
                     " for " + str(subscribor))
        if status_type not in self.status_subscribors:
            self.status_subscribors[status_type] = {}
        self.status_subscribors[status_type][subscribor] = callback

    def unsubscribe(self, key: str, subscribor: str) -> bool:
        if key in self.subscribors_queue and subscribor in \
                self.subscribors_queue[key]:
            logger.debug(__name__ + ": Unsubscribe from queue: " +
                         str(key) + " for " + str(subscribor))
            self.subscribors_queue[key].pop(subscribor)
        if key in self.subscribors and subscribor in self.subscribors[key]:
            logger.debug(__name__ + ": Unsubscribe: " +
                         str(key) + " for " + str(subscribor))
            self.subscribors[key].pop(subscribor)
            if not self.subscribors[key]:
                self.subscribors.pop(key)
                return True
        return False

    def unsubscribe_msg_type(self, msg_type, subscribor: str):
        if msg_type in self.msg_type_subscribors and subscribor in \
                self.msg_type_subscribors[msg_type]:
            logger.debug(__name__ + ": Unsubscribe msg type: " +
                         str(msg_type) +
                         " for " + str(subscribor))
            self.msg_type_subscribors[msg_type].pop(subscribor)

    def unsubscribe_status(self, status_type: netutils.ConnectionStatusType,
                           subscribor: str):
        if status_type in self.status_subscribors and subscribor in \
                self.status_subscribors[status_type]:
            logger.debug(__name__ + ": Unsubscribe status: " +
                         str(status_type) +
                         " for " + str(subscribor))
            self.status_subscribors[status_type].pop(subscribor)
