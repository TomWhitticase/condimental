"""The per-sauce line cards."""
from plate import *
from voxlib import icon, Image
import models as M, machines as X

W = 1620
NODE = 180
MARGIN = 95

FLUID = {
    "vinegar": "#AD6B23", "ketchup": "#C4190E", "bbq": "#6B3A24",
    "mustard_mix": "#C9A63A", "mustard": "#E0A81B", "honey": "#C98A12",
    "honey_mustard": "#D2A02A", "oil": "#D9A93A", "mayo_base": "#EFE0BE",
    "mayonnaise": "#F2EAD4", "aioli": "#EFE6C8", "mash": "#A83320",
    "hot_sauce": "#D81F16", "chilli_mayo": "#F0A883", "reserve": "#8A5A34",
    "tartare": "#EDE7CE", "burger_sauce": "#E09A62", "pickle": "#6F8C3A",
}

_cache = {}
def cicon(fn, size=150, tw=6, key=None):
    # NB the key must include the closure: every lambda is named "<lambda>",
    # so keying on the name alone hands six different bottles the same render.
    k = (key or getattr(fn, "__name__", str(fn)), fn.__defaults__, size, tw)
    if k not in _cache:
        _cache[k] = icon(fn(), size, tw=tw)
    return _cache[k]

def item_im(spec, size=104):
    """'game:item_vinegar' for a shipped icon, or a concept model function."""
    if isinstance(spec, str):
        return game_icon(spec, size)
    return cicon(spec, size)

def feed(p, cx, y, entries):
    """The hoppers, tanks and taps standing at one stage of the line.

    A tap is a pipe off a sauce the factory already makes — the move BBQ
    invented, and the one every derived sauce below reuses.
    """
    ew = 132
    wid = max(236, len(entries) * ew + 30)
    arrow_x = cx
    cx = max(70 + wid // 2, min(W - 70 - wid // 2, cx))
    box = [cx - wid // 2, y, cx + wid // 2, y + 152]
    p.card(box, fill=CARD, edge=EDGE, r=10)
    x = cx - wid // 2 + (wid - len(entries) * ew) // 2
    fluid = None
    for (kind, src, name, qty) in entries:
        vessel = {"hopper": "hopper", "tank": "fluid_tank",
                  "tap": "pipe_fitting_machine", "belt": "belt_f0"}[kind]
        p.im.alpha_composite(game_icon(vessel, 60), (x + 4, y + 16))
        p.im.alpha_composite(item_im(src, 74), (x + 52, y + 12))
        p.text((x + ew // 2, y + 96), name, f_demi(20), CREAM, anchor="ma")
        note = {"tap": "  tapped", "belt": "  belted"}.get(kind, "")
        p.text((x + ew // 2, y + 120), f"×{qty}{note}",
               f_med(18), AMBER if kind in ("tap", "belt") else STEEL, anchor="ma")
        if kind == "tap":
            fluid = name
        x += ew
    col = STEEL_DK if fluid is None else hexc(FLUID.get(
        fluid.lower().replace(" ", "_"), "#9BA3AC"))
    p.d.line([(arrow_x, y + 152), (arrow_x, y + 188)], fill=col, width=5)
    p.d.polygon([(arrow_x, y + 202), (arrow_x - 10, y + 186), (arrow_x + 10, y + 186)],
                fill=col)

def line_card(path, num, title, sub, stages, links, feeds, out_bottle, out_box,
              econ, fun, gate, height=1120, out_kicker="off the packer",
              fun_kicker="the new toy"):
    n = len(stages)
    pitch = (W - 2 * MARGIN - NODE) / (n - 1)
    x0 = MARGIN + NODE // 2
    p = Plate(W, height)
    p.title(70, 48, f"{num} · {title}", sub)
    ROW = 510
    for i, st in enumerate(stages):
        cx = int(x0 + i * pitch)
        p.node(cx, ROW, st["im"], st["label"], st.get("detail"), size=NODE,
               badge=st.get("badge"), tint=st.get("tint"), new=st.get("new", False))
    for i, lk in enumerate(links):
        a = int(x0 + i * pitch) + NODE // 2 + 10
        b = int(x0 + (i + 1) * pitch) - NODE // 2 - 10
        if lk[0] == "pipe":
            p.pipe(a, b, ROW, FLUID[lk[2]], lk[1])
        else:
            p.belt(a, b, ROW, lk[1])
    for i, entries in feeds.items():
        feed(p, int(x0 + i * pitch), ROW - NODE // 2 - 238, entries)
    y = ROW + NODE // 2 + 68
    H = height - y - 60
    p.card([70, y, 800, y + H], fill=CARD, r=10)
    p.kicker(96, y + 22, out_kicker)
    p.im.alpha_composite(cicon(out_bottle, 200), (96, y + 66))
    p.im.alpha_composite(cicon(out_box, 200), (300, y + 82))
    p.text((520, y + 96), econ[0], f_demi(25), CREAM)
    p.text((520, y + 134), econ[1], f_med(22), STEEL)
    p.text((520, y + 168), econ[2], f_med(22), GREEN)
    p.card([830, y, W - 70, y + H], fill=CARD, r=10)
    p.kicker(856, y + 22, fun_kicker, GREEN)
    p.text((856, y + 56), fun, f_med(25), CREAM, spacing=12)
    p.d.line([(856, y + H - 70), (W - 96, y + H - 70)], fill=EDGE, width=1)
    p.kicker(856, y + H - 52, "gate")
    p.text((934, y + H - 54), gate, f_med(23), STEEL, anchor="la")
    return p.save(path)
