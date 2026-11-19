# Architecture

This document covers: the model, the voting mechanism, and the policy run conditions.

## Overview

app-nap has two parts:

- KWin script (`kwin-appnap/`): runs inside KWin. Watches window events and forwards them to the daemon over D-Bus. The script only forwards normal windows that have a usable PID.
- Rust daemon (`src/`): holds the state, computes the policies, and applies the actions.

The script only observes. All decisions happen in the daemon.

```text
KWin script ──D-Bus──▶ server ──┐
watchers ──────D-Bus──▶         ├─▶ EventLoop ──▶ Daemon ──▶ reconcile ──▶ PolicyRouter ──▶ actions
                    cpu tick ───┘                    │
                                              cpu.stat (cgroup)
```

The daemon splits into parts:

- `dbus/server.rs`: the D-Bus endpoint. Validates calls and turns them into `ChannelEvent`s.
- `daemon/event_loop.rs`: the single loop. Reads `ChannelEvent`s and the CPU tick.
- `daemon/mod.rs` (`Daemon`): the state and the event handlers.
- `daemon/models/`: the domain types. `WindowGroup`, `ManagedUnit`, `Tier`, `Load`, `Policy`, `WakeSignals`, `CpuSample`.
- `daemon/policy_router.rs`: turns a policy into its configured actions. Applies and reverts them.
- `cgroup/`: resolves the systemd units of a PID and reads `cpu.stat`.
- `dbus/client/`: D-Bus proxies for systemd, PowerDevil, and MPRIS.
- `inhibitor/`: the wake-signal watchers. Watches PowerDevil inhibitions and MPRIS playback.
- `actions/`: the enforcement actions. E-core affinity, systemd freeze, CPU quota, CPU weight.
- `config/`: loads and validates the TOML configuration.

## Model

The daemon manages two entities:

- Window group (`daemon/models/window_group.rs`): the tracked windows of one PID, as reported by KWin. Holds the windows, the related units, the load, the tier, and the policy vote.
- Managed unit (`daemon/models/managed_unit.rs`): one systemd unit. Holds the voters and the applied policy. A voter is a window group that links to the unit.

```text
Window A ─┐
Window B ─┴─▶ WindowGroup(pid 100) ─┬─▶ ManagedUnit X
                                    └─▶ ManagedUnit Y
Window C ───▶ WindowGroup(pid 200) ───▶ ManagedUnit Y
```

A window group links to units through the cgroup resolver (`cgroup/resolve.rs`). The resolver walks the ancestors of the PID until systemd and collects the app unit of each cgroup.

Rules of the model:

1. A window belongs to the window group of its reported PID.
2. A window group exists only while it has tracked windows.
3. A window group links to one or more managed units.
4. A window group votes for one policy.
5. A managed unit collects the votes of all linked window groups.
6. Actions apply to managed units, not to window groups or single processes.
7. The managed unit records the applied policy.
8. When the last window of a group closes, the daemon removes the group and its votes.
9. When the last voter leaves a managed unit, the daemon reverts the applied policy and removes the unit.
10. The daemon does not manage processes that have no tracked windows.

## Voting Mechanism

Each window group votes for one policy. The vote has two inputs: the tier and the load.

### Tier

A wake signal is an outside hint that the app must stay awake. Two wake signals exist: a PowerDevil inhibition and MPRIS media playback.

The first matching row wins:

| Condition | Tier |
|---|---|
| A window is active, or a PowerDevil inhibition names the group | `Performance` |
| A window is unminimized, or the group plays media | `Background` |
| All windows are minimized | `Nap` |

Media playback lands in `Background`, not `Performance`. A background player is throttled lightly by design.

### Load

The load is `Busy` or `Idle`. A new group starts as `Busy`. The load only changes through CPU polling. See [CPU Load Polling](configuration.md#cpu-load-polling) for the definitions of `usage` and `throttle`.

### The vote

| Tier | Load | Policy |
|---|---|---|
| `Performance` | any | `performance` |
| `Background` | `Busy` | `background_busy` |
| `Background` | `Idle` | `background_idle` |
| `Nap` | `Busy` | `nap_busy` |
| `Nap` | `Idle` | `nap_idle` |

### The decision

A managed unit collects the votes of its voters and applies the most wakeful vote. The policies are ordered:

```text
nap_idle < nap_busy < background_idle < background_busy < performance
```

The `Ord` derive on `Policy` in `daemon/models/policy.rs` encodes this order. `reconcile_policy` in `daemon/mod.rs` takes the maximum vote. The maximum is the safe choice: one awake voter keeps the whole unit awake.

When the winning vote differs from the applied policy, the daemon:

1. Reverts the applied policy.
2. Applies the new policy through `PolicyRouter`.
3. Records the new policy on the managed unit.

`PolicyRouter` applies the actions of a policy in order. On failure, it rolls back the actions it already applied. A revert works the same way in reverse order.

## Policy Run Conditions

The daemon recomputes the vote of a group when:

- A tracked window is added, removed, minimized, or activated.
- A wake signal changes. A PowerDevil inhibition starts or stops, or an MPRIS player starts or stops playing.
- The load of the group flips between `Busy` and `Idle`.

The daemon polls CPU load every `interval_ms`, but only while at least one group is outside the `Performance` tier. With no polled groups, the daemon is fully event-driven.

On each tick, for every polled group:

1. The daemon reads `cpu.stat` of the units of the group and computes `usage` and `throttle`.
2. The group evaluates a candidate load:
   - `Idle` when `usage` is below the idle `usage_thres` and `throttle` is below the idle `throttle_thres`.
   - `Busy` when `usage` is above the busy `usage_thres` or `throttle` is above the busy `throttle_thres`.
   - No candidate between the two bands.
3. The candidate must repeat for `confirm_ticks` consecutive ticks before the load flips. A tick with no candidate does not reset the count.
4. On a flip, the group recomputes its vote.

A group that enters the `Performance` tier stops being polled. Its CPU state resets, so polling starts fresh when the group leaves `Performance` again.

A managed unit applies a policy when its winning vote changes. A managed unit reverts its policy and disappears when its last voter leaves.

## Boundaries

What the daemon does not do:

- It does not track processes that have no tracked windows.
- It does not apply actions to processes. The systemd unit is the smallest target.
- It does not merge window groups. Two groups that share a unit keep separate votes.
- `usage` sums `usage_usec` over the units of a group. `cpu.stat` is subtree-inclusive, so a unit nested inside another unit of the same group is counted twice.
