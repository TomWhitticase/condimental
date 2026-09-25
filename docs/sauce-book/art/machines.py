"""Concept models for the machines the proposed sauces would need.

Built to the shipped rules so a generator could be written straight off them:
one tile = 32 voxels; only a tile's +X and +Y faces are ever seen, so the
outlet is a doorway carved through +X and the inlet is a hooded housing cut
open on the camera-facing side; food contact is stainless, structure is
painted, emissive is status lamps only.
"""
from voxlib import Model
from models import COLD_WHITE, COLD_BLUE, ACID_STRIPE, MASH_RED, OIL_GOLD, MAYO_CREAM

T = 32

def skirt(m, w, d, top=4, paint="paint_blue"):
    m.box(1, w - 2, 1, d - 2, 0, top, paint)
    m.box(0, w - 1, 0, d - 1, 0, 1, "concrete_dark")
    return m

def panel(m, x, y, z, lamp="status_amber"):
    """The control panel every machine carries, on a bright plate."""
    m.box(x, x + 1, y, y + 6, z, z + 9, "steel_light")
    m.box(x + 2, x + 2, y + 1, y + 5, z + 1, z + 5, "hazard_black")
    m.box(x + 2, x + 2, y + 2, y + 3, z + 7, z + 8, lamp)
    return m

def outlet_door(m, w, d, lane_y, z0, z1):
    """A doorway carved clean through the +X wall — read by value, not hue."""
    m.cut(w - 4, w - 1, lane_y, lane_y + 7, z0, z1)
    m.box(w - 4, w - 1, lane_y - 1, lane_y - 1, z0, z1 + 1, "steel_dark")
    m.box(w - 4, w - 1, lane_y + 8, lane_y + 8, z0, z1 + 1, "steel_dark")
    return m

def inlet_hood(m, x0, y0, z, w=10, h=9):
    m.box(x0, x0 + w, y0, y0 + 5, z, z + h, "steel_dark")
    m.cut(x0 + 1, x0 + w - 1, y0, y0 + 2, z + 2, z + h - 2)
    m.box(x0, x0 + w, y0, y0 + 5, z + h, z + h, "steel")
    return m

def emulsifier():
    """1x2. A shear head on a stainless vessel, with the oil dosing line
    arched over the top — the slow drizzle made visible from across the floor."""
    w, d = T, T * 2
    m = Model()
    skirt(m, w, d, 5, "paint_blue")
    m.box(3, w - 4, 4, d - 5, 5, 8, "steel_dark")
    m.cyl(15.5, 20, 11.5, 8, 30, "steel")                 # the vessel
    m.cyl(15.5, 20, 12.0, 27, 28, "steel_light")
    m.cyl(15.5, 20, 12.0, 9, 10, "steel_light")
    m.cyl(15.5, 20, 6.0, 31, 34, "steel_dark")            # shear-head housing
    m.cyl(15.5, 20, 2.2, 35, 41, "steel_light")           # drive shaft
    m.cyl(15.5, 20, 5.0, 42, 44, "paint_blue")            # motor
    m.cyl(15.5, 20, 4.0, 45, 46, "steel_dark")
    for z in range(16, 24):                               # sight window
        m.cyl(15.5, 20, 12.2, z, z, MAYO_CREAM)
    # the oil dosing line: up the far corner, over the lid, down into the head.
    # Kept to the -X/-Y side so it never crosses the vessel's sight window.
    m.cyl(5, 8, 1.8, 6, 40, "brass")
    for x in range(5, 16):
        m.box(x, x, 7, 8, 40, 41, "brass")
    for y in range(9, 20):
        m.box(15, 16, y, y, 40, 41, "brass")
    m.box(15, 16, 19, 20, 36, 41, "brass")
    m.cyl(5, 8, 2.8, 4, 6, "steel_dark")                  # oil port, tank side
    inlet_hood(m, 6, d - 7, 9)                            # eggs, on the long side
    outlet_door(m, w, d, 8, 6, 14)
    panel(m, w - 3, 6, 8)
    return m

