# SPDX-License-Identifier: Apache-2.0
"""formsculptor GoF receiver for the Unreal Editor.

Watches formsculptor's GoF folder and imports what formsculptor sends to Unreal (Send with GoF, target Unreal Engine)
into /Game/GoF, replacing the asset when the same object is sent again. formsculptor writes GLB in metres; Unreal's
importer brings glTF in at its own centimetres.

Start it from the Python console with `import formsculptor_gof; formsculptor_gof.start()`, or let the init_unreal.py
beside this file start it when the editor opens. Every Unreal call here is from Epic's Python API reference:
unreal.register_slate_post_tick_callback, AssetImportTask (filename, destination_path, destination_name,
replace_existing, automated, save, imported_object_paths), AssetToolsHelpers.get_asset_tools and
AssetTools.import_asset_tasks, unreal.log and unreal.log_warning.
"""

import json
import os

import unreal

DESTINATION = "/Game/GoF"
TARGETS = ("unreal", "any")
FORMATS = ("glb",)
INTERVAL = 1.0

_state = {"handle": None, "elapsed": 0.0, "stamp": None}


def gof_folder():
    """FORMSCULPTOR_GOF when set, otherwise Formsculptor/GoF in the home folder (formsculptor's own default)."""
    folder = os.environ.get("FORMSCULPTOR_GOF", "").strip()
    return folder or os.path.join(os.path.expanduser("~"), "Formsculptor", "GoF")


def _seen_path(folder):
    return os.path.join(folder, ".seen", "unreal.json")


def _load_seen(folder):
    try:
        with open(_seen_path(folder), encoding="utf-8") as f:
            return json.load(f)
    except (OSError, ValueError):
        return {}


def _save_seen(folder, seen):
    os.makedirs(os.path.dirname(_seen_path(folder)), exist_ok=True)
    with open(_seen_path(folder), "w", encoding="utf-8") as f:
        json.dump(seen, f, indent=1)


def entries(folder):
    """gof.json's entries meant for Unreal (or for any app) in a format Unreal imports here."""
    try:
        with open(os.path.join(folder, "gof.json"), encoding="utf-8") as f:
            objects = json.load(f).get("objects", [])
    except (OSError, ValueError):
        return []
    return [o for o in objects if o.get("target") in TARGETS and o.get("format") in FORMATS]


def import_entry(folder, entry):
    """Import one GoF file into DESTINATION, replacing an earlier import of the same file. Returns the asset paths."""
    task = unreal.AssetImportTask()
    task.filename = os.path.join(folder, entry["file"])
    task.destination_path = DESTINATION
    task.destination_name = os.path.splitext(entry["file"])[0]
    task.replace_existing = True
    task.automated = True
    task.save = True
    unreal.AssetToolsHelpers.get_asset_tools().import_asset_tasks([task])
    return list(task.imported_object_paths)


def check(folder=None):
    """Import every Unreal entry that is new or was sent again since it was last imported. Returns how many."""
    folder = folder or gof_folder()
    seen = _load_seen(folder)
    done = 0
    for entry in entries(folder):
        sent = int(entry.get("sent", 0))
        if seen.get(entry["file"], -1) >= sent:
            continue
        if not os.path.isfile(os.path.join(folder, entry["file"])):
            unreal.log_warning(f"formsculptor GoF: {entry['file']} is listed but not in {folder}")
            continue
        paths = import_entry(folder, entry)
        if paths:
            seen[entry["file"]] = sent
            done += 1
            unreal.log(f"formsculptor GoF: imported {entry['file']} as {', '.join(paths)}")
        else:
            unreal.log_warning(f"formsculptor GoF: Unreal imported nothing from {entry['file']}")
    if done:
        _save_seen(folder, seen)
    return done


def _tick(delta):
    _state["elapsed"] += delta
    if _state["elapsed"] < INTERVAL:
        return
    _state["elapsed"] = 0.0
    folder = gof_folder()
    try:
        stamp = os.path.getmtime(os.path.join(folder, "gof.json"))
    except OSError:
        return
    if stamp == _state["stamp"]:
        return
    _state["stamp"] = stamp
    check(folder)


def start():
    """Watch the GoF folder (once a second, on the editor's tick)."""
    if _state["handle"] is None:
        _state["stamp"] = None
        _state["handle"] = unreal.register_slate_post_tick_callback(_tick)
        unreal.log(f"formsculptor GoF: watching {gof_folder()}")


def stop():
    if _state["handle"] is not None:
        unreal.unregister_slate_post_tick_callback(_state["handle"])
        _state["handle"] = None
