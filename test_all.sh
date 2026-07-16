#!/bin/bash
# HackMagic Music Player - GUI Test Suite (bash version)
# Usage: bash test_all.sh

MCP="/tmp/Winapp-MCP/WinApp-MCP/bin/Debug/net9.0-windows/win-x64/WinApp-MCP.exe"
GUI="K:/workspaces/hackmagic/hackmagic-music/target/debug/hm-gui.exe"
PASS=0
FAIL=0
TOTAL=0

mcp_call() {
    local name="$1" method="$2" args="${3:-{}}"
    ((TOTAL++))
    echo ""
    echo "=== $name ==="
    local result
    result=$(echo "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"$method\",\"arguments\":$args}}" | timeout 10 "$MCP" 2>/dev/null)
    if [ -n "$result" ]; then
        echo "$result" | head -c 500
        echo ""
        # Check if result contains "error" key
        if echo "$result" | grep -q '"isError":true\|"error"'; then
            ((FAIL++))
            echo "  [FAIL]"
        else
            ((PASS++))
            echo "  [PASS]"
        fi
    else
        ((FAIL++))
        echo "  [FAIL] No response"
    fi
}

echo "============================================"
echo "HackMagic Music Player - GUI Test Suite"
echo "============================================"

# 1. Start GUI
echo ""
echo "=== 1. Starting hm-gui ==="
start "" "$GUI" 2>/dev/null || "$GUI" &
GUI_PID=$!
sleep 4

# 2-18. Run tests
mcp_call "2. List windows" "list_windows" "{\"processName\":\"hm-gui\"}"
mcp_call "3. Attach to app" "attach_application" "{\"processName\":\"hm-gui\"}"
mcp_call "4. Get window tree" "get_window_tree" "{\"maxDepth\":3}"
mcp_call "5. Read visible text" "get_visible_text" "{}"
mcp_call "6. Screenshot" "screenshot" "{}"
mcp_call "7. Send Space (play/pause)" "send_keys" "{\"keys\":\"Space\"}"
mcp_call "8. Send Right (next)" "send_keys" "{\"keys\":\"Right\"}"
mcp_call "9. Send Up (vol up)" "send_keys" "{\"keys\":\"Up\"}"
mcp_call "10. Send Down (vol down)" "send_keys" "{\"keys\":\"Down\"}"
mcp_call "11. Send R (repeat)" "send_keys" "{\"keys\":\"R\"}"
mcp_call "12. Send M (mute)" "send_keys" "{\"keys\":\"M\"}"
mcp_call "13. Send Ctrl+O (open file)" "send_keys" "{\"keys\":\"Ctrl+O\"}"
mcp_call "14. Send Ctrl+F (open folder)" "send_keys" "{\"keys\":\"Ctrl+F\"}"
mcp_call "15. Send S (stop)" "send_keys" "{\"keys\":\"S\"}"
mcp_call "16. Send Escape (close dialogs)" "send_keys" "{\"keys\":\"Escape\"}"
mcp_call "17. Send Alt+F4 (close window)" "send_keys" "{\"keys\":\"Alt+F4\"}"

# Cleanup
echo ""
echo "=== Cleaning up ==="
taskkill /f /im hm-gui.exe 2>/dev/null || true

echo ""
echo "============================================"
echo "Results: $PASS/$TOTAL passed, $FAIL failed"
echo "============================================"
[ $FAIL -gt 0 ] && exit 1
exit 0