#!/usr/bin/env python3
"""Smoke-test the *actual* local MCP wire protocol. No network access beyond loopback."""

import json
import os
import socket
import subprocess
import sys
import time
import urllib.error
import urllib.request

ENDPOINT = "http://127.0.0.1:8765/mcp"


def mcp_call(method, request_id, params=None):
    payload = {
        "jsonrpc": "2.0",
        "id": request_id,
        "method": method,
        "params": {} if params is None else params,
    }
    request = urllib.request.Request(
        ENDPOINT,
        data=json.dumps(payload).encode("utf-8"),
        method="POST",
        headers={
            "Content-Type": "application/json",
            "Accept": "application/json, text/event-stream",
            "MCP-Protocol-Version": "2025-11-25",
        },
    )
    with urllib.request.urlopen(request, timeout=10) as response:
        text = response.read().decode("utf-8")
        content_type = response.headers.get("Content-Type", "")
    if "text/event-stream" in content_type:
        messages = [
            json.loads(line[5:].strip())
            for line in text.splitlines()
            if line.startswith("data:")
        ]
        assert messages, f"No JSON event in SSE response: {text!r}"
        result = next((v for v in messages if v.get("id") == request_id), messages[-1])
    else:
        result = json.loads(text)
    if "error" in result:
        raise AssertionError(f"MCP {method} failed: {result['error']}")
    assert result.get("id") == request_id, result
    return result["result"]


def wait_until_ready(process):
    for _ in range(60):
        if process.poll() is not None:
            raise RuntimeError(f"Postern exited during startup with {process.returncode}")
        try:
            with socket.create_connection(("127.0.0.1", 8765), timeout=0.2):
                return
        except OSError:
            time.sleep(0.1)
    raise TimeoutError("Postern development endpoint did not start")


def main():
    env = os.environ.copy()
    env["POSTERN_CONFIG"] = "config/postern.example.json"
    process = subprocess.Popen(
        ["target/debug/postern"],
        env=env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
    )
    try:
        wait_until_ready(process)
        initialise = mcp_call(
            "initialize",
            1,
            {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": {"name": "postern-smoke", "version": "0.1.0"},
            },
        )
        assert "capabilities" in initialise, initialise
        tools = mcp_call("tools/list", 2)
        names = [item["name"] for item in tools["tools"]]
        assert names == ["hosts"], names
        result = mcp_call("tools/call", 3, {"name": "hosts", "arguments": {}})
        payload = json.loads(result["content"][0]["text"])
        assert payload == {
            "hosts": [
                {
                    "id": "lab",
                    "description": "Disposable local test machine (illustrative only)",
                }
            ]
        }, payload
        print("MCP initialize, tools/list, and hosts() passed")
    finally:
        process.terminate()
        try:
            _, stderr = process.communicate(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            _, stderr = process.communicate()
        if process.returncode not in (0, -15):
            print(stderr.decode("utf-8", errors="replace"), file=sys.stderr)


if __name__ == "__main__":
    main()
