# app-nap

`app-nap` is a Linux PoC that freezes inactive apps on KDE Plasma 6.

## How It Works

- KWin script watches each window's `minimized` state.
- When a window becomes inactive or minimized, the KWin script sends that
  window's state and PID to the Rust daemon over session D-Bus.
- The daemon tracks all windows per PID, not per window. If any window for a
  PID is active, that app stays awake. If all windows are inactive and no media
  is playing, the app can nap.
- "Nap" is the power-saving action. The backend is selected in
  `~/.config/app-nap/app-nap.toml`:
  - `signal`: send `SIGSTOP` to freeze the process, then `SIGCONT` to resume
    it. Caveat: a minimized window usually has to be unminimized before it can
    be closed.
  - `systemd-cpu-quota`: find the app's user scope or service and set a low (5%)
    `CPUQuota` while it is napping, then clear the quota to resume.
  - `systemd-freeze`: find the app's user scope or service and freeze/thaw the
    unit with `systemctl freeze`/`systemctl thaw`. This suspends the whole unit
    cgroup; it only works on systems using the unified cgroup v2 hierarchy and
    when the app is in a freezable systemd unit (`.scope` or `.service`).
- If a minimized app becomes active again, the daemon restores it.

## Install

Install the release binary, user systemd service, config, and KWin script:

```bash
./scripts/install.sh
```

Enable the service:

```sh
systemctl --user enable app-nap.service
```

Lastly, enable the KWin Script in KDE Settings.

This creates:

- `~/.local/bin/app-nap`
- `~/.config/systemd/user/app-nap.service`
- `~/.config/app-nap/app-nap.toml` if missing
- KWin script package `appnap`

The service runs as a user service because the daemon listens on the session
D-Bus:

```bash
systemctl --user status app-nap.service
```

## Uninstall

Disable the service, remove installed files, remove the KWin script, and
disable the KWin plugin:

```bash
./scripts/uninstall.sh
```

The uninstall script removes the generated default config only if it was not
edited.

## Quick Test Run

Open a normal GUI app, find its pid, then minimize and restore it:

```bash
pgrep -a firefox
ps -o pid,stat,cmd -p <PID>
```

When the window is minimized, the process should move to stopped state (`T` in
`ps` output) only when using the `signal` backend. With the `systemd-freeze`
backend, check the unit status with `systemctl --user status <unit>`; with the
`systemd-cpu-quota` backend, the process stays runnable but is CPU-throttled.

## Direct Daemon Smoke Test

You can also test the Rust daemon without KWin by sending a D-Bus update
directly:

```bash
qdbus dev.appnap.AppNap /dev/appnap/AppNap dev.appnap.AppNap1.AddWindow test <PID>
qdbus dev.appnap.AppNap /dev/appnap/AppNap dev.appnap.AppNap1.MinimizedChanged test <PID> true
qdbus dev.appnap.AppNap /dev/appnap/AppNap dev.appnap.AppNap1.MinimizedChanged test <PID> false
```

Use any stable string for the window id. Replace `<PID>` with a real process id.

## Notes

- The KWin script lives in `kwin-appnap/`.
- The daemon listens on the session bus as `dev.appnap.AppNap`.
- The KWin package metadata follows the Plasma 6 KPackage layout.
