use std::{fs, io};

#[derive(Debug, Clone, Copy)]
pub struct CpuStat {
    pub usage_usec: u64,
    pub throttled_usec: u64,
}

pub fn get_cpu_stat(cgroup_path: &str) -> io::Result<CpuStat> {
    let stat_path = format!("/sys/fs/cgroup{cgroup_path}/cpu.stat");
    let content = fs::read_to_string(&stat_path)?;
    parse_cpu_stat(&content)
        .map_err(|err| io::Error::new(err.kind(), format!("{stat_path}: {err}")))
}

/// `throttled_usec` may be absent when the bandwidth controller never ran;
/// treat it as zero. `usage_usec` is mandatory.
fn parse_cpu_stat(content: &str) -> io::Result<CpuStat> {
    let mut usage_usec = None;
    let mut throttled_usec = 0;

    for line in content.lines() {
        let (key, value) = line.split_once(' ').unwrap_or((line, ""));
        match key {
            "usage_usec" => {
                usage_usec = Some(
                    value
                        .trim()
                        .parse::<u64>()
                        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?,
                );
            }
            "throttled_usec" => {
                throttled_usec = value
                    .trim()
                    .parse::<u64>()
                    .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
            }
            _ => {}
        }
    }

    Ok(CpuStat {
        usage_usec: usage_usec.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "usage_usec not found in cpu.stat",
            )
        })?,
        throttled_usec,
    })
}
