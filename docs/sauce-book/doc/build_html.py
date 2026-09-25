import base64, io, glob, os, re
from PIL import Image

ART = os.path.join(os.path.dirname(os.path.abspath(__file__)), "../art")
DOC = os.path.dirname(os.path.abspath(__file__))

def data_uri(name, maxw=1280, q=84):
    im = Image.open(f"{ART}/{name}.jpg").convert("RGB")
    if im.width > maxw:
        im = im.resize((maxw, round(im.height * maxw / im.width)), Image.LANCZOS)
    buf = io.BytesIO(); im.save(buf, "JPEG", quality=q, optimize=True)
    return "data:image/jpeg;base64," + base64.b64encode(buf.getvalue()).decode()

body = "".join(open(f"{DOC}/body_{i}.html").read() for i in (1, 2, 3, 4))

def sub(m):
    name = m.group(1)
    return f'<img src="{data_uri(name)}" width="640">'

body = re.sub(r"<p>IMG:([a-z0-9_]+)</p>", sub, body)

def code_block(m):
    """Docs ignores a stylesheet rule on <pre> — it applies its own handling,
    which loses the monospace and puts a paragraph break between every line.
    One paragraph of <br>-separated lines with the font set INLINE survives
    intact; leading spaces have to be &nbsp; or Docs collapses the indent."""
    lines = m.group(1).split("\n")
    out = []
    for ln in lines:
        stripped = ln.lstrip(" ")
        out.append("&nbsp;" * (len(ln) - len(stripped)) + stripped)
    return ('<p style="font-family:\'Courier New\',monospace;font-size:9pt;'
            'background:#f3f3f0">' + "<br>".join(out) + "</p>")

body = re.sub(r"<pre>(.*?)</pre>", code_block, body, flags=re.S)
assert "<pre>" not in body
assert "IMG:" not in body, "an image placeholder went unreplaced"

html = f"""<!doctype html><meta charset="utf-8">
<style>
body {{ font-family: Arial, Helvetica, sans-serif; font-size: 11pt; line-height: 1.45;
        color: #1f1f1f; max-width: 660px; margin: 40px auto; }}
h1 {{ font-size: 20pt; margin: 30px 0 6px; }}
h2 {{ font-size: 15pt; margin: 26px 0 4px; }}
h3 {{ font-size: 12pt; margin: 18px 0 2px; }}
h2 span {{ color: #9a6b00; font-size: 10pt; }}
pre {{ font-family: 'Courier New', monospace; font-size: 8.5pt; background: #f3f3f0;
       padding: 10px; white-space: pre-wrap; line-height: 1.3; }}
code {{ font-family: 'Courier New', monospace; font-size: 9.5pt; }}
img {{ display: block; margin: 10px 0; }}
table {{ border-collapse: collapse; width: 100%; font-size: 9.5pt; }}
td, th {{ border: 1px solid #c8c8c8; padding: 4px 7px; text-align: left; }}
th {{ background: #eeeeea; }}
</style>
{body}
"""
open(f"{DOC}/sauce_book.html", "w").write(html)
print(f"{len(html)/1e6:.2f} MB")
