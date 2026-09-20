#!/usr/bin/env python3
"""Tests for require-pr-screenshot.sh — run it: python3 this_file.py

The hook is a guard, so the cases that matter come in pairs: every command it
must now let through is next to the one it must still refuse. Two of the four
are the bug (issue #50) and two are the rule it exists to enforce; a fix that
regresses either pair is worse than the bug.

`gh` is stubbed on PATH, so no test needs the network or a GitHub login.
"""

import json
import os
import subprocess
import sys
import tempfile
import textwrap
import unittest

HOOK = os.path.join(os.path.dirname(os.path.abspath(__file__)),
                    "require-pr-screenshot.sh")
IMAGE_MD = "![the vat](https://github.com/o/r/blob/pr-media/a.png?raw=true)"


def stub_gh(directory, stdout="", code=0):
    """A `gh` on PATH that prints what a PR body lookup would have printed."""
    path = os.path.join(directory, "gh")
    script = "\n".join([
        "#!/bin/sh",
        'printf "%s\\n" "$@" > "$(dirname "$0")/argv"',   # what the hook asked
        "cat <<'GH_BODY'",
        stdout,
        "GH_BODY",
        "exit " + str(code),
        "",
    ])
    with open(path, "w") as handle:
        handle.write(script)
    os.chmod(path, 0o755)
    return directory


class HookCase(unittest.TestCase):
    def run_hook(self, command, gh_stdout="", gh_code=0, path_has_gh=True,
                 env_extra=None):
        env = dict(os.environ)
        env.update(env_extra or {})
        with tempfile.TemporaryDirectory() as bin_dir:
            if path_has_gh:
                stub_gh(bin_dir, gh_stdout, gh_code)
                # The stub shadows the real gh; /usr/bin:/bin is only there so
                # the stub itself can run `cat`.
                env["PATH"] = os.pathsep.join([bin_dir, "/usr/bin", "/bin"])
            else:
                env["PATH"] = bin_dir  # an empty directory: no gh anywhere
            done = subprocess.run(
                [sys.executable, HOOK],
                input=json.dumps({"tool_name": "Bash",
                                  "tool_input": {"command": command}}),
                capture_output=True, text=True, env=env, timeout=30)
            argv = os.path.join(bin_dir, "argv")
            done.gh_argv = (open(argv).read().split("\n") if os.path.exists(argv)
                            else None)
        return done

    def assertAllowed(self, command, **kw):
        done = self.run_hook(command, **kw)
        self.assertEqual(done.returncode, 0,
                         "expected pass, got %d: %s" % (done.returncode, done.stderr))

    def assertBlocked(self, command, **kw):
        done = self.run_hook(command, **kw)
        self.assertEqual(done.returncode, 2,
                         "expected block, got %d: %s" % (done.returncode, done.stdout))
        self.assertIn("Blocked", done.stderr)


class TheGuardStillGuards(HookCase):
    """The rule the hook exists for. These must never start passing."""

    def test_create_without_an_image_is_blocked(self):
        self.assertBlocked('gh pr create --title "t" --body "no picture here"')

    def test_create_with_fill_is_blocked(self):
        self.assertBlocked("gh pr create --fill")

    def test_edit_rewriting_the_body_without_an_image_is_blocked(self):
        # Even though the PR's stored body HAS an image: this command replaces
        # it with one that does not.
        self.assertBlocked('gh pr edit 45 --body "now without the shot"',
                           gh_stdout=IMAGE_MD)

    def test_edit_with_body_file_without_an_image_is_blocked(self):
        with tempfile.TemporaryDirectory() as tmp:
            body = os.path.join(tmp, "body.md")
            with open(body, "w") as handle:
                handle.write("words only\n")
            self.assertBlocked("gh pr edit 45 --body-file %s" % body,
                               gh_stdout=IMAGE_MD)

    def test_create_with_an_image_passes(self):
        self.assertAllowed('gh pr create --body "## Screenshots\n\n%s"' % IMAGE_MD)

    def test_create_with_an_image_in_a_heredoc_passes(self):
        self.assertAllowed(textwrap.dedent("""\
            gh pr create --title "t" --body "$(cat <<'EOF'
            ## Screenshots

            %s
            EOF
            )"
            """) % IMAGE_MD)

    def test_a_second_command_on_its_own_line_is_still_seen(self):
        # shlex drops newlines as whitespace unless they are kept as
        # separators; if they are not, this segment reads as `git add -A` and
        # the create sails through unchecked.
        self.assertBlocked('git add -A\ngh pr create --body "nothing shown"')


