param(
    [string]$VcpkgPath = "`${workspaceRoot}\..",
    [string]$SimRobotPath = "`${workspaceRoot}\tools\SimRobot",
    [Parameter(Mandatory=$true)][string]$QtPath
)

$Configuration = @{VcpkgPath=$VcpkgPath;QtPath=$QtPath;SimRobotPath=$SimRobotPath}
$Configuration | ConvertTo-Json | Set-Content $PSScriptRoot\..\settings.json

#$Configuration = $(Get-Content -Raw -Path $PSScriptRoot\settings.json | ConvertFrom-Json)

$VcpkgPath = $Configuration.VcpkgPath
$VcpkgNaoToolchainPath = ($VcpkgPath + "\vcpkg\scripts\buildsystems\vcpkg.cmake").replace("\", "\\")
$VcpkgSimRobotToolchainPath = ($VcpkgPath + "\..\..\vcpkg\scripts\buildsystems\vcpkg.cmake").replace("\", "\\")
$SimRobotIncludeDir = ($SimRobotPath + "\Src\SimRobotCore2").replace('\', '\\')
$NaoCMakeSettings = (Get-Content $PSScriptRoot\..\.CMakeSettings.json.template).replace('VCPKG_PATH_HERE', $VcpkgNaoToolchainPath).replace('SIMROBOT_PATH_HERE', $SimRobotIncludeDir)

$RealSimRobotPath = $SimRobotPath.replace("`${workspaceRoot}", $PSScriptRoot + "\..")
$SimRobotCMakeSettings = (Get-Content $RealSimRobotPath\.CMakeSettings.json.template).replace('VCPKG_PATH_HERE', $VcpkgSimRobotToolchainPath)
$RealVcpkgPath = ($Configuration.VcpkgPath).replace("`${workspaceRoot}", $PSScriptRoot + "\..") + "\vcpkg"

Set-Content -Path $PSScriptRoot\..\CMakeSettings.json -Value $NaoCMakeSettings
Set-Content -Path $RealSimRobotPath\CMakeSettings.json -Value $SimRobotCMakeSettings

$confirmation = Read-Host "Do you want to init vcpkg? y/n"
if ($confirmation -eq 'y'){
	Push-Location -Path $RealVcpkgPath
	& .\bootstrap-vcpkg.bat
	& .\vcpkg install --triplet x64-windows zlib libpng libjpeg-turbo boost fftw3 eigen3 portaudio glew ode libsndfile libxml2
	Pop-Location
}

echo "done"

Read-Host -Prompt "Press Enter to continue"
