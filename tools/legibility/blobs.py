"""Find isolated, case-sized warm blobs in a region — the candidate subjects.

Prints one line per blob: centroid, bounding box, pixel count and mean luma
over the blob itself, so every window this analysis uses can be quoted with
its coordinates (the review of PR #98: an unlogged window is not a
measurement).
"""
import sys
from legibility import load, luma

def warm(p):
    return p[0] > p[1] > p[2] and (p[0] - p[2]) > 18

def blobs(px, x0, y0, x1, y1, lo=90, hi=2000):
    pts = {(x, y) for y in range(y0, y1) for x in range(x0, x1) if warm(px[x, y])}
    seen, out = set(), []
    for p in pts:
        if p in seen:
            continue
        st, comp = [p], set()
        seen.add(p)
        while st:
            c = st.pop()
            comp.add(c)
            for d in ((1,0),(-1,0),(0,1),(0,-1),(1,1),(-1,-1),(1,-1),(-1,1)):
                n = (c[0] + d[0], c[1] + d[1])
                if n in pts and n not in seen:
                    seen.add(n)
                    st.append(n)
        if lo <= len(comp) <= hi:
            xs = [c[0] for c in comp]; ys = [c[1] for c in comp]
            mean = sum(luma(px[c[0], c[1]]) for c in comp) / len(comp)
            out.append((min(xs), min(ys), max(xs), max(ys), len(comp), mean))
    return sorted(out)

if __name__ == '__main__':
    path = sys.argv[1]
    x0, y0, x1, y1 = (int(v) for v in sys.argv[2:6])
    px = load(path)
    for (bx0, by0, bx1, by1, n, mean) in blobs(px, x0, y0, x1, y1):
        print('%-16s bbox=(%4d,%4d)-(%4d,%4d) w=%2d h=%2d px=%4d blobLuma=%6.2f' % (
            path, bx0, by0, bx1, by1, bx1 - bx0 + 1, by1 - by0 + 1, n, mean))
