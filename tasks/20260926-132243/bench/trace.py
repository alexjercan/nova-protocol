import json, sys, math
path = sys.argv[1]
prev = None
for l in open(path):
    e = json.loads(l)
    ev = e.get("event")
    if ev == "channel_out":
        line = e.get("line") or e.get("data")
        print(f"  OUT t{e.get('tick')} {json.dumps(line)[:160]}")
        continue
    if ev != "channel_in" or not e.get("raw"): continue
    raw = e["raw"]
    row = [f"tick {e.get('tick')}"]
    for s in raw["ships"]:
        v = s["linear_velocity"]; w = s["angular_velocity"]
        dead = sum(1 for x in s["sections"] if not x["alive"])
        dmg = sum(x["health"]["max"] - x["health"]["current"] for x in s["sections"])
        row.append(f"{s['id']}: p=({s['position'][0]*10:.1f},{s['position'][1]*10:.1f},{s['position'][2]*10:.1f})m |v|={math.hypot(*v)*10:.2f}m/s |w|={math.degrees(math.hypot(*w)):.2f}dps hp={s['health']['current']:.1f} secdmg={dmg:.1f} dead={dead} dock={json.dumps(s['docking'].get('docked'))}/{json.dumps(s['docking'].get('connection'))}")
    print(" | ".join(row))
