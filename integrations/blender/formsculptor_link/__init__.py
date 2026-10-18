# SPDX-License-Identifier: Apache-2.0
"""formsculptor Live Link for Blender.

Sends Blender mesh objects to a running formsculptor and keeps both sides in step: an edit in Blender is pushed
(link_push), a sculpt in formsculptor is pulled back (link_pull), each on a timer while the link is live. Meshes travel
as base64 little-endian arrays over formsculptor's local dispatch (POST /api/engine/dispatch), in formsculptor's axes
(y up); Blender is z up, so x, y, z here is x, z, -y there, the same turn Blender's glTF exporter makes.

When only the positions change on either side, only the positions move: Blender keeps its quads, UVs and modifiers,
formsculptor keeps its paint, mask and face groups.
"""

import base64
import hashlib
import json
import urllib.error
import urllib.request

import bpy
import numpy as np

ID_KEY = "formsculptor_id"
REV_KEY = "formsculptor_revision"

# Blender object name -> {"fp": fingerprint of what both sides last agreed on, "tris": triangle indices as bytes}.
_seen = {}
_status = {"text": ""}

# Blender (x, y, z) to formsculptor (x, z, -y), and back.
TO_FSC = np.array([[1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, -1.0, 0.0]], dtype=np.float64)
TO_BLENDER = TO_FSC.T


class LinkError(Exception):
    pass


def _prefs():
    addon = bpy.context.preferences.addons.get(__package__)
    return addon.preferences if addon else None


def _url():
    prefs = _prefs()
    return (prefs.url if prefs else "http://127.0.0.1:8196").rstrip("/")


