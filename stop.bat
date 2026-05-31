@echo off
title Local Translator - Stop

echo ========================================
echo   Local Translator - Stopping...
echo ========================================
echo.

:: Stop llama-server (unload model from memory)
tasklist /fi "imagename eq llama-server.exe" | find /i "llama-server.exe" >nul 2>nul
if not errorlevel 1 (
    echo [1/2] Stopping llama-server...
    taskkill /im llama-server.exe /f >nul 2>nul
    echo       Done
) else (
    echo [1/2] llama-server not running
)

:: Stop Java translator
wmic process where "commandline like '%%translater.jar%%'" get processid 2>nul | findstr /r "[0-9]" >nul 2>nul
if not errorlevel 1 (
    echo [2/2] Stopping translator...
    wmic process where "commandline like '%%translater.jar%%'" call terminate >nul 2>nul
    echo       Done
) else (
    echo [2/2] Translator not running
)

echo.
echo   Service stopped. Model unloaded.
echo ========================================
timeout /t 3 /nobreak >nul
