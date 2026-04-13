@echo off
title Hermes Agent - STOP

echo =========================================
echo    Stopping All Services
echo =========================================
echo.

echo Stopping services...
taskkill /F /IM hermes.exe 2>nul
taskkill /F /IM node.exe 2>nul

echo.
echo All services stopped.
echo.

pause
