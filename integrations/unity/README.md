# formsculptor GoF for Unity

An editor script that imports what formsculptor sends to Unity with GoF, formsculptor's own hand-off.

## Install

Copy `Editor/FormsculptorGoF.cs` into an `Editor` folder of the Unity project (`Assets/Editor` works). Unity compiles it and it starts watching.

## Use

In formsculptor, File > GoF target Unity, then Send with GoF. Within a second the FBX is copied into `Assets/GoF` and imported; sending the same object again re-imports it. Files sent for any app as OBJ are brought in too. The Tools > formsculptor menu has Import from GoF (by hand), Watch GoF (on by default) and Set GoF Folder; the folder is otherwise `Formsculptor/GoF` in the home folder, formsculptor's own default.

formsculptor writes FBX in centimetres, the unit the file declares; keep the model's Convert Units import setting on and Unity brings it in at its own metre, so an object one metre across in formsculptor is one unit in Unity (or its real size when one is set in formsculptor).

## Status

Not yet run inside Unity: it was written against Unity's Scripting Reference (InitializeOnLoad, EditorApplication.update and timeSinceStartup, EditorPrefs, JsonUtility.FromJson, Application.dataPath, AssetDatabase.ImportAsset, MenuItem, Menu.SetChecked, EditorUtility.OpenFolderPanel), compiled with the .NET 8 SDK at C# 9 against stand-ins with those signatures, and its logic run on formsculptor's real GoF output. Edits made in Unity do not come back to formsculptor yet.

## License

Apache-2.0, like the rest of formsculptor.
