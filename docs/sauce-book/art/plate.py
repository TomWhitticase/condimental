"""Plate kit: the dark 'spec sheet' cards the sauce book is built out of.

Everything is drawn in the game's own palette so a plate reads as a page torn
out of the factory rather than as slideware.
"""
from PIL import Image, ImageDraw, ImageFont
from voxlib import PAL, icon, REPO

FONT = "/System/Library/Fonts/Avenir Next Condensed.ttc"
MONO = "/System/Library/Fonts/Menlo.ttc"
def f_bold(s):  return ImageFont.truetype(FONT, s, index=0)
def f_demi(s):  return ImageFont.truetype(FONT, s, index=2)
def f_med(s):   return ImageFont.truetype(FONT, s, index=5)
def f_reg(s):   return ImageFont.truetype(FONT, s, index=7)
def f_mono(s):  return ImageFont.truetype(MONO, s, index=0)

BG       = (0x1E, 0x1E, 0x1C)
GRID     = (0x2A, 0x2A, 0x26)
CARD     = (0x2E, 0x2E, 0x2A)
CARD_HI  = (0x38, 0x38, 0x33)
EDGE     = (0x4A, 0x4A, 0x43)
CREAM    = (0xE5, 0xDC, 0xC5)
STEEL    = (0x9B, 0xA3, 0xAC)
STEEL_DK = (0x6F, 0x76, 0x80)
YELLOW   = (0xD9, 0xA2, 0x1B)
GREEN    = (0x4A, 0xE0, 0x6B)
AMBER    = (0xFF, 0xB0, 0x20)
RED      = (0xFF, 0x3B, 0x30)
BLUE     = (0x7F, 0xC8, 0xE8)

def hexc(h):
    h = h.lstrip("#"); return (int(h[0:2],16), int(h[2:4],16), int(h[4:6],16))

_game_icons = {}
def game_icon(name, size=128):
    """A real shipped icon from assets/voxels/icons, scaled up crisply."""
    key = (name, size)
    if key not in _game_icons:
        im = Image.open(f"{REPO}/assets/voxels/icons/{name}.png").convert("RGBA")
        s = size / max(im.width, im.height)
        im = im.resize((int(im.width*s), int(im.height*s)),
                       Image.NEAREST if s >= 1.6 else Image.LANCZOS)
        _game_icons[key] = im
    return _game_icons[key]

