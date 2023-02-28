from controller import Supervisor
import websockets

TIME_STEP = 10

supervisor = Supervisor()

async def command_handler(ws, path):
    message = await ws.recv()
    if message == "reset":
        supervisor.simulatorReset()

webots_supervisor_websocket = websockets.serve(command_handler, "localhost", 9980)
chest_button_channel = supervisor.getDevice('ChestButton Channel')
count = 0
pressed = 0

while supervisor.step(TIME_STEP) != -1:
    if count == 20:
        pressed += 1
        chest_button_channel.send(b'\x01')
    if count == 220:
        pressed += 1
        chest_button_channel.send(b'\x01')
    if count == 250:
        pressed += 1
        chest_button_channel.send(b'\x01')
    count += 1
