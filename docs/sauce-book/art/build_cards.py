from cards import *
import models as M, machines as X

def g(n): return game_icon(n, 150)
def nm(fn): return cicon(fn, 150, tw=3)

CR, MX, VT, BO, PK, DK = (g("machine_crusher"), g("machine_mixer"),
                          g("machine_cooking_vat"), g("machine_bottler"),
                          g("machine_packer"), g("machine_dock"))

out = []

# ---------------------------------------------------------------- 01 mustard
out.append(line_card("card_01_mustard.jpg", "01", "Wholegrain Mustard",
  "Pure data — no new machine, no new mechanic. And it still changes how you plumb the factory.",
  [dict(im=CR, label="Crusher", detail="mill the seed · 70t", badge="shared"),
   dict(im=MX, label="Mixer", detail="grits + turmeric · 90t", badge="shared"),
   dict(im=VT, label="Cooking vat", detail="warm steep · 180t", badge="shared"),
   dict(im=BO, label="Bottler", detail="60t"),
   dict(im=PK, label="Packer", detail="40t"),
   dict(im=DK, label="Dock", detail="ships the box")],
  [("belt", "mustard grits"), ("pipe", "mustard mix", "mustard_mix"),
   ("pipe", "mustard", "mustard"), ("belt", "bottled"), ("belt", "boxed")],
  {0: [("hopper", M.item_mustard_seed, "Mustard seed", 3)],
   1: [("hopper", M.item_turmeric, "Turmeric", 1)],
   2: [("tank", "item_vinegar", "Vinegar", 4)]},
  M.bottle_mustard, M.box_mustard,
  ("18 per box in ingredients", "ketchup is 11, BBQ is 23", "contracts pay 48–56"),
  "Mustard drinks FOUR vinegar a batch\nwhere ketchup drinks two. One tank\ncannot feed both lines any more — so\nthe first thing this sauce teaches is\ntank throughput, and the splitter you\nnever needed until now.",
  "25 deliveries · releases seed and turmeric"))

# ----------------------------------------------------------- 02 honey mustard
out.append(line_card("card_02_honey_mustard.jpg", "02", "Honey Mustard",
  "Pure data. Three lines in recipes.ron — and the first sauce in the game that is never cooked.",
  [dict(im=VT, label="Mustard vat", detail="tap the run", badge="shared", tint=AMBER),
   dict(im=MX, label="Mixer", detail="cold blend · 110t"),
   dict(im=BO, label="Bottler", detail="60t"),
   dict(im=PK, label="Packer", detail="40t"),
   dict(im=DK, label="Dock", detail="ships the box")],
  [("pipe", "mustard, tapped before the bottler", "mustard"),
   ("pipe", "honey mustard", "honey_mustard"), ("belt", "bottled"), ("belt", "boxed")],
  {1: [("tank", M.item_honey, "Honey", 1)]},
  M.bottle_honey_mustard, lambda: M.carton(M.MUSTARD_DEEP),
  ("22 per box in ingredients", "18 of it is mustard you made", "contracts pay 58–64"),
  "Heat dulls honey, so this one never\ngoes near a vat — it taps the mustard\nrun and goes straight to a bottler.\nA derived sauce that skips the slowest\nmachine is free throughput, and that\nis a lesson worth teaching once.",
  "35 deliveries · releases honey"))

# ------------------------------------------------------------------- 03 mayo
out.append(line_card("card_03_mayonnaise.jpg", "03", "Mayonnaise",
  "The cold-chain tier. Two new machines, and the first ingredient in the game that can go off.",
  [dict(im=nm(X.emulsifier), label="Emulsifier", detail="thread the oil · 150t", new=True, badge="new"),
   dict(im=nm(X.chiller), label="Chiller", detail="set cold · 90t", new=True, badge="new"),
   dict(im=BO, label="Bottler", detail="60t"),
   dict(im=PK, label="Packer", detail="40t"),
   dict(im=DK, label="Dock", detail="ships the box")],
  [("pipe", "mayo base", "mayo_base"), ("pipe", "mayonnaise", "mayonnaise"),
   ("belt", "bottled"), ("belt", "boxed")],
  {0: [("hopper", M.item_egg, "Egg", 2), ("tank", M.item_oil, "Oil", 3),
       ("tap", M.bottle_mustard, "Mustard", 1)]},
  M.bottle_mayo, M.box_mayo,
  ("33 per box in ingredients", "18 of it is mustard you made", "contracts pay 92–104"),
  "Eggs spoil. A spoil clock runs on every\nbuffer that is not a chiller, so a long\nfeed belt now kills stock outright.\nFor the first time the thing beating you\nis your own floor plan — compact wins,\nsprawl rots.",
  "45 deliveries · releases egg and oil"))

