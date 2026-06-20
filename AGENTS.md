# AGENTS.md

## App Concept

`app-nap` is a Linux service that freezes apps when they are minimized.
The goal is to save battery on Linux laptops.

Current focus: a PoC for KDE Plasma 6.

## Reference Docs

- KWin scripting: https://develop.kde.org/docs/plasma/kwin/api/ + web search
- Rust libraries: use Context7 tool

## Service Architecture

1. Rust daemon:
   - listens for DBus messages from the KWin script
   - maintains per-window `active` and `pid` state
   - maintains per-pid freeze state
   - tracks whether matching media is playing
   - applies the configured nap backend to the related pid (e.g. `SIGSTOP`/`SIGCONT`,
     systemd `CPUQuota`, or systemd `freeze`/`thaw`)
2. KWin script:
   - listens to KWin events and read-only values
   - sends window state to the Rust daemon via DBus
   - does not decide freeze or resume policy

## State Rules

- Treat `window.active == false` as minimized or inactive for the PoC.
- Aggregate state by pid, not by single window.
- If one window for a pid is active, that pid must not stay frozen.
- If all windows for a pid are inactive, that pid can be frozen.
- Never resume a pid unless app-nap previously napped that pid.

## Media Playback

- Use MPRIS over DBus to detect playing media.
- If the app mapped to a pid is playing media, do not freeze it while playback is active.
- When checking MPRIS players, map the player service name back to a Unix pid through DBus.

## Idle Inhibition

- Use KDE PowerDevil's `org.kde.Solid.PowerManagement.PolicyAgent` over DBus.
- `ListInhibitions` returns `aas` (a list of `[who, why]` string lists), not
  `a(ss)`. Only the `who` string is used.
- The `who` string is the app's desktop/Flatpak ID (e.g.
  `com.obsproject.Studio`, `firefox`), not the process name. It cannot be
  matched against `/proc/<pid>/comm` (which is truncated to 15 chars and
  doesn't carry the app ID for Flatpaks).
- Instead, match `who` as a substring of `/proc/<pid>/cgroup`. The systemd
  unit name in the cgroup path contains the app ID (e.g.
  `app-flatpak-com.obsproject.Studio-<id>.scope`).
- Idle inhibitors (screen recording, streaming, presentations, video calls)
  are a hard "do not throttle" signal: a pid with an active inhibitor is never
  frozen, and an already-frozen pid is thawed on the next reconcile.
- No global fallback flag; unmatched inhibitors are ignored.
- Detection is poll-based: `ListInhibitions` is queried during `reconcile_pid`,
  matching the `MprisMediaService` pattern. A frozen app will not thaw until the
  next KWin window event triggers a reconcile.


## Flow

Example target: `firefox`

1. User minimizes `firefox`.
2. KDE emits `activeChanged()` and the window `active` state becomes `false`.
3. The KWin script sends the window state to the daemon over DBus.
4. The Rust daemon updates its internal state.
5. If all windows for `firefox`'s pid are inactive and no media is playing, apply the nap action (e.g. `SIGSTOP`, freeze the unit, or throttle the unit).
6. User unminimizes or focuses `firefox`.
7. KDE emits `activeChanged()` and the window `active` state becomes `true`.
8. The KWin script sends the updated state to the daemon over DBus.
9. The Rust daemon updates its internal state.
10. If at least one window for that pid is active and the pid was previously napped by app-nap, apply the resume action (e.g. `SIGCONT` or thaw the unit).

## Multiple Windows

- Handle apps with multiple windows, such as Chromium PWAs, by pid.
- Freeze only when every window for that pid is inactive.
- Resume as soon as any window for that pid becomes active again.
- A frozen window can still be unminimized; that event must still flow through KWin to the daemon.

## Install and Uninstall

Installation should:

1. Check system dependencies
2. Build rust release binary
3. Install rust binary as service
4. Install KWin script using `kpackagetool6`

Uninstall should:

1. Cleanly revert every file, service, package entry, and config created by install.
2. Mirror install step-for-step where possible.
3. Leave user data and unrelated system state untouched.

Install and uninstall must stay symmetrical:

- Every new install action needs a matching uninstall cleanup in the same change.
- Any artifact created by install must be removed or disabled by uninstall.
- If install becomes idempotent, uninstall should be safe to rerun too.
