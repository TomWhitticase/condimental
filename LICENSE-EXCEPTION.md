# Additional permission under GNU GPL version 3 section 7

CONDIMENTAL is licensed under the GNU General Public License, version 3 only
([LICENSE](LICENSE)), with the following additional permission, granted
under section 7 of that licence by its copyright holders:

> If you modify this Program, or any covered work, by linking or combining it
> with the Steamworks SDK (or a modified version of it, including as bundled
> by GodotSteam), or with the proprietary software development kit of any
> other game distribution platform or game console (collectively, "Platform
> SDKs"), containing parts covered by the terms of those Platform SDKs'
> licences, the licensors of this Program grant you additional permission to
> convey the resulting work. Corresponding Source for a non-source form of
> such a combination shall not include the source code for the parts of the
> Platform SDKs used, but shall include the source code for the parts of this
> Program used as well as that of the covered work.

## Why it exists

Platform SDKs such as Valve's Steamworks SDK (achievements, overlay, cloud
saves) are proprietary, and plain GPL-3.0 forbids conveying a program linked
against them. This permission makes a Steam build with those features
lawful, while every line of CONDIMENTAL's own code — and of any build that
uses them — stays GPL-3.0 and published.

As section 7 allows, anyone conveying a modified version may remove this
permission from their version. Contributions to this repository are made
under GPL-3.0 **with** this permission (see
[CONTRIBUTING.md](CONTRIBUTING.md)), so it continues to cover the whole
program as it grows.