# ------------------------------------------------------------------ 04 aioli
out.append(line_card("card_04_aioli.jpg", "04", "Garlic Aioli",
  "A variant, not a tier: one crusher recipe and one more emulsifier recipe on machines you own.",
  [dict(im=CR, label="Crusher", detail="pound to paste · 80t", badge="shared"),
   dict(im=nm(X.emulsifier), label="Emulsifier", detail="finish · 90t", badge="2nd recipe"),
   dict(im=BO, label="Bottler", detail="60t"),
   dict(im=PK, label="Packer", detail="40t"),
   dict(im=DK, label="Dock", detail="ships the box")],
  [("belt", "garlic paste"), ("pipe", "aioli", "aioli"),
   ("belt", "bottled"), ("belt", "boxed")],
  {0: [("hopper", M.item_garlic, "Garlic", 2), ("hopper", M.item_salt, "Salt", 1)],
   1: [("tap", M.bottle_mayo, "Mayonnaise", 1)]},
  M.bottle_aioli, lambda: M.carton(M.CHILLI_STEM),
  ("38 per box in ingredients", "33 of it is mayonnaise", "contracts pay 104–118"),
  "Aioli steals mayo exactly the way BBQ\nsteals ketchup — same tank, same\ncontention. What it adds is a board\nthat can ask for 40 mayo AND 20 aioli,\nwhich is a harder order than 60 of\neither, off one line.",
  "60 deliveries · releases garlic and salt"))

# -------------------------------------------------------------- 05 hot sauce
out.append(line_card("card_05_hot_sauce.jpg", "05", "Hot Sauce",
  "The acid tier. One hot sauce, and it is fermented: 3 000 ticks, and a factory that grows sideways.",
  [dict(im=CR, label="Crusher", detail="mash · 80t", badge="shared"),
   dict(im=nm(X.fermenter), label="Fermenter", detail="3 000t · 2×2", new=True, badge="new"),
   dict(im=VT, label="Cooking vat", detail="pasteurise · 120t", badge="shared"),
   dict(im=BO, label="Bottler", detail="60t"),
   dict(im=PK, label="Packer", detail="40t"),
   dict(im=DK, label="Dock", detail="ships the box")],
  [("belt", "chilli mash"), ("pipe", "mash · lined pipe", "mash"),
   ("pipe", "hot sauce", "hot_sauce"), ("belt", "bottled"), ("belt", "boxed")],
  {0: [("hopper", M.item_chilli, "Chilli", 3), ("hopper", M.item_salt, "Salt", 1)],
   2: [("tank", "item_vinegar", "Vinegar", 2)]},
  M.bottle_hot_sauce, M.box_hot_sauce,
  ("11 per box in ingredients", "the cheapest bill in the game", "contracts pay 66–74"),
  "One fermenter makes a unit every\n100 seconds. Nothing you build\ndownstream fixes that — the only\nanswer is a BANK of them, so for the\nfirst time the factory grows sideways\ninstead of longer.",
  "75 deliveries · releases chilli"))

# ------------------------------------------------------------- 06 chilli mayo
out.append(line_card("card_06_chilli_mayo.jpg", "06", "Chilli Mayo",
  "The cross-product. Two tiers you already paid for, meeting at one mixer for the price of a data edit.",
  [dict(im=MX, label="Mixer", detail="fold, don't whisk · 80t", badge="shared"),
   dict(im=BO, label="Bottler", detail="60t"),
   dict(im=PK, label="Packer", detail="40t"),
   dict(im=DK, label="Dock", detail="ships the box")],
  [("pipe", "chilli mayo", "chilli_mayo"), ("belt", "bottled"), ("belt", "boxed")],
  {0: [("tap", M.bottle_mayo, "Mayonnaise", 2), ("tap", M.bottle_hot_sauce, "Hot sauce", 1)]},
  M.bottle_chilli_mayo, lambda: M.carton(M.CHILLI_MAYO),
  ("77 per box in ingredients", "all of it sauce you made", "contracts pay 210–235"),
  "Nothing new is bought and nothing\nnew is built. Unlocking hot sauce\nquietly unlocked this too — which is\nthe whole argument for a sauce TREE\nrather than a sauce list: late capital\npays off across every branch at once.",
  "95 deliveries · contracts only"))