def chiller():
    """1x2. Insulated panel box: the only white machine on the floor, so cold
    reads at a glance. Compressor deck on the roof, frost window on the face."""
    w, d = T, T * 2
    m = Model()
    skirt(m, w, d, 3, "steel_dark")
    m.box(2, w - 3, 3, d - 4, 3, 30, COLD_WHITE)
    m.box(2, w - 3, 3, d - 4, 30, 32, "steel_light")      # capping rail
    for z in (10, 20):                                    # panel seams
        m.box(2, w - 3, 3, d - 4, z, z, "steel_dark")
    # Frosted window on the +Y face and wrapping the +X corner — the two
    # faces this camera ever sees.
    m.box(6, w - 3, d - 4, d - 4, 12, 26, COLD_BLUE)
    m.box(w - 3, w - 3, d - 16, d - 4, 12, 26, COLD_BLUE)
    for z in (11, 27):
        m.box(6, w - 3, d - 4, d - 4, z, z, "steel_light")
        m.box(w - 3, w - 3, d - 16, d - 4, z, z, "steel_light")
    m.box(6, w - 7, 6, d - 7, 32, 38, "paint_grey")       # compressor deck
    for x in range(7, w - 7, 3):                          # condenser fins
        m.box(x, x, 6, d - 7, 38, 41, "steel_dark")
    m.box(6, w - 7, 6, d - 7, 41, 42, "paint_grey")
    m.box(w - 3, w - 3, 6, 9, 20, 23, "status_green")
    outlet_door(m, w, d, 8, 6, 14)
    inlet_hood(m, 8, d - 7, 8)
    return m

def fermenter():
    """2x2. The long one: a crock under a domed lid with a brass airlock that
    is the only moving part, and a hazard band saying acid-rated."""
    w = d = T * 2
    m = Model()
    skirt(m, w, d, 6, "paint_grey")
    m.box(0, w - 1, 0, d - 1, 5, 7, "hazard_yellow")      # acid-rated band
    for x in range(2, w - 2, 8):
        m.box(x, x + 3, 0, 0, 5, 7, "hazard_black")
        m.box(w - 1, w - 1, x, x + 3, 5, 7, "hazard_black")
    m.cyl(31.5, 31.5, 26.0, 7, 44, "steel")               # the vessel
    m.cyl(31.5, 31.5, 26.6, 9, 10, "steel_light")
    m.cyl(31.5, 31.5, 26.6, 41, 42, "steel_light")
    for z in range(20, 32):                               # mash window
        m.cyl(31.5, 31.5, 26.8, z, z, MASH_RED)
    m.cyl(31.5, 31.5, 24.0, 45, 47, "steel_light")        # domed lid
    m.cyl(31.5, 31.5, 19.0, 48, 49, "steel_light")
    m.cyl(31.5, 31.5, 12.0, 50, 51, "steel_dark")
    m.cyl(31.5, 31.5, 4.0, 52, 58, "brass")               # airlock stem
    m.ball(31.5, 31.5, 61, 5.5, "brass")                  # bubbler bowl
    m.cyl(31.5, 31.5, 2.0, 64, 66, "steel_light")
    panel(m, w - 3, 10, 12)
    outlet_door(m, w, d, 12, 8, 16)
    inlet_hood(m, 10, d - 7, 10, w=12, h=11)
    return m

def lined_pipe():
    """1x1. The ordinary pipe with an acid-rated stripe and a lined bore, so a
    hot-sauce run is legible as a different run from across the floor."""
    m = Model()
    m.box(2, T - 3, 12, 19, 0, 2, "concrete_dark")
    m.cyl(15.5, 15.5, 7.0, 4, 4, None)
    for x in range(0, T):
        m.box(x, x, 9, 22, 4, 17, "pipe_grey")
    m.cut(0, T - 1, 11, 20, 6, 15)
    for x in range(0, T):
        m.box(x, x, 11, 20, 6, 15, ACID_STRIPE if 12 <= x <= 19 else None)
    for x in (12, 19):
        m.box(x, x, 9, 22, 4, 17, ACID_STRIPE)
    m.box(2, 2, 9, 22, 4, 17, "steel_light")
    m.box(T - 3, T - 3, 9, 22, 4, 17, "steel_light")
    return m