def dispatch(name, args, timeout=30.0):
    """One formsculptor command; returns its result or raises LinkError with the engine's own message."""
    body = json.dumps({"name": name, "args": args}).encode("utf-8")
    request = urllib.request.Request(_url() + "/api/engine/dispatch", data=body, headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(request, timeout=timeout) as reply:
            data = json.loads(reply.read().decode("utf-8"))
    except (urllib.error.URLError, OSError) as e:
        raise LinkError(f"formsculptor is not reachable at {_url()} ({e})") from e
    if not data.get("ok", False):
        error = data.get("error") or {}
        raise LinkError(error.get("message") or str(error) or "the command failed")
    return data.get("result", {})


def _mesh_arrays(obj):
    """World positions in formsculptor's axes (f32) and the triangle indices (u32) of a mesh object."""
    mesh = obj.data
    n = len(mesh.vertices)
    co = np.empty(n * 3, dtype=np.float32)
    mesh.vertices.foreach_get("co", co)
    co = co.reshape(n, 3).astype(np.float64)
    m = np.array(obj.matrix_world, dtype=np.float64)
    world = co @ m[:3, :3].T + m[:3, 3]
    positions = (world @ TO_FSC.T).astype(np.float32)
    tris = np.empty(len(mesh.loop_triangles) * 3, dtype=np.uint32)
    mesh.loop_triangles.foreach_get("vertices", tris)
    return positions, tris


def _fingerprint(obj):
    mesh = obj.data
    co = np.empty(len(mesh.vertices) * 3, dtype=np.float32)
    mesh.vertices.foreach_get("co", co)
    h = hashlib.blake2b(co.tobytes(), digest_size=16)
    h.update(np.array(obj.matrix_world, dtype=np.float32).tobytes())
    h.update(len(mesh.polygons).to_bytes(8, "little"))
    return h.hexdigest()


def _b64(array):
    return base64.b64encode(np.ascontiguousarray(array).tobytes()).decode("ascii")


def push(obj, force_topology=False):
    """Send obj; makes the formsculptor object on the first push. Returns the engine reply."""
    positions, tris = _mesh_arrays(obj)
    if len(tris) == 0:
        raise LinkError(f"{obj.name} has no faces to send")
    args = {"positions": _b64(positions)}
    seen = _seen.get(obj.name_full)
    fid = obj.get(ID_KEY)
    if fid is None:
        args["name"] = obj.name
        args["indices"] = _b64(tris)
    else:
        args["id"] = int(fid)
        if force_topology or seen is None or seen["tris"] != tris.tobytes():
            args["indices"] = _b64(tris)
    try:
        result = dispatch("link_push", args)
    except LinkError as e:
        if fid is not None and "no object" in str(e):
            # The formsculptor object is gone; send it again as a new one.
            del obj[ID_KEY]
            return push(obj, force_topology=True)
        raise
    obj[ID_KEY] = int(result["object"]["id"])
    obj[REV_KEY] = int(result["revision"])
    _seen[obj.name_full] = {"fp": _fingerprint(obj), "tris": tris.tobytes()}
    return result


def _write(obj, positions, tris):
    """Write formsculptor positions (and triangles when they changed) into obj's mesh."""
    mesh = obj.data
    m = np.array(obj.matrix_world, dtype=np.float64)
    inverse = np.linalg.inv(m)
    world = positions.astype(np.float64) @ TO_BLENDER.T
    local = (world @ inverse[:3, :3].T + inverse[:3, 3]).astype(np.float32)
    seen = _seen.get(obj.name_full)
    same = len(mesh.vertices) == len(local) and seen is not None and seen["tris"] == tris.tobytes()
    if same:
        mesh.vertices.foreach_set("co", local.ravel())
    else:
        mesh.clear_geometry()
        mesh.from_pydata(local.tolist(), [], tris.reshape(-1, 3).tolist())
        mesh.validate()
    mesh.update()
    _seen[obj.name_full] = {"fp": _fingerprint(obj), "tris": tris.tobytes()}


def pull(obj):
    """Bring formsculptor's edit of obj back. Returns True when something changed."""
    fid = obj.get(ID_KEY)
    if fid is None:
        return False
    since = obj.get(REV_KEY)
    args = {"id": int(fid)}
    if since is not None and obj.name_full in _seen:
        args["since"] = int(since)
    result = dispatch("link_pull", args)
    if not result.get("changed", True):
        return False
    positions = np.frombuffer(base64.b64decode(result["positions"]), dtype="<f4").reshape(-1, 3)
    tris = np.frombuffer(base64.b64decode(result["indices"]), dtype="<u4").astype(np.uint32)
    _write(obj, positions, tris)
    obj[REV_KEY] = int(result["revision"])
    return True


def fetch(context, fid=None):
    """Make a Blender object from a formsculptor object (the active one when fid is None) and link it."""
    args = {} if fid is None else {"id": int(fid)}
    result = dispatch("link_pull", args)
    mesh = bpy.data.meshes.new(result["name"])
    obj = bpy.data.objects.new(result["name"], mesh)
    context.collection.objects.link(obj)
    positions = np.frombuffer(base64.b64decode(result["positions"]), dtype="<f4").reshape(-1, 3)
    tris = np.frombuffer(base64.b64decode(result["indices"]), dtype="<u4").astype(np.uint32)
    _write(obj, positions, tris)
    obj[ID_KEY] = int(result["id"])
    obj[REV_KEY] = int(result["revision"])
    return obj


def linked_objects():
    return [o for o in bpy.data.objects if o.type == "MESH" and ID_KEY in o]


def sync_once():
    """One round of the live link: push what changed in Blender, pull what changed in formsculptor."""
    pushed = pulled = 0
    for obj in linked_objects():
        if obj.mode != "OBJECT":
            continue  # edit mode keeps its own copy of the mesh; it is sent once the object is back in object mode
        seen = _seen.get(obj.name_full)
        if seen is None or seen["fp"] != _fingerprint(obj):
            push(obj)
            pushed += 1
        elif pull(obj):
            pulled += 1
    return pushed, pulled


def _tick():
    prefs = _prefs()
    if prefs is not None and not prefs.live:
        return None
    try:
        pushed, pulled = sync_once()
        if pushed or pulled:
            _status["text"] = f"Sent {pushed}, received {pulled}"
            for window in bpy.context.window_manager.windows:
                for area in window.screen.areas:
                    if area.type == "VIEW_3D":
                        area.tag_redraw()
    except LinkError as e:
        _status["text"] = str(e)
    return prefs.interval if prefs is not None else 0.5


def _set_live(self, context):
    if self.live and not bpy.app.timers.is_registered(_tick):
        bpy.app.timers.register(_tick, first_interval=0.1, persistent=True)
    elif not self.live and bpy.app.timers.is_registered(_tick):
        bpy.app.timers.unregister(_tick)


class FormsculptorPreferences(bpy.types.AddonPreferences):
    bl_idname = __package__

    url: bpy.props.StringProperty(name="Address", default="http://127.0.0.1:8196", description="Where formsculptor's engine service listens")
    interval: bpy.props.FloatProperty(name="Interval", default=0.5, min=0.1, max=10.0, subtype="TIME_ABSOLUTE", unit="TIME_ABSOLUTE", description="Seconds between live updates")
    live: bpy.props.BoolProperty(name="Live", default=False, update=_set_live, description="Keep linked objects in step both ways")

    def draw(self, context):
        self.layout.prop(self, "url")
        self.layout.prop(self, "interval")


class FORMSCULPTOR_OT_send(bpy.types.Operator):
    bl_idname = "formsculptor.send"
    bl_label = "Send to formsculptor"
    bl_description = "Send the selected mesh objects to formsculptor and link them"
    bl_options = {"REGISTER"}

    @classmethod
    def poll(cls, context):
        return context.mode == "OBJECT" and any(o.type == "MESH" for o in context.selected_objects)

    def execute(self, context):
        sent = 0
        for obj in context.selected_objects:
            if obj.type != "MESH":
                continue
            try:
                push(obj, force_topology=True)
                sent += 1
            except LinkError as e:
                self.report({"ERROR"}, str(e))
                return {"CANCELLED"}
        self.report({"INFO"}, f"Sent {sent} to formsculptor")
        return {"FINISHED"}


class FORMSCULPTOR_OT_fetch(bpy.types.Operator):
    bl_idname = "formsculptor.fetch"
    bl_label = "Get from formsculptor"
    bl_description = "Bring formsculptor's active object into Blender and link it"
    bl_options = {"REGISTER", "UNDO"}

    def execute(self, context):
        try:
            obj = fetch(context)
        except LinkError as e:
            self.report({"ERROR"}, str(e))
            return {"CANCELLED"}
        self.report({"INFO"}, f"Linked {obj.name}")
        return {"FINISHED"}


class FORMSCULPTOR_OT_sync(bpy.types.Operator):
    bl_idname = "formsculptor.sync"
    bl_label = "Sync now"
    bl_description = "Send Blender's changes and bring formsculptor's back once"

    def execute(self, context):
        try:
            pushed, pulled = sync_once()
        except LinkError as e:
            self.report({"ERROR"}, str(e))
            return {"CANCELLED"}
        self.report({"INFO"}, f"Sent {pushed}, received {pulled}")
        return {"FINISHED"}


class FORMSCULPTOR_OT_unlink(bpy.types.Operator):
    bl_idname = "formsculptor.unlink"
    bl_label = "Unlink"
    bl_description = "Stop keeping the selected objects in step (both copies stay)"

    def execute(self, context):
        for obj in context.selected_objects:
            for key in (ID_KEY, REV_KEY):
                if key in obj:
                    del obj[key]
            _seen.pop(obj.name_full, None)
        return {"FINISHED"}


class FORMSCULPTOR_PT_link(bpy.types.Panel):
    bl_label = "Live Link"
    bl_idname = "FORMSCULPTOR_PT_link"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_category = "formsculptor"

    def draw(self, context):
        layout = self.layout
        prefs = _prefs()
        if prefs is not None:
            layout.prop(prefs, "live", toggle=True)
        col = layout.column(align=True)
        col.operator("formsculptor.send", icon="EXPORT")
        col.operator("formsculptor.fetch", icon="IMPORT")
        col.operator("formsculptor.sync", icon="FILE_REFRESH")
        col.operator("formsculptor.unlink", icon="UNLINKED")
        linked = linked_objects()
        layout.label(text=f"Linked: {len(linked)}")
        if _status["text"]:
            layout.label(text=_status["text"])


CLASSES = (FormsculptorPreferences, FORMSCULPTOR_OT_send, FORMSCULPTOR_OT_fetch, FORMSCULPTOR_OT_sync, FORMSCULPTOR_OT_unlink, FORMSCULPTOR_PT_link)


def register():
    for cls in CLASSES:
        bpy.utils.register_class(cls)


def unregister():
    if bpy.app.timers.is_registered(_tick):
        bpy.app.timers.unregister(_tick)
    for cls in reversed(CLASSES):
        bpy.utils.unregister_class(cls)
    _seen.clear()
