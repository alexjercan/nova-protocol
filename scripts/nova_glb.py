"""glTF 2.0 binary (.glb) reading and writing, standard library only.

The repo's asset scripts emit meshes: `cut-obj-into-hulls.py` cuts a ship into
grid cubes, `cut-obj-into-parts.py` cuts it into semantic parts, and
`gen-greebles.py` builds decoration props from recipes. They share THIS writer
rather than each carrying a copy - a fix to the container (chunk padding,
accessor bounds) has to land in one place, and every generated asset in the
repo has to come out of the same serializer or "regenerate and diff" stops
proving anything.

Not a package and not installed: `scripts/` is on `sys.path` whenever a script
in it runs, so a sibling `import nova_glb` resolves.

A triangle here is any object with `.a`/`.b`/`.c` vertex tuples, a `.verts()`
accessor and a `.material` name. Each caller keeps its own Triangle type (they
carry different extra fields) and this module never constructs one.

Output is byte-stable: material groups are emitted in sorted order and the
glTF JSON is dumped with sorted keys, so the same input always produces the
same bytes and committed art does not churn between runs.
"""

from __future__ import annotations

import json
import math
import struct
from collections import defaultdict

GLB_MAGIC = 0x46546C67  # "glTF"
CHUNK_JSON = 0x4E4F534A  # "JSON"
CHUNK_BIN = 0x004E4942  # "BIN\0"
FLOAT = 5126
UINT = 5125
ARRAY_BUFFER = 34962
ELEMENT_ARRAY_BUFFER = 34963


def flat_normal(tri):
    """Unit face normal from the triangle winding (flat shading)."""
    ax, ay, az = tri.a
    bx, by, bz = tri.b
    cx, cy, cz = tri.c
    ux, uy, uz = bx - ax, by - ay, bz - az
    vx, vy, vz = cx - ax, cy - ay, cz - az
    nx, ny, nz = uy * vz - uz * vy, uz * vx - ux * vz, ux * vy - uy * vx
    length = math.sqrt(nx * nx + ny * ny + nz * nz)
    if length == 0.0:
        return (0.0, 0.0, 0.0)
    return (nx / length, ny / length, nz / length)


def pbr_material(name, colour, metallic=0.1, roughness=0.8, double_sided=True):
    """One flat-shaded glTF material: a base colour and nothing else.

    The whole repo's generated art is untextured flat colour, so a material is
    a name plus `baseColorFactor` - no maps, no alpha mode.
    """
    r, g, b = colour
    return {
        "name": name,
        "pbrMetallicRoughness": {
            "baseColorFactor": [r, g, b, 1.0],
            "metallicFactor": metallic,
            "roughnessFactor": roughness,
        },
        "doubleSided": double_sided,
    }


def write_glb(triangles, materials, material_index, generator):
    """Serialize a triangle soup to a glTF-binary blob (bytes).

    - `materials`: ordered list of glTF material dicts, embedded whole in every
      glb (there are only a handful, so global indices stay stable and simple).
    - `material_index`: {material_name: index into `materials`}.
    - `generator`: the producing script's name, recorded in `asset.generator`
      so a committed .glb says what built it.

    Triangles are grouped into one primitive per material. Each primitive gets a
    flat-shaded NORMAL and an explicit index buffer.
    """
    return write_glb_nodes(triangles, (), materials, material_index, generator)


