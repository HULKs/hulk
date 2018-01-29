# SimRobot

## Dependencies

 - Qt5 (Core Gui Widgets)
 - LibXml2
 - libode
 - Glew
 - OpenGL

## For Linux

### Building SimRobot

```bash
./build_simrobot
```

### Building nao code for SimRobot

Note: if your system doesn't have eigen 3.3 or later installed, follow the fix in below link.
https://github.com/HULKs/nao/wiki/Fix-old-Eigen-Versions

*Skip `nao/scripts/setup simrobot` if using the above eigen fix script*

```bash
nao/scripts/setup simrobot
nao/scripts/compile -t simrobot -b <BUILD_TYPE>
```

### Run SimRobot

```bash
nao/tools/SimRobot/build/SimRobot
```

Inside File->Open select a Scene from `nao/tools/SimRobot/Scenes`.

## For Windows

### Building SimRobot

 1. Goto `nao/scripts`
 2. Make sure you had run the `bootstrap.ps1` correctly
 3. Run `VS_SimRobot.ps1` and compile for `x64_RelWithDebInfo` inside Visual Studio 2017

### Building nao code for SimRobot

 1. Goto `nao/scripts`
 2. Run `VS_Nao.ps1` and compile for `x64_SimRobot_RelWithDebInfo`

### Run SimRobot

 1. Goto `nao/scripts`
 2. Run `SimRobot.ps1`
 3. Select Scene from `nao/tools/SimRobot/Scenes`
