# Security policy

## Supported versions

Security fixes go into the newest release. Please update before reporting, and
check whether the problem is still there.

| Version | Supported |
| --- | --- |
| 1.23.x (latest release) | Yes |
| Anything older | No |

## Reporting a vulnerability

**Do not open a public issue for a security problem.** Use GitHub's private
reporting instead: on the repository's **Security** tab, choose **Report a
vulnerability**. Include what you found, the steps to reproduce it, the version and
platform, and what an attacker could do with it.

You can expect an acknowledgement within a week. We will keep you informed while it
is fixed and credit you in the release notes, unless you would rather not be named.

## What counts

FormSculpt runs on your own machine, so the things that matter most are the ways
something outside it could get in:

- **The engine service** binds to loopback (127.0.0.1, port 8260 from source).
  Anything that makes it reachable from another machine, or lets a web page you
  visit call it.
- **Agents over MCP** (`./formsculpt mcp` and the service's `/mcp` endpoint): an
  agent changing the scene without the person's yes in the window, or with no window
  open and no earlier permission.
- **The terminal dock and its agent programs** (Claude Code, Codex, Cursor,
  Antigravity): starting a shell or a program there from anywhere but the app's own
  window, a plain shell tab getting an agent token, or a tab's token still working
  after its tab has closed.
- **Plugins and scripts** (`plugins/`, `./formsculpt script`): a plugin or script
  running without the person choosing to run it.
- **Files**: opening or importing a project (`.fsc`), a mesh (OBJ, STL, PLY, glTF,
  3MF, FBX, USD) or an image in a way that crashes the engine, exhausts memory or
  writes outside where it should; and the sculpt library and its Trash moving,
  restoring or deleting anything outside the library folder.
- **Hand-offs** to other apps (the Blender add-on, the GoF folder and its Unreal and
  Unity receivers): writing outside the hand-off folder, or running anything on the
  other side.

Bugs that need someone to already control your machine, or to run a modified build
of FormSculpt, are ordinary bugs: please open an issue for those.
