@echo off
echo ========================================
echo HackMagic Music Player - Startup Script
echo ========================================
echo.

REM Check if Rust backend is built
if not exist "target\release\hm.exe" (
    echo Building Rust backend...
    cargo build --release
    if errorlevel 1 (
        echo Failed to build Rust backend!
        pause
        exit /b 1
    )
    echo.
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

REM Start backend
start "Music Player Backend" /MIN target\release\hm.exe daemon

REM Wait a moment for backend to start
timeout /t 2 /nobreak >nul

REM Start Electron GUI
cd gui
npm start
cd ..