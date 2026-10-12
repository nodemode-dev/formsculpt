"""Display pedestal
Puts a round pedestal under the active object, sized to its footprint, so a model reads as standing on a base for a
turntable or a render. args: height (fraction of the object's height, default 0.15).
"""


def run(fsc, args):
    target = fsc.active()
    if target is None:
        raise ValueError("select the object to stand on a pedestal first")
    lo, hi = target["bounds"]["min"], target["bounds"]["max"]
    width = max(hi[0] - lo[0], hi[2] - lo[2])
    height = (hi[1] - lo[1]) * float(args.get("height", 0.15))
    fsc.call("add_primitive", kind="cylinder", size=width * 0.6, height=height, detail=96, name="Pedestal")
    fsc.call("transform_object", translate=[(lo[0] + hi[0]) / 2, lo[1] - height / 2, (lo[2] + hi[2]) / 2])
    fsc.call("select_object", id=target["id"])
    return {"pedestal_height": height}
