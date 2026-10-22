"""Generate the app icons in src-tauri/icons/ (requires Pillow). Run once; output is committed."""
from pathlib import Path
from PIL import Image, ImageDraw

OUT = Path(__file__).resolve().parent.parent / "src-tauri" / "icons"
OUT.mkdir(parents=True, exist_ok=True)

S = 1024
img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
d = ImageDraw.Draw(img)
d.rounded_rectangle([40, 40, S - 40, S - 40], radius=200, fill=(38, 70, 120, 255))
d.rounded_rectangle([40, 40, S - 40, S - 40], radius=200, outline=(255, 255, 255, 40), width=12)

W = (255, 255, 255, 255)
# "M": two verticals and a V.
top, bot = 300, 724
x0, x1 = 170, 560
st = 92
d.polygon([(x0, bot), (x0, top), (x0 + st, top), ((x0 + x1) / 2, top + 230),
           (x1 - st, top), (x1, top), (x1, bot), (x1 - st, bot), (x1 - st, top + 190),
           ((x0 + x1) / 2, top + 380), (x0 + st, top + 190), (x0 + st, bot)], fill=W)
# Down arrow.
ax = 730
d.rectangle([ax - st / 2, top, ax + st / 2, bot - 170], fill=W)
d.polygon([(ax - 150, bot - 200), (ax + 150, bot - 200), (ax, bot)], fill=W)

def save(size, name):
    img.resize((size, size), Image.LANCZOS).save(OUT / name)

save(32, "32x32.png")
save(128, "128x128.png")
save(256, "128x128@2x.png")
save(512, "icon.png")
img.save(OUT / "icon.ico", sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
print("icons written to", OUT)
