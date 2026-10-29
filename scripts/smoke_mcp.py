#!/usr/bin/env python3
"""Exercise the authenticated MCP endpoint over real HTTPS introspection.

The fake local Keycloak endpoint is ONLY for CI. Postern requires TLS and a
configured trust root even in tests. No actual Keycloak or SSH host is used.
"""
import base64
import json
import os
import socket
import ssl
import subprocess
import sys
import tempfile
import threading
import time
import urllib.error
import urllib.parse
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

MCP = "http://127.0.0.1:8765/mcp"
SUBJECT = "00000000-0000-0000-0000-000000000001"


class FakeKeycloak(BaseHTTPRequestHandler):
    calls = 0
    issuer = ""

    def do_POST(self):
        self.__class__.calls += 1
        expected_auth = "Basic " + base64.b64encode(b"postern-test:fixture-secret").decode()
        if (self.path != "/realms/test/protocol/openid-connect/token/introspect"
                or self.headers.get("Authorization") != expected_auth):
            self.send_error(401)
            return
        length = int(self.headers.get("Content-Length", "0"))
        params = urllib.parse.parse_qs(self.rfile.read(length).decode())
        token = params.get("token", [""])[0]
        if token == "outage":
            self.send_error(503)
            return
        now = int(time.time())
        claims = {
            "token_type": "Bearer",
            "active": token in ("alice", "bob", "expired", "wrong-aud", "wrong-iss", "wrong-scope"),
            "sub": SUBJECT if token != "bob" else "other-user",
            "iss": self.issuer if token != "wrong-iss" else "https://malicious.example/realms/test",
            "aud": ["postern"] if token != "wrong-aud" else ["another-service"],
            "scope": "postern:use" if token != "wrong-scope" else "other",
            "exp": now + 600 if token != "expired" else now - 10,
            "nbf": now - 10,
        }
        data = json.dumps(claims).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def log_message(self, *_):
        pass


def new_request(method, id_num, token=None, params=None):
    payload = json.dumps({
        "jsonrpc": "2.0", "id": id_num, "method": method,
        "params": params or {},
    }).encode()
    headers = {
        "Content-Type": "application/json",
        "Accept": "application/json, text/event-stream",
        "MCP-Protocol-Version": "2025-11-25",
    }
    if token is not None:
        headers["Authorization"] = "Bearer " + token
    return urllib.request.Request(MCP, data=payload, method="POST", headers=headers)


def send(request, expected=200):
    try:
        with urllib.request.urlopen(request, timeout=10) as response:
            status = response.status
            body = response.read().decode()
            mime = response.headers.get("Content-Type", "")
    except urllib.error.HTTPError as error:
        status = error.code
        body = error.read().decode()
        mime = ""
    assert status == expected, f"Expected HTTP {expected}, got {status}: {body[:250]}"
    if status != 200:
        return None
    if "text/event-stream" in mime:
        entries = [json.loads(line[5:].strip()) for line in body.splitlines() if line.startswith("data:")]
        assert entries, body
        return entries[-1]
    return json.loads(body)


def ready(process):
    for _ in range(100):
        if process.poll() is not None:
            raise RuntimeError(f"Postern stopped unexpectedly: {process.returncode}")
        try:
            with socket.create_connection(("127.0.0.1", 8765), timeout=0.2):
                return
        except OSError:
            time.sleep(0.1)
    raise TimeoutError("Postern failed to listen")


