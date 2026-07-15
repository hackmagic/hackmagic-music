@echo off
chcp 65001 >nul
echo ========================================
echo HackMagic Music Player - Startup Script
echo ========================================
echo.

REM Check if Rust backend is built
if not exist "target\release\hm.exe" (
    if not exist "target\debug\hm.exe" (
        echo Building Rust backend...
        cargo build --release
        if errorlevel 1 (
            echo Failed to build Rust backend!
            pause
            exit /b 1
        )
        echo.
    )
)

REM Check if Electron dependencies are installed
if not exist "gui\node_modules" (
    echo Installing Electron dependencies...
    cd gui
    npm install
    cd ..
    if errorlevel 1 (
        echo Failed to install Electron dependencies!
        pause
        exit /b 1
    )
    echo.
)

echo Starting Music Player...
echo.
echo   Backend will be auto-started by Electron on port 10280.
echo.

REM Start Electron GUI (it will auto-start the backend daemon)
cd gui
npm start
cd ..

pause
