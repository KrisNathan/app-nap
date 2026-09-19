# Configuration

## Configuration Format

The configuration file is in TOML format.

## Configuration File Location

By default, app-nap reads the configuration file from `~/.config/app-nap/config.toml`.

If `$HOME` is not set, app-nap reads the configuration file from `./.config/app-nap/config.toml`, relative to the current working directory.

## CPU Load Polling

app-nap measures two values for each app on every poll:

- `usage`: the CPU usage of the app in seconds per second, summed over the systemd units of the app. A value of `1.0` means one fully used core. On a multicore CPU, the value can be greater than `1.0`.
- `throttle`: the fraction of the poll interval that the app spent throttled. app-nap computes it from `throttled_usec` in `cpu.stat`, using the most-throttled systemd unit of the app.

### Interval

The interval at which app-nap polls the CPU load.

#### Options

|Option|Default|Description|
|------|-------|-----------|
|`interval_ms`|`10000`|The interval between two CPU load polls, in milliseconds.|

#### Example

```toml
[cpu_load_polling]
interval_ms = 10000 # poll every 10 seconds
```

### Idle Thresholds

This section defines the thresholds for the transition to an idle state. The idle states are `background_idle` and `nap_idle`.

An app is idle when `usage` is less than `usage_thres` and `throttle` is less than `throttle_thres`.

The idle thresholds must be less than the busy thresholds:

- The idle `usage_thres` must be less than the busy `usage_thres`.
- The idle `throttle_thres` must be less than the busy `throttle_thres`.

app-nap rejects a configuration that breaks these rules.

#### Options

|Option|Default|Description|
|------|-------|-----------|
|`usage_thres`|`0.10`|The `usage` below which the app counts as idle.|
|`throttle_thres`|`0.01`|The `throttle` below which the app counts as idle.|
|`confirm_ticks`|`2`|The number of ticks required before the transition to idle takes effect. One poll is one tick.|

#### Example

```toml
[cpu_load_polling.idle]
# transition to idle when CPU usage is below 10% and process throttle is below 1%
usage_thres    = 0.10
throttle_thres = 0.01
confirm_ticks  = 2    # 1 poll = 1 tick
```

### Busy Thresholds

This section defines the thresholds for the transition to a busy state. The busy states are `background_busy` and `nap_busy`.

An app is busy when `usage` is greater than `usage_thres` or `throttle` is greater than `throttle_thres`.

The idle thresholds must be less than the busy thresholds, as described in [Idle Thresholds](#idle-thresholds).

#### Options

|Option|Default|Description|
|---|---|---|
|`usage_thres`|`0.20`|The `usage` above which the app counts as busy.|
|`throttle_thres`|`0.05`|The `throttle` above which the app counts as busy.|
|`confirm_ticks`|`1`|The number of ticks required before the transition to busy takes effect. One poll is one tick.|

#### Example

```toml
[cpu_load_polling.busy]
# transition to busy when CPU usage is above 20% or process throttle is above 5%
usage_thres    = 0.20
throttle_thres = 0.05
confirm_ticks  = 1 # we want apps to "wake up" quickly if they are supposed to be busy
```

### Policies

A policy is a profile that lists the actions to apply to an app. Each policy has its own list of actions. The run conditions of the policies are not configurable. For the run conditions, see the [model documentation](../model.md).

The 5 policies are `performance`, `background_busy`, `background_idle`, `nap_busy`, and `nap_idle`.

```toml
[policies.performance]
actions = []

[policies.background_busy]
actions = []

[policies.background_idle]
actions = []

[policies.nap_busy]
actions = []

[policies.nap_idle]
actions = []
```

The available actions are:

#### Ecore

Pins every process of the app to the E-cores (efficiency cores). On revert, app-nap restores all online cores. This action only works on Intel hybrid CPUs.

- `type`: `ecore`

```toml
actions = [
  { type = "ecore" },
]
```

#### SystemdFreeze

Freezes the systemd service of the app. A frozen app uses almost no CPU, but it can stop responding. If the user closes a frozen app, the app can take time to exit.

- `type`: `systemd-freeze`

#### SystemdCpuQuota

Sets the CPU quota for the systemd service of the app.

- `type`: `systemd-cpu-quota`
- `percent`: the CPU quota in percent of one core. For example, `10` limits the app to 10 percent of one core.

```toml
actions = [
  { type = "systemd-cpu-quota", percent = 10 },
]
```

#### SystemdCpuWeight

Sets the CPU weight for the systemd service of the app. The kernel scheduler treats the weight as a hint, not a limit.

- `type`: `systemd-cpu-weight`
- `weight`: the CPU time weight, from `1` to `10000`. The kernel default is `100`.

```toml
actions = [
  { type = "systemd-cpu-weight", weight = 50 },
]
```

On revert, app-nap sets the weight to `100`, the cgroup default.

## Defaults

The default configuration is in [example/config.toml](../example/config.toml). The file contains the default values and suggested policies.
