use crate::systemd::cgroup::get_pids_from_cgroups;
use libc::{self, cpu_set_t, pid_t};
use std::fs;
use std::io;
use std::mem;

/// Nap backend that pushes napped apps onto the CPU's efficiency cores
/// (E-cores) and restores them to all cores on resume.
///
/// On hybrid CPUs (Intel Alder Lake and later) the kernel exposes the
/// E-cores via `/sys/devices/cpu_atom/cpus` and the full online set via
/// `/sys/devices/system/cpu/online`. Both files use the kernel's CPU-list
/// format: a comma-separated list of single numbers and `start-end` ranges
/// (e.g. `0-3,8-11,15`).
///
/// `send_stop` pins every PID in the app's cgroup to the E-cores;
/// `send_cont` restores them to all online cores. Applying affinity to the
/// whole cgroup matches the `SystemSignalController` behavior so child
/// processes are throttled too.
pub struct ECoreAction {
    ecores: cpu_set_t,
    allcores: cpu_set_t,
}

impl ECoreAction {
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            ecores: read_cpuset("/sys/devices/cpu_atom/cpus")?,
            allcores: read_cpuset("/sys/devices/system/cpu/online")?,
        })
    }

    pub fn apply(&self, cgroups: &[String]) -> io::Result<()> {
        set_affinity_for_cgroup(cgroups, &self.ecores)
    }

    pub fn revert(&self, cgroups: &[String]) -> io::Result<()> {
        set_affinity_for_cgroup(cgroups, &self.allcores)
    }
}

fn set_affinity_for_cgroup(cgroups: &[String], cpuset: &cpu_set_t) -> io::Result<()> {
    let pids = get_pids_from_cgroups(cgroups)?;

    for p in pids {
        // A pid may have exited between enumeration and the syscall; treat
        // ESRCH as success so a racing exit doesn't fail the whole nap.
        match set_affinity(p, cpuset) {
            Ok(()) => {}
            Err(err) if err.raw_os_error() == Some(libc::ESRCH) => {}
            Err(err) => return Err(err),
        }
    }

    Ok(())
}

fn set_affinity(pid: pid_t, cpuset: &cpu_set_t) -> io::Result<()> {
    // SAFETY: `cpuset` is a fully initialized cpu_set_t; pid is a process id we
    // enumerated from the cgroup.
    let rc = unsafe { libc::sched_setaffinity(pid, mem::size_of_val(cpuset), cpuset) };
    if rc == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

fn empty_cpuset() -> cpu_set_t {
    unsafe {
        let mut set = mem::zeroed();
        libc::CPU_ZERO(&mut set);
        set
    }
}

fn cpu_count() -> usize {
    mem::size_of::<cpu_set_t>() * 8
}

fn cpu_set(cpu: usize, cpuset: &mut cpu_set_t) -> io::Result<()> {
    if cpu >= cpu_count() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("cpu id {cpu} exceeds cpu_set_t capacity"),
        ));
    }
    unsafe {
        libc::CPU_SET(cpu, cpuset);
    }
    Ok(())
}

/// Parse a kernel CPU-list string (e.g. `"0-3,8-11,15"`) into a `cpu_set_t`.
fn parse_cpu_list(raw: &str) -> io::Result<cpu_set_t> {
    let mut cpuset = empty_cpuset();

    for token in raw.trim().split(',') {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }

        let (start, end) = match token.split_once('-') {
            Some((s, e)) => (s.trim(), e.trim()),
            None => (token, token),
        };

        let start: usize = start
            .parse()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let end: usize = end
            .parse()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        if start > end {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid cpu range: {token}"),
            ));
        }

        for id in start..=end {
            cpu_set(id, &mut cpuset)?;
        }
    }

    Ok(cpuset)
}

/// Read a kernel CPU-list file and parse it into a `cpu_set_t`.
fn read_cpuset(path: &str) -> io::Result<cpu_set_t> {
    let raw = fs::read_to_string(path)?;
    parse_cpu_list(&raw)
}

#[cfg(test)]
mod tests {
    use super::parse_cpu_list;

    #[track_caller]
    fn check_cpu_list(raw: &str, expected: &[usize]) {
        let set = parse_cpu_list(raw).unwrap();
        let max = expected
            .iter()
            .copied()
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        for cpu in 0..=max {
            let want = expected.contains(&cpu);
            let got = unsafe { libc::CPU_ISSET(cpu, &set) };
            assert_eq!(got, want, "cpu {cpu}");
        }
    }

    #[track_caller]
    fn check_cpu_list_err(raw: &str) {
        assert!(parse_cpu_list(raw).is_err(), "{raw:?}");
    }

    #[test]
    fn parses_single_cpu() {
        check_cpu_list("4", &[4]);
    }

    #[test]
    fn parses_range() {
        check_cpu_list("4-7", &[4, 5, 6, 7]);
    }

    #[test]
    fn parses_mixed_list() {
        check_cpu_list("0-3,8-11,15", &[0, 1, 2, 3, 8, 9, 10, 11, 15]);
    }

    #[test]
    fn parses_whitespace_and_trailing_newline() {
        check_cpu_list(" 4 - 7 \n", &[4, 5, 6, 7]);
    }

    #[test]
    fn rejects_reversed_range() {
        check_cpu_list_err("7-4");
    }

    #[test]
    fn rejects_non_numeric() {
        check_cpu_list_err("a-b");
    }
}
