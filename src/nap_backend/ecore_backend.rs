use super::NapBackend;
use crate::systemd::cgroup::get_related_pids;
use libc::pid_t;
use nix::sched::{CpuSet, sched_setaffinity};
use nix::unistd::Pid;
use std::fs;
use std::io;

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
pub struct ECoreBackend {
    ecores: CpuSet,
    allcores: CpuSet,
}

impl ECoreBackend {
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            ecores: read_cpuset("/sys/devices/cpu_atom/cpus")?,
            allcores: read_cpuset("/sys/devices/system/cpu/online")?,
        })
    }
}

impl NapBackend for ECoreBackend {
    fn send_stop(&self, pid: pid_t) -> io::Result<()> {
        self.set_affinity_for_cgroup(pid, &self.ecores)
    }

    fn send_cont(&self, pid: pid_t) -> io::Result<()> {
        self.set_affinity_for_cgroup(pid, &self.allcores)
    }
}

impl ECoreBackend {
    fn set_affinity_for_cgroup(&self, pid: pid_t, cpuset: &CpuSet) -> io::Result<()> {
        let pids = get_related_pids(pid)?;

        for p in pids {
            // A pid may have exited between enumeration and the syscall; treat
            // ESRCH as success so a racing exit doesn't fail the whole nap.
            if let Err(err) = sched_setaffinity(Pid::from_raw(p), cpuset)
                && err != nix::errno::Errno::ESRCH
            {
                return Err(err.into());
            }
        }

        Ok(())
    }
}

/// Parse a kernel CPU-list string (e.g. `"0-3,8-11,15"`) into a `CpuSet`.
fn parse_cpu_list(raw: &str) -> io::Result<CpuSet> {
    let mut cpuset = CpuSet::new();

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
            cpuset
                .set(id)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        }
    }

    Ok(cpuset)
}

/// Read a kernel CPU-list file and parse it into a `CpuSet`.
fn read_cpuset(path: &str) -> io::Result<CpuSet> {
    let raw = fs::read_to_string(path)?;
    parse_cpu_list(&raw)
}

#[cfg(test)]
mod tests {
    use super::parse_cpu_list;

    #[test]
    fn parses_single_cpu() {
        let set = parse_cpu_list("4").unwrap();
        assert!(set.is_set(4).unwrap());
        assert!(!set.is_set(3).unwrap());
        assert!(!set.is_set(5).unwrap());
    }

    #[test]
    fn parses_range() {
        let set = parse_cpu_list("4-7").unwrap();
        for id in 4..=7 {
            assert!(set.is_set(id).unwrap(), "cpu {id} should be set");
        }
        assert!(!set.is_set(3).unwrap());
        assert!(!set.is_set(8).unwrap());
    }

    #[test]
    fn parses_mixed_list() {
        let set = parse_cpu_list("0-3,8-11,15").unwrap();
        for id in [0, 1, 2, 3, 8, 9, 10, 11, 15] {
            assert!(set.is_set(id).unwrap(), "cpu {id} should be set");
        }
        for id in [4, 5, 6, 7, 12, 13, 14, 16] {
            assert!(!set.is_set(id).unwrap(), "cpu {id} should not be set");
        }
    }

    #[test]
    fn parses_whitespace_and_trailing_newline() {
        let set = parse_cpu_list(" 4 - 7 \n").unwrap();
        for id in 4..=7 {
            assert!(set.is_set(id).unwrap());
        }
    }

    #[test]
    fn rejects_reversed_range() {
        assert!(parse_cpu_list("7-4").is_err());
    }

    #[test]
    fn rejects_non_numeric() {
        assert!(parse_cpu_list("a-b").is_err());
    }
}
