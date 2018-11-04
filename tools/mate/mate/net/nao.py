import asyncio
import threading
import typing as ty
import uuid

import mate.net.utils as net_utils
from mate.net.nao_data import Data, ConfigMount
from mate.net.nao_debug import NaoDebugProtocol
from mate.net.nao_config import NaoConfigProtocol


class Nao:
    def __init__(self):
        self.identifier = uuid.uuid4()

        self.debug_thread = threading.Thread()
        self.config_thread = threading.Thread()

        self.debug_loop = None
        self.config_loop = None
        self.nao_address = None
        self.debug_port = None
        self.config_port = None
        self.connection_post_hook = None
        self.debug_transport = None
        self.debug_protocol = None
        self.config_transport = None
        self.config_protocol = None

    def connect(self,
                nao_address: str,
                debug_port: int = 12441,
                config_port: int = 12442,
                post_hook: ty.Callable = None):
        self.nao_address = nao_address
        self.debug_port = debug_port
        self.config_port = config_port
        self.connection_post_hook = post_hook
        self.debug_thread = threading.Thread(target=lambda: asyncio.new_event_loop().run_until_complete(self.__connect_debug()))
        self.debug_thread.start()
        self.config_thread = threading.Thread(target=lambda: asyncio.new_event_loop().run_until_complete(self.__connect_config()))
        self.config_thread.start()

    async def __connect_debug(self):
        self.debug_loop = asyncio.get_event_loop()
        if self.nao_address.startswith("/"):
            self.debug_transport, self.debug_protocol = await self.debug_loop.create_unix_connection(
                lambda: NaoDebugProtocol(self.debug_loop), self.nao_address + "/debug")
        else:
            self.debug_transport, self.debug_protocol = await self.debug_loop.create_connection(
                lambda: NaoDebugProtocol(self.debug_loop), self.nao_address, self.debug_port)
        await self.connection_established()

        self.debug_protocol.subscribe_msg_type(
            net_utils.DebugMsgType.list, self.identifier,
            self.debug_protocol.subscribe_queued)

        try:
            await self.debug_protocol.on_con_lost
        finally:
            self.debug_transport.close()

    async def __connect_config(self):
        self.config_loop = asyncio.get_event_loop()
        if self.nao_address.startswith("/"):
            self.config_transport, self.config_protocol = await self.config_loop.create_unix_connection(
                lambda: NaoConfigProtocol(self.config_loop), self.nao_address + "/config")
        else:
            self.config_transport, self.config_protocol = await self.config_loop.create_connection(
                lambda: NaoConfigProtocol(self.config_loop), self.nao_address, self.config_port)
        await self.connection_established()
        try:
            await self.config_protocol.on_con_lost
        finally:
            self.config_transport.close()

    async def connection_established(self):
        if self.debug_protocol and self.config_protocol:
            self.connection_post_hook()

    def get_debug_data(self) -> ty.Dict[str, Data]:
        if self.is_connected():
            return self.debug_protocol.data
        return {}

    def get_config_data(self) -> ty.Dict[str, ConfigMount]:
        if self.is_connected():
            return self.config_protocol.data
        return {}

    debug_data = property(get_debug_data)
    config_data = property(get_config_data)

    def disconnect(self):
        for key in self.debug_protocol.subscribors:
            self.debug_protocol.send_debug_msg(
                net_utils.DebugMsgType.unsubscribe, key)
        self.debug_protocol.flush_all()
        self.config_protocol.flush_all()

        self.debug_loop.call_soon_threadsafe(self.debug_transport.close)
        self.config_loop.call_soon_threadsafe(self.config_transport.close)

    def is_connected(self) -> bool:
        return self.debug_thread.is_alive() and self.config_thread.is_alive()
