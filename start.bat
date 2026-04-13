@echo off
title Hermes Agent Launcher

echo Stopping existing services...
taskkill /F /IM hermes.exe 2>nul
echo Done.

echo.
echo =========================================
echo    Hermes Agent - Starting All Services
echo =========================================
echo.

echo [1/2] Starting backend (port 3848)...
start "Hermes-Backend" cmd /k "cd /d G:\opencode-project\hermes-rs\target\release && hermes.exe gateway start"

echo [2/2] Starting frontend (port 1420)...
start "Hermes-Frontend" cmd /k "cd /d G:\opencode-project\hermes-rs\crates\ui && npm run dev"

echo.
echo =========================================
echo    STARTUP COMPLETE!
echo    Backend:  http://localhost:3848
echo    Frontend: http://localhost:1420
echo =========================================
echo.
echo Two windows will open - each shows a service log.
echo Close either window to stop that service.
echo.

pause
