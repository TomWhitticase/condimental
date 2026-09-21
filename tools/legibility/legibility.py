"""#57: can a player tell boxed_ketchup from boxed_bbq where they mix?

The project's voxel-legibility method, with the mask done properly: ONE mask,
built ONCE from the SHIPPED ketchup case, applied unchanged to every subject
and every candidate after a silhouette-overlap alignment; 1:1 crops, nothing
upscaled to judge.

THE MASK IS THE CASE'S WHOLE SILHOUETTE, HOLES FILLED. A warm-pixel mask
(r>g>b) looks right and is wrong here: `label_cream` renders near-neutral
(241,239,234), so a warm mask drops the ketchup case's printed panels --
exactly the pixels under test -- and scores every candidate as if only its
dark ink existed. The kraft outline is warm and encloses the print on every
face the camera sees, so filling the interior holes of the warm component
recovers the full silhouette.

Two statistics per subject, because lightness alone is not the only way two
objects can differ: mean Rec.709 luma (0-255) and mean CIELAB (D65), from
which dL* / dC (hue+chroma only, lighting-invariant) / dE76.
"""
from PIL import Image
import math

def _f(t):
    return t ** (1/3) if t > 0.008856 else (7.787 * t + 16 / 116)

def lab(p):
    r, g, b = (c / 255 for c in p)
    r, g, b = ((c / 12.92) if c <= 0.04045 else (((c + 0.055) / 1.055) ** 2.4) for c in (r, g, b))
    x = (0.4124 * r + 0.3576 * g + 0.1805 * b) / 0.95047
    y = (0.2126 * r + 0.7152 * g + 0.0722 * b)
    z = (0.0193 * r + 0.1192 * g + 0.9505 * b) / 1.08883
    fx, fy, fz = _f(x), _f(y), _f(z)
    return (116 * fy - 16, 500 * (fx - fy), 200 * (fy - fz))

def luma(p):
    return 0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2]

def silhouette(px, ox, oy, win):
    warm = {(i, j) for j in range(win) for i in range(win)
            if (lambda q: q[0] > q[1] > q[2] and (q[0] - q[2]) > 18)(px[ox + i, oy + j])}
    seen, best = set(), set()
    for p in warm:
        if p in seen:
            continue
        st, comp = [p], set()
        seen.add(p)
        while st:
            c = st.pop()
            comp.add(c)
            for d in ((1,0),(-1,0),(0,1),(0,-1),(1,1),(-1,-1),(1,-1),(-1,1)):
                n = (c[0] + d[0], c[1] + d[1])
                if n in warm and n not in seen:
                    seen.add(n)
                    st.append(n)
        if len(comp) > len(best):
            best = comp
    rest = {(i, j) for j in range(win) for i in range(win)} - best
    st = [p for p in rest if p[0] in (0, win - 1) or p[1] in (0, win - 1)]
    outside = set(st)
    while st:
        c = st.pop()
        for d in ((1,0),(-1,0),(0,1),(0,-1)):
            n = (c[0] + d[0], c[1] + d[1])
            if n in rest and n not in outside:
                outside.add(n)
                st.append(n)
    return best | (rest - outside)

def align(px, mask, x, y, win, span=4):
    best, bo = -1e9, (0, 0)
    for dy in range(-span, span + 1):
        for dx in range(-span, span + 1):
            o = silhouette(px, x + dx, y + dy, win)
            s = len(mask & o) - 0.6 * len(o - mask)
            if s > best:
                best, bo = s, (dx, dy)
    return bo

def stats(px, mask, x, y):
    Y = L = A = B = 0.0
    for (i, j) in mask:
        p = px[x + i, y + j]
        Y += luma(p)
        l, a, b = lab(p)
        L += l; A += a; B += b
    n = len(mask)
    return (Y / n, L / n, A / n, B / n)

def dist(p, q, use_L=True):
    d = (p[2] - q[2]) ** 2 + (p[3] - q[3]) ** 2
    if use_L:
        d += (p[1] - q[1]) ** 2
    return math.sqrt(d)

def load(path):
    return Image.open(path).convert('RGB').load()
