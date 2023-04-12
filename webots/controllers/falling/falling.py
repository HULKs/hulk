from controller import Supervisor

TIME_STEP = 10

supervisor = Supervisor()
chest_button_channel = supervisor.getDevice('ChestButton Channel')

count = 0
pressed = 0

nao_node = supervisor.getFromDef("NAO")
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
    if count == 330:
        nao_node.setVelocity([0, -2.5, 0])
    count += 1
