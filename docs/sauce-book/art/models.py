"""Concept models for the proposed sauces, their raws and their machines.

Colour names that are not in palette.ron are written as hex and listed in the
doc as proposed palette additions — the shipped rule is that a generator only
ever names a palette colour, so every hex here is a colour the real generator
would have to append first.
"""
from voxlib import Model, icon, render, Image

# ---- proposed palette additions ------------------------------------------
MUSTARD_SEED = "#D6C083"   # pale brassica seed
MUSTARD      = "#E0A81B"   # finished mustard, turmeric-lifted
MUSTARD_DEEP = "#A9761A"   # deep enough to read apart from MUSTARD at icon size
TURMERIC     = "#D08A1E"
HONEY        = "#C98A12"
EGG_SHELL    = "#EFE3CC"
YOLK         = "#E8B32A"
OIL_GOLD     = "#D9A93A"
GARLIC_PAPER = "#EDE4D2"
GARLIC_SKIN  = "#CFC3A8"
CHILLI_RED   = "#D8231F"
CHILLI_STEM  = "#4B7A33"
SALT_WHITE   = "#F5F6F2"
SALT_SACK    = "#B9C0C4"
MASH_RED     = "#A83320"
MAYO_CREAM   = "#F2EAD4"
AIOLI_CREAM  = "#EFE6C8"
HOT_RED      = "#D81F16"
CHILLI_MAYO  = "#F0A883"
RESERVE_DARK = "#3A2016"
COLD_WHITE   = "#DDE6EA"
COLD_BLUE    = "#7FC8E8"
ACID_STRIPE  = "#C8D24A"
CUKE_DARK    = "#3E6B2E"   # cucumber skin
CUKE_PALE    = "#93B45E"   # the pale stripe down a cucumber
PICKLE_GREEN = "#6F8C3A"   # fermented, duller and yellower than raw
PICKLE_PALE  = "#A8B96A"   # cut face of a diced gherkin
BRINE        = "#D6D49A"
CAPER_GREEN  = "#556B33"
TARTARE      = "#EDE7CE"   # mayonnaise, greyer for the pickle in it
BURGER_PINK  = "#E09A62"   # mayo plus ketchup, the classic burger orange

# ---- shared primitives ---------------------------------------------------

def sack(body, grain, h=15, r=6.2):
    """A kraft sack with a rolled-open top and a heap of contents, the shape
    `item_sugar` already uses for a bulk solid. The belly is widest a third of
    the way up, the way a sack of anything heavy actually sits."""
    m = Model()
    for z in range(0, h):
        t = z / (h - 1)
        belly = 0.80 + 0.22 * (1 - ((t - 0.33) / 0.62) ** 2)
        m.cyl(8, 8, r * min(1.0, belly), z, z, body)
    m.cyl(8, 8, r * 0.90, h, h + 1, body)          # rolled cuff
    m.cyl(8, 8, r * 0.66, h + 1, h + 2, grain)     # heap of contents
    m.cyl(8, 8, r * 0.44, h + 3, h + 3, grain)
    m.cyl(8, 8, r * 0.22, h + 4, h + 4, grain)
    return m

def drum(body, window, cap="steel_light", h=17, r=5.6):
    """A steel drum with a sight window — how a bought fluid is drawn."""
    m = Model()
    m.cyl(8, 8, r, 0, h, body)
    m.cyl(8, 8, r + 0.4, 1, 2, cap)
    m.cyl(8, 8, r + 0.4, h - 2, h - 1, cap)
    for z in range(5, h - 4):                      # window band on the two seen faces
        m.cyl(8, 8, r + 0.3, z, z, window)
    m.cyl(8, 8, r * 0.55, h + 1, h + 2, cap)       # bung
    return m

def pail(contents, body="pail_grey", handle="steel_light"):
    m = Model()
    m.cyl(8, 8, 6.2, 0, 10, body)
    m.cyl(8, 8, 5.4, 10, 11, contents)
    m.box(2, 2, 7, 8, 11, 15, handle)
    m.box(13, 13, 7, 8, 11, 15, handle)
    m.box(3, 12, 7, 8, 15, 15, handle)
    return m

def bottle(sauce, label, slim=False, wax=None):
    m = Model()
    r, h = (4.4, 15) if slim else (5.5, 12)
    m.cyl(8, 8, r, 0, h, sauce)
    m.cyl(8, 8, r + 0.3, int(h * .42), int(h * .62), label)   # label band
    m.cyl(8, 8, r * 0.46, h + 1, h + 5, sauce)                # neck
    m.cyl(8, 8, r * 0.56, h + 6, h + 7, wax or "cap_white")   # cap
    return m

