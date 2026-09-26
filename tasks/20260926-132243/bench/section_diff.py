"""List sections that vanished or lost health between two raw snapshots."""
import json, sys
path, a, b = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
snaps = {}
for l in open(path):
    e = json.loads(l)
    if e.get("event") == "channel_in" and e.get("raw") and e["tick"] in (a, b):
        snaps[e["tick"]] = {s["id"]: {x["id"]: x for x in s["sections"]} for s in e["raw"]["ships"]}
for ship in snaps[a]:
    sa, sb = snaps[a][ship], snaps[b].get(ship, {})
    gone = [k for k in sa if k not in sb]
    hurt = [(k, sa[k]["health"]["current"], sb[k]["health"]["current"]) for k in sa if k in sb and sb[k]["health"]["current"] < sa[k]["health"]["current"]]
    print(f"{ship}: {len(sa)} -> {len(sb)} sections")
    for k in gone:
        print(f"  gone {k} {sa[k]['prototype']} local cell {sa[k]['position']}")
    for k, h0, h1 in hurt:
        print(f"  hurt {k} {sa[k]['prototype']} cell {sa[k]['position']} {h0} -> {h1}")
