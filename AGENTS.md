# AGENTS.md

## App Concept

The app is called "app-nap".

We're making a linux service that automatically freezes apps when they are minimized.
The intention is to save battery on linux laptops.

At the moment the focus is to implement a PoC for KDE Plasma 6.

## Reference Docs

KWin Scripting: https://develop.kde.org/docs/plasma/kwin/api/
Rust libraries: use context7 tool

## Service Architecture

1. Rust Daemon:
  - service that listens for DBUS messages from KWin script
  - maintains window.active and window.pid states and playingMedia state
  - sends SIGSTOP and SIGCONT to the related window.pid on certain events 
2. KWin script: listens to kwin events and readonly values and sends to Rust Daemon via DBUS

## Flow

As an example the app to be manipulated will be 'firefox'

1. User minimizes firefox
2. KDE sends activeChanged() and firefox's window.active is set to false
3. KWin script sends this information to daemon via dbus
4. Rust daemon updates its internal state
5. If all window related to firefox's pid is inactive (active = false) and no media is playing then send SIGSTOP to that pid
6. User unminimizes/focuses on firefox
7. KDE sends activeChanged() and firefox's window.active is set to true
8. KWin script sends this information to daemon via dbus
9. Rust daemon updates its internal state
10. If there is at least one window related to firefox's pid is active then send SIGCONT to that pid (avoid sending SIGCONT if it was never SIGSTOP'ed previously)
