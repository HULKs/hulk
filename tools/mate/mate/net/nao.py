import asyncio
import threading
import typing as ty
import uuid

import mate.net.utils as net_utils
from mate.net.nao_data import DebugImage, DebugValue, ConfigMount
from mate.net.nao_debug import NaoDebugProtocol
from mate.net.nao_config import NaoConfigProtocol
from mate.debug.colorlog import ColorLog

from hulks.util import make_thread_target

logger = ColorLog()


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
        self.connection_established_hook = None
        self.connection_failure_hook = None
        self.debug_transport = None
        self.debug_protocol = None
        self.config_transport = None
        self.config_protocol = None
        self.debug_running = False
        self.config_running = False
        self.connection_lock = threading.Lock()
        self.timeout = 3.0

        # Nao info
        self.nao_head = None
        self.nao_body = None
        self.location = None

    def connect(self,
                nao_address: str,
                established_hook: ty.Callable,
                failure_hook: ty.Callable,
                debug_port: int = 12441,
                config_port: int = 12442):
        self.nao_address = nao_address
        self.debug_port = debug_port
        self.config_port = config_port
        self.connection_established_hook = established_hook
        self.connection_failure_hook = failure_hook
        self.debug_running = False
        self.config_running = False
        self.debug_thread = threading.Thread(
            target=make_thread_target(self.__connect_debug()))
        self.debug_thread.start()
        thread_count = threading.active_count()
        logger.debug(__name__ +
                     ": A new thread has been started. There are " +
                     str(thread_count) + " active threads now.")
        self.config_thread = threading.Thread(
            target=make_thread_target(self.__connect_config()))
        self.config_thread.start()
        thread_count = threading.active_count()
        logger.debug(__name__ +
                     ": A new thread has been started. There are " +
                     str(thread_count) + " active threads now.")

    async def __connect_debug(self):
        self.debug_loop = asyncio.get_event_loop()
        try:
            if self.nao_address.startswith("/"):
                self.debug_transport, self.debug_protocol = await asyncio.wait_for(self.debug_loop.create_unix_connection(
                    lambda: NaoDebugProtocol(self.debug_loop),
                    self.nao_address + "/debug"), self.timeout)
            else:
                self.debug_transport, self.debug_protocol = await asyncio.wait_for(self.debug_loop.create_connection(
                    lambda: NaoDebugProtocol(self.debug_loop),
                    self.nao_address,
                    self.debug_port), self.timeout)
        except ConnectionRefusedError as e:
            self.connection_failure_hook(e)
            return
        except OSError as e:
            self.connection_failure_hook(e)
            return
        except asyncio.futures.TimeoutError as e:
            self.connection_failure_hook(
                asyncio.futures.TimeoutError("Timed out"))
            return

        self.connection_lock.acquire()
        try:
            self.debug_running = True
            if self.is_connected():
                self.connection_established_hook()
        finally:
            self.connection_lock.release()

        try:
            await self.debug_protocol.on_con_lost
        finally:
            self.debug_running = False
            self.debug_transport.close()

    async def __connect_config(self):
        self.config_loop = asyncio.get_event_loop()
        try:
            if self.nao_address.startswith("/"):
                self.config_transport, self.config_protocol = await asyncio.wait_for(self.config_loop.create_unix_connection(
                    lambda: NaoConfigProtocol(self.config_loop),
                    self.nao_address + "/config"), self.timeout)
            else:
                self.config_transport, self.config_protocol = await asyncio.wait_for(self.config_loop.create_connection(
                    lambda: NaoConfigProtocol(self.config_loop),
                    self.nao_address,
                    self.config_port), self.timeout)
        except ConnectionRefusedError as e:
            self.connection_failure_hook(e)
            return
        except OSError as e:
            self.connection_failure_hook(e)
            return
        except asyncio.futures.TimeoutError as e:
            self.connection_failure_hook(
                asyncio.futures.TimeoutError("Timed out"))
            return

        self.connection_lock.acquire()
        try:
            self.config_running = True
            if self.is_connected():
                self.connection_established_hook()
        finally:
            self.connection_lock.release()

        try:
            await self.config_protocol.on_con_lost
        finally:
            self.config_running = False
            self.config_transport.close()

    def get_debug_data(self) -> ty.Dict[str, ty.Union[DebugValue, DebugImage]]:
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
            logger.debug(__name__ + ": Unsubscribing: " + key)
            self.debug_protocol.send_debug_msg(
                net_utils.DebugMsgType.unsubscribe, key)
        self.debug_protocol.flush_all()
        self.config_protocol.flush_all()
        self.debug_running = False
        self.config_running = False
        self.debug_loop.call_soon_threadsafe(self.debug_transport.close)
        self.config_loop.call_soon_threadsafe(self.config_transport.close)

    def is_connected(self) -> bool:
        return self.debug_running and self.config_running