def main():
    with tempfile.TemporaryDirectory(prefix="postern-auth-test-") as tmp:
        # Rustls correctly rejects a CA certificate used directly as a TLS
        # leaf. Generate a proper local CA and a distinct signed server leaf.
        ca_key = Path(tmp) / "ca-key.pem"
        ca_cert = Path(tmp) / "ca-cert.pem"
        key = Path(tmp) / "server-key.pem"
        csr = Path(tmp) / "server.csr"
        cert = Path(tmp) / "server-cert.pem"
        extensions = Path(tmp) / "extensions.conf"
        extensions.write_text(
            "basicConstraints=critical,CA:FALSE\n"
            "keyUsage=critical,digitalSignature,keyEncipherment\n"
            "extendedKeyUsage=serverAuth\n"
            "subjectAltName=DNS:localhost\n"
        )
        def openssl(*args):
            subprocess.run(["openssl", *map(str, args)], check=True,
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        openssl("req", "-x509", "-newkey", "rsa:2048", "-nodes",
                "-keyout", ca_key, "-out", ca_cert, "-days", "1",
                "-subj", "/CN=Postern CI CA", "-addext", "basicConstraints=critical,CA:TRUE")
        openssl("req", "-new", "-newkey", "rsa:2048", "-nodes",
                "-keyout", key, "-out", csr, "-subj", "/CN=localhost")
        openssl("x509", "-req", "-in", csr, "-CA", ca_cert, "-CAkey", ca_key,
                "-CAcreateserial", "-out", cert, "-days", "1", "-extfile", extensions)
        server = ThreadingHTTPServer(("127.0.0.1", 0), FakeKeycloak)
        FakeKeycloak.issuer = f"https://localhost:{server.server_port}/realms/test"
        ssl_ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        ssl_ctx.load_cert_chain(certfile=str(cert), keyfile=str(key))
        server.socket = ssl_ctx.wrap_socket(server.socket, server_side=True)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        env = os.environ.copy()
        env.update({
            "POSTERN_CONFIG": "config/postern.example.json",
            "POSTERN_KEYCLOAK_ISSUER": FakeKeycloak.issuer,
            "POSTERN_KEYCLOAK_AUDIENCE": "postern",
            "POSTERN_KEYCLOAK_SCOPE": "postern:use",
            "POSTERN_KEYCLOAK_CLIENT_ID": "postern-test",
            "POSTERN_KEYCLOAK_CLIENT_SECRET": "fixture-secret",
            "POSTERN_KEYCLOAK_CA_FILE": str(ca_cert),
        })
        # Independently verify the HTTPS test issuer before Postern starts. This
        # tests TLS, Basic auth and the mock endpoint without disabling TLS.
        test_req = urllib.request.Request(
            FakeKeycloak.issuer + "/protocol/openid-connect/token/introspect",
            data=urllib.parse.urlencode({"token": "inactive"}).encode(),
            headers={"Authorization": "Basic " + base64.b64encode(b"postern-test:fixture-secret").decode()},
            method="POST",
        )
        with urllib.request.urlopen(test_req, context=ssl.create_default_context(cafile=str(ca_cert)), timeout=5) as response:
            assert json.load(response)["active"] is False
        p = subprocess.Popen(["target/debug/postern"], env=env,
                             stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        failed = False
        try:
            ready(p)
            send(new_request("initialize", 1), 401)
            send(new_request("tools/list", 2, "inactive"), 401)
            send(new_request("initialize", 3, "expired"), 401)
            send(new_request("initialize", 4, "wrong-aud"), 401)
            send(new_request("initialize", 5, "wrong-iss"), 401)
            send(new_request("initialize", 6, "wrong-scope"), 401)
            send(new_request("initialize", 7, "outage"), 503)
            for method in ("GET", "DELETE"):
                send(urllib.request.Request(MCP, method=method), 401)
            init = send(new_request("initialize", 10, "alice", {
                "protocolVersion": "2025-11-25", "capabilities": {},
                "clientInfo": {"name": "postern-auth-smoke", "version": "0.1.0"}
            }))
            assert "capabilities" in init["result"], init
            assert "mcp-session-id" not in {k.lower() for k in init.get("headers", {})}
            tools = send(new_request("tools/list", 11, "bob"))
            assert [t["name"] for t in tools["result"]["tools"]] == ["hosts"]
            alice = send(new_request("tools/call", 12, "alice", {"name": "hosts", "arguments": {}}))
            bob = send(new_request("tools/call", 13, "bob", {"name": "hosts", "arguments": {}}))
            for result, expected in [(alice, [{"id": "lab", "description": "Disposable test host (illustrative only)"}]), (bob, [])]:
                assert "error" not in result, result
                hosts = json.loads(result["result"]["content"][0]["text"])
                assert hosts == {"hosts": expected}, hosts
                assert "allowed_subjects" not in str(hosts)
            send(new_request("tools/call", 14, "inactive", {"name": "hosts", "arguments": {}}), 401)
            assert FakeKeycloak.calls >= 13, f"Expected Keycloak introspection for every call, got {FakeKeycloak.calls}"
            print("PASS: HTTPS token introspection, deny-by-default, cross-user isolation, MCP hosts")
        except BaseException:
            failed = True
            print(f"Fake Keycloak served {FakeKeycloak.calls} introspection requests", file=sys.stderr)
            raise
        finally:
            p.terminate()
            try:
                _, stderr = p.communicate(timeout=5)
            except subprocess.TimeoutExpired:
                p.kill()
                _, stderr = p.communicate()
            if failed:
                print("Postern debug output:", stderr.decode(errors="replace"), file=sys.stderr)
            server.shutdown()
            server.server_close()


if __name__ == "__main__":
    main()
