@echo off
@pushd "%~dp0"
@REM echo If you killed the background process you can run this to start it again
@REM pause
start /b /min ..\..\memurycard.exe -b
@popd
