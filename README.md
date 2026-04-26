# app-nap

`app-nap` is a Linux PoC that freezes inactive apps on KDE Plasma 6.

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
`ps` output). When the window is restored, it should continue.

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
