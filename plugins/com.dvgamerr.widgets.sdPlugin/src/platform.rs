use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use tokio::process::Command;

fn plugin_root() -> Result<PathBuf> {
	let executable = std::env::current_exe().context("plugin executable path is unavailable")?;
	executable
		.parent()
		.and_then(Path::parent)
		.and_then(Path::parent)
		.map(Path::to_path_buf)
		.ok_or_else(|| anyhow::anyhow!("plugin root is unavailable"))
}

fn script_path(name: &str) -> Result<PathBuf> {
	let name = name.trim().trim_end_matches(".ps1");
	if name.is_empty()
		|| !name
			.chars()
			.all(|character| character.is_ascii_alphanumeric() || "._-".contains(character))
	{
		bail!("invalid script name");
	}
	let path = plugin_root()?.join("scripts").join(format!("{name}.ps1"));
	if !path.is_file() {
		bail!("script is not installed: {name}.ps1");
	}
	Ok(path)
}

pub async fn run_script(name: &str) -> Result<()> {
	let path = script_path(name)?;
	#[cfg(windows)]
	let mut command = {
		const CREATE_NO_WINDOW: u32 = 0x0800_0000;
		let mut command = Command::new("powershell.exe");
		command
			.creation_flags(CREATE_NO_WINDOW)
			.args([
				"-NoLogo",
				"-NoProfile",
				"-NonInteractive",
				"-ExecutionPolicy",
				"Bypass",
				"-WindowStyle",
				"Hidden",
				"-File",
			])
			.arg(&path);
		command
	};
	#[cfg(not(windows))]
	let mut command = {
		let mut command = Command::new("pwsh");
		command.args(["-NoLogo", "-NoProfile", "-NonInteractive", "-File"]);
		command.arg(&path);
		command
	};
	let status = command
		.status()
		.await
		.context("failed to start PowerShell")?;
	if !status.success() {
		bail!("PowerShell exited with {status}");
	}
	Ok(())
}
