# Building on windows

## Dependencies

 - install Visual Studio 2017 Community Edition
 - install git for windows (can be installed via VS2017, have not tried it)
 - install at least QT 5.9

## Install steps

Create a new NaoWorkspace folder.
I will refer to this workspace with $NaoWorkspace in the following snippets.

### Clone repositories

Clone the repositories into the `$NaoWorkspace` using a powershell window or your git tool of choice:

```
git clone https://github.com/Microsoft/vcpkg.git
git clone https://github.com/HULKs/nao.git
```

The directory structure should now look like this:

```
$NaoWorkspace
|- nao
|- SimRobot
|- tools
|- vcpkg
```

Run the following commands in a powershell window:

```
cd $NaoWorkspace/nao
.\scripts\bootstrap.ps1 -QtPath [insert qtpath here]
```

HINT: the QtPath may look like this: "C:\Qt\5.9.1\msvc2017_64\bin"


On the first run, answer with 'y' to the question:

```
Do you want to init vcpkg? y/n: 
``` 

Now you can right-click and `execute with powershell` the `VS_SimRobot.ps1` and the `VS_Nao.ps1` script. Compile SimRobot in the `x64_Release` configuration and the Nao code as `simrobot_x64_Release`.

When this is compiled you can run `SimRobot.ps1` and you should be able to run our code within SimRobot.

## Known issues

 - GLEW library naming see: https://github.com/Microsoft/vcpkg/pull/1164

