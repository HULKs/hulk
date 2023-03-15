from controller import Supervisor
from simple_websocket_server import WebSocketServer, WebSocket
from threading import Thread

TIME_STEP = 10

supervisor = Supervisor()
chest_button_channel = supervisor.getDevice('ChestButton Channel')
scene_control_server = None

class SceneControl(WebSocket):
    def handle(self):
        if self.data == "reset":
            supervisor.worldReload()
            scene_control_server.close()

def run_scene_control_server():
    scene_control_server = WebSocketServer("localhost", 9980, SceneControl)
    scene_control_server.serve_forever()

websocket_thread = Thread(target=run_scene_control_server)
websocket_thread.start()

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