class Plate:
    def __init__(self, w, h, grid=True):
        self.im = Image.new("RGBA", (w, h), BG)
        self.d = ImageDraw.Draw(self.im)
        self.w, self.h = w, h
        if grid:
            for x in range(0, w, 50):
                self.d.line([(x, 0), (x, h)], fill=GRID)
            for y in range(0, h, 50):
                self.d.line([(0, y), (w, y)], fill=GRID)

    # ---- text -----------------------------------------------------------
    def text(self, xy, s, font, fill=CREAM, anchor="la", spacing=4):
        self.d.text(xy, s, font=font, fill=fill, anchor=anchor, spacing=spacing)
        return self

    def wrap(self, s, font, width):
        """Greedy word wrap to a pixel width — no text may leave its card."""
        lines, cur = [], ""
        for w in s.split():
            t = (cur + " " + w).strip()
            if not cur or self.d.textlength(t, font=font) <= width:
                cur = t
            else:
                lines.append(cur); cur = w
        if cur:
            lines.append(cur)
        return "\n".join(lines)

    def title(self, x, y, s, sub=None, rule=YELLOW):
        self.d.rectangle([x, y, x + 7, y + 44], fill=rule)
        self.text((x + 22, y - 5), s.upper(), f_bold(46))
        if sub:
            self.text((x + 22, y + 50), sub, f_med(26), STEEL)
        return self

    def kicker(self, x, y, s, fill=YELLOW):
        self.text((x, y), s.upper(), f_bold(22), fill)
        return self

    # ---- boxes ----------------------------------------------------------
    def card(self, box, fill=CARD, edge=EDGE, r=8):
        self.d.rounded_rectangle(box, radius=r, fill=fill, outline=edge, width=1)
        return self

    def tag(self, x, y, s, fill=YELLOW, fg=(0x19,0x19,0x19), pad=7, font=None):
        font = font or f_bold(20)
        tw = self.d.textlength(s.upper(), font=font)
        self.d.rounded_rectangle([x, y, x + tw + pad*2, y + 30], radius=5, fill=fill)
        self.text((x + pad, y + 4), s.upper(), font, fg)
        return x + tw + pad*2

    # ---- flow -----------------------------------------------------------
    def node(self, cx, cy, im, label, detail=None, size=210, badge=None,
             tint=None, new=False):
        half = size // 2
        box = [cx - half, cy - half, cx + half, cy + half]
        self.card(box, fill=CARD_HI if new else CARD,
                  edge=YELLOW if new else EDGE, r=10)
        if new:
            self.d.rounded_rectangle(box, radius=10, outline=YELLOW, width=2)
        pad = 18
        fit = size - pad*2
        s = min(fit / im.width, fit / im.height, 1.0)
        sc = im.resize((max(1,int(im.width*s)), max(1,int(im.height*s))), Image.LANCZOS)
        self.im.alpha_composite(sc, (cx - sc.width//2, cy - sc.height//2 - 18))
        self.text((cx, cy + half - 52), label, f_demi(26), CREAM, anchor="ma")
        if detail:
            self.text((cx, cy + half - 25), detail, f_med(21), STEEL, anchor="ma")
        if badge:
            self.tag(cx - half + 10, cy - half + 10, badge, fill=tint or YELLOW)
        return self

    def linklabel(self, x0, x1, y, text, colour):
        """Centre a run's label on the gap, shrunk until it clears the cards
        either side — a label that reads over a machine is worse than a
        small one. The gap itself is only ~50 px, so a label is allowed to
        bleed into the gutter, and no further."""
        limit = (x1 - x0) + 150
        size = 21
        while size > 14 and self.d.textlength(text, font=f_med(size)) > limit:
            size -= 1
        self.text(((x0 + x1)//2, y), text, f_med(size), colour, anchor="ma")
        return self

    def belt(self, x0, x1, y, label=None, colour=STEEL):
        """A conveyor: one solid rail with a chevron head."""
        self.d.line([(x0, y), (x1 - 12, y)], fill=colour, width=4)
        self.d.polygon([(x1, y), (x1 - 15, y - 9), (x1 - 15, y + 9)], fill=colour)
        for x in range(x0 + 10, x1 - 22, 22):
            self.d.line([(x, y - 6), (x + 7, y), (x, y + 6)], fill=STEEL_DK, width=3)
        if label:
            self.linklabel(x0, x1, y - 40, label, colour)
        return self

    def pipe(self, x0, x1, y, fluid, label=None):
        """A pipe run: two rails with the fluid's own colour between them."""
        c = hexc(fluid) if isinstance(fluid, str) and fluid.startswith("#") else fluid
        self.d.line([(x0, y - 7), (x1 - 10, y - 7)], fill=STEEL_DK, width=3)
        self.d.line([(x0, y + 7), (x1 - 10, y + 7)], fill=STEEL_DK, width=3)
        self.d.line([(x0, y), (x1 - 14, y)], fill=c, width=8)
        self.d.polygon([(x1, y), (x1 - 16, y - 11), (x1 - 16, y + 11)], fill=c)
        if label:
            self.linklabel(x0, x1, y - 44, label, c)
        return self

    def vbelt(self, x, y0, y1, colour=STEEL):
        self.d.line([(x, y0), (x, y1 - 12)], fill=colour, width=4)
        self.d.polygon([(x, y1), (x - 9, y1 - 15), (x + 9, y1 - 15)], fill=colour)
        return self

    def elbow(self, x0, y0, x1, y1, colour, pipe=False):
        """An L from (x0,y0) across then down to (x1,y1)."""
        w = 8 if pipe else 4
        self.d.line([(x0, y0), (x1, y0)], fill=colour, width=w)
        self.d.line([(x1, y0), (x1, y1 - 14)], fill=colour, width=w)
        self.d.polygon([(x1, y1), (x1 - 10, y1 - 16), (x1 + 10, y1 - 16)], fill=colour)
        return self

    def save(self, path, scale=1.0):
        im = self.im
        if scale != 1.0:
            im = im.resize((int(im.width*scale), int(im.height*scale)), Image.LANCZOS)
        im.convert("RGB").save(path, quality=95)
        return path
