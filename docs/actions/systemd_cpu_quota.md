# Systemd CPU Quota

This action sets the CPU quota for a systemd service.
Specifically it sets the `CPUQuotaPerSecUSec` property via systemd DBus.

I tried writing an action that directly sets the `cpu.max` cgroupv2 sysfs file, but it didn't work as expected as it needed modifications to permissions. I do not wish to modify permissions for this action.

In our configuration we use percent. It is translated to `CPUQuotaPerSecUSec` values with the following equation: `percent * 10_000`.

## Example

```toml
[policies.nap_busy]
actions = [
  { type = "systemd-cpu-quota", percent = 50 },
]
```
