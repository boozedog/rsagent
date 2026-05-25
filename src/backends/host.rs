use std::collections::HashMap;

use serde_json::Value;
use sysinfo::Disks;
use sysinfo::{MemoryRefreshKind, RefreshKind, System};

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
