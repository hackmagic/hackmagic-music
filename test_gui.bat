@echo off
REM GUI自动化测试脚本 - 使用Winapp-MCP
REM 用法: test_gui.bat

set MCP=C:\Users\angel\AppData\Local\Temp\Winapp-MCP\WinApp-MCP\bin\Debug\net9.0-windows\win-x64\WinApp-MCP.exe
set GUI=K:\workspaces\hackmagic\hackmagic-music\target\debug\hm-gui.exe

echo ============================================================
echo 1. Starting HackMagic Music Player GUI...
start "" "%GUI%"
timeout /t 4 /nobreak >nul

echo.
echo 2. Listing windows...
echo {"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"list_windows","arguments":{"processName":"hm-gui"}}} | "%MCP%" 2>nul
echo.

echo 3. Attaching to hm-gui...
echo {"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"attach_application","arguments":{"processName":"hm-gui"}}} | "%MCP%" 2>nul
echo.

echo 4. Getting window tree...
echo {"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_window_tree","arguments":{"maxDepth":4}}} | "%MCP%" 2>nul
echo.

echo 5. Reading visible text...
echo {"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_visible_text","arguments":{}}} | "%MCP%" 2>nul
echo.

echo 6. Taking screenshot...
echo {"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"screenshot","arguments":{}}} | "%MCP%" 2>nul
echo.

echo 7. Finding buttons...
echo {"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"find_elements","arguments":{"controlType":"Button"}}} | "%MCP%" 2>nul
echo.

echo ============================================================
echo Tests complete. Closing hm-gui...
taskkill /f /im hm-gui.exe 2>nul
echo Done.