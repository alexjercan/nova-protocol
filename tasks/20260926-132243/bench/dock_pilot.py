"""Scripted docking pilot for the bench referee protocol.

Env knobs:
  CLOSE_MPS   closing speed held on the final approach (default 4.5)
  FACE_BIAS   extra yaw in degrees held after squaring up (default 0)
  SPIN_DPS    yaw turn started a few ticks before DOCK (0 = none)
  PORT_AXIS   'nose' (port on the bow) or 'starboard' (port faces +X)
  PITCH_OVER  degrees of camera pitch flown before the lock (180 = belly up,
              nose aft: the hull turns about its own X axis, as a mouse does)
"""
import json, math, os, socket, sys

CLOSE = float(os.environ.get("CLOSE_MPS", "4.5"))
FACE_BIAS = float(os.environ.get("FACE_BIAS", "0"))
SPIN = float(os.environ.get("SPIN_DPS", "0"))
BEAM = os.environ.get("PORT_AXIS", "nose") == "starboard"
ALIGN_AZ = 90.0 if BEAM else 0.0
DOCK_AT_GAP = float(os.environ.get("DOCK_AT_GAP", "10"))
DOCK_MAX_REL = float(os.environ.get("DOCK_MAX_REL", "99"))
SETTLE = int(os.environ.get("SETTLE", "150"))
PITCH_OVER = float(os.environ.get("PITCH_OVER", "0"))
PX_PER_DEG = 27.0
FULL = 33.0
MPS_PER_FULL_TICK = 0.83


def log(*a):
    print(*a, file=sys.stderr, flush=True)


def req(obj):
    s = socket.socket(socket.AF_UNIX)
    s.connect(os.environ["NOVA_BENCH_SOCKET"])
    s.sendall((json.dumps(obj) + "\n").encode())
    buf = b""
    while not buf.endswith(b"\n"):
        c = s.recv(1 << 20)
        if not c:
            break
        buf += c
    s.close()
    r = json.loads(buf)
    if "error" in r:
        log("REFEREE ERROR", r["error"])
        return None
    return r["ok"]


def act(gestures, ticks):
    v = req({"act": {"gestures": gestures, "ticks": ticks}})
    if v is None:
        sys.exit(2)
    if v.get("over"):
        log("run over", v.get("ended_by"))
        sys.exit(0)
    return v


def finish(status, report):
    req({"finish": {"status": status, "report": report}})
    sys.exit(0)


def pair(v):
    return v["me"]["docking"]["pair"]


def summary(v):
    p = pair(v)
    me = v["me"]
    if p is None:
        return f"t{v['tick']} no pair"
    return (f"t{v['tick']} gap={p['gap_m']:.2f} off={p['their_face_offset_m']} "
            f"align={p['align_bearing_deg']} face={p['facing_deg']:.1f} "
            f"rel={p['relative_mps']:.2f} spin={p['relative_spin_dps']:.2f} "
            f"ok={p['gap_ok']}/{p['facing_ok']}/{p['motion_ok']} elig={p['eligible']} "
            f"spd={me['speed_mps']} vb={me.get('velocity_bearing_deg')} "
            f"hp={me['health']['current']}")


def hull_velocity(v):
    me = v["me"]
    s = me["speed_mps"] or 0.0
    vb = me.get("velocity_bearing_deg") or [0.0, 0.0]
    az, el = math.radians(vb[0]), math.radians(vb[1])
    return (s * math.cos(el) * math.sin(az), s * math.sin(el), s * math.cos(el) * math.cos(az))


def turn(az_deg, el_deg, settle):
    ticks = max(1, int(math.ceil(max(abs(az_deg), abs(el_deg)) * PX_PER_DEG / 80.0)))
    d = [az_deg * PX_PER_DEG / ticks, -el_deg * PX_PER_DEG / ticks]
    return act([{"aim": "camera.camera_rotate", "delta": d, "ticks": ticks}], ticks + settle)


