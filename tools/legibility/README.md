# tools/legibility — the voxel-legibility instrument

A dev tool, not part of the game (same status as `tools/asset-browser/`).

This is the measurement that decided #40 and #57 (twice). It had been rebuilt
from scratch in a session scratchpad every time, and each rebuild got something
wrong before it got it right — see #109 for the two failures and
`docs/DECISIONS.md` (2026-09-20) for the rules. The code lives here so the next
pass measures with the instrument instead of re-deriving it.

## The rules this code exists to enforce

- **One mask, built once**, from the shipped subject, and applied unchanged to
  every subject and candidate after a silhouette-overlap alignment. The mask is
  the whole silhouette **with interior holes filled** — largest warm connected
  component (`r > g > b and r - b > 18`), then flood the complement from the
  window border (4-connectivity) and keep everything the flood cannot reach.
- **Every window's coordinates are printed with every statistic**, always. A
  number whose window was not logged cannot be checked and gets withdrawn (it
  happened, under review of PR #98).
- **A control must be the same subject kind as the subject.** Cases standing
  back to back in a packer queue are a different pixel population from one case
  alone on a belt.
- **1:1 crops.** Nothing is upscaled to judge it.

The method's own anchors: **1.2/255 = invisible**, **~26/255 = reads reliably**.

## Scripts

| file | what it does |
|---|---|
| `legibility.py` | the mask, the alignment, the stats, CIELAB by hand |
| `blobs.py` | finds isolated case-sized warm blobs and prints their coordinates — how you get the windows in the first place |
| `floor57.py` | the #57 measurement as re-run after the review of PR #98 |

`python3` here has PIL and no numpy, and these are written for that: a ~40x40
region is instant in plain Python.

## A caveat you must read before you compare two frames

Per #82 the screenshot harness is **not reproducible run to run**, so a frame
cited by a decision record cannot in general be re-rendered — cite the artefact
by its `pr-media` path, and measure an A/B pair from renders you made in the
same session.
