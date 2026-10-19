# SPDX-License-Identifier: Apache-2.0
"""formsculptor Live Link for Blender.

Sends Blender mesh objects to a running formsculptor and keeps both sides in step: an edit in Blender is pushed
(link_push), a sculpt in formsculptor is pulled back (link_pull), each on a timer while the link is live. Meshes travel
as base64 little-endian arrays over formsculptor's local dispatch (POST /api/engine/dispatch), in formsculptor's axes
(y up); Blender is z up, so x, y, z here is x, z, -y there, the same turn Blender's glTF exporter makes.

When only the positions change on either side, only the positions move: Blender keeps its quads, UVs and modifiers,
formsculptor keeps its paint, mask and face groups.

GoF, formsculptor's own hand-off, works beside the link: formsculptor's Send with GoF (target Blender) writes a GLB
into the GoF folder and lists it in gof.json; Import from GoF (or Watch GoF) brings it in through Blender's own glTF
importer, and Send back to GoF writes the object over that GLB with Blender's glTF exporter, which formsculptor's
bridge sync reads back into the same object.
"""

import base64
import hashlib
import json
import os
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


GOF_FILE = "gof_file"
GOF_SENT = "gof_sent"
_gof = {"stamp": None}


def gof_folder():
    """The GoF folder: the preference, else the one formsculptor's service reports, else ~/Formsculptor/GoF."""
    prefs = _prefs()
    if prefs is not None and prefs.gof_folder.strip():
        return bpy.path.abspath(prefs.gof_folder.strip())
    try:
        with urllib.request.urlopen(_url() + "/api/gof", timeout=3) as reply:
            folder = json.loads(reply.read().decode("utf-8")).get("folder")
            if folder:
                return folder
    except (urllib.error.URLError, OSError, ValueError):
        pass
    return os.path.join(os.path.expanduser("~"), "Formsculptor", "GoF")


def gof_entries(folder):
    """gof.json's GLB entries meant for Blender (or for no app in particular), oldest first."""
    try:
        with open(os.path.join(folder, "gof.json"), encoding="utf-8") as f:
            objects = json.load(f).get("objects", [])
    except (OSError, ValueError):
        return []
    return [o for o in objects if o.get("format") == "glb" and o.get("target") in ("blender", "any")]


def gof_import(context, folder, entry):
    """Import one GoF GLB with Blender's glTF importer. An object already brought from the same file keeps its
    transform and name and takes the new mesh. Returns the objects that now hold it."""
    path = os.path.join(folder, entry["file"])
    if not os.path.isfile(path):
        raise LinkError(f"{entry['file']} is not in {folder}")
    before = set(bpy.data.objects)
    result = bpy.ops.import_scene.gltf(filepath=path, merge_vertices=True)
    if "FINISHED" not in result:
        raise LinkError(f"Blender's glTF importer could not read {entry['file']}")
    new = [o for o in bpy.data.objects if o not in before]
    meshes = [o for o in new if o.type == "MESH"]
    existing = [o for o in bpy.data.objects if o not in new and o.get(GOF_FILE) == entry["file"] and o.type == "MESH"]
    if existing and meshes:
        target = existing[0]
        old = target.data
        target.data = meshes[0].data
        for o in new:
            bpy.data.objects.remove(o, do_unlink=True)
        if old.users == 0:
            bpy.data.meshes.remove(old)
        held = [target]
    else:
        held = meshes
    for o in held:
        o[GOF_FILE] = entry["file"]
        o[GOF_SENT] = int(entry.get("sent", 0))
    return held


def gof_check(context, folder=None):
    """Import every GoF entry for Blender that is new or was sent again since it was last imported."""
    folder = folder or gof_folder()
    done = 0
    for entry in gof_entries(folder):
        current = [o for o in bpy.data.objects if o.get(GOF_FILE) == entry["file"]]
        if current and all(int(o.get(GOF_SENT, 0)) >= int(entry.get("sent", 0)) for o in current):
            continue
        gof_import(context, folder, entry)
        done += 1
    return done


