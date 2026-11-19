# Systemd Freeze Action

I don't recommend using this.

Uses `FreezeUnit` dbus call to systemd.
I believe systemd uses [cgroup.freeze](https://docs.kernel.org/admin-guide/cgroup-v2.html) under the hood.

Maybe someday I will make an option for an action that doesn't use systemd and directly modifies the cgroup.freeze cgroup sysfs.

Behavioraly this is kind of similar to `SIGSTOP` except that it applies to an entire cgroup and not just a single process.
Which is why I don't recommend using this.
In particular, say you freeze on minimize, then you try to close the window without unminimizing it, you will be unable to do so. You will be faced with "Not responding" dialogs. You would need to unminimize the window before closing it. This wouldn't have been an issue if we have a kwin event for "before close is acted on". Unfortunately I couldn't find such an event.

## Example

```toml
[policies.nap_idle] # full nap
actions = [
  { type = "systemd-freeze" },
]
```
