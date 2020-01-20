import asyncio
import sys
import threading

import PyQt5.QtCore as qtc

from hulks import aliveness
from hulks.aliveness.shortcuts import read_aliveness_cache
from hulks.util import make_thread_target


class Loader:

    def __init__(self):
        self.loader_loop = None
        self.loader_thread = None

        self.aliveness_info = None

        self.signal = None

    def start(self):
        try:
            aliveness.locks.create_listen_lock("mate")
            ar = aliveness.receivers.AlivenessReceiver(
                verbose=False, up_callback=self.__receiver_is_up)
            ar.start_receiving()
            print("Started a new aliveness receiver!", file=sys.stderr)
        except aliveness.errors.LockError:
            print("Using existing aliveness receiver ({})".format(
                aliveness.locks.read_listen_lock(), file=sys.stderr))
            self.__receiver_is_up()

        return self

    def set_signal(self, signal: qtc.pyqtSignal):
        self.signal = signal

    def robots(self):
        return self.aliveness_info["robots"]

    async def __interval(self, sleep_for):
        while True:
            self.aliveness_info = read_aliveness_cache()

            if self.signal is not None:
                self.signal.emit()
            await asyncio.sleep(sleep_for)

    def __receiver_is_up(self):
        # It is safe to read aliveness information at this point
        self.loader_thread = threading.Thread(
            target=make_thread_target(self.__connect_loader()))
        self.loader_thread.setDaemon(True)
        self.loader_thread.start()

    async def __connect_loader(self):
        self.loader_loop = asyncio.get_event_loop()
        self.loader_loop.create_task(self.__interval(sleep_for=2))

        will_never_happen = self.loader_loop.create_future()
        await will_never_happen
