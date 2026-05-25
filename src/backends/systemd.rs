use std::collections::HashMap;

use serde_json::Value;

use super::{ListUnitsInput, UnitInput};
use crate::backends::{ensure_allowed_units, parse_input, validate_unit};
use crate::error::{Result, RsagentError};

pub fn unit_status(params: &HashMap<String, toml::Value>, input: Value) -> Result<String> {
	let call: UnitInput = parse_input("systemd.unit_status", input)?;
	let allowed = ensure_allowed_units(params)?;
	validate_unit("systemd.unit_status", &call.unit, &allowed)?;

	#[cfg(all(target_os = "linux", feature = "systemd"))]
	{
		return unit_status_linux(&call.unit);
	}

	#[cfg(not(all(target_os = "linux", feature = "systemd")))]
	{
		Err(RsagentError::tool(
			"systemd.unit_status",
			"systemd support requires Linux and the `systemd` build feature",
		))
	}
}

pub fn list_units(params: &HashMap<String, toml::Value>, input: Value) -> Result<String> {
	let call: ListUnitsInput = parse_input("systemd.list_units", input)?;
	let filter = call
		.filter
		.or_else(|| {
			params
				.get("filter")
				.and_then(|v| v.as_str())
				.map(str::to_string)
		});

	#[cfg(all(target_os = "linux", feature = "systemd"))]
	{
		return list_units_linux(filter.as_deref());
	}

	#[cfg(not(all(target_os = "linux", feature = "systemd")))]
	{
		let _ = filter;
		Err(RsagentError::tool(
			"systemd.list_units",
			"systemd support requires Linux and the `systemd` build feature",
		))
	}
}

#[cfg(all(target_os = "linux", feature = "systemd"))]
fn unit_status_linux(unit: &str) -> Result<String> {
	tokio::runtime::Handle::try_current()
		.map_err(|e| RsagentError::tool("systemd.unit_status", e.to_string()))?
		.block_on(async { unit_status_linux_async(unit).await })
}

#[cfg(all(target_os = "linux", feature = "systemd"))]
async fn unit_status_linux_async(unit: &str) -> Result<String> {
	use zbus::zvariant::OwnedObjectPath;
	use zbus_systemd::systemd1::manager::ManagerProxy;
	use zbus_systemd::systemd1::unit::UnitProxy;

	let connection = zbus::Connection::system()
		.await
		.map_err(|e| RsagentError::tool("systemd.unit_status", e.to_string()))?;

	let manager = ManagerProxy::new(&connection)
		.await
		.map_err(|e| RsagentError::tool("systemd.unit_status", e.to_string()))?;

	let unit_path: OwnedObjectPath = manager
		.get_unit(unit)
		.await
		.map_err(|e| RsagentError::tool("systemd.unit_status", e.to_string()))?;

	let unit_proxy = UnitProxy::builder(&connection)
		.path(unit_path)
		.map_err(|e| RsagentError::tool("systemd.unit_status", e.to_string()))?
		.build()
		.await
		.map_err(|e| RsagentError::tool("systemd.unit_status", e.to_string()))?;

	let active = unit_proxy
		.active_state()
		.await
		.map_err(|e| RsagentError::tool("systemd.unit_status", e.to_string()))?;
	let sub = unit_proxy
		.sub_state()
		.await
		.map_err(|e| RsagentError::tool("systemd.unit_status", e.to_string()))?;
	let description = unit_proxy
		.description()
		.await
		.map_err(|e| RsagentError::tool("systemd.unit_status", e.to_string()))?;

	Ok(format!(
		"Unit: {unit}\nState: {active} ({sub})\nDescription: {description}"
	))
}

#[cfg(all(target_os = "linux", feature = "systemd"))]
fn list_units_linux(filter: Option<&str>) -> Result<String> {
	tokio::runtime::Handle::try_current()
		.map_err(|e| RsagentError::tool("systemd.list_units", e.to_string()))?
		.block_on(async { list_units_linux_async(filter).await })
}

#[cfg(all(target_os = "linux", feature = "systemd"))]
async fn list_units_linux_async(filter: Option<&str>) -> Result<String> {
	use zbus_systemd::systemd1::manager::ManagerProxy;

	let connection = zbus::Connection::system()
		.await
		.map_err(|e| RsagentError::tool("systemd.list_units", e.to_string()))?;

	let manager = ManagerProxy::new(&connection)
		.await
		.map_err(|e| RsagentError::tool("systemd.list_units", e.to_string()))?;

	let units = manager
		.list_units()
		.await
		.map_err(|e| RsagentError::tool("systemd.list_units", e.to_string()))?;

	let mut out = String::new();
	for unit in units {
		if let Some(filter) = filter {
			if !unit.0.contains(filter) {
				continue;
			}
		}
		out.push_str(&format!(
			"{} → {} ({})\n",
			unit.0, unit.3, unit.2
		));
	}

	if out.is_empty() {
		return Ok("No matching units".into());
	}

	Ok(out)
}