# ----------------------------------------------------------- 07 house reserve
out.append(line_card("card_07_house_reserve.jpg", "07", "House Reserve Steak Sauce",
  "Tier 7, and not one new machine. It is gated by your floor plan: three pipe runs into one vat.",
  [dict(im=VT, label="Cooking vat", detail="3 fluids in · 400t", badge="3 taps", tint=RED),
   dict(im=BO, label="Bottler", detail="wax dip · 60t"),
   dict(im=PK, label="Packer", detail="40t"),
   dict(im=DK, label="Dock", detail="ships the box")],
  [("pipe", "house reserve", "reserve"), ("belt", "bottled"), ("belt", "boxed")],
  {0: [("tap", "item_bottle_bbq", "BBQ", 1), ("tap", M.bottle_mustard, "Mustard", 1),
       ("tap", M.bottle_hot_sauce, "Hot sauce", 1),
       ("hopper", "item_molasses", "Molasses", 1)]},
  M.bottle_reserve, M.box_reserve,
  ("56 per box in ingredients", "three sauces you did not ship", "contracts pay 190–210"),
  "A 2×2 vat has eight perimeter tiles\nand the outlet eats two. Three runs\nplus a hopper have to fit in the six\nthat are left, without crossing. That\nis the whole puzzle — the payoff tier\nis a LAYOUT tier, and it costs nothing.",
  "150 deliveries · contracts only"))

# ------------------------------------------------------------- P  pickles
out.append(line_card("card_08a_pickles.jpg", "P", "Pickles",
  "Not a sauce. The feedstock 08 and 09 are both built on — and the second job for a machine you already own.",
  [dict(im=nm(X.fermenter), label="Fermenter", detail="brine · 600t · 2×2", badge="shared", tint=YELLOW),
   dict(im=CR, label="Crusher", detail="dice · 80t", badge="shared"),
   dict(im=MX, label="Mixer", detail="08 and 09 start here", badge="shared")],
  [("belt", "whole pickles"), ("belt", "diced pickle")],
  {0: [("hopper", M.item_cucumber, "Cucumber", 3), ("hopper", M.item_salt, "Salt", 1),
       ("tank", "item_vinegar", "Vinegar", 1)]},
  M.item_pickle, M.item_diced_pickle,
  ("9 per unit in ingredients", "one crate makes one pail", "no contract — this is feedstock"),
  "The fermenter you bought for one\nsauce now sits upstream of three. A\nbank of slow machines that only ever\nmade hot sauce becomes general\nplant, and the player who over-built\none is suddenly right, not stuck.",
  "arrives with 08 — you already own the vessel",
  out_kicker="off the crusher", fun_kicker="why it earns its place"))

# --------------------------------------------------------------- 08 tartare
out.append(line_card("card_08_tartare.jpg", "08", "Tartare Sauce",
  "The first sauce in the game that is not a flat colour. Bits, and a sixth recipe on the mixer.",
  [dict(im=MX, label="Mixer", detail="fold, don't whisk · 100t", badge="shared"),
   dict(im=BO, label="Bottler", detail="60t"),
   dict(im=PK, label="Packer", detail="40t"),
   dict(im=DK, label="Dock", detail="ships the box")],
  [("pipe", "tartare", "tartare"), ("belt", "bottled"), ("belt", "boxed")],
  {0: [("tap", M.bottle_mayo, "Mayonnaise", 2), ("belt", M.item_diced_pickle, "Diced pickle", 1),
       ("hopper", M.item_capers, "Capers", 1)]},
  M.bottle_tartare, M.box_tartare,
  ("81 per box in ingredients", "66 of it is mayonnaise", "contracts pay 225–250"),
  "The first sauce fed by a line you\nbuilt rather than a hopper you fill.\nThe pickle branch has to be RUNNING,\nnot just built, so a sauce depends on\na second line keeping pace with it\nfor the first time.",
  "170 deliveries · releases cucumber and capers"))

# ---------------------------------------------------------- 09 burger sauce
out.append(line_card("card_09_burger_sauce.jpg", "09", "Burger Sauce",
  "Mayonnaise, ketchup and pickles in one bottle — and the sauce you started with matters again.",
  [dict(im=MX, label="Mixer", detail="fold · 100t", badge="2 taps"),
   dict(im=BO, label="Bottler", detail="60t"),
   dict(im=PK, label="Packer", detail="40t"),
   dict(im=DK, label="Dock", detail="ships the box")],
  [("pipe", "burger sauce", "burger_sauce"), ("belt", "bottled"), ("belt", "boxed")],
  {0: [("tap", M.bottle_mayo, "Mayonnaise", 2), ("tap", "item_bottle", "Ketchup", 1),
       ("belt", M.item_diced_pickle, "Diced pickle", 1)]},
  M.bottle_burger, M.box_burger,
  ("86 per box in ingredients", "11 of it is ketchup", "contracts pay 235–260"),
  "The ketchup line never becomes dead\nweight. Nine sauces deep, the first\nthing the player ever built is still\nrunning — and is now feeding the\nmost valuable order on the board.\nThat is what a tree is FOR.",
  "195 deliveries · contracts only, no new raw"))

for o in out: print(o)