class IssueFifty(HookCase):
    """The two misfires in issue #50, each proved by the command that hit it."""

    def test_label_only_edit_passes_when_the_pr_body_has_an_image(self):
        self.assertAllowed("gh pr edit 45 --add-label full-verify",
                           gh_stdout="line one\n\n" + IMAGE_MD)

    def test_label_only_edit_is_blocked_when_the_pr_body_has_none(self):
        self.assertBlocked("gh pr edit 45 --add-label full-verify",
                           gh_stdout="a body with no picture in it")

    def test_label_only_edit_of_the_current_branch_pr_is_resolved(self):
        self.assertAllowed("gh pr edit --add-label full-verify",
                           gh_stdout=IMAGE_MD)

    def test_the_lookup_asks_about_the_pr_the_edit_targets(self):
        done = self.run_hook(
            "gh pr edit --add-label full-verify --repo o/r 45",
            gh_stdout=IMAGE_MD)
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(done.gh_argv[:3], ["pr", "view", "45"])
        self.assertIn("--repo", done.gh_argv)
        self.assertIn("o/r", done.gh_argv)

    def test_writing_about_the_rule_does_not_trip_it(self):
        # Filing issue #50 itself: a plain `gh issue create` whose body quotes
        # the blocked command inside a fenced block.
        self.assertAllowed(textwrap.dedent("""\
            gh issue create --label deferred --title "hook inspects the command" --body 'The command

            ```
            gh pr edit 45 --add-label full-verify
            ```

            is blocked, and should not be.'
            """))

    def test_documenting_the_rule_in_a_file_does_not_trip_it(self):
        self.assertAllowed(textwrap.dedent("""\
            cat > docs/note.md <<'EOF'
            Land the PR with `gh pr create --body ...`; a body with no image
            is refused.
            EOF
            """))


class WhenGitHubCannotBeReached(HookCase):
    """Fail open, and only for an edit that cannot touch the body."""

    def test_a_failed_lookup_lets_a_body_preserving_edit_through(self):
        done = self.run_hook("gh pr edit 45 --add-label full-verify",
                             gh_stdout="gh: not authenticated", gh_code=1)
        self.assertEqual(done.returncode, 0, done.stderr)

    def test_no_gh_on_path_lets_a_body_preserving_edit_through(self):
        done = self.run_hook("gh pr edit 45 --add-label full-verify",
                             path_has_gh=False)
        self.assertEqual(done.returncode, 0, done.stderr)

    def test_a_failed_lookup_does_not_rescue_a_create(self):
        done = self.run_hook('gh pr create --body "no picture"',
                             gh_stdout="gh: not authenticated", gh_code=1)
        self.assertEqual(done.returncode, 2, done.stdout)


class IssueFiftyThree(HookCase):
    """A `--body-file` path the hook is not a shell for.

    The hook `open()`s the path it finds in the command text, so a variable or
    a `~` in it used to be read as those literal characters, fail, and count
    as "no image" — refusing a body that opens with the screenshot. The two
    halves are inseparable: expanding must rescue the paths a shell would have
    resolved WITHOUT rescuing a body that really has no image in it.
    """

    def write_body(self, directory, text):
        path = os.path.join(directory, "body.md")
        with open(path, "w") as handle:
            handle.write(text)
        return path

    def test_a_variable_in_the_path_is_expanded(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.write_body(tmp, "## Screenshots\n\n%s\n" % IMAGE_MD)
            self.assertAllowed("gh pr create --title t --body-file $SCRATCH/body.md",
                               env_extra={"SCRATCH": tmp})

    def test_a_tilde_in_the_path_is_expanded(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.write_body(tmp, "## Screenshots\n\n%s\n" % IMAGE_MD)
            self.assertAllowed("gh pr create --title t --body-file ~/body.md",
                               env_extra={"HOME": tmp})

    def test_an_expanded_path_with_no_image_is_still_blocked(self):
        # The half that must not regress: expansion finds the body, and the
        # body is what gets judged.
        with tempfile.TemporaryDirectory() as tmp:
            self.write_body(tmp, "A body with words in it and no picture.\n")
            self.assertBlocked("gh pr create --title t --body-file $SCRATCH/body.md",
                               env_extra={"SCRATCH": tmp})

    def test_a_genuinely_missing_body_file_is_blocked(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.assertBlocked(
                "gh pr create --title t --body-file %s/nowhere.md" % tmp)

    def test_an_undefined_variable_is_left_standing_and_blocked(self):
        # Nothing to expand to, so the path stays literal, fails to open, and
        # the command is refused exactly as it was before the fix.
        self.assertBlocked(
            "gh pr create --title t --body-file $NO_SUCH_VAR_HERE/body.md")

    def test_command_substitution_is_never_run(self):
        # `--body "$(cat body.md)"` is the other half of issue #53, and stays
        # refused: the hook reads the command text, and a subshell is not a
        # path it can open.
        with tempfile.TemporaryDirectory() as tmp:
            self.write_body(tmp, "## Screenshots\n\n%s\n" % IMAGE_MD)
            done = self.run_hook(
                'gh pr create --title t --body "$(cat %s/body.md)"' % tmp)
            self.assertEqual(done.returncode, 2, done.stdout)


class CommandsThatAreNotPullRequests(HookCase):
    def test_an_unrelated_command_passes(self):
        self.assertAllowed("git status && cargo test --workspace")

    def test_gh_pr_view_passes(self):
        self.assertAllowed("gh pr view 45 --json body")

    def test_a_quoted_invocation_in_a_title_does_not_count(self):
        self.assertAllowed("""gh issue comment 7 --body "run 'gh pr edit 9 --add-label x'" """)


if __name__ == "__main__":
    unittest.main(verbosity=2)
