import collections
import datetime

RobotInfo = collections.namedtuple("RobotInfo", ["head_num", "body_num", "player_num"])


class Robot:

    def __init__(self, info: RobotInfo, ip_lan: str, ip_wlan: str):
        self.info = info
        self.IP_LAN = ip_lan
        self.IP_WLAN = ip_wlan
        self.is_lan = False
        self.is_wlan = False
        self.is_alive = False
        self.last_address = ""
        self.timestamp = None
        self.player_num = 0

    def __eq__(self, other) -> bool:
        if type(other) != Robot:
            raise TypeError('Item is not of type %s' % Robot)
        return (self.info.head_num == other.info.head_num) and \
               (self.info.body_num == other.info.body_num)

    def __repr__(self):
        return "{}(info={}, IP_LAN='{}', alive={})".format(
            self.__class__.__name__, self.info,
            self.IP_LAN, self.is_alive)

    def set_aliveness(self, alive: bool, lan: bool, wlan: bool,
                      timestamp: datetime, address: str, player_num: int):
        self.is_alive = alive
        self.is_lan = lan
        self.is_wlan = wlan
        self.timestamp = timestamp
        self.last_address = address
        self.player_num = player_num


class RobotList(list):

    def append(self, item: Robot):
        if not isinstance(item, Robot):
            raise TypeError('Item is not of type %s' % Robot)
        super(RobotList, self).append(item)

    def __contains__(self, info: RobotInfo) -> bool:
        for robot in self:
            if robot.info == info:
                return True
        return False

    def __repr__(self):
        return "{}[{}]".format(self.__class__.__name__,
                               ", ".join([str(robot) for robot in self]))

    def remove(self, info: RobotInfo):
        for i, r in enumerate(self):
            if r.info == info:
                self.__delitem__(i)