def main():
    v = req({"observe": {}})
    contact = v["contacts"][0]
    target = contact["id"]
    log("start", json.dumps(contact))

    # 0. Optional pitch-over: the attitude command follows the camera rig,
    # which pitches about its own right axis, so this flips the hull.
    if PITCH_OVER:
        v = turn(0.0, PITCH_OVER, 600)
        c = next(c for c in v["contacts"] if c["id"] == target)
        log("pitched", PITCH_OVER, "tender bearing", c["bearing_deg"], "turn", v["me"]["turn_rate_dps"])

    # 1. Nose on the target, hold the radar until the travel lock names it.
    for _ in range(6):
        c = next(c for c in v["contacts"] if c["id"] == target)
        az, el = c["bearing_deg"]
        if abs(az) < 1.5 and abs(el) < 1.5:
            break
        v = turn(az, el, SETTLE)
        v = act([{"release": "flight.rcs_modifier"}] if "flight.rcs_modifier" in v["inputs"]["held"] else [], 1)
        v = act([{"tap": "flight.autopilot_stop"}], 2)
        v = act([{"tap": "flight.autopilot_off"}], 2)
    v = act([{"press": "targeting.radar_hold"}], 30)
    for _ in range(20):
        if v["me"]["travel_lock"] == target:
            break
        v = act([], 15)
    v = act([{"release": "targeting.radar_hold"}], 1)
    log("lock", v["me"]["travel_lock"], summary(v))
    if v["me"]["travel_lock"] != target:
        finish("gave_up", "no travel lock")

    # 2. Square up: nose onto the port axis (plus the facing bias), then stop.
    for _ in range(10):
        p = pair(v)
        az, el = p["align_bearing_deg"]
        az = az - ALIGN_AZ + FACE_BIAS
        if BEAM:
            el = 0.0
        log("align", summary(v))
        if abs(az) < 0.7 and abs(el) < 0.7:
            break
        v = turn(az, el, SETTLE)
        v = act([{"tap": "flight.autopilot_stop"}], 30)
        v = act([{"tap": "flight.autopilot_off"}], 2)
    log("squared", summary(v))

    # 3a. Null the starboard offset on the RCS, holding the range.
    v = act([{"press": "flight.rcs_modifier"}], 1)
    for step in range(3000):
        s, u, a = pair(v)["their_face_offset_m"]
        vs, vu, va = hull_velocity(v)
        lat, vlat = (a, va) if BEAM else (s, vs)
        if abs(lat) < 0.8 and abs(vlat) < 0.3:
            break
        want = max(-4.0, min(4.0, 0.25 * lat))
        ws, wa = (0.0, want) if BEAM else (want, 0.0)
        dx = max(-FULL, min(FULL, 0.6 * (ws - vs) / MPS_PER_FULL_TICK * FULL))
        dy = max(-FULL, min(FULL, -0.6 * (wa - va) / MPS_PER_FULL_TICK * FULL))
        v = act([{"aim": "flight.rcs_aim", "delta": [dx, dy], "ticks": 1}], 5)
        if step % 20 == 0:
            log("null", summary(v))
    # 3b. Nose on their port face, so the closing run needs no vertical push.
    for _ in range(0 if BEAM else 4):
        v = act([{"release": "flight.rcs_modifier"}, {"tap": "flight.autopilot_stop"}], 20)
        v = act([{"tap": "flight.autopilot_off"}], 2)
        az, el = pair(v)["their_face_bearing_deg"]
        log("aim face", summary(v), "face bearing", az, el)
        if abs(az) < 0.4 and abs(el) < 0.4:
            break
        v = turn(az, el, 100)
    log("on face", summary(v))
    # 3c. Close along the nose at CLOSE m/s; 1-tick acts under 14 m.
    v = act([{"press": "flight.rcs_modifier"}], 1)
    spun = False
    docked_tick = None
    for step in range(4000):
        p = pair(v)
        me = v["me"]
        if me["docking"]["docked"]:
            docked_tick = v["tick"]
            break
        s, u, a = p["their_face_offset_m"]
        vs, vu, va = hull_velocity(v)
        if p["eligible"] and p["gap_m"] <= DOCK_AT_GAP and p["relative_mps"] <= DOCK_MAX_REL:
            log("ELIGIBLE -> DOCK", summary(v))
            v = act([{"release": "flight.rcs_modifier"}, {"tap": "flight.dock"}], 1)
            log("after dock tap", summary(v), json.dumps(v["me"]["docking"]))
            continue
        rng = s if BEAM else a
        if rng < -2.0:
            finish("gave_up", "passed the face without eligibility: " + summary(v))
        if BEAM:
            ws, wa = CLOSE, max(-2.0, min(2.0, 0.25 * a))
        else:
            ws, wa = max(-2.0, min(2.0, 0.25 * s)), CLOSE
        dx = max(-FULL, min(FULL, 0.6 * (ws - vs) / MPS_PER_FULL_TICK * FULL))
        dy = max(-FULL, min(FULL, -0.6 * (wa - va) / MPS_PER_FULL_TICK * FULL))
        gestures = []
        if not spun and "flight.rcs_modifier" not in v["inputs"]["held"]:
            gestures.append({"press": "flight.rcs_modifier"})
        if abs(dx) > 0.5 or abs(dy) > 0.5:
            gestures.append({"aim": "flight.rcs_aim", "delta": [dx, dy], "ticks": 1})
        if SPIN and not spun and p["gap_m"] < 11.0:
            gestures = [{"release": "flight.rcs_modifier"},
                        {"aim": "camera.camera_rotate", "delta": [SPIN * PX_PER_DEG / 3.0, 0], "ticks": 3}]
            spun = True
        if p["eligible"]:
            log("eligible, holding tap", summary(v))
        ticks = 1 if p["gap_m"] < 14.0 else 5
        v = act(gestures, ticks)
        if step % 10 == 0 or p["gap_m"] < 14.0:
            log("fly", summary(v))

    if docked_tick is None:
        finish("gave_up", "never docked: " + summary(v))

    # 4. Aftermath: 1-tick acts for 180 ticks, then coarser.
    log("DOCKED at", docked_tick, json.dumps(v["me"]["docking"]))
    for i in range(180):
        v = act([], 1)
        if i % 10 == 0:
            log("after", v["tick"], v["me"]["speed_mps"], v["me"]["health"],
                json.dumps(v["me"]["docking"].get("connection")),
                [(c["id"], c["distance_m"], c["health"]) for c in v["contacts"]])
    for i in range(10):
        v = act([], 30)
    log("end", summary(v), json.dumps(v["me"]["docking"]))
    finish("done", "docked at tick %d; end %s" % (docked_tick, summary(v)))


main()
