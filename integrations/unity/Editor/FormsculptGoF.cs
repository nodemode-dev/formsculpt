// SPDX-License-Identifier: Apache-2.0
// formsculpt GoF receiver for the Unity Editor.
//
// Watches formsculpt's GoF folder and brings what formsculpt sends to Unity (Send with GoF, target Unity) into
// Assets/GoF, re-importing when the same object is sent again. formsculpt writes FBX in centimetres (the unit the
// file declares), which Unity's FBX importer converts to its metre with Convert Units. Every Unity call here is from
// Unity's Scripting Reference: InitializeOnLoad, EditorApplication.update and timeSinceStartup, EditorPrefs,
// JsonUtility.FromJson, Application.dataPath, AssetDatabase.ImportAsset with ImportAssetOptions.ForceUpdate,
// MenuItem and Menu.SetChecked.
//
// Put this file in any Editor folder of the project (Assets/Editor works). Tools > formsculpt has Import from GoF,
// Watch GoF and Set GoF Folder.

using System;
using System.IO;
using UnityEditor;
using UnityEngine;

[InitializeOnLoad]
public static class FormsculptGoF
{
    const string FolderKey = "formsculpt.gof.folder";
    const string WatchKey = "formsculpt.gof.watch";
    const string SentKey = "formsculpt.gof.sent.";
    const string WatchMenu = "Tools/formsculpt/Watch GoF";

    // Filled by JsonUtility from gof.json, never assigned in code (CS0649 says so otherwise).
#pragma warning disable 0649
    [Serializable]
    class Entry
    {
        public string file;
        public string format;
        public string target;
        public long sent;
    }

    [Serializable]
    class Manifest
    {
        public Entry[] objects;
    }
#pragma warning restore 0649

    static double next;
    static DateTime stamp;

    static FormsculptGoF()
    {
        EditorApplication.update += Tick;
    }

    /// The GoF folder: the one set here, otherwise Formsculpt/GoF in the home folder (formsculpt's own default).
    public static string Folder
    {
        get
        {
            string set = EditorPrefs.GetString(FolderKey, "");
            if (!string.IsNullOrEmpty(set)) return set;
            string home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
            return Path.Combine(home, "Formsculpt", "GoF");
        }
    }

    static bool Watching => EditorPrefs.GetBool(WatchKey, true);

    [MenuItem("Tools/formsculpt/Import from GoF")]
    public static void ImportNow()
    {
        int done = Check();
        Debug.Log(done > 0 ? $"formsculpt GoF: imported {done}" : "formsculpt GoF: nothing new");
    }

    [MenuItem(WatchMenu)]
    static void ToggleWatch()
    {
        EditorPrefs.SetBool(WatchKey, !Watching);
        Menu.SetChecked(WatchMenu, Watching);
    }

    [MenuItem("Tools/formsculpt/Set GoF Folder")]
    static void SetFolder()
    {
        string chosen = EditorUtility.OpenFolderPanel("formsculpt GoF folder", Folder, "");
        if (!string.IsNullOrEmpty(chosen)) EditorPrefs.SetString(FolderKey, chosen);
    }

    static void Tick()
    {
        if (EditorApplication.timeSinceStartup < next) return;
        next = EditorApplication.timeSinceStartup + 1.0;
        // The check mark is cleared when the Editor restarts (Menu.SetChecked), so it is set again from the pref.
        Menu.SetChecked(WatchMenu, Watching);
        if (!Watching) return;
        string manifest = Path.Combine(Folder, "gof.json");
        if (!File.Exists(manifest)) return;
        DateTime written = File.GetLastWriteTimeUtc(manifest);
        if (written == stamp) return;
        stamp = written;
        Check();
    }

    /// Imports every entry for Unity (or any app) in a format Unity reads that is new or sent again. Returns how many.
    public static int Check()
    {
        string folder = Folder;
        string manifestPath = Path.Combine(folder, "gof.json");
        if (!File.Exists(manifestPath)) return 0;
        Manifest manifest = JsonUtility.FromJson<Manifest>(File.ReadAllText(manifestPath));
        if (manifest == null || manifest.objects == null) return 0;
        string into = Path.Combine(Application.dataPath, "GoF");
        int done = 0;
        foreach (Entry entry in manifest.objects)
        {
            if (entry.target != "unity" && entry.target != "any") continue;
            if (entry.format != "fbx" && entry.format != "obj") continue;
            string key = SentKey + entry.file;
            if (EditorPrefs.GetString(key, "") == entry.sent.ToString()) continue;
            string source = Path.Combine(folder, entry.file);
            if (!File.Exists(source))
            {
                Debug.LogWarning($"formsculpt GoF: {entry.file} is listed but not in {folder}");
                continue;
            }
            Directory.CreateDirectory(into);
            File.Copy(source, Path.Combine(into, entry.file), true);
            AssetDatabase.ImportAsset("Assets/GoF/" + entry.file, ImportAssetOptions.ForceUpdate);
            EditorPrefs.SetString(key, entry.sent.ToString());
            done++;
        }
        return done;
    }
}
