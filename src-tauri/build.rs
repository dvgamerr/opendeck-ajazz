use std::fs;
use std::path::Path;

fn main() {
	let daisyui_source = Path::new("../node_modules/daisyui/daisyui.css");
	println!("cargo:rerun-if-changed={}", daisyui_source.display());
	let daisyui_css = fs::read(daisyui_source).expect("failed to read the DaisyUI bundle");
	let daisyui_hash = daisyui_css.iter().fold(0xcbf29ce484222325_u64, |hash, byte| (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3));
	let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR is not available");
	fs::write(Path::new(&out_dir).join("property_inspector_daisyui.css"), daisyui_css).expect("failed to stage the property inspector DaisyUI bundle");
	println!("cargo:rustc-env=OPENDECK_DAISYUI_HASH={daisyui_hash:016x}");

	// Rebuild bundled plugins after source or asset changes without runtime polling.
	println!("cargo:rerun-if-changed=../plugins");
	if let Err(error) = || -> Result<(), std::io::Error> {
		for entry in fs::read_dir("../plugins")?.flatten() {
			let out_dir = std::path::Path::new("target").join("plugins").join(entry.file_name());
			fs::create_dir_all(&out_dir)?;
			let status = std::process::Command::new("bun")
				.args(["run", "build.ts", fs::canonicalize(out_dir)?.to_string_lossy().as_ref(), &std::env::var("TARGET").unwrap()])
				.current_dir(entry.path())
				.status()?;
			if !status.success() {
				panic!("Failed to build plugin {}: status code {}", entry.file_name().to_string_lossy(), status.code().unwrap());
			}
		}

		Ok(())
	}() {
		#[cfg(debug_assertions)]
		eprintln!("Failed to build builtin plugins: {error}");
		#[cfg(not(debug_assertions))]
		panic!("Failed to build builtin plugins: {error}");
	}

	built::write_built_file().expect("failed to acquire build-time information");
	tauri_build::build();
}
