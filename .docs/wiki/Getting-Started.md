# Getting Started

Requirements: Rust 1.80 or newer, Node 20.19 or newer, Python 3.12 or newer.

```bash
./formsculpt_oc_install
./formsculpt doctor
./formsculpt
```

The frontend opens on http://localhost:5260 and the engine service on http://127.0.0.1:8260. Both bind to the loopback address only.

For the desktop app: `./formsculpt install-desktop`, then `./formsculpt desktop build`. On Linux this needs `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf`.
