import json, os, socket, sys
def req(obj):
    s = socket.socket(socket.AF_UNIX); s.connect(os.environ["NOVA_BENCH_SOCKET"])
    s.sendall((json.dumps(obj) + "\n").encode()); buf = b""
    while not buf.endswith(b"\n"):
        c = s.recv(65536)
        if not c: break
        buf += c
    s.close(); return json.loads(buf)
v = req({"observe": {}})
print(json.dumps(v, indent=1))
v = req({"act": {"gestures": [], "ticks": 1}})
print(json.dumps(v.get("ok", v).get("me", {}).get("docking"), indent=1))
req({"finish": {"status": "done", "report": "t0 observe"}})
