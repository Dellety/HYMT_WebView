@echo off
title Translator - Build

echo ========================================
echo   Local Translator - Build
echo ========================================
echo.

:: Check Java
where java >nul 2>nul
if errorlevel 1 (
    echo [ERROR] java not found, please install JRE and add to PATH
    pause
    exit /b 1
)

echo [1/4] Compiling...
if not exist src\TranslatorServer.java (
    echo [ERROR] src\TranslatorServer.java not found
    pause
    exit /b 1
)

if exist build rmdir /s /q build
mkdir build

javac -source 8 -target 8 -encoding UTF-8 -d build src\TranslatorServer.java
if errorlevel 1 (
    echo [ERROR] Compilation failed
    pause
    exit /b 1
)
echo       OK

echo [2/4] Copying web assets...
xcopy /e /i /q web build\web >nul
echo       OK

echo [3/4] Creating JAR...
cd build
jar cfe translater.jar TranslatorServer TranslatorServer.class TranslatorServer\$*.class
cd ..
echo       OK: build\translater.jar

echo [4/4] Preparing dist...
if exist dist rmdir /s /q dist
mkdir dist

copy build\translater.jar dist\ >nul
xcopy /e /i /q web dist\web >nul
if exist start.bat copy start.bat dist\ >nul
if exist stop.bat  copy stop.bat  dist\ >nul
if exist config.yaml copy config.yaml dist\ >nul

:: Copy llama.cpp
if exist llama-b9103-bin-win-cpu-x64 (
    echo       Copying llama.cpp...
    xcopy /e /i /q llama-b9103-bin-win-cpu-x64 dist\llama-b9103-bin-win-cpu-x64 >nul
) else if exist lib\llama-server.exe (
    mkdir dist\lib
    copy lib\llama-server.exe dist\lib\ >nul
) else (
    echo       [WARN] llama-server.exe not found
)

:: Copy models
if exist models\*.gguf (
    xcopy /e /i /q models dist\models >nul
) else (
    mkdir dist\models 2>nul
    if exist models\download.txt copy models\download.txt dist\models\ >nul
)

echo.
echo ========================================
echo   Build complete! Output: dist\
echo ========================================
pause
