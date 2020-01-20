#!/usr/bin/env python3
import socket
import struct
import json
import argparse
from datetime import datetime
import logging


class GameControllerMessage:
    """
    Type to load, store, and format messages sent by the Robocup Game Controller
     bytes  field-name
       4    header
       1    version
       1    packet number
       1    numPlayers
       1    competitionPhase
       1    competitionType
       1    gamePhase
       1    gameState
       1    setPlay
       1    firstHalf
       1    kickingTeam
       2    secsRemaining
       2    secondaryTime
    """

    def __init__(self, buf):
        self.loadFromBytes(buf)

    def loadFromBytes(self, buf):
        d = struct.unpack_from("<4s10Bhh", buf)
        self.header = d[0].decode()
        self.version = d[1]
        self.packet_number = d[2]
        self.numPlayers = d[3]
        self.competitionPhase = d[4]
        self.competitionType = d[5]
        self.gamePhase = d[6]
        self.gameState = d[7]
        self.setPlay = d[8]
        self.firstHalf = d[9]
        self.kickingTeam = d[10]
        self.secsRemaining = d[11]
        self.secondaryTime = d[12]

    def dump(self):
        s = "time " + fmtTime(self.secsRemaining) + "-" + \
            fmtTime(self.secondaryTime)
        for k, v in self.__dict__.items():
            s += f"\n{k} {v}"
        return s


class SPLReceiver:

    """Listens for broadcast GameControllerMessage-s on a given port"""

    def __init__(self, address="", port=3838, dumpfile=None, callback=None, verbose=False):
        self.address = address
        self.port = port
        self.dumpfile = dumpfile
        self.callback = callback
        self.verbose = verbose
        self.lastSender = None
        self.lastConflict = datetime(1970, 1, 1)

    def run(self):
        sock = socket.socket(
            socket.AF_INET, socket.SOCK_DGRAM, socket.IPPROTO_UDP)
        # Important! Enable other programs to listen on the same port
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        sock.bind((self.address, self.port))

        while True:
            try:
                data, addr = sock.recvfrom(1024)

                message = GameControllerMessage(data)
                # Check whether the last message came from a different sender
                if not self.lastSender is None and self.lastSender != addr:
                    logging.warning(
                        f"Different message origin! {self.lastSender} -> {addr}")
                    self.lastConflict = datetime.now()
                self.lastSender = addr
            except socket.error as e:
                logging.warning(e)
            else:
                logging.info(f"From {addr}")
                logging.info(f"Size {len(data)}")
                logging.info(f"Data {data}")
                if self.dumpfile:
                    with open(self.dumpfile, "w") as outfile:
                        outfile.seek(0)
                        timediff = (datetime.now() -
                                    self.lastConflict).total_seconds()
                        if self.dumpfile.endswith(".json"):
                            d = {}
                            d.update(message.__dict__)
                            d.update({
                                "lastConflict": str(self.lastConflict),
                                "timeSinceLastConflict": timediff
                            })
                            json.dump(d, outfile, indent=2)
                        else:
                            outfile.write(message.dump())
                            outfile.write(
                                f"\nlastConflict {self.lastConflict}")
                            outfile.write(
                                f"\ntimeSinceLastConflict {timediff:.2f}")
                            outfile.truncate()
                if self.callback:
                    self.callback(addr, message)


def fmtTime(seconds):
    prefix = "" if seconds >= 0 else "-"
    seconds = abs(seconds)
    return "{}{:02}:{:02}".format(prefix, seconds//60, seconds % 60)


def main():
    parser = argparse.ArgumentParser("Gamestate Receiver")
    parser.add_argument("file", default="/tmp/gamestate.txt", nargs="?")
    parser.add_argument("-v", "--verbose", action="store_true")
    args = parser.parse_args()

    loglevel = logging.WARNING
    if args.verbose:
        loglevel = logging.DEBUG
    logging.basicConfig(format='%(levelname)s:%(message)s', level=loglevel)

    receiver = SPLReceiver(dumpfile=args.file, verbose=args.verbose)
    receiver.run()


if __name__ == "__main__":
    main()
