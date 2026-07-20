# app-nap

`app-nap` is a Linux daemon that saves power by napping inactive apps on KDE Plasma 6.

## How It Works

A KWin script watches each window's `active` and `minimized` state and forwards
window state plus the window's PID to the Rust daemon over session D-Bus. The
daemon, not the script, decides when to throttle or freeze.

The daemon groups windows by PID and resolves each PID's systemd cgroup(s). On
every state change it reconciles the PID into one of three tiers:

- **Performance**: at least one window is active.
- **Background**: no active window, but at least one window is unminimized.
- **Nap**: all windows are minimized.

Two signals keep an app awake regardless of window state:

- **Media playback**: detected via MPRIS over D-Bus. If the app is playing
  media, it stays in Background even when all windows are minimized.
- **Idle inhibition**: detected via KDE PowerDevil's `PolicyAgent` over D-Bus.
  Screen recording, streaming, or presentation inhibitors are a hard
  do-not-throttle signal. The inhibitor's app ID is matched against the app's
  cgroup path, not its process name.

Each tier runs every configured action against the app's cgroup(s). The daemon
only reverts a nap if it previously applied one, so it never resumes a process
it didn't freeze. Failed tier transitions are retried on the next window state
change, not automatically.

## Configuration

Configure tier actions in `~/.config/app-nap/app-nap.toml`. Active apps use the
performance tier. Inactive but unminimized apps use the background tier. Fully
minimized apps use the nap tier unless media playback or an idle inhibitor keeps
them awake.

```toml
[tiers.performance]
actions = [{ type = "systemd-cpu-weight", weight = 100 }]

[tiers.background]
actions = [{ type = "systemd-cpu-weight", weight = 1 }]

[tiers.nap]
actions = [
  { type = "systemd-cpu-quota", percent = 10 },
  { type = "ecore" },
]
```

Each `actions` array can contain multiple actions:

- `signal` sends `SIGSTOP` when applied and `SIGCONT` when reverted.
- `systemd-freeze` freezes and thaws the app's user unit.
- `systemd-cpu-quota` sets `CPUQuota` using its `percent` value.
- `systemd-cpu-weight` sets `CPUWeight` using its `weight` value.
- `ecore` pins the app to efficiency cores and restores all online cores when
  reverted. It requires a hybrid CPU that exposes `/sys/devices/cpu_atom/cpus`. (only for hybrid Intel CPUs: alder lake and newer)

See `example/app-nap.toml` for the complete example.

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

When the window is minimized, the configured nap-tier actions should take
effect. A `signal` action moves the process to stopped state (`T` in `ps`
output). For `systemd-freeze`, `systemd-cpu-quota`, and `systemd-cpu-weight`,
inspect the unit with `systemctl --user status <unit>`.

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
