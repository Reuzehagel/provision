use sysinfo::System;

pub struct SystemInfo {
    pub hostname: String,
    pub os_version: String,
    pub cpu_name: String,
    pub ram_gb: f64,
}

pub fn gather() -> SystemInfo {
    let hostname = System::host_name().unwrap_or_else(|| "Unknown".into());

    let os_version = System::long_os_version().unwrap_or_else(|| {
        System::os_version()
            .map(|v| format!("Windows {v}"))
            .unwrap_or_else(|| "Windows".into())
    });

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
        os_version,
        cpu_name,
        ram_gb,
    }
}
