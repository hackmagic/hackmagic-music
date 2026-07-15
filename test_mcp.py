#!/usr/bin/env python3
"""MCP client test script for Winapp-MCP.
Usage: python3 test_mcp.py

Sends JSON-RPC requests to Winapp-MCP and prints responses.
"""
import json
import subprocess
import sys
import time

MCP_EXE = r"C:\Users\angel\AppData\Local\Temp\Winapp-MCP\WinApp-MCP\bin\Debug\net9.0-windows\win-x64\WinApp-MCP.exe"

def call_mcp(method: str, args: dict = None) -> dict:
    """Send a tools/call request to the MCP server and return the result."""
    req = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": method,
            "arguments": args or {}
        }
    }
    proc = subprocess.Popen(
        [MCP_EXE],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True
    )
    stdout, stderr = proc.communicate(input=json.dumps(req), timeout=15)
    # stderr contains logs, stdout contains the JSON-RPC response
    result = json.loads(stdout)
    return result

def main():
    # 1. Start hm-gui in background
    print("=== Starting hm-gui ===")
    gui_proc = subprocess.Popen(
        [r"K:\workspaces\hackmagic\hackmagic-music\target\debug\hm-gui.exe"],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL
    )
    time.sleep(3)  # Wait for window to appear

    try:
        # 2. List windows
        print("\n=== Listing windows ===")
        result = call_mcp("list_windows", {"processName": "hm-gui"})
        print(json.dumps(result, indent=2, ensure_ascii=False))

        # 3. Attach to the app
        print("\n=== Attaching to hm-gui ===")
        result = call_mcp("attach_application", {"processName": "hm-gui"})
        print(json.dumps(result, indent=2, ensure_ascii=False))

        # 4. Get window tree
        print("\n=== Window tree ===")
        result = call_mcp("get_window_tree", {"maxDepth": 3})
        print(json.dumps(result, indent=2, ensure_ascii=False))

        # 5. Take screenshot
        print("\n=== Screenshot ===")
        result = call_mcp("screenshot", {})
        # Only print first 100 chars of base64 data
        data = result.get("result", {}).get("content", [{}])[0].get("text", "")
        print(f"Screenshot data length: {len(data)} chars")
        if len(data) > 100:
            print(f"Preview: {data[:80]}...")

    finally:
        # Cleanup
        print("\n=== Closing hm-gui ===")
        gui_proc.kill()
        gui_proc.wait()

if __name__ == "__main__":
    main()