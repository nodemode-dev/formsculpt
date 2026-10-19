# formsculptor Live Link for Blender

A Blender add-on that keeps mesh objects in step with a running formsculptor, both ways, without a file in between.

## Install

1. In formsculptor, File > Blender add-on saves `formsculptor_link-<version>.zip` (or zip this folder yourself).
2. In Blender, Edit > Preferences > Extensions, then Install from Disk, and pick the zip.
3. The 3D Viewport's sidebar (N) has a formsculptor tab.

Tested with Blender 5.2.2. formsculptor must be running (the window, `./formsculptor boot`, or the desktop app); the address is in the add-on's preferences (default `http://127.0.0.1:8196`).

## Use

- **Send to formsculptor** sends the selected mesh objects and links them.
- **Get from formsculptor** brings formsculptor's active object into Blender and links it.
- **Live** keeps every linked object in step on a timer (Interval in the preferences): an edit in Blender is sent, a sculpt in formsculptor comes back. Objects in Edit Mode wait until they are back in Object Mode.
- **Sync now** does one round by hand; **Unlink** stops linking the selected objects (both copies stay).

### GoF

GoF is formsculptor's own hand-off to other apps: a folder (Formsculptor/GoF in the home folder by default) with one file per object and a `gof.json` manifest. The add-on reads it beside the live link:

- In formsculptor, File > GoF target Blender, then Send with GoF, writes the object as a GLB.
- **Import from GoF** brings in every file sent to Blender that is new or was sent again; an object already brought in keeps its name and transform and takes the new mesh. **Watch GoF** does it on its own as files arrive.
- **Send back to GoF** writes the selected GoF objects over their files with Blender's glTF exporter (+Y up, modifiers applied); formsculptor brings the edit back into the same object.

The GoF folder is in the add-on's preferences; left empty, the add-on asks formsculptor (`GET /api/gof`) and falls back to Formsculptor/GoF in the home folder. Files go through Blender's own glTF importer and exporter, so GoF needs nothing else in Blender.

When only the vertex positions change, only positions travel: Blender keeps its quads, UVs and modifiers, and formsculptor keeps its paint, mask and face groups. A change of topology on either side replaces the other side's mesh. formsculptor works in triangles, so a mesh replaced from formsculptor arrives in Blender as triangles.

## How it talks

The add-on posts to formsculptor's local dispatch (`POST /api/engine/dispatch` with `{"name", "args"}`), using two engine commands:

- `link_pull {id, since}` returns `revision` and, when the object has changed since `since`, `positions` (base64 little-endian f32 x, y, z per vertex) and `indices` (base64 little-endian u32, three per triangle).
- `link_push {id, positions, indices, name}` writes a mesh back in the same layout; without `id` it makes a new object.

Positions are in world space in formsculptor's axes (y up). Blender is z up, so Blender's x, y, z is formsculptor's x, z, -y, the same turn Blender's glTF exporter makes.

## License

Apache-2.0, like the rest of formsculptor, and written for formsculptor: no Blender code is copied into it. Blender's license page asks that published add-ons using its Python API be under a GPL-compliant license; Blender's scripts are GPL-2.0-or-later, and the Apache Software Foundation records that the FSF considers Apache-2.0 compatible with GPL version 3 (not version 2), so the add-on can be combined with Blender under GPL-3.0. This is a reading of those two pages, not legal advice.
