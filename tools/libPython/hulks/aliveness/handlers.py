import asyncio
import itertools
import json
import os
import re
import typing as ty
from datetime import datetime, timedelta

from . import simrobot
from hulks.constants import (
    ALIVENESS_TIMEOUT_ALIVE,
    ALIVENESS_TIMEOUT_OFFLINE
)
from .models import Robot, RobotInfo, RobotList

ROBOT_IPs_PATH = os.path.abspath(os.path.join(os.path.dirname(__file__),
                                              "ips.json"))


class AlivenessHandler:
    """AlivenessHandler processes incoming aliveness messages, sets the
    aliveness status based on timeouts, etc.
    """

    robots = RobotList()
    robot_IPs = ty.Dict[str, str]

    def __init__(self, event_loop: asyncio.AbstractEventLoop,
                 results_callback=None):
        self.event_loop = event_loop
        self.results_callback = results_callback

        self.event_loop.create_task(self.__interval(sleep_for=2))

        with open(ROBOT_IPs_PATH, "r") as f:
            self.robot_IPs = json.load(f)

    async def __interval(self, sleep_for):
        while True:
            # Most of this is blocking, yet it does
            # not consume a relevant amount of time

            # Cram SimRobot pseudo-aliveness in there
            all_robots = list(
                itertools.chain(self.robots, simrobot.virtual_robots()))

            self.check_timestamps()

            if self.results_callback is not None:
                self.results_callback(all_robots)
            await asyncio.sleep(sleep_for)

    def handle_message(self, info: RobotInfo, address: str, timestamp: datetime):
        if info not in self.robots:
            self.add_robot(info)
        ip_match = re.search("10.([01]).24.(\\d{2})", address)
        second_byte_ip = int(ip_match.group(1))
        is_lan = False
        if second_byte_ip == 1:
            is_lan = True
        for index, robot in enumerate(self.robots):
            if robot.info == info:
                self.robots[index].set_aliveness(alive=True, lan=is_lan, wlan=not is_lan,
                                                 timestamp=timestamp, address=address,
                                                 player_num=info.player_num)
                return

    def add_robot(self, info: RobotInfo):
        ip_lan = self.robot_IPs["LAN"]["tuhhNao" + str(info.head_num)]
        ip_wlan = self.robot_IPs["WLAN"]["tuhhNao" + str(info.head_num)]
        # Check if robot exist already
        info: RobotInfo
        if info in self.robots:
            self.robots.remove(info)
        # Add robot
        self.robots.append(Robot(info=info, ip_lan=ip_lan, ip_wlan=ip_wlan))

    def check_timestamps(self):
        for robot in self.robots:
            if (datetime.now() - robot.timestamp) > \
               timedelta(seconds=ALIVENESS_TIMEOUT_ALIVE):
                robot.is_alive = False
            if (datetime.now() - robot.timestamp) > \
               timedelta(seconds=ALIVENESS_TIMEOUT_OFFLINE):
                self.robots.remove(robot.info)
