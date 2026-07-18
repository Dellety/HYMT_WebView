@echo off
title Local Translator

echo ========================================
echo   Local Translator - Starting...
echo ========================================
echo.

set "APPDIR=%~dp0"
cd /d "%APPDIR%"

:: Check JAR
if not exist translater.jar (
    echo [ERROR] translater.jar not found. Run build.bat first.
    pause
    exit /b 1
)

:: Check model
set "HAS_MODEL=0"
for %%f in (models\*.gguf) do set "HAS_MODEL=1"
if "%HAS_MODEL%"=="0" (
    echo [ERROR] No GGUF model file in models\ directory
    echo        See models\download.txt for download links
    pause
    exit /b 1
)

:: Open browser after delay
start "" /b cmd /c "timeout /t 3 /nobreak >nul & start http://localhost:7779"

:: Start server
echo [INFO] Starting translator...
java -Xms64m -Xmx256m -jar translater.jar
