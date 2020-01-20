import asyncio
import re
import struct
from datetime import datetime

from .models import RobotInfo


class BroadcastListenerProtocol(asyncio.BaseProtocol):
    """BroadcastListenerProtocol takes care of parsing
    incoming aliveness packets into a RobotInfo structure
    """

    transport = None

    def __init__(self, aliveness_handler, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.aliveness_handler = aliveness_handler

    def connection_made(self, transport):
        self.transport = transport

    def datagram_received(self, data, addr):
        dataformat = "4s32s32sB"
        (header, body_name, head_name, p_num) = struct.unpack(dataformat, data)

        header = header.decode("ascii")
        if header != "LIVE":
            return

        # Keep chars until null terminator and decode
        (head_name, body_name) = [
            c_str[:c_str.index(b'\0')].decode("ascii")
            for c_str in [head_name, body_name]
        ]

        if not head_name.startswith("tuhhnao"):
            return

        # Extract numbers (e.g. 22 for tuhhnao22)
        (head_number, body_number) = [
            int(re.search("(?:tuhhnao)(\\d{2})", name).group(1))
            for name in [head_name, body_name]
        ]

        ri = RobotInfo(head_num=head_number, body_num=body_number, player_num=p_num)
        self.aliveness_handler.handle_message(info=ri, address=addr[0],
                                              timestamp=datetime.now())
