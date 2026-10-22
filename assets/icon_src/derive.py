import base64, io, shutil, sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw
from scipy import ndimage

sys.path.insert(0, str(Path(__file__).parent))
from tile import squircle

TILE = 824
MARGIN = (1024 - TILE) // 2
PNG_SIZES = {
    "32x32.png": 32, "64x64.png": 64, "128x128.png": 128, "128x128@2x.png": 256, "icon.png": 512,
    "Square30x30Logo.png": 30, "Square44x44Logo.png": 44, "Square71x71Logo.png": 71,
    "Square89x89Logo.png": 89, "Square107x107Logo.png": 107, "Square142x142Logo.png": 142,
    "Square150x150Logo.png": 150, "Square284x284Logo.png": 284, "Square310x310Logo.png": 310,
    "StoreLogo.png": 50,
}
ICO_SIZES = [16, 24, 32, 48, 64, 128, 256]
ICNS_TYPES = [
    (b"icp4", 16), (b"icp5", 32), (b"icp6", 64), (b"ic07", 128), (b"ic08", 256), (b"ic09", 512),
    (b"ic10", 1024), (b"ic11", 32), (b"ic12", 64), (b"ic13", 256), (b"ic14", 512),
]


def icns(img, path):
    body = b""
    for kind, size in ICNS_TYPES:
        buf = io.BytesIO()
        img.resize((size, size), Image.LANCZOS).save(buf, "PNG", optimize=True)
        data = buf.getvalue()
        body += kind + (len(data) + 8).to_bytes(4, "big") + data
    Path(path).write_bytes(b"icns" + (len(body) + 8).to_bytes(4, "big") + body)


def squircle_mask(canvas, size, ss=4):
    c = canvas / 2
    pts = [tuple(float(v) * ss for v in p.split(",")) for p in squircle(c, c, size)[1:-2].split(" L")]
    m = Image.new("L", (canvas * ss, canvas * ss), 0)
    ImageDraw.Draw(m).polygon(pts, fill=255)
    return np.asarray(m.resize((canvas, canvas), Image.LANCZOS))


def full_bleed(mac):
    a = np.asarray(mac.convert("RGBA")).copy()
    solid = a[..., 3] >= 250
    _, (iy, ix) = ndimage.distance_transform_edt(~solid, return_indices=True)
    a[..., :3] = a[iy, ix, :3]
    a[..., 3] = 255
    tile = Image.fromarray(a).crop((MARGIN, MARGIN, MARGIN + TILE, MARGIN + TILE)).resize((1024, 1024), Image.LANCZOS)
    tile.putalpha(Image.fromarray(squircle_mask(1024, 1024)))
    return tile


def png(img, size, path):
    img.resize((size, size), Image.LANCZOS).save(path, optimize=True)


def main(master, repo):
    repo = Path(repo)
    mac = Image.open(master).convert("RGBA")
    assert mac.size == (1024, 1024)
    full = full_bleed(mac)
    shutil.copyfile(master, repo / "assets/formsculpt_icon_macos.png")
    full.save(repo / "assets/formsculpt_icon.png", optimize=True)
    full.save(repo / "assets/formsculpt_folder_icon.png", optimize=True)
    icons = repo / "src-tauri/icons"
    for name, size in PNG_SIZES.items():
        png(full, size, icons / name)
    full.resize((256, 256), Image.LANCZOS).save(icons / "icon.ico", sizes=[(s, s) for s in ICO_SIZES])
    icns(mac, icons / "icon.icns")
    pub = repo / "frontend/public"
    png(full, 144, pub / "splash-icon.png")
    buf = io.BytesIO()
    full.resize((192, 192), Image.LANCZOS).save(buf, "PNG", optimize=True)
    data = base64.b64encode(buf.getvalue()).decode()
    (pub / "favicon.svg").write_text(
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 192 192"><image href="data:image/png;base64,{data}" width="192" height="192"/></svg>'
    )


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
