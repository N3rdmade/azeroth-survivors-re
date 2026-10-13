@echo off
setlocal
title Azeroth Survivors - LAN Lobby Preview
cd /d "%~dp0"
if not exist survivors-coop.exe (
  echo Missing survivors-coop.exe in this folder.
  pause
  exit /b 1
)
echo.
echo  AZEROTH SURVIVORS - LAN LOBBY
echo  ----------------------------
echo   1. Host a lobby on this PC
echo   2. Join another PC
echo.
choice /C 12 /N /M "Choose 1 or 2: "
if errorlevel 2 goto join
echo.
echo Starting host and connecting your local player...
start "Azeroth Survivors Lobby Server" cmd /k ""%~dp0survivors-coop.exe" host 0.0.0.0:45454"
timeout /t 2 /nobreak >nul
"%~dp0survivors-coop.exe" join 127.0.0.1:45454
pause
exit /b
:join
set "HOSTIP="
set /p "HOSTIP=Enter host's LAN IP address (e.g. 192.168.1.100): "
if not defined HOSTIP exit /b 1
"%~dp0survivors-coop.exe" join %HOSTIP%:45454
pause
