use iced::futures;
use iced::futures::SinkExt as _;
use iced::stream;
use windows_sys::Win32::System::Registry::HKEY;

use crate::install::{self, InstallProgress, Sender};
use crate::upgrade::InstalledPackage;

pub fn uninstall_all(
    packages: Vec<InstalledPackage>,
    dry_run: bool,
    extra_args: Vec<String>,
) -> impl futures::Stream<Item = InstallProgress> + Send {
    stream::channel(100, move |mut sender: Sender| async move {
        for (i, pkg) in packages.iter().enumerate() {
            let _ = sender.send(InstallProgress::Started { index: i }).await;

            if dry_run {
                let _ = sender
                    .send(InstallProgress::Log {
                        index: i,
                        line: format!(
                            "[DRY RUN] Would run: winget uninstall --id {} -e",
                            pkg.winget_id
                        ),
                    })
                    .await;

                tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                let _ = sender.send(InstallProgress::Succeeded { index: i }).await;
                continue;
            }

            let mut args: Vec<String> = vec![
                "uninstall".into(),
                "--id".into(),
                pkg.winget_id.clone(),
                "-e".into(),
                "--accept-source-agreements".into(),
            ];
            args.extend(extra_args.iter().cloned());

            match install::run_command("winget", &args, i, &mut sender).await {
                Ok(()) => {
                    let _ = sender.send(InstallProgress::Succeeded { index: i }).await;
                }
                Err(e) => {
                    let _ = sender
                        .send(InstallProgress::Failed { index: i, error: e })
                        .await;
                }
            }
        }
        let _ = sender.send(InstallProgress::Completed).await;
    })
}

/// Scan Windows registry for installed package sizes.
/// Returns (winget_id_lower, size_in_bytes) pairs.
pub async fn scan_sizes(packages: Vec<InstalledPackage>) -> Vec<(String, u64)> {
    tokio::task::spawn_blocking(move || scan_sizes_blocking(&packages))
        .await
        .unwrap_or_default()
}

fn scan_sizes_blocking(packages: &[InstalledPackage]) -> Vec<(String, u64)> {
    use windows_sys::Win32::System::Registry::*;

    let mut results = Vec::new();
    let mut matched: std::collections::HashSet<String> = std::collections::HashSet::new();

    let reg_paths: &[(&str, HKEY)] = &[
        (
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
            HKEY_LOCAL_MACHINE,
        ),
        (
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
            HKEY_CURRENT_USER,
        ),
        (
            "SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
            HKEY_LOCAL_MACHINE,
        ),
    ];

    for &(subkey, hive) in reg_paths {
        if let Some(entries) = read_uninstall_key(hive, subkey) {
            for (display_name, size_kb) in entries {
                let display_lower = display_name.to_lowercase();
                for pkg in packages {
                    if matched.contains(&pkg.winget_id_lower) {
                        continue;
                    }
                    if display_lower == pkg.name_lower {
                        results.push((pkg.winget_id_lower.clone(), size_kb * 1024));
                        matched.insert(pkg.winget_id_lower.clone());
                        break;
                    }
                    let id_segment = pkg.winget_id.rsplit('.').next().unwrap_or("");
                    if !id_segment.is_empty()
                        && id_segment.len() >= 3
                        && display_lower.contains(&id_segment.to_lowercase())
                    {
                        results.push((pkg.winget_id_lower.clone(), size_kb * 1024));
                        matched.insert(pkg.winget_id_lower.clone());
                        break;
                    }
                }
            }
        }
    }

    results
}

fn read_uninstall_key(hive: HKEY, subkey: &str) -> Option<Vec<(String, u64)>> {
    use windows_sys::Win32::System::Registry::*;

    let subkey_wide: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();
    let mut hkey: HKEY = std::ptr::null_mut();

    let status = unsafe { RegOpenKeyExW(hive, subkey_wide.as_ptr(), 0, KEY_READ, &mut hkey) };
    if status != 0 {
        return None;
    }

    let mut entries = Vec::new();
    let mut index = 0u32;

    loop {
        let mut name_buf = [0u16; 256];
        let mut name_len = name_buf.len() as u32;

        let status = unsafe {
            RegEnumKeyExW(
                hkey,
                index,
                name_buf.as_mut_ptr(),
                &mut name_len,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        if status != 0 {
            break;
        }

        let subkey_name = String::from_utf16_lossy(&name_buf[..name_len as usize]);
        let full_path = format!("{subkey}\\{subkey_name}");
        let full_wide: Vec<u16> = full_path.encode_utf16().chain(std::iter::once(0)).collect();

        let mut entry_key: HKEY = std::ptr::null_mut();
        let open_status =
            unsafe { RegOpenKeyExW(hive, full_wide.as_ptr(), 0, KEY_READ, &mut entry_key) };

        if open_status == 0 {
            let display_name = read_reg_string(entry_key, "DisplayName");
            let size = read_reg_dword(entry_key, "EstimatedSize");
            unsafe { RegCloseKey(entry_key) };

            if let (Some(name), Some(size_kb)) = (display_name, size)
                && !name.is_empty()
                && size_kb > 0
            {
                entries.push((name, size_kb as u64));
            }
        }

        index += 1;
    }

    unsafe { RegCloseKey(hkey) };
    Some(entries)
}

fn read_reg_string(hkey: HKEY, value_name: &str) -> Option<String> {
    use windows_sys::Win32::System::Registry::*;

    let name_wide: Vec<u16> = value_name
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let mut buf = [0u16; 512];
    let mut buf_size = (buf.len() * 2) as u32;
    let mut reg_type = 0u32;

    let status = unsafe {
        RegQueryValueExW(
            hkey,
            name_wide.as_ptr(),
            std::ptr::null_mut(),
            &mut reg_type,
            buf.as_mut_ptr().cast(),
            &mut buf_size,
        )
    };

    if status != 0 || reg_type != REG_SZ {
        return None;
    }

    let len = (buf_size as usize / 2).saturating_sub(1);
    Some(String::from_utf16_lossy(&buf[..len]))
}

fn read_reg_dword(hkey: HKEY, value_name: &str) -> Option<u32> {
    use windows_sys::Win32::System::Registry::*;

    let name_wide: Vec<u16> = value_name
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let mut value: u32 = 0;
    let mut buf_size = std::mem::size_of::<u32>() as u32;
    let mut reg_type = 0u32;

    let status = unsafe {
        RegQueryValueExW(
            hkey,
            name_wide.as_ptr(),
            std::ptr::null_mut(),
            &mut reg_type,
            (&mut value as *mut u32).cast(),
            &mut buf_size,
        )
    };

    if status != 0 || reg_type != REG_DWORD {
        return None;
    }

    Some(value)
}