def fleck(m, base, bits, every=7):
    """Bits in the sauce. Tartare and burger sauce are the first sauces in the
    game that are not a flat colour, so the flecks are load-bearing art, not
    decoration — a hash scatter rather than a stride, which would band."""
    for k, c in list(m.v.items()):
        if c == base:
            x, y, z = k
            if (x * 73856093 ^ y * 19349663 ^ z * 83492791) % every == 0:
                m.v[k] = bits
    return m

def carton(stripe):
    m = Model()
    m.box(0, 15, 0, 15, 0, 13, "cardboard")
    m.box(0, 15, 0, 0, 2, 5, stripe)      # printed band, both seen faces
    m.box(15, 15, 0, 15, 2, 5, stripe)
    m.box(4, 11, 0, 0, 7, 11, "cardboard_dark")
    m.box(15, 15, 4, 11, 7, 11, "cardboard_dark")
    m.box(0, 15, 0, 15, 13, 13, "cardboard_dark")
    m.box(6, 9, 0, 15, 13, 13, "label_kraft")   # tape down the lid
    return m

# ---- raw ingredients -----------------------------------------------------

def item_mustard_seed():  return sack("sack_kraft", MUSTARD_SEED)
def item_salt():          return sack(SALT_SACK, SALT_WHITE, h=13, r=5.6)

def item_turmeric():
    """A pressed spice tin — turmeric is bought ground, so it arrives sealed."""
    m = Model()
    m.cyl(8, 8, 6.0, 0, 9, "brass")
    m.cyl(8, 8, 6.3, 3, 6, TURMERIC)
    m.cyl(8, 8, 6.2, 10, 11, "steel_light")
    m.cyl(8, 8, 4.2, 12, 12, TURMERIC)
    return m

def item_honey():  return drum("brass", HONEY, h=15, r=5.4)
def item_oil():
    m = drum("steel", OIL_GOLD, h=18, r=5.2)
    m.box(3, 3, 7, 8, 12, 17, "steel_light")     # carry handle
    m.box(3, 7, 7, 8, 17, 17, "steel_light")
    return m

def item_egg():
    """A moulded tray — eggs are the first thing that arrives needing cold."""
    m = Model()
    m.box(0, 15, 0, 15, 0, 4, "cardboard_dark")
    m.cut(2, 13, 2, 13, 3, 4)
    m.box(0, 15, 0, 15, 0, 1, "cardboard")
    for cx in (4.0, 11.5):
        for cy in (4.0, 11.5):
            m.ball(cx, cy, 5.2, 3.6, EGG_SHELL)
    return m

def item_garlic():
    m = Model()
    m.ball(8, 8, 6.5, 5.6, GARLIC_PAPER)
    m.cut(0, 15, 0, 15, 0, 2)
    m.cyl(8, 8, 5.7, 3, 3, GARLIC_SKIN)
    for dx, dy in ((-3, -3), (3, -3), (-3, 3), (3, 3)):   # clove seams
        m.ball(8 + dx, 8 + dy, 7.0, 2.2, GARLIC_SKIN)
    m.box(7, 8, 7, 8, 12, 15, "#B8A98A")
    return m

