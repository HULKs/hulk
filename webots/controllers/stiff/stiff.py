from controller import Supervisor
import sys

TIME_STEP = 10

supervisor = Supervisor()

chest_button_channel = supervisor.getDevice('ChestButton Channel')
count = 0
pressed = 0

while pressed < 1 and supervisor.step(TIME_STEP) != -1:
    if count == 20:
        pressed += 1
        chest_button_channel.send(b'\x01')
    count += 1
