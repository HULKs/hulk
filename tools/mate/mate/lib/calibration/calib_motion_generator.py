'''
Generate head motion parameter set for the Nao.

__author__ = "Darshana Adikari"
__copyright__ = "Copyright 2018, RobotING@TUHH / HULKs"
__license__ = ""
__version__ = "0.2"
__maintainer__ = "Darshana Adikari"
__email__ = "darshana.adikari@tuhh.de, darshanaads@gmail.com"
__status__ = "Alpha"
'''

import json
import math


class CalibMotionGenerator(object):
    def __init__(self):
        super(CalibMotionGenerator, self).__init__()

    @staticmethod
    def sign(x):
        return x and (1, -1)[x < 0]

    @staticmethod
    def generateHeadMotion(yawMeasures=3, pitchMeasures=2, interpolateMax=3):
        yawAbsMax = 11.45
        pitchLower = 5.72958
        pitchUpper = 6.60507

        yawIncrement = math.fabs(yawAbsMax / yawMeasures)
        pitchIncrement = math.fabs((pitchUpper - pitchLower) / pitchMeasures)

        headMotions = []

        yawName = "yaw"
        pitchName = "pitch"

        # yaw pitch smoothing

        for j in range(0, interpolateMax):
            headMotions.append({
                yawName: (yawAbsMax * j / interpolateMax),
                pitchName: (pitchLower * j / interpolateMax)
            })

        # direction toggle
        dir = 1

        for j in range(0, pitchMeasures + 1):
            pitch = (j * pitchIncrement + pitchLower)
            if (math.fabs(pitch) > pitchUpper):
                pitch = CalibMotionGenerator.sign(pitch) * pitchUpper

            for i in reversed(range(0, yawMeasures)):
                headMotions.append({
                    yawName: (dir * i * yawIncrement),
                    pitchName: pitch
                })

            j += 1
            dir = -dir

            for i in range(1, yawMeasures + 1):
                headMotions.append({
                    yawName: (dir * i * yawIncrement),
                    pitchName: pitch
                })

        # second interpolation to end the motion
        for j in reversed(range(0, interpolateMax)):
            headMotions.append({
                yawName: -(yawAbsMax * j / interpolateMax),
                pitchName: (pitchUpper * j / interpolateMax)
            })

        # currently just dumps to the terminal.
        # print(json.dumps(headMotions, indent=2))
        return headMotions


if __name__ == '__main__':
    print(json.dumps(CalibMotionGenerator.generateHeadMotion(), indent=2))
