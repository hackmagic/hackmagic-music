#!/usr/bin/env python3
"""GUI自动化测试脚本 - 使用 Winapp-MCP 测试 HackMagic Music Player UI

用法:
    python test_gui.py              # 运行所有测试
    python test_gui.py --screenshot  # 只截图
"""
import json
import subprocess
import sys
import time
import os

MCP_EXE = r"C:\Users\angel\AppData\Local\Temp\Winapp-MCP\WinApp-MCP\bin\Debug\net9.0-windows\win-x64\WinApp-MCP.exe"
HM_GUI = r"K:\workspaces\hackmagic\hackmagic-music\target\debug\hm-gui.exe"

def call_mcp(method: str, args: dict = None, timeout: int = 15) -> dict:
    """Send a JSON-RPC request to Winapp-MCP and return the result."""
    req = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {"name": method, "arguments": args or {}}
    }
    req_str = json.dumps(req) + "\n"
    proc = subprocess.Popen(
        [MCP_EXE],
        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        text=True
    )
    stdout, stderr = proc.communicate(input=req_str, timeout=timeout)
    # Parse JSON-RPC response from stdout
    try:
        return json.loads(stdout)
    except json.JSONDecodeError:
        print(f"[ERROR] Failed to parse MCP response: {stdout[:200]}")
        return {"error": str(stderr[:500])}

def test():
    passed = 0
    failed = 0

    # 1. Start hm-gui
    print("=" * 60)
    print("1. Starting HackMagic Music Player GUI...")
    gui_proc = subprocess.Popen([HM_GUI], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    time.sleep(4)

    try:
        # 2. List windows
        print("\n2. Listing windows...")
        result = call_mcp("list_windows", {"processName": "hm-gui"})
        text = result.get("result", {}).get("content", [{}])[0].get("text", "")
        if "hm-gui" in text.lower() or "HackMagic" in text:
            print("   [PASS] Window found")
            passed += 1
        else:
            print(f"   [FAIL] Window not found. Output: {text[:200]}")
            failed += 1

        # 3. Attach to app
        print("\n3. Attaching to hm-gui...")
        result = call_mcp("attach_application", {"processName": "hm-gui"})
        text = result.get("result", {}).get("content", [{}])[0].get("text", "")
        if "attached" in text.lower() or "handle" in text.lower():
            print(f"   [PASS] Attached: {text[:100]}")
            passed += 1
        else:
            print(f"   [FAIL] Attach failed: {text[:200]}")
            failed += 1

        # 4. Get window tree
        print("\n4. Window UI tree (depth 4)...")
        result = call_mcp("get_window_tree", {"maxDepth": 4})
        text = result.get("result", {}).get("content", [{}])[0].get("text", "")
        # Print tree
        for line in text.split("\n")[:30]:
            print(f"   {line}")
        if "Button" in text or "Text" in text or "title" in text.lower():
            print("   [PASS] UI elements detected")
            passed += 1
        else:
            print("   [FAIL] No UI elements found")
            failed += 1

        # 5. Get visible text
        print("\n5. Reading visible text...")
        result = call_mcp("get_visible_text", {})
        text = result.get("result", {}).get("content", [{}])[0].get("text", "")
        print(f"   Visible text: {text[:200]}")
        if "HackMagic" in text or "No track" in text or "Player" in text:
            print("   [PASS] Text content found")
            passed += 1
        else:
            print("   [FAIL] No text content")
            failed += 1

        # 6. Screenshot
        print("\n6. Taking screenshot...")
        result = call_mcp("screenshot", {})
        content = result.get("result", {}).get("content", [{}])[0]
        img_data = content.get("text", "")
        mime_type = content.get("mimeType", "")
        if img_data and len(img_data) > 1000:
            # Save screenshot
            import base64
            with open("gui_screenshot.png", "wb") as f:
                f.write(base64.b64decode(img_data))
            print(f"   [PASS] Screenshot saved: gui_screenshot.png ({len(img_data)} bytes base64)")
            passed += 1
        else:
            print(f"   [FAIL] Screenshot empty or too small: {len(img_data)} chars")
            failed += 1

        # 7. Find buttons
        print("\n7. Finding buttons...")
        result = call_mcp("find_elements", {"controlType": "Button"})
        text = result.get("result", {}).get("content", [{}])[0].get("text", "")
        btn_count = text.count("Button") if "Button" in text else 0
        print(f"   Buttons found: {text[:200]}")
        if btn_count > 0:
            print("   [PASS] Buttons detected")
            passed += 1
        else:
            print("   [WARN] No buttons found (UI might be using custom rendering)")
            # Don't fail for this, Bevy uses custom rendering

    finally:
        # Cleanup
        print("\n" + "=" * 60)
        print(f"Results: {passed} passed, {failed} failed")
        print("Closing hm-gui...")
        gui_proc.kill()
        gui_proc.wait()

if __name__ == "__main__":
    test()