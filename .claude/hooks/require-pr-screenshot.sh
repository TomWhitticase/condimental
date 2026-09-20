#!/usr/bin/env python3
"""PreToolUse hook: refuse a `gh pr create`/`gh pr edit` whose body has no image.

docs/WORKFLOW.md, "Every pull request shows the change". CI enforces the rule
after the fact; this catches it before the PR exists, while the branch is still
in front of whoever is writing it.

The parsing earns its keep three times over.

Deciding whether the command really INVOKES gh means ignoring heredoc bodies
first — a command that merely writes the string into a doc is an edit, not a
pull request, and a plain substring match blocked exactly that. It also means
tokenising the rest with shlex rather than splitting the raw text on `;`/`&&`,
so `gh pr edit` quoted inside a `--body` or `--title` VALUE stays one word and
is never mistaken for a command. Writing about the rule used to trip the rule:
issue #50 was itself blocked, from a plain `gh issue create`, because its body
quoted the blocked command in a fenced block.

Deciding whether the body carries an image means searching the whole command
INCLUDING heredocs, because that is where a written-up body usually lives, and
OPENING any `--body-file` the way the shell would have (issue #53): the hook is
not a shell, so `--body-file $SCRATCH/body.md` used to be opened as the literal
nine characters `$SCRATCH`, fail, and count as "no image" — refusing a body
whose first line was the screenshot. `expand_body_path` below expands `$VARS`
and `~` and nothing else; a path that needs a subshell (`--body "$(cat x.md)"`)
is still unreadable here, and still refused.

Deciding about `gh pr edit` means looking at the PULL REQUEST, not just the
command. `gh pr edit 45 --add-label full-verify` carries no image because it
carries no body at all — it cannot remove the screenshot PR #45 already has.
For an edit with no body-bearing flag the hook therefore asks GitHub for the
body that is already there. That matters for `full-verify` in particular: the
changes that most deserve the Godot gate are the ones where you reach for the
label after the PR is open, and a hook that forces `gh api .../labels` instead
has routed people around itself.

Run its test with:  python3 .claude/hooks/test_require_pr_screenshot.py
"""

import json
import os
import re
import shlex
import subprocess
import sys

HEREDOC_START = re.compile(r"<<-?\s*[\"']?([A-Za-z_][A-Za-z0-9_]*)[\"']?")
IMAGE = re.compile(r"!\[[^\]]*\]\(\s*https?://|<img[^>]+src=")
BODY_FILE = re.compile(r"--body-file[=\s]+(\S+)")

# A flag that puts a body on the pull request. An edit carrying one of these
# rewrites the body, so the command itself has to show the image.
BODY_FLAGS = ("--body", "-b", "--body-file", "-F", "--fill", "--fill-first",
              "--fill-verbose")

# `gh pr edit` flags that swallow the word after them, so that word is not the
# PR selector. Only needed to find the selector when it is not the first
# argument; `gh pr edit 45 --add-label x` is the ordinary shape.
VALUE_FLAGS = ("--add-assignee", "--add-label", "--add-project",
               "--add-reviewer", "--base", "--body", "-b", "--body-file", "-F",
               "--milestone", "--remove-assignee", "--remove-label",
               "--remove-project", "--remove-reviewer", "--title", "-t",
               "--repo", "-R")

TIMEOUT_S = 10


def expand_body_path(raw):
    """A `--body-file` argument as the shell that ran it would have opened it.

    Deliberately the two bounded expansions and no more (issue #53):

    * `os.path.expandvars` over THIS PROCESS'S environment — which is the
      environment the command was about to be run in, since a PreToolUse hook
      is spawned from it. `$SCRATCH/body.md` therefore resolves to the same
      file `gh` would have opened. An undefined variable is left standing, so
      the path simply fails to open and the command is refused, as before.
    * `os.path.expanduser` for a leading `~`.

    No subshell, ever: `$(cat body.md)` is not a path, is never run, and a
    `--body` carrying one is still judged on the command text alone.

    The one over-read this accepts is a single-quoted `'$SCRATCH/body.md'`,
    which a shell would keep literal and this expands anyway. It cannot let an
    image-less PR through — the file `gh` then opens is the literal one, which
    does not exist, so no PR is created at all — and the alternative is
    re-implementing quote removal that `shlex` has already done elsewhere.
    """
    return os.path.expanduser(os.path.expandvars(raw.strip("'\"")))


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


