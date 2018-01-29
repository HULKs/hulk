# Remove the directory of anaconda and intel,
# because they deliver their own version of zlib
$($Env:PATH).Split(';') | where { $_ -notmatch "Anaconda" -and $_ -notmatch "Intel" -and $_ -notmatch "MikTex" } | %{ $temppath += "$_;"};
$Env:PATH = $temppath

$Configuration = $(Get-Content -Raw -Path $PSScriptRoot\..\..\settings.json | ConvertFrom-Json)

$QtPath = $Configuration.QtPath
$VcpkgPath = ($Configuration.VcpkgPath).replace("`${workspaceRoot}", $PSScriptRoot + "\..\..") + "\vcpkg"
$SimRobotPath = ($Configuration.SimRobotPath).replace("`${workspaceRoot}", $PSScriptRoot + "\..\..")

$Path = $Env:Path

$NewPath = $QtPath + ";"
$NewPath += $VcpkgPath + "\installed\x64-windows\bin" + ";"
$NewPath += $SimRobotPath + "\build-vc\x64_RelWithDebInfo\RelWithDebInfo" + ";"
$NewPath += $PSScriptRoot + "\..\..\build-vc\simrobot_x64_RelWithDebInfo\src\tuhhsdk\RelWithDebInfo" + ";"

$Env:Path = $NewPath + $Path

$vcPath = $(& $PSScriptRoot\vswhere.exe -format json | ConvertFrom-Json).installationPath
$devEnv = $vcPath + "\Common7\IDE\devenv.exe"
