from controller import Supervisor
import sys

TIME_STEP = 10

supervisor = Supervisor()

# do this once only
#robot_node = supervisor.getFromDef("MY_ROBOT")
#if robot_node is None:
#    sys.stderr.write("No DEF MY_ROBOT node found in the current world file\n")
#    sys.exit(1)
#trans_field = robot_node.getField("translation")#

chest_button_channel = supervisor.getDevice('ChestButton Channel')
count = 0
pressed = 0

while pressed < 3 and supervisor.step(TIME_STEP) != -1:
    if count == 20:
        pressed += 1
        print("pressed", pressed)
        chest_button_channel.send(b'\x01')
    if count == 220:
        pressed += 1
        print("pressed", pressed)
        chest_button_channel.send(b'\x01')
    if count == 250:
        pressed += 1
        print("pressed", pressed)
        chest_button_channel.send(b'\x01')
    count += 1
#    # this is done repeatedly
#    values = trans_field.getSFVec3f()
#    print("MY_ROBOT is at position: %g %g %g" % (values[0], values[1], values[2]))

