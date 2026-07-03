# Finding your E-cores for `low_power_cores`

`app-nap` can push napped apps onto your CPU's efficiency cores (E-cores) so
the performance cores (P-cores) can reach deeper idle states. Set the
`low_power_cores` list in `app-nap.toml` to the logical CPU numbers of your
E-cores:

```toml
low_power_cores = [4, 5, 6, 7]
```

## How to find the E-core CPU numbers

The kernel exposes the hybrid CPU split directly:

```
$ cat /sys/devices/cpu_atom/cpus
4-7
```

`cpu_atom` lists the E-cores; `cpu_core` lists the P-cores. Expand the range
into a comma-separated list and put it in `low_power_cores`.

If `/sys/devices/cpu_atom` does not exist, your CPU is not hybrid (or the
kernel does not know about the split) and you should leave
`low_power_cores = []`.

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

Here CPUs 4–7 have `MAXMHZ` of 3700, well below the 4700–4800 of CPUs 0–3 —
matching the `4-7` from `cpu_atom`. The lower-frequency cluster is always the
E-cores.
