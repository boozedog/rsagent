use std::collections::HashMap;

use serde_json::Value;
use sysinfo::Disks;
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

use super::{DiskInput, EmptyInput};
use crate::backends::{param_string_opt, parse_input};
use crate::error::{Result, RsagentError};

pub fn memory(params: &HashMap<String, toml::Value>, input: Value) -> Result<String> {
	let _: EmptyInput = parse_input("host.memory", input)?;

	let _ = params;
	let mut system = System::new_with_specifics(
		RefreshKind::nothing().with_memory(MemoryRefreshKind::everything()),
	);
	system.refresh_memory();

	let used = system.used_memory();
	let total = system.total_memory();
	let available = system.available_memory();

	Ok(format!(
		"Memory: used {} MiB / total {} MiB (available {} MiB)",
		used / 1024 / 1024,
		total / 1024 / 1024,
		available / 1024 / 1024,
	))
}

pub fn disk(params: &HashMap<String, toml::Value>, input: Value) -> Result<String> {
	let call: DiskInput = parse_input("host.disk", input)?;
	let mount = call
		.mount
		.or_else(|| param_string_opt(params, "mount"))
		.unwrap_or_else(|| "/".into());

	let disks = Disks::new_with_refreshed_list();
	let Some(disk) = disks.iter().find(|d| d.mount_point().to_string_lossy() == mount) else {
		return Err(RsagentError::tool(
			"host.disk",
			format!("no disk mounted at `{mount}`"),
		));
	};

	let total = disk.total_space();
	let available = disk.available_space();
	let used = total.saturating_sub(available);

	Ok(format!(
		"Disk {mount}: used {} GiB / total {} GiB (available {} GiB)",
		used / 1024 / 1024 / 1024,
		total / 1024 / 1024 / 1024,
		available / 1024 / 1024 / 1024,
	))
}

pub fn load(params: &HashMap<String, toml::Value>, input: Value) -> Result<String> {
	let _: EmptyInput = parse_input("host.load", input)?;
	let _ = params;

	let mut system = System::new_with_specifics(
		RefreshKind::nothing()
			.with_cpu(CpuRefreshKind::everything())
			.with_memory(MemoryRefreshKind::everything()),
	);
	system.refresh_cpu_usage();
	system.refresh_memory();

	let cpus = system.cpus().len().max(1);
	let load = System::load_average();

	Ok(format!(
		"Load average: {:.2} {:.2} {:.2} ({} CPUs)\nUptime: {}s",
		load.one,
		load.five,
		load.fifteen,
		cpus,
		System::uptime()
	))
}

pub fn nixos_revision(params: &HashMap<String, toml::Value>, input: Value) -> Result<String> {
	let _: EmptyInput = parse_input("host.nixos_revision", input)?;
	let _ = params;

	#[cfg(target_os = "linux")]
	{
		return nixos_revision_linux();
	}

	#[cfg(not(target_os = "linux"))]
	{
		Err(RsagentError::tool(
			"host.nixos_revision",
			"NixOS revision is only available on Linux hosts",
		))
	}
}

#[cfg(target_os = "linux")]
fn nixos_revision_linux() -> Result<String> {
	use std::process::Command;

	let output = Command::new("nixos-version")
		.arg("--json")
		.output()
		.map_err(|e| RsagentError::tool("host.nixos_revision", e.to_string()))?;

	if output.status.success() {
		let stdout = String::from_utf8_lossy(&output.stdout);
		if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
			let version = json["version"].as_str().unwrap_or("unknown");
			let revision = json["revision"].as_str().unwrap_or("unknown");
			let dirty = json["dirty"].as_bool().unwrap_or(false);
			return Ok(format!(
				"NixOS {version} (revision {revision}{})",
				if dirty { ", dirty" } else { "" }
			));
		}
		return Ok(stdout.trim().to_string());
	}

	for path in ["/etc/nixos/flake.nix", "/etc/nixos/configuration.nix"] {
		if std::path::Path::new(path).exists() {
			return Ok(format!("NixOS config present at {path} (nixos-version unavailable)"));
		}
	}

	Err(RsagentError::tool(
		"host.nixos_revision",
		"could not determine NixOS revision",
	))
}
