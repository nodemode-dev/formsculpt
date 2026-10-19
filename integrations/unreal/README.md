# formsculptor GoF for Unreal Engine

A Python script for the Unreal Editor that imports what formsculptor sends to Unreal with GoF, formsculptor's own hand-off.

## Install

1. In the Unreal project, turn on the Python Editor Script Plugin (Edit > Plugins).
2. Copy `Content/Python` from here into the project's `Content/Python` (if the project already has an `init_unreal.py`, add its two lines to that file instead of replacing it).
3. Restart the editor: `init_unreal.py` starts the receiver, and the Output Log says which folder it watches.

The folder is `FORMSCULPTOR_GOF` when that is set, otherwise `Formsculptor/GoF` in the home folder, formsculptor's own default.

## Use

In formsculptor, File > GoF target Unreal Engine, then Send with GoF. Within a second the GLB is imported into `/Game/GoF` under the object's name; sending the same object again re-imports it over the same asset. `formsculptor_gof.check()` in the Python console imports by hand, and `formsculptor_gof.stop()` stops watching.

formsculptor writes GLB in metres (glTF's unit), with the scene's real size when one is set; Unreal works in centimetres and its glTF import converts.

## Status

Not yet run inside the Unreal Editor: it was written against Epic's Python API reference (AssetImportTask, AssetToolsHelpers.get_asset_tools, AssetTools.import_asset_tasks, register_slate_post_tick_callback, log) and its logic run against a stand-in `unreal` module with formsculptor's real GoF output. Edits made in Unreal do not come back to formsculptor yet.

## License

Apache-2.0, like the rest of formsculptor.
