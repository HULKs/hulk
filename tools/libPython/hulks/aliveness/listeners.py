import asyncio
import socket
import threading

from hulks.constants import ALIVENESS_BROADCAST_PORT
from hulks.util import make_thread_target

from .handlers import AlivenessHandler
from .protocols import BroadcastListenerProtocol


class AlivenessListener:
    """AlivenessListener takes care of creating the infrastructure to
    receive aliveness messages (via BroadcastListenerProtocol) and
    process them (via AlivenessHandler)
    """

    def __init__(self, results_callback):

        self.loop = None
        self.sock = None

        self.aliveness_handler = None
        self.results_callback = results_callback
        self.aliveness_loop = None
        self.aliveness_thread = threading.Thread()

    def start_listening(self):
        self.aliveness_thread = threading.Thread(
            target=make_thread_target(self.__connect_aliveness()))
        self.aliveness_thread.setDaemon(True)
        self.aliveness_thread.start()

        return self

    async def __connect_aliveness(self):
        self.aliveness_loop = asyncio.get_event_loop()

        self.aliveness_handler = AlivenessHandler(
            event_loop=self.aliveness_loop,
            results_callback=self.results_callback)

        endpoint = self.aliveness_loop.create_datagram_endpoint(
            lambda: BroadcastListenerProtocol(
                aliveness_handler=self.aliveness_handler),
            sock=self.__make_socket()
        )

        transport, protocol = await asyncio.wait_for(endpoint, 20)

        try:
            # We need to wait forever since there is no real
            # "connection" that can be lost when listening
            # to broadcast messages.
            will_never_happen = self.aliveness_loop.create_future()
            await will_never_happen
        finally:
            transport.close()

    @staticmethod
    def __make_socket():
        sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_BROADCAST, 1)
        sock.bind(('', ALIVENESS_BROADCAST_PORT))
        return sock
