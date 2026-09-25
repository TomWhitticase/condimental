"""Tiny isometric voxel renderer for CONDIMENTAL concept art.

Reads the game's real palette (assets/voxels/palette.ron) so concept icons sit
in the same colour world as the shipped ones. Not the game pipeline — the
shipped pipeline is a Rust generator + vengi render; this is a sketch tool.
"""
import os, re, math
from PIL import Image, ImageDraw

REPO = os.path.abspath(os.path.join(os.path.dirname(__file__), "../../.."))

def load_palette():
    txt = open(f"{REPO}/assets/voxels/palette.ron").read()
    pal = {}
    for name, rgb, cls in re.findall(
        r'\(name:\s*"([a-z_0-9]+)",\s*rgb:\s*"#([0-9A-Fa-f]{6})",\s*class:\s*"(\w+)"\)', txt):
        pal[name] = (int(rgb[0:2], 16), int(rgb[2:4], 16), int(rgb[4:6], 16), cls)
    return pal

PAL = load_palette()

def rgb(name):
    if name.startswith("#"):
        return (int(name[1:3],16), int(name[3:5],16), int(name[5:7],16), "matte")
    return PAL[name]

class Model:
    """A voxel grid. x -> east, y -> south, z -> up."""
    def __init__(self):
        self.v = {}

    def set(self, x, y, z, c):
        if c is None:
            self.v.pop((x, y, z), None)
        else:
            self.v[(x, y, z)] = c

    def box(self, x0, x1, y0, y1, z0, z1, c):
        for x in range(x0, x1 + 1):
            for y in range(y0, y1 + 1):
                for z in range(z0, z1 + 1):
                    self.set(x, y, z, c)
        return self

    def cut(self, x0, x1, y0, y1, z0, z1):
        return self.box(x0, x1, y0, y1, z0, z1, None)

    def cyl(self, cx, cy, r, z0, z1, c, hollow=None):
        """Filled circle extruded in z. cx/cy may be half-integers."""
        for x in range(int(cx - r) - 1, int(cx + r) + 2):
            for y in range(int(cy - r) - 1, int(cy + r) + 2):
                d = math.hypot(x + .5 - cx, y + .5 - cy)
                if d <= r and (hollow is None or d >= hollow):
                    for z in range(z0, z1 + 1):
                        self.set(x, y, z, c)
        return self

    def ball(self, cx, cy, cz, r, c):
        for x in range(int(cx - r) - 1, int(cx + r) + 2):
            for y in range(int(cy - r) - 1, int(cy + r) + 2):
                for z in range(int(cz - r) - 1, int(cz + r) + 2):
                    if math.dist((x + .5, y + .5, z + .5), (cx, cy, cz)) <= r:
                        self.set(x, y, z, c)
        return self

    def recolour(self, old, new):
        for k, c in list(self.v.items()):
            if c == old:
                self.v[k] = new
        return self

    def copy(self):
        m = Model(); m.v = dict(self.v); return m

    def translate(self, dx, dy, dz):
        m = Model()
        m.v = {(x+dx, y+dy, z+dz): c for (x, y, z), c in self.v.items()}
        return m

    def merge(self, other):
        self.v.update(other.v); return self

# face shading: top, left(-y side, camera-facing), right(+x side)
SH_TOP, SH_LEFT, SH_RIGHT = 1.0, 0.76, 0.54

def shade(c, f):
    r, g, b = c[0], c[1], c[2]
    if c[3] == "emissive":
        f = min(1.0, f + 0.35)
    if c[3] == "metal":
        f = f ** 0.75          # metal keeps the brightest pixel in the frame
    if c[3] == "liquid":
        f = min(1.0, f * 1.06)
    return (int(r * f), int(g * f), int(b * f), 255)

def render(model, tw=4, trim=True, pad=2):
    """Isometric render. Camera sits at the +x/+y/+z corner: a voxel's top,
    its +y face (screen left) and its +x face (screen right) are all we see."""
    th = tw // 2
    tz = tw
    pts = model.v
    if not pts:
        raise ValueError("empty model")
    def proj(x, y, z):
        return ((x - y) * tw, (x + y) * th - z * tz)
    xs, ys = zip(*[proj(x, y, z) for (x, y, z) in pts])
    minx, maxx = min(xs) - tw, max(xs) + tw
    miny, maxy = min(ys) - th, max(ys) + th + tz
    W, H = int(maxx - minx) + pad * 2, int(maxy - miny) + pad * 2
    img = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    visible = [k for k in pts
               if (k[0] + 1, k[1], k[2]) not in pts
               or (k[0], k[1] + 1, k[2]) not in pts
               or (k[0], k[1], k[2] + 1) not in pts]
    for (x, y, z) in sorted(visible, key=lambda p: p[0] + p[1] + p[2]):
        c = rgb(pts[(x, y, z)])
        sx, sy = proj(x, y, z)
        sx = sx - minx + pad; sy = sy - miny + pad
        # top face: hidden if a voxel sits directly above
        if (x, y, z + 1) not in pts:
            d.polygon([(sx, sy - th), (sx + tw, sy), (sx, sy + th), (sx - tw, sy)],
                      fill=shade(c, SH_TOP))
        if (x, y + 1, z) not in pts:      # +y face, to the screen-left
            d.polygon([(sx - tw, sy), (sx, sy + th), (sx, sy + th + tz), (sx - tw, sy + tz)],
                      fill=shade(c, SH_LEFT))
        if (x + 1, y, z) not in pts:      # +x face, to the screen-right
            d.polygon([(sx, sy + th), (sx + tw, sy), (sx + tw, sy + tz), (sx, sy + th + tz)],
                      fill=shade(c, SH_RIGHT))
    if trim:
        img = img.crop(img.getbbox())
    return img

def icon(model, size=96, tw=6):
    """Render then fit into a square of `size`, keeping the pixels crisp."""
    im = render(model, tw=tw)
    s = min(size / im.width, size / im.height)
    im = im.resize((max(1, int(im.width * s)), max(1, int(im.height * s))), Image.LANCZOS)
    out = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    out.alpha_composite(im, ((size - im.width) // 2, (size - im.height) // 2))
    return out
