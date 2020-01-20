# a quick-and-dirty module to provide
# pseudo-aliveness by discovering open
# SimRobot sockets (Linux-only for now)

from datetime import datetime

import platform
import sys
import time

from .models import Robot
from .models import RobotInfo
from .models import RobotList


def socket_paths():
    # This whole function feels terrible,
    # any better idea? --martin

    if platform.system() != "Linux":
        print("warning: SimRobot pseudo-aliveness "
              "is only supported on Linux!",
              file=sys.stderr)
        return []

    # Look for open unix sockets
    with open("/proc/net/unix", 'r') as f:
        lines = f.readlines()

    # Keep lines containing "simrobot"
    lines = filter(lambda line: "simrobot" in line, lines)

    # Extract socket paths
    # (like /tmp/simrobot/robot4/config, /tmp/simrobot/robot4/debug)
    lines = map(lambda line: line.split()[-1], lines)

    # Throw away /config, /debug and remove duplicates
    lines = [l.replace("/config", '').replace("/debug", '')
             for l in lines]
    lines = list(set(lines))

    lines.sort()
    return lines


def virtual_robots():
    robots = RobotList()

    for socket_path in socket_paths():
        robot = Robot(
            info=RobotInfo(
                head_num=0,
                body_num=0,
                player_num=0
            ),
            ip_lan=socket_path,
            ip_wlan=socket_path,
        )
        robot.set_aliveness(
            alive=True,
            lan=True,
            wlan=True,
            timestamp=datetime.now(),
            address=socket_path,
            player_num=0
        )
        robots.append(robot)

    return robots
