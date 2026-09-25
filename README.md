# app-nap

`app-nap` is a Linux daemon that saves power by napping inactive apps on KDE Plasma 6.

## How It Works

Triggers:

- Window focus, minimize changes
- Powerdevil inhibition
- MPRIS media playback
- Cgroup cpu.stat `usage_usec` and `throttled_usec`

How those signals apply to policy decisions:

A tier (the wake level of an app) comes from the window signals. A load (the CPU state of an app) comes from the CPU counters. The daemon applies the most wakeful signal of each app, so one focused window keeps every unit of the app awake.

- Performance: a window has focus, or a PowerDevil inhibition names the app. The daemon stops CPU load polling. The policy is always `performance`.
- Background: a window is unminimized, or the app plays media over MPRIS. The load selects `background_busy` or `background_idle`. Media playback lands here, not in Performance, so a background player is throttled lightly by design.
- Nap: all windows are minimized. The load selects `nap_busy` or `nap_idle`. This tier applies the most aggressive actions.
- Busy and idle: the daemon reads `usage_usec` and `throttled_usec` from `cpu.stat` in the cgroup of the app. `usage` is the CPU time in seconds per second, and `throttle` is the fraction of the poll interval that the app spent throttled. The app is idle when both values are below the idle thresholds, and busy when either value is above the busy threshold. A candidate must repeat for `confirm_ticks` polls before the load flips.

## Installation

```sh
./scripts/install.sh
```

1. Builds the daemon
2. Installs the kwin script
3. Installs the daemon binary to `~/.local/bin/app-nap`
4. Installs the systemd service to `~/.local/share/systemd/user/app-nap.service`
5. Enables and starts the systemd service

## Uninstallation

```sh
./scripts/uninstall.sh
```

Cleanly reverts `install.sh`.

## Configuration

See [configuration.md](docs/configuration.md).
