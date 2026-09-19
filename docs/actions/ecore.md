# E-core action

The `ecore` action pushes napped apps onto the CPU's efficiency cores (E-cores)
so the performance cores (P-cores) can reach deeper idle states. It requires a
hybrid Intel CPU (Alder Lake and newer) that exposes
`/sys/devices/cpu_atom/cpus`.

The action auto-detects the E-core and online CPU sets at startup from the
kernel, so no manual configuration is needed:

```toml
[tiers.nap]
actions = [
  { type = "systemd-cpu-quota", percent = 10 },
  { type = "ecore" },
]
```

## How the kernel exposes the split

```
$ cat /sys/devices/cpu_atom/cpus
4-7
```

`cpu_atom` lists the E-cores; `cpu_core` lists the P-cores. If
`/sys/devices/cpu_atom` does not exist, the CPU is not hybrid (or the kernel
does not know about the split) and the `ecore` action is skipped at load time
with a warning.

## Verifying with `lscpu -e`

To sanity-check which cores those are, run `lscpu -e` and look at the
`MAXMHZ` column. E-cores run at a lower max frequency than P-cores on the
same package:

```
$ lscpu -e
CPU NODE SOCKET CORE L1d:L1i:L2:L3 ONLINE    MAXMHZ   MINMHZ       MHZ
  0    0      0    0 0:0:0:0          yes 4700.0000 400.0000  400.2610
  1    0      0    1 4:4:1:0          yes 4800.0000 400.0000 1178.8060
  2    0      0    2 8:8:2:0          yes 4700.0000 400.0000 1172.1949
  3    0      0    3 12:12:3:0        yes 4800.0000 400.0000  967.2010
  4    0      0    4 64:64:8          yes 3700.0000 400.0000 1334.2030
  5    0      0    5 66:66:8          yes 3700.0000 400.0000 1360.1370
  6    0      0    6 68:68:8          yes 3700.0000 400.0000 1305.0490
  7    0      0    7 70:70:8          yes 3700.0000 400.0000 1575.0000
```

Here CPUs 4-7 have `MAXMHZ` of 3700, well below the 4700-4800 of CPUs 0-3,
matching the `4-7` from `cpu_atom`. The lower-frequency cluster is always the
E-cores.
