@echo off
@pushd "%~dp0"
echo If you killed the background process you can run this to start it again
pause
start /b /min ..\..\savedir.exe -b
@popd