def item_chilli():
    """Three pods in a shallow crate — bulk produce, like the tomato."""
    m = Model()
    m.box(0, 15, 0, 15, 0, 2, "cardboard_dark")
    m.box(0, 15, 0, 0, 0, 4, "cardboard")
    m.box(15, 15, 0, 15, 0, 4, "cardboard")
    m.box(0, 0, 0, 15, 0, 4, "cardboard")
    m.box(0, 15, 15, 15, 0, 4, "cardboard")
    for (x0, y0, z0) in ((1, 3, 3), (2, 9, 3), (3, 6, 6)):
        for i in range(10):                      # a pod that tapers and curls
            w = 2 if i < 6 else 1
            m.box(x0 + i, x0 + i, y0 + (i // 3), y0 + w + (i // 3),
                  z0, z0 + (1 if i < 7 else 0), CHILLI_RED)
        m.box(x0 + 10, x0 + 12, y0 + 3, y0 + 3, z0, z0, CHILLI_STEM)
    return m

def item_cucumber():
    """A shallow crate of cucumbers — bulk produce, the same crate as chilli
    so the two read as a pair on the shop shelf."""
    m = Model()
    m.box(0, 15, 0, 15, 0, 2, "cardboard_dark")
    for x0, x1, y0, y1 in ((0, 15, 0, 0), (0, 15, 15, 15), (0, 0, 0, 15), (15, 15, 0, 15)):
        m.box(x0, x1, y0, y1, 0, 4, "cardboard")
    for (x0, y0, z0) in ((2, 3, 3), (2, 8, 3), (3, 5, 6)):
        m.box(x0, x0 + 10, y0, y0 + 3, z0, z0 + 2, CUKE_DARK)
        m.box(x0 + 1, x0 + 9, y0 + 1, y0 + 2, z0 + 3, z0 + 3, CUKE_DARK)
        m.box(x0 + 2, x0 + 8, y0 + 3, y0 + 3, z0 + 1, z0 + 2, CUKE_PALE)   # the stripe
    return m

def item_capers():
    """A squat brine jar. Capers are the one thing here bought by the jar —
    they never arrive loose — so the vessel is the whole read."""
    m = Model()
    m.cyl(8, 8, 5.2, 0, 10, BRINE)
    m.cyl(8, 8, 5.5, 3, 6, "label_cream")      # paper band
    m.cyl(8, 8, 5.5, 10, 11, "steel_light")    # rim
    m.cyl(8, 8, 4.8, 11, 12, CAPER_GREEN)      # the buds, heaped above it
    for (dx, dy) in ((-2, -1), (2, 1), (0, 2), (-1, -2), (2, -2)):
        m.ball(8 + dx, 8 + dy, 13, 1.6, CAPER_GREEN)
    m.cyl(8, 8, 3.0, 14, 14, CAPER_GREEN)
    return m

# ---- intermediates -------------------------------------------------------

def item_mustard_grits(): return pail(MUSTARD_SEED)
def item_chilli_mash():   return pail(MASH_RED)
def item_garlic_paste():  return pail(GARLIC_PAPER)
def item_yolk_pail():     return pail(YOLK)

def item_pickle():
    """Whole gherkins in a crate — the first SOLID the factory makes in its
    middle rather than buys, so it reads as objects, not as a slurry, and it
    reuses the produce crate the raws already use."""
    m = Model()
    m.box(0, 15, 0, 15, 0, 2, "cardboard_dark")
    for x0, x1, y0, y1 in ((0, 15, 0, 0), (0, 15, 15, 15), (0, 0, 0, 15), (15, 15, 0, 15)):
        m.box(x0, x1, y0, y1, 0, 4, "cardboard")
    for (x0, y0, z0) in ((3, 2, 3), (2, 8, 3), (4, 5, 6)):
        m.box(x0, x0 + 8, y0, y0 + 3, z0, z0 + 2, PICKLE_GREEN)
        m.box(x0 + 1, x0 + 7, y0 + 1, y0 + 2, z0 + 3, z0 + 3, PICKLE_GREEN)
        m.box(x0 + 8, x0 + 8, y0, y0 + 3, z0, z0 + 2, PICKLE_PALE)   # cut end
    return m

def item_diced_pickle():
    m = pail(PICKLE_GREEN)
    for (dx, dy) in ((-3, -3), (-1, 1), (2, -2), (3, 2), (0, -3), (-3, 2)):
        m.box(8 + dx, 9 + dx, 8 + dy, 9 + dy, 11, 11, PICKLE_PALE)
    return m

# ---- finished goods ------------------------------------------------------

def bottle_mustard():      return bottle(MUSTARD, "label_kraft")
def bottle_honey_mustard(): return bottle(MUSTARD_DEEP, "label_cream")
def bottle_mayo():         return bottle(MAYO_CREAM, "paint_blue")
def bottle_aioli():        return bottle(AIOLI_CREAM, CHILLI_STEM)
def bottle_hot_sauce():    return bottle(HOT_RED, "label_cream", slim=True)
def bottle_chilli_mayo():  return bottle(CHILLI_MAYO, "paint_red")
def bottle_tartare():
    return fleck(bottle(TARTARE, "paint_blue"), TARTARE, PICKLE_GREEN)
def bottle_burger():
    return fleck(bottle(BURGER_PINK, "label_kraft"), BURGER_PINK, PICKLE_GREEN, every=9)
def bottle_reserve():      return bottle(RESERVE_DARK, "brass", slim=True, wax="hazard_black")

def box_mustard():      return carton(MUSTARD)
def box_mayo():         return carton("paint_blue")
def box_hot_sauce():    return carton(HOT_RED)
def box_reserve():      return carton("brass")
def box_tartare():      return carton(PICKLE_PALE)
def box_burger():       return carton("#F3A24A")
