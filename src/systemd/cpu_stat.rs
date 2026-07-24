use std::{fs, io};

pub struct CpuStat {
    usage_usec: u64,
    // at the moment we don't care about the other values.
}

pub fn get_cpu_stat(cgroup_path: &str) -> io::Result<CpuStat> {
    let stat_path = format!("/sys/fs/cgroup{cgroup_path}/cpu.stat");
    let content = fs::read_to_string(&stat_path)?;

    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("usage_usec ") {
            let usage_usec = rest
                .trim()
                .parse::<u64>()
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
            return Ok(CpuStat { usage_usec });
        }
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        format!("usage_usec not found in {stat_path}"),
    ))
}