def write_glb_nodes(triangles, nodes, materials, material_index, generator):
    """`write_glb`, plus extra NAMED child nodes a runtime can animate.

    - `triangles`: the static root geometry, exactly as in `write_glb`.
    - `nodes`: ordered [(name, translation, rotation, node_triangles)]. Each
      entry becomes one named glTF node with its own mesh; `translation` is
      (x, y, z), `rotation` a quaternion (x, y, z, w), and `node_triangles`
      are authored in the NODE'S LOCAL frame - the node transform places
      them, so a runtime that rotates the node about a local axis gets the
      authored hinge for free.

    With `nodes` empty the output is byte-identical to `write_glb`.
    """
    bin_blob = bytearray()
    buffer_views = []
    accessors = []

    def add_view(data, target):
        while len(bin_blob) % 4 != 0:  # 4-byte align each view
            bin_blob.append(0)
        offset = len(bin_blob)
        bin_blob.extend(data)
        buffer_views.append(
            {"buffer": 0, "byteOffset": offset, "byteLength": len(data), "target": target}
        )
        return len(buffer_views) - 1

    def add_mesh(mesh_triangles):
        by_material = defaultdict(list)
        for tri in mesh_triangles:
            by_material[tri.material].append(tri)

        primitives = []
        for material_name in sorted(by_material, key=lambda m: (m is None, m)):
            tris = by_material[material_name]
            positions = []
            normals = []
            for tri in tris:
                n = flat_normal(tri)
                for v in tri.verts():
                    positions.append(v)
                    normals.append(n)
            indices = list(range(len(positions)))

            pos_bytes = b"".join(struct.pack("<3f", *v) for v in positions)
            pos_view = add_view(pos_bytes, ARRAY_BUFFER)
            lo = [min(v[k] for v in positions) for k in range(3)]
            hi = [max(v[k] for v in positions) for k in range(3)]
            pos_acc = len(accessors)
            accessors.append(
                {
                    "bufferView": pos_view,
                    "componentType": FLOAT,
                    "count": len(positions),
                    "type": "VEC3",
                    "min": lo,
                    "max": hi,
                }
            )

            nrm_bytes = b"".join(struct.pack("<3f", *v) for v in normals)
            nrm_view = add_view(nrm_bytes, ARRAY_BUFFER)
            nrm_acc = len(accessors)
            accessors.append(
                {
                    "bufferView": nrm_view,
                    "componentType": FLOAT,
                    "count": len(normals),
                    "type": "VEC3",
                }
            )

            idx_bytes = b"".join(struct.pack("<I", i) for i in indices)
            idx_view = add_view(idx_bytes, ELEMENT_ARRAY_BUFFER)
            idx_acc = len(accessors)
            accessors.append(
                {
                    "bufferView": idx_view,
                    "componentType": UINT,
                    "count": len(indices),
                    "type": "SCALAR",
                }
            )

            primitives.append(
                {
                    "attributes": {"POSITION": pos_acc, "NORMAL": nrm_acc},
                    "indices": idx_acc,
                    "material": material_index.get(material_name, 0),
                }
            )
        return primitives

    meshes = [{"primitives": add_mesh(triangles)}]
    node_list = [{"mesh": 0}]
    for name, translation, rotation, node_triangles in nodes:
        mesh_id = len(meshes)
        meshes.append({"primitives": add_mesh(node_triangles)})
        node_list.append(
            {
                "mesh": mesh_id,
                "name": name,
                "rotation": list(rotation),
                "translation": list(translation),
            }
        )

    # Pad the BIN chunk to a 4-byte boundary BEFORE reporting its length, so
    # buffers[0].byteLength always equals the emitted chunk length regardless of
    # which attribute happens to be the last view.
    while len(bin_blob) % 4 != 0:  # pad BIN chunk with zeros
        bin_blob.append(0)

    gltf = {
        "asset": {"version": "2.0", "generator": generator},
        "scene": 0,
        "scenes": [{"nodes": list(range(len(node_list)))}],
        "nodes": node_list,
        "meshes": meshes,
        "materials": materials,
        "accessors": accessors,
        "bufferViews": buffer_views,
        "buffers": [{"byteLength": len(bin_blob)}],
    }

    json_bytes = json.dumps(gltf, separators=(",", ":"), sort_keys=True).encode("utf-8")
    while len(json_bytes) % 4 != 0:  # pad JSON chunk with spaces
        json_bytes += b" "

    total = 12 + 8 + len(json_bytes) + 8 + len(bin_blob)
    out = bytearray()
    out += struct.pack("<III", GLB_MAGIC, 2, total)
    out += struct.pack("<II", len(json_bytes), CHUNK_JSON)
    out += json_bytes
    out += struct.pack("<II", len(bin_blob), CHUNK_BIN)
    out += bin_blob
    return bytes(out)


def read_glb(path):
    """Parse a .glb from disk. Returns (gltf_json, positions) where positions
    is every decoded POSITION vertex. Raises on any structural mismatch, so a
    truncated or mis-sized file cannot pass verification."""
    blob = open(path, "rb").read()
    magic, version, total = struct.unpack("<III", blob[:12])
    if magic != GLB_MAGIC or version != 2:
        raise ValueError("%s: not a glb v2" % path)
    if total != len(blob):
        raise ValueError("%s: header length %d != file size %d" % (path, total, len(blob)))
    json_len, json_tag = struct.unpack("<II", blob[12:20])
    if json_tag != CHUNK_JSON:
        raise ValueError("%s: first chunk is not JSON" % path)
    doc = json.loads(blob[20 : 20 + json_len].decode("utf-8"))
    bin_off = 20 + json_len
    bin_len, bin_tag = struct.unpack("<II", blob[bin_off : bin_off + 8])
    if bin_tag != CHUNK_BIN:
        raise ValueError("%s: second chunk is not BIN" % path)
    if doc["buffers"][0]["byteLength"] != bin_len:
        raise ValueError("%s: buffer byteLength != BIN chunk length" % path)
    binary = blob[bin_off + 8 : bin_off + 8 + bin_len]

    positions = []
    for mesh in doc["meshes"]:
        for prim in mesh["primitives"]:
            acc = doc["accessors"][prim["attributes"]["POSITION"]]
            view = doc["bufferViews"][acc["bufferView"]]
            off = view.get("byteOffset", 0)
            for i in range(acc["count"]):
                positions.append(struct.unpack_from("<3f", binary, off + 12 * i))
    return doc, positions
