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
   - sends `SIGSTOP` and `SIGCONT` to the related pid
2. KWin script:
   - listens to KWin events and read-only values
   - sends window state to the Rust daemon via DBus
   - does not decide freeze or resume policy

## State Rules

- Treat `window.active == false` as minimized or inactive for the PoC.
- Aggregate state by pid, not by single window.
- If one window for a pid is active, that pid must not stay frozen.
- If all windows for a pid are inactive, that pid can be frozen.
- Never send `SIGCONT` unless app-nap previously sent `SIGSTOP` for that pid.

## Media Playback

- Use MPRIS over DBus to detect playing media.
- If the app mapped to a pid is playing media, do not freeze it while playback is active.
- When checking MPRIS players, map the player service name back to a Unix pid through DBus.

## Flow

Example target: `firefox`

1. User minimizes `firefox`.
2. KDE emits `activeChanged()` and the window `active` state becomes `false`.
3. The KWin script sends the window state to the daemon over DBus.
4. The Rust daemon updates its internal state.
5. If all windows for `firefox`'s pid are inactive and no media is playing, send `SIGSTOP`.
6. User unminimizes or focuses `firefox`.
7. KDE emits `activeChanged()` and the window `active` state becomes `true`.
8. The KWin script sends the updated state to the daemon over DBus.
9. The Rust daemon updates its internal state.
10. If at least one window for that pid is active and the pid was previously stopped by app-nap, send `SIGCONT`.

## Multiple Windows

- Handle apps with multiple windows, such as Chromium PWAs, by pid.
- Freeze only when every window for that pid is inactive.
- Resume as soon as any window for that pid becomes active again.
- A frozen window can still be unminimized; that event must still flow through KWin to the daemon.
