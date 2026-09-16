#!/usr/bin/env python3
"""PreToolUse hook: refuse a `gh pr create`/`gh pr edit` whose body has no image.

docs/WORKFLOW.md, "Every pull request shows the change". CI enforces the rule
after the fact; this catches it before the PR exists, while the branch is still
in front of whoever is writing it.

The parsing earns its keep twice over. Deciding whether the command really
INVOKES gh means ignoring heredoc bodies first — a command that merely writes
the string into a doc is an edit, not a pull request, and a plain substring
match blocked exactly that. Deciding whether the body carries an image means
searching the whole command INCLUDING heredocs, because that is where a
written-up body usually lives.
"""

import json
import re
import shlex
import sys

HEREDOC_START = re.compile(r"<<-?\s*[\"']?([A-Za-z_][A-Za-z0-9_]*)[\"']?")
IMAGE = re.compile(r"!\[[^\]]*\]\(\s*https?://|<img[^>]+src=")
BODY_FILE = re.compile(r"--body-file[=\s]+(\S+)")


def strip_heredocs(command):
    """Drop heredoc bodies, so only the shell's own words are left."""
    out, lines, i = [], command.split("\n"), 0
    while i < len(lines):
        line = lines[i]
        out.append(line)
        match = HEREDOC_START.search(line)
        i += 1
        if match:
            terminator = match.group(1)
            while i < len(lines) and lines[i].strip() != terminator:
                i += 1
            i += 1  # the terminator line itself
    return "\n".join(out)


def invokes_gh_pr(command):
    for segment in re.split(r"\n|;|&&|\|\||\|", strip_heredocs(command)):
        try:
            words = shlex.split(segment)
        except ValueError:
            words = segment.split()
        while words and re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*=.*", words[0]):
            words = words[1:]  # leading env assignments
        if words[:3] in (["gh", "pr", "create"], ["gh", "pr", "edit"]):
            return True
    return False


payload = json.load(sys.stdin)
command = payload.get("tool_input", {}).get("command", "")

if not invokes_gh_pr(command):
    sys.exit(0)

text = command
for path in BODY_FILE.findall(strip_heredocs(command)):
    try:
        with open(path.strip("'\"")) as handle:
            text += "\n" + handle.read()
    except OSError:
        pass

if IMAGE.search(text):
    sys.exit(0)

sys.stderr.write("""Blocked: this pull request body embeds no image, and every PR in this
repository shows the change (docs/WORKFLOW.md, "GitHub").

  1. scripts/screenshot-scene.sh <out.png> 1600x900 <the flag that shows it>
  2. Read the PNG - a shot nobody looked at has verified nothing.
  3. scripts/pr-screenshot.sh <out.png>   # prints the markdown to paste
  4. Put that markdown under '## Screenshots' in the body, with a line
     saying what it shows.

--fill cannot satisfy this: a commit message holds no image. Write the body.
""")
sys.exit(2)