def gof_return(context, obj, folder=None):
    """Write obj over its GoF GLB with Blender's glTF exporter (+Y up, glTF's convention), so formsculptor's bridge
    sync brings the edit back into the object it came from."""
    name = obj.get(GOF_FILE)
    if not name:
        raise LinkError(f"{obj.name} did not come from GoF")
    path = os.path.join(folder or gof_folder(), name)
    selected = list(context.selected_objects)
    active = context.view_layer.objects.active
    for o in selected:
        o.select_set(False)
    obj.select_set(True)
    context.view_layer.objects.active = obj
    try:
        result = bpy.ops.export_scene.gltf(filepath=path, export_format="GLB", use_selection=True, export_yup=True, export_apply=True, export_materials="NONE")
    finally:
        obj.select_set(False)
        for o in selected:
            o.select_set(True)
        context.view_layer.objects.active = active
    if "FINISHED" not in result:
        raise LinkError(f"Blender's glTF exporter could not write {name}")
    return path


def _gof_tick():
    prefs = _prefs()
    if prefs is not None and not prefs.gof_watch:
        return None
    folder = gof_folder()
    try:
        stamp = os.path.getmtime(os.path.join(folder, "gof.json"))
    except OSError:
        stamp = None
    if stamp is not None and stamp != _gof["stamp"]:
        _gof["stamp"] = stamp
        try:
            done = gof_check(bpy.context, folder)
            if done:
                _status["text"] = f"GoF: imported {done}"
        except LinkError as e:
            _status["text"] = str(e)
    return 1.0


def _set_gof_watch(self, context):
    if self.gof_watch and not bpy.app.timers.is_registered(_gof_tick):
        _gof["stamp"] = None
        bpy.app.timers.register(_gof_tick, first_interval=0.1, persistent=True)
    elif not self.gof_watch and bpy.app.timers.is_registered(_gof_tick):
        bpy.app.timers.unregister(_gof_tick)


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
    gof_folder: bpy.props.StringProperty(name="GoF folder", default="", subtype="DIR_PATH", description="formsculptor's GoF folder; empty asks formsculptor, then uses Formsculptor/GoF in the home folder")
    gof_watch: bpy.props.BoolProperty(name="Watch GoF", default=False, update=_set_gof_watch, description="Import what formsculptor sends to Blender with GoF as it arrives")

    def draw(self, context):
        self.layout.prop(self, "url")
        self.layout.prop(self, "interval")
        self.layout.prop(self, "gof_folder")


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


class FORMSCULPTOR_OT_gof_import(bpy.types.Operator):
    bl_idname = "formsculptor.gof_import"
    bl_label = "Import from GoF"
    bl_description = "Bring in what formsculptor sent to Blender with GoF (new files, and newer copies of ones already here)"
    bl_options = {"REGISTER", "UNDO"}

    def execute(self, context):
        try:
            done = gof_check(context)
        except LinkError as e:
            self.report({"ERROR"}, str(e))
            return {"CANCELLED"}
        self.report({"INFO"}, f"GoF: imported {done}" if done else "GoF: nothing new")
        return {"FINISHED"}


class FORMSCULPTOR_OT_gof_return(bpy.types.Operator):
    bl_idname = "formsculptor.gof_return"
    bl_label = "Send back to GoF"
    bl_description = "Write the selected GoF objects over their files, so formsculptor brings the edits back"

    @classmethod
    def poll(cls, context):
        return context.mode == "OBJECT" and any(o.get(GOF_FILE) for o in context.selected_objects)

    def execute(self, context):
        folder = gof_folder()
        sent = 0
        for obj in [o for o in context.selected_objects if o.get(GOF_FILE)]:
            try:
                gof_return(context, obj, folder)
                sent += 1
            except LinkError as e:
                self.report({"ERROR"}, str(e))
                return {"CANCELLED"}
        self.report({"INFO"}, f"GoF: sent back {sent}")
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
        layout.separator()
        layout.label(text="GoF")
        if prefs is not None:
            layout.prop(prefs, "gof_watch", toggle=True)
        col = layout.column(align=True)
        col.operator("formsculptor.gof_import", icon="IMPORT")
        col.operator("formsculptor.gof_return", icon="EXPORT")
        if _status["text"]:
            layout.label(text=_status["text"])


CLASSES = (FormsculptorPreferences, FORMSCULPTOR_OT_send, FORMSCULPTOR_OT_fetch, FORMSCULPTOR_OT_sync, FORMSCULPTOR_OT_unlink, FORMSCULPTOR_OT_gof_import, FORMSCULPTOR_OT_gof_return, FORMSCULPTOR_PT_link)


def register():
    for cls in CLASSES:
        bpy.utils.register_class(cls)


def unregister():
    for timer in (_tick, _gof_tick):
        if bpy.app.timers.is_registered(timer):
            bpy.app.timers.unregister(timer)
    for cls in reversed(CLASSES):
        bpy.utils.unregister_class(cls)
    _seen.clear()
