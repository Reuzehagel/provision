use sysinfo::System;

pub struct SystemInfo {
    pub hostname: String,
    pub cpu_name: String,
    pub ram_gb: f64,
}

pub fn gather() -> SystemInfo {
    let hostname = System::host_name().unwrap_or_else(|| "Unknown".into());

    let sys = System::new_with_specifics(
        sysinfo::RefreshKind::nothing()
            .with_cpu(sysinfo::CpuRefreshKind::everything())
            .with_memory(sysinfo::MemoryRefreshKind::nothing().with_ram()),
    );

    let cpu_name = sys
        .cpus()
        .first()
        .map(|c| c.brand().trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Unknown CPU".into());

    let ram_gb = sys.total_memory() as f64 / 1_073_741_824.0;

    SystemInfo {
        hostname,
        cpu_name,
        ram_gb,
    }
}
