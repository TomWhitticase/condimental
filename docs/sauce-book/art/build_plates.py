from plate import *
from cards import cicon, FLUID, W
from voxlib import Image
import models as M, machines as X

# ---------------------------------------------------------------- cover ----
def cover():
    p = Plate(W, 710, grid=True)
    # A plinth, not a floor: tiled floor art at this size reads as a quilt and
    # fights the bottles standing on it.
    SHELF = 470
    p.d.rectangle([60, SHELF, W - 60, SHELF + 72], fill=(0x33, 0x33, 0x2E))
    p.d.rectangle([60, SHELF, W - 60, SHELF + 4], fill=(0x40, 0x40, 0x39))
    p.d.line([(60, SHELF + 72), (W - 60, SHELF + 72)], fill=(0x24, 0x24, 0x20), width=3)
    p.title(80, 96, "The Sauce Book",
            "Nine sauces after BBQ — recipes, lines, machines and art. CONDIMENTAL, pre-alpha.")
    p.text((80, 196), "Seven of the nine are a data-file edit. Two buy a whole new axis.",
           f_med(28), CREAM)
    shelf = [("item_bottle", "Ketchup", "shipped", STEEL_DK),
             ("item_bottle_bbq", "BBQ", "shipped", STEEL_DK),
             (M.bottle_mustard, "Mustard", "01", GREEN),
             (M.bottle_honey_mustard, "Honey mustard", "02", GREEN),
             (M.bottle_mayo, "Mayonnaise", "03", YELLOW),
             (M.bottle_aioli, "Aioli", "04", GREEN),
             (M.bottle_hot_sauce, "Hot sauce", "05", YELLOW),
             (M.bottle_chilli_mayo, "Chilli mayo", "06", GREEN),
             (M.bottle_reserve, "House reserve", "07", GREEN),
             (M.bottle_tartare, "Tartare", "08", GREEN),
             (M.bottle_burger, "Burger sauce", "09", GREEN)]
    step = (W - 220) / (len(shelf) - 1)
    for i, (src, name, tag, col) in enumerate(shelf):
        cx = int(110 + i * step)
        im = game_icon(src, 150) if isinstance(src, str) else cicon(src, 150)
        im = im.crop(im.getbbox())   # icon() pads to a square and a padded bottle floats
        p.im.alpha_composite(im, (cx - im.width // 2, SHELF + 2 - im.height))
        p.text((cx, SHELF + 92), name, f_demi(20), CREAM, anchor="ma")
        p.text((cx, SHELF + 118), tag.upper(), f_bold(17), col, anchor="ma")
    p.d.line([(80, 636), (W - 80, 636)], fill=EDGE, width=1)
    p.text((80, 652), "Every sauce below is drawn in the game's own palette; "
                      "machine art is a concept, not a shipped asset.",
           f_med(22), STEEL_DK)
    return p.save("plate_cover.jpg")

# ----------------------------------------------------------------- tree ----
def tree():
    p = Plate(W, 1580)
    p.title(70, 48, "The sauce tree",
            "Every arrow is a sauce used as an ingredient. Unlock one branch and everything downstream comes with it.")
    N = 78
    nodes = {
      "ketchup":  (185, 280, "item_bottle", "Ketchup", "SHIPPED", STEEL_DK),
      "mustard":  (185, 640, M.bottle_mustard, "Wholegrain mustard", "01 · DATA", GREEN),
      "hot":      (185, 1000, M.bottle_hot_sauce, "Hot sauce", "05 · NEW MACHINE", YELLOW),
      "pickle":   (185, 1330, M.item_pickle, "Pickles", "FEEDSTOCK", BLUE),
      "bbq":      (620, 230, "item_bottle_bbq", "BBQ", "SHIPPED", STEEL_DK),
      "honey":    (620, 520, M.bottle_honey_mustard, "Honey mustard", "02 · DATA", GREEN),
      "mayo":     (620, 860, M.bottle_mayo, "Mayonnaise", "03 · NEW MACHINE", YELLOW),
      "aioli":    (1040, 640, M.bottle_aioli, "Garlic aioli", "04 · DATA", GREEN),
      "chilli":   (1040, 1000, M.bottle_chilli_mayo, "Chilli mayo", "06 · DATA", GREEN),
      "tartare":  (1040, 1330, M.bottle_tartare, "Tartare", "08 · DATA", GREEN),
      "reserve":  (1450, 420, M.bottle_reserve, "House reserve", "07 · DATA", GREEN),
      "burger":   (1450, 1165, M.bottle_burger, "Burger sauce", "09 · DATA", GREEN),
    }
    edges = [("ketchup","bbq","ketchup"), ("mustard","honey","mustard"),
             ("mustard","mayo","mustard"), ("mayo","aioli","mayonnaise"),
             ("mayo","chilli","mayonnaise"), ("hot","chilli","hot_sauce"),
             ("bbq","reserve","bbq"), ("mustard","reserve","mustard"),
             ("hot","reserve","hot_sauce"),
             ("pickle","tartare","pickle"), ("mayo","tartare","mayonnaise"),
             ("pickle","burger","pickle"), ("mayo","burger","mayonnaise"),
             ("ketchup","burger","ketchup")]
    # One vertical trunk per SOURCE, and a stacked entry per TARGET. With
    # fourteen edges a shared-midpoint router draws four lines down the same
    # column and nothing can be traced; a bus per sauce can be.
    srcs = list(dict.fromkeys(a for a, _, _ in edges))
    lane = {a: i for i, a in enumerate(srcs)}
    ins = {}
    for a, b, _ in edges:
        ins.setdefault(b, []).append(a)
    for k, (a, b, fl) in enumerate(edges):
        x0, y0 = nodes[a][0] + N + 6, nodes[a][1]
        x1 = nodes[b][0] - N - 18
        sibs = ins[b]
        y1 = nodes[b][1] + (sibs.index(a) - (len(sibs) - 1) / 2) * 17
        mid = nodes[a][0] + N + 34 + lane[a] * 26
        c = hexc(FLUID[fl])
        p.d.line([(x0, y0), (mid, y0), (mid, y1), (x1, y1)], fill=c, width=5, joint="curve")
        p.d.polygon([(x1 + 16, y1), (x1 - 4, y1 - 11), (x1 - 4, y1 + 11)], fill=c)
    for key, (cx, cy, src, name, tag, col) in nodes.items():
        p.d.ellipse([cx - N, cy - N, cx + N, cy + N], fill=CARD, outline=col, width=3)
        im = game_icon(src, 124) if isinstance(src, str) else cicon(src, 124)
        p.im.alpha_composite(im, (cx - im.width // 2, cy - im.height // 2 - 4))
        tw = max(p.d.textlength(name, font=f_demi(24)),
                 p.d.textlength(tag, font=f_bold(18))) // 2 + 14
        p.d.rounded_rectangle([cx - tw, cy + N + 4, cx + tw, cy + N + 64],
                              radius=8, fill=BG)
        p.text((cx, cy + N + 10), name, f_demi(24), CREAM, anchor="ma")
        p.text((cx, cy + N + 38), tag, f_bold(18), col, anchor="ma")
    y = 1490
    for x, lbl, col in ((70, "shipped today", STEEL_DK), (400, "a data-file edit", GREEN),
                        (760, "buys a new machine", YELLOW),
                        (1170, "feedstock, not a sauce", BLUE)):
        p.d.ellipse([x, y, x + 20, y + 20], fill=col)
        p.text((x + 32, y - 4), lbl, f_med(24), STEEL)
    return p.save("plate_tree.jpg")

# ------------------------------------------------------------- machines ----
def machines_plate():
    p = Plate(W, 1180)
    p.title(70, 48, "Four things to build",
            "Two sauces out of nine need new equipment. This is all of it.")
    specs = [
      (X.emulsifier, "Emulsifier", "1×2 · manufacturing 1 · 180",
       "Threads oil into yolk under a high-shear head. The dosing line is the "
       "tell: it is the only machine where you can SEE the slow step.",
       "Pure data + one model. No sim work.", GREEN, "new machine"),
      (X.chiller, "Chiller", "1×2 · manufacturing 1 · 150",
       "Sets mayo cold — and while stock sits in its buffer the spoil clock "
       "stops. The only white machine on the floor, so cold reads at a glance.",
       "Needs the perishable axis (below).", AMBER, "new machine"),
      (X.fermenter, "Fermenter", "2×2 · manufacturing 2 · 260",
       "3 000 ticks, the longest process in the game by 5×. Airlock bubbles on "
       "the lid are the motion frames; the hazard band says acid-rated.",
       "Pure data + one model. No sim work.", GREEN, "new machine"),
      (X.lined_pipe, "Lined pipe", "1×1 · no crew · 24",
       "Ordinary pipe corrodes on a corrosive fluid and eventually faults. "
       "Lined pipe never does, at 3× the price of plain pipe.",
       "Needs the corrosion axis (below).", AMBER, "new pipe"),
    ]
    for i, (fn, name, stats, blurb, cost, col, tag) in enumerate(specs):
        cx = 70 + (i % 2) * 762
        cy = 170 + (i // 2) * 490
        p.card([cx, cy, cx + 718, cy + 450], fill=CARD, r=12)
        im = cicon(fn, 300, tw=4)
        p.im.alpha_composite(im, (cx + 18, cy + 60))
        tx = cx + 330
        p.tag(cx + 18, cy + 16, tag, fill=YELLOW)
        p.text((tx, cy + 62), name, f_bold(34), CREAM)
        p.text((tx, cy + 106), stats, f_med(22), STEEL)
        p.text((tx, cy + 148), p.wrap(blurb, f_med(22), cx + 696 - tx),
               f_med(22), CREAM, spacing=9)
        p.d.line([(tx, cy + 372), (cx + 696, cy + 372)], fill=EDGE, width=1)
        p.text((tx, cy + 390), cost, f_demi(21), col)
    return p.save("plate_machines.jpg")

# ------------------------------------------------------------------ raws ----
def raws():
    p = Plate(W, 1090)
    p.title(70, 48, "Ten new raws",
            "Everything the nine sauces buy that the factory cannot make for itself.")
    items = [
      (M.item_mustard_seed, "Mustard seed", "2", "", None),
      (M.item_turmeric, "Turmeric", "4", "", None),
      (M.item_honey, "Honey", "4", "fluid", BLUE),
      (M.item_egg, "Egg", "3", "perishable", AMBER),
      (M.item_oil, "Oil", "3", "fluid", BLUE),
      (M.item_garlic, "Garlic", "2", "", None),
      (M.item_chilli, "Chilli", "2", "", None),
      (M.item_salt, "Salt", "1", "", None),
      (M.item_cucumber, "Cucumber", "2", "", None),
      (M.item_capers, "Capers", "6", "priciest", STEEL_DK),
    ]
    for i, (fn, name, price, tag, col) in enumerate(items):
        cx = 70 + (i % 4) * 380
        cy = 190 + (i // 4) * 270
        p.card([cx, cy, cx + 350, cy + 250], fill=CARD, r=12)
        im = cicon(fn, 150)
        p.im.alpha_composite(im, (cx + 24, cy + 46))
        p.text((cx + 190, cy + 72), name, f_demi(28), CREAM)
        p.text((cx + 190, cy + 110), f"{price} each", f_med(24), STEEL)
        if tag:
            # PRICIEST is a note on the price, not a rule the sim enforces the
            # way FLUID and PERISHABLE are, so it wears the quiet grey.
            p.tag(cx + 190, cy + 150, tag, fill=col,
                  fg=CREAM if col == STEEL_DK else (0x19, 0x19, 0x19))
    p.text((70, 1022), "Prices sit where the shipped ones do: a box of sauce costs "
                      "about a third of what a contract pays for it.", f_med(24), STEEL_DK)
    return p.save("plate_raws.jpg")

# -------------------------------------------------------------- variants ----
def variants():
    p = Plate(W, 700)
    p.title(70, 48, "The cheap seats",
            "Once the tree exists, a new sellable sauce is one interesting recipe, a bottle, a box and a colour.")
    v = [(M.MUSTARD, "Hot honey", "honey mustard + a hot sauce draw"),
         ("#8A4A2A", "Smoky BBQ", "BBQ + a second liquid-smoke draw"),
         (M.CHILLI_MAYO, "Chipotle mayo", "mayo + smoked chilli mash"),
         ("#8A5A2A", "Honey BBQ", "BBQ + a honey draw"),
         ("#B8462A", "Sriracha", "hot sauce + sugar + garlic"),
         (M.RESERVE_DARK, "Steakhouse", "reserve + a molasses finish")]
    for i, (col, name, how) in enumerate(v):
        cx = 70 + (i % 3) * 500
        cy = 200 + (i // 3) * 230
        p.card([cx, cy, cx + 470, cy + 200], fill=CARD, r=12)
        im = cicon(lambda c=col: M.bottle(c, "label_cream"), 130, key=col)
        p.im.alpha_composite(im, (cx + 20, cy + 34))
        p.text((cx + 166, cy + 62), name, f_demi(30), CREAM)
        p.text((cx + 166, cy + 104), how, f_med(22), STEEL)
    return p.save("plate_variants.jpg")

for f in (cover, tree, machines_plate, raws, variants):
    print(f())
