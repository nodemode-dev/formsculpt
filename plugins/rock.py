"""Rock from a sphere
Adds a detailed sphere, squashes it with a lattice, then layers ridged crests and cell pitting as noise layers whose
strengths stay adjustable. args: size (default 1), seed (default 0).
"""


def run(fsc, args):
    size = float(args.get("size", 1.0))
    seed = int(args.get("seed", 0))
    fsc.call("add_primitive", kind="sphere", detail=6, size=size, name="Rock")
    fsc.call("lattice_deform", moves=[
        {"point": [1, 2, 1], "offset": [0, -0.35 * size, 0]},
        {"point": [2, 1, 1], "offset": [0.25 * size, 0, 0]},
        {"point": [1, 1, 0], "offset": [0, 0, -0.15 * size]},
    ])
    fsc.call("surface_noise", pattern="ridged", scale=0.6 * size, amount=0.12 * size, octaves=4, seed=seed, name="Crests")
    fsc.call("surface_noise", pattern="cells", scale=0.12 * size, amount=-0.02 * size, octaves=2, seed=seed + 1, name="Pitting")
    rock = fsc.active()
    print(f"Rock {rock['id']}: {rock['vertices']} vertices, layers Crests and Pitting")
    return {"id": rock["id"]}
