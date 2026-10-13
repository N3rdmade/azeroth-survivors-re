@echo off
setlocal EnableExtensions
cd /d "%~dp0"
title Azeroth Survivors - Windows Launcher
echo =========================================
echo      AZEROTH SURVIVORS - RUST
echo =========================================
echo.
set "DATADIR=%~dp0Data"
if exist "%DATADIR%\terrain.MPQ" if exist "%DATADIR%\patch.MPQ" goto launch
echo A complete WoW 1.12.1 Data folder was not found beside the game.
echo The small custom patch MPQs alone are not sufficient for this build.
echo.
echo Copy a compatible WoW 1.12.1 Data folder next to the EXE,
echo or enter the full path to an existing Data folder here.
echo.
set /p "DATADIR=Path to Data folder (blank to exit): "
if not defined DATADIR goto end
set "DATADIR=%DATADIR:"=%"
if not exist "%DATADIR%\terrain.MPQ" goto missing
if not exist "%DATADIR%\patch.MPQ" goto missing
goto launch
:missing
echo.
echo Required files missing: terrain.MPQ and/or patch.MPQ.
echo Checked: "%DATADIR%"
pause
goto end
:launch
set "WOW_DATA=%DATADIR%"
echo.
echo Launching with assets at "%WOW_DATA%"
echo If the loading screen hangs, close the game and send startup-log.txt.
"%~dp0warcraft-survivors.exe" > "%~dp0startup-log.txt" 2>&1
echo.
echo Game exited. Diagnostic output saved to startup-log.txt
pause
:end
endlocal
