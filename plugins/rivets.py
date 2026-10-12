"""Rivet ring
A click tool: click a surface and a ring of rivets is set around that point, each standing on the surface, as one
object. args: point and normal (from the click), radius, count, size and shape.
"""

import math

CLICK = True
PARAMS = {
    "radius": {"type": "number", "label": "Ring radius", "default": 0.25, "min": 0.01, "max": 10, "step": 0.01},
    "count": {"type": "integer", "label": "Rivets", "default": 8, "min": 3, "max": 64},
    "size": {"type": "number", "label": "Rivet size", "default": 0.06, "min": 0.005, "max": 2, "step": 0.005},
    "shape": {"type": "choice", "label": "Shape", "default": "sphere", "options": ["sphere", "cylinder", "cube"]},
}


def run(fsc, args):
    point, normal = args.get("point"), args.get("normal")
    if not point or not normal:
        raise ValueError("click a surface with the plugin's tool, or pass point and normal")
    radius, count = float(args.get("radius", 0.25)), int(args.get("count", 8))
    n = [float(c) for c in normal]
    length = math.sqrt(sum(c * c for c in n)) or 1.0
    n = [c / length for c in n]
    # Two directions across the surface, then points around the ring, each pulled back onto the surface by a pick.
    seed = [1.0, 0.0, 0.0] if abs(n[0]) < 0.9 else [0.0, 1.0, 0.0]
    d = sum(a * b for a, b in zip(seed, n))
    u = [a - d * b for a, b in zip(seed, n)]
    lu = math.sqrt(sum(c * c for c in u))
    u = [c / lu for c in u]
    v = [n[1] * u[2] - n[2] * u[1], n[2] * u[0] - n[0] * u[2], n[0] * u[1] - n[1] * u[0]]
    path = []
    for k in range(count):
        a = 2 * math.pi * k / count
        p = [point[i] + radius * (math.cos(a) * u[i] + math.sin(a) * v[i]) for i in range(3)]
        hit = fsc.call("pick", origin=[p[i] + n[i] * radius for i in range(3)], direction=[-c for c in n])
        if hit.get("hit"):
            path.append(hit["point"] + hit["normal"])
    if len(path) < 2:
        raise ValueError("the ring does not lie on the surface; try a smaller radius")
    # One copy per ring point: the spacing is the chord between neighbours, a hair short so rounding never skips one.
    chord = 2 * radius * math.sin(math.pi / count)
    spacing = min(20.0, max(0.1, chord * 0.999 / float(args.get("size", 0.06))))
    placed = fsc.call("insert_along", path=path, kind=args.get("shape", "sphere"), size=float(args.get("size", 0.06)), spacing=spacing, sink=0.4)
    print(f"{placed['copies']} rivets around the point")
    return {"copies": placed["copies"]}
