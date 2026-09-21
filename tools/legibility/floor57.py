"""Issue #57, re-measured after the review of PR #98.

ONE fixed filled-silhouette mask, built from the shipped ketchup case on the
shared exit belt in bbq_mixed.png, applied to every window below.  EVERY
window's coordinates are printed, both as authored and as aligned, because an
unreproducible measurement is not a measurement.

Three groups:
  A  the pairwise subjects: one ketchup and one BBQ case on the SAME shared
     exit belt in the SAME frame.  This is the claim.
  B  the six windows the PR called a "noise floor" -- boxed_ketchup standing
     BACK TO BACK in the packer queue.  A different pixel population from an
     isolated case on dark belt.  Printed so the number is traceable; the
     floor and the 2.4x ratio derived from it are WITHDRAWN.
  C  the same-subject-kind control: ONE single BBQ case on the shared exit
     belt, across 14 independent renders, at the belt positions and animation
     frames those renders happened to catch.
"""
import sys
from legibility import *

WIN = 26
MASK = silhouette(load('bbq_mixed.png'), 964, 561, WIN)
print('mask: %d px in a %dx%d window, built once from the ketchup case on the'
      ' shared belt in bbq_mixed.png at (964,561)\n' % (len(MASK), WIN, WIN))

def row(label, path, x, y, px=None):
    px = px or load(path)
    dx, dy = align(px, MASK, x, y, WIN)
    st = stats(px, MASK, x + dx, y + dy)
    print('  %-34s %-12s window (%4d,%4d)-(%4d,%4d)  luma %6.2f  L* %5.1f a* %5.1f b* %5.1f'
          % (label, path, x + dx, y + dy, x + dx + WIN - 1, y + dy + WIN - 1,
             st[0], st[1], st[2], st[3]))
    return st[0]

print('A  THE PAIRWISE CLAIM -- two cases, one frame, one belt')
ket = row('boxed_ketchup, shared belt', 'bbq_mixed.png', 964, 561)
bbq = row('boxed_bbq, shared belt', 'bbq_mixed.png', 1023, 537)
print('   gap = %.2f/255\n' % abs(ket - bbq))

print('B  THE WITHDRAWN "NOISE FLOOR" -- packer-queue windows, back-to-back cases')
q = [row('boxed_ketchup, packer queue', 'bbq_mixed.png', x, y)
     for x, y in [(663,330),(676,337),(688,315),(726,337),(751,380),(776,366)]]
print('   spread = %.2f/255  <- NOT a floor for an isolated case; withdrawn\n'
      % (max(q) - min(q)))

print('C  SAME-SUBJECT-KIND CONTROL -- one single boxed_bbq on the shared belt,'
      ' 14 renders')
c = [row('boxed_bbq, shared belt', p, 964, 561)
     for p in ['nh_f%d.png' % i for i in range(1, 7)]
        + ['nh_g%d.png' % i for i in range(1, 9)]]
print('   spread = %.2f/255 over %d renders (min %.2f, max %.2f)'
      % (max(c) - min(c), len(c), min(c), max(c)))