def segments(command):
    """The command's words, split at every point a new command can start.

    shlex does the work that matters: a quoted argument stays a single token
    however many newlines or `&&`s are inside it. Newline is punctuation here
    rather than whitespace, so two commands on two lines are two segments.
    """
    text = strip_heredocs(command)
    try:
        lexer = shlex.shlex(text, posix=True, punctuation_chars=";|&()<>\n")
        lexer.whitespace = " \t\r"
        lexer.whitespace_split = True
        lexer.commenters = ""
        tokens = list(lexer)
    except ValueError:
        # Unbalanced quotes: no honest tokenisation exists. Fall back to the
        # blunt split, which over-detects rather than under-detects — a guard
        # that guesses should guess towards guarding.
        return [segment.split() for segment
                in re.split(r"\n|;|&&|\|\||\|", text)]

    out, current = [], []
    for token in tokens:
        if token and not token.strip(";|&()<>\n"):
            out.append(current)
            current = []
        else:
            current.append(token)
    out.append(current)
    return out


def invocations(command):
    """Every segment that really runs `gh pr create` / `gh pr edit`."""
    found = []
    for words in segments(command):
        while words and re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*=.*", words[0]):
            words = words[1:]  # leading env assignments
        if words[:3] in (["gh", "pr", "create"], ["gh", "pr", "edit"]):
            found.append(words)
    return found


def carries_a_body(words):
    return any(word == flag or word.startswith(flag + "=")
               for word in words for flag in BODY_FLAGS)


def view_argv(words):
    """`gh pr view` for the PR the given `gh pr edit` targets.

    The selector is optional: `gh pr edit --add-label x` edits the PR of the
    current branch, and `gh pr view` with no selector resolves it the same way.
    """
    argv = ["gh", "pr", "view"]
    skip = False
    for word in words[3:]:
        if skip:
            skip = False
            continue
        if word.startswith("-"):
            skip = word in VALUE_FLAGS
            continue
        argv.append(word)  # the first positional is the number/url/branch
        break
    for i, word in enumerate(words):
        if word in ("--repo", "-R") and i + 1 < len(words):
            argv += ["--repo", words[i + 1]]
        elif word.startswith("--repo="):
            argv += [word]
    return argv + ["--json", "body", "-q", ".body"]


def pr_body_has_image(words):
    """True / False / None, where None means we could not find out.

    Callers treat None as a pass. An edit with no body flag cannot REMOVE an
    image — the body it leaves behind is the body GitHub already has — so when
    the lookup fails (offline, not logged in, a selector we could not parse)
    the safe side is to let it through: the worst case is a label landing on a
    PR that CI's own body-embeds-an-image check fails anyway, while the other
    choice makes `--add-label` impossible whenever GitHub is unreachable.
    `gh pr create`, and any edit that rewrites the body, stay fail-closed:
    there the command IS the body, and nothing downstream sees it first.
    """
    try:
        done = subprocess.run(view_argv(words), capture_output=True, text=True,
                              timeout=TIMEOUT_S)
    except (OSError, subprocess.SubprocessError):
        return None
    if done.returncode != 0:
        return None
    return bool(IMAGE.search(done.stdout))


payload = json.load(sys.stdin)
command = payload.get("tool_input", {}).get("command", "")

commands = invocations(command)
if not commands:
    sys.exit(0)

text = command
for path in BODY_FILE.findall(strip_heredocs(command)):
    try:
        with open(expand_body_path(path)) as handle:
            text += "\n" + handle.read()
    except OSError:
        # A body file that genuinely is not there stays "no image": the
        # command is about to fail anyway, and a guard that guesses guesses
        # towards guarding.
        pass

if IMAGE.search(text):
    sys.exit(0)

# Nothing in the command shows an image. An edit that touches no body is still
# fine if the PR it targets already shows one.
body_preserving = [words for words in commands
                   if words[2] == "edit" and not carries_a_body(words)]
if len(body_preserving) == len(commands):
    if all(pr_body_has_image(words) is not False for words in body_preserving):
        sys.exit(0)
    sys.stderr.write("""Blocked: the pull request this edits embeds no image in its body, and
every PR in this repository shows the change (docs/WORKFLOW.md, "GitHub").

Fix the body first — `gh pr edit <n> --body "$(cat body.md)"` with the
markdown scripts/pr-screenshot.sh printed in it — then re-run this.
""")
    sys.exit(2)

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
