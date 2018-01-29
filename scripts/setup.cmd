@echo off

:: save the working directory of the caller and change to the directory where the script is located
pushd %~dp0
:: change to the nao directory
cd ..
:: remove the build directory if it already exists
if exist build rmdir /s /q build
:: create the build directory
mkdir build
:: change to that directory
cd build
:: run cmake
cmake -DREPLAY=ON ..
:: restore the working directory of the caller
popd
:: wait for keypress (needed when started from explorer)
pause
