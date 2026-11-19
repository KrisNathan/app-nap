# Systemd CPU Weight

This action sets the CPU weight for a systemd service.
Specifically it sets the `CPUWeight` property via systemd DBus.

This gives a soft signal to the scheduler that the service should be treated as less important. It doesn't necessarily affect power consumption but may help with performance of your prioritized services.

The revert for this action would set the `CPUWeight` property to `100`. On my system 100 is the default value for `app-*` units.

## Example

```toml
[policies.background_idle]
actions = [
  { type = "systemd-cpu-weight", weight = 1 },
]
```
