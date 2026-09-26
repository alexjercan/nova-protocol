import json, sys, math
path, lo, hi = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
base = {}
for l in open(path):
    e = json.loads(l)
    if e.get("event") != "channel_in" or not e.get("raw"): continue
    t = e.get("tick")
    raw = e["raw"]
    for s in raw["ships"]:
        dmg = sum(x["health"]["max"] - x["health"]["current"] for x in s["sections"])
        dead = sum(1 for x in s["sections"] if not x["alive"])
        plates = [f for x in s["sections"] for f in x.get("fixtures", []) if f.get("kind") == "skin_plate"]
        pd = sum(f["health"]["max"] - f["health"]["current"] for f in plates)
        if s["id"] not in base: base[s["id"]] = (dmg, pd)
        if lo <= t <= hi:
            v = [x*10 for x in s["linear_velocity"]]; w = s["angular_velocity"]
            print(f"t{t} {s['id']:7s} p=({s['position'][0]*10:8.2f},{s['position'][1]*10:7.2f},{s['position'][2]*10:8.2f}) v=({v[0]:6.2f},{v[1]:6.2f},{v[2]:6.2f}) |v|={math.hypot(*v):6.2f} |w|={math.degrees(math.hypot(*w)):6.2f}dps hp={s['health']['current']:.1f} secdmg={dmg:.1f} platedmg={pd:.1f} dead={dead} conn={json.dumps(s['docking'].get('connection'))[:60]}")
