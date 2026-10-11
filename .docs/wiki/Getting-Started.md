# Getting Started

Requirements: Rust 1.80 or newer, Node 20.19 or newer, Python 3.12 or newer.

```bash
./formsculptor_oc_install
./formsculptor doctor
./formsculptor
```

The frontend opens on http://localhost:5196 and the engine service on http://127.0.0.1:8196. Both bind to the loopback address only.

For the desktop app: `./formsculptor install-desktop`, then `./formsculptor desktop build`. On Linux this needs `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf`.
