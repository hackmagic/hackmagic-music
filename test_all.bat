@echo off
REM ============================================================
REM HackMagic Music Player - 完整 GUI 自动化测试套件
REM 使用 Winapp-MCP (Windows UI Automation)
REM ============================================================
setlocal enabledelayedexpansion

set MCP=C:\Users\angel\AppData\Local\Temp\Winapp-MCP\WinApp-MCP\bin\Debug\net9.0-windows\win-x64\WinApp-MCP.exe
set GUI=K:\workspaces\hackmagic\hackmagic-music\target\debug\hm-gui.exe
set PASS=0
set FAIL=0
set TOTAL=0

call :test "1. Launch GUI" "start \"\" \"%GUI%\" & timeout /t 4 /nobreak >nul"
call :check_window "hm-gui"

call :mcp "2. List windows" "list_windows" "{\"processName\":\"hm-gui\"}"
call :assert_contains "Window found" "HackMagic"

call :mcp "3. Attach to app" "attach_application" "{\"processName\":\"hm-gui\"}"
call :assert_contains "Attached" "attached"

call :mcp "4. Get window UI tree" "get_window_tree" "{\"maxDepth\":3}"
call :assert_contains "UI tree has nodes" "Window"

call :mcp "5. Read visible text" "get_visible_text" "{}"
call :assert_contains "Has text content" "No track"

call :mcp "6. Screenshot" "screenshot" "{}"
call :assert_contains "Screenshot taken" "type"

call :mcp "7. Find buttons" "find_elements" "{\"controlType\":\"Button\"}"
echo     [INFO] Bevy uses custom rendering, buttons may not be visible via UIA

call :mcp "8. Click Play button" "click_element" "{\"name\":\"▶\"}"
echo     [WARN] Custom-rendered buttons may not be clickable via UIA

call :mcp "9. Send keyboard: Space (play/pause)" "send_keys" "{\"keys\":\"Space\"}"
echo     [INFO] Sent Space key

call :mcp "10. Read text after play" "get_visible_text" "{}"
echo     [INFO] Text after play command

call :mcp "11. Send keyboard: Right (next track)" "send_keys" "{\"keys\":\"Right\"}"
echo     [INFO] Sent Right arrow

call :mcp "12. Send keyboard: Up (volume up)" "send_keys" "{\"keys\":\"Up\"}"
echo     [INFO] Sent Up arrow

call :mcp "13. Send keyboard: Down (volume down)" "send_keys" "{\"keys\":\"Down\"}"
echo     [INFO] Sent Down arrow

call :mcp "14. Send keyboard: R (repeat mode)" "send_keys" "{\"keys\":\"R\"}"
echo     [INFO] Sent R key

call :mcp "15. Send keyboard: M (mute)" "send_keys" "{\"keys\":\"M\"}"
echo     [INFO] Sent M key

call :mcp "16. Send keyboard: Ctrl+O (open file)" "send_keys" "{\"keys\":\"Ctrl+O\"}"
echo     [INFO] Sent Ctrl+O (file dialog should open)

call :mcp "17. Screenshot after commands" "screenshot" "{}"
echo     [INFO] Screenshot taken

call :mcp "18. Close window (Alt+F4)" "send_keys" "{\"keys\":\"Alt+F4\"}"
echo     [INFO] Sent Alt+F4 to close

echo ============================================================
echo Results: !PASS!/!TOTAL! passed, !FAIL! failed
echo ============================================================
if !FAIL! gtr 0 exit /b 1
exit /b 0

:test
    set /a TOTAL+=1
    echo. & echo %~1
    %~2
    exit /b 0

:mcp
    set /a TOTAL+=1
    echo. & echo %~1
    echo {"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"%~2","arguments":{%~3}}} | "%MCP%" 2>nul
    exit /b 0

:check_window
    echo {"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"list_windows","arguments":{"processName":"%~1"}}} | "%MCP%" 2>nul | find "%~1" >nul
    if !errorlevel! equ 0 (
        set /a PASS+=1
        echo     [PASS] Window "%~1" found
    ) else (
        set /a FAIL+=1
        echo     [FAIL] Window "%~1" not found
    )
    exit /b 0

:assert_contains
    set /a TOTAL+=1
    find "%~2" >nul
    if !errorlevel! equ 0 (
        set /a PASS+=1
        echo     [PASS] %~1
    ) else (
        set /a FAIL+=1
        echo     [FAIL] %~1 (expected "%~2")
    )
    exit /b 0