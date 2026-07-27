use base64::Engine as _;
use image::ImageFormat;
use tauri::command;

use super::Error;

const MAX_STARTUP_IMAGE_BYTES: usize = 10 * 1024 * 1024;

fn decode_startup_image(value: &str) -> Result<image::DynamicImage, anyhow::Error> {
	let (metadata, encoded) = value.split_once(',').ok_or_else(|| anyhow::anyhow!("The selected image is invalid"))?;

	if !matches!(metadata, "data:image/png;base64" | "data:image/jpeg;base64" | "data:image/bmp;base64" | "data:image/x-ms-bmp;base64") {
		return Err(anyhow::anyhow!("Only PNG, JPG, JPEG, and BMP images are supported"));
	}

	if encoded.len() > MAX_STARTUP_IMAGE_BYTES.saturating_mul(4).div_ceil(3) + 4 {
		return Err(anyhow::anyhow!("The selected image must be 10 MB or smaller"));
	}

	let bytes = base64::engine::general_purpose::STANDARD.decode(encoded)?;
	if bytes.len() > MAX_STARTUP_IMAGE_BYTES {
		return Err(anyhow::anyhow!("The selected image must be 10 MB or smaller"));
	}

	let format = image::guess_format(&bytes)?;
	if !matches!(format, ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::Bmp) {
		return Err(anyhow::anyhow!("Only PNG, JPG, JPEG, and BMP images are supported"));
	}

	Ok(image::load_from_memory_with_format(&bytes, format)?)
}

#[command]
pub async fn set_startup_image(device: &str, image: &str) -> Result<(), Error> {
	let image = decode_startup_image(image)?;
	crate::ajazz::set_startup_image(device, image).await?;
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::decode_startup_image;

	#[test]
	fn rejects_gif_data_urls() {
		let error = decode_startup_image("data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///ywAAAAAAQABAAACAUwAOw==").expect_err("GIF must not be accepted");
		assert!(error.to_string().contains("Only PNG, JPG, JPEG, and BMP"));
	}

	#[test]
	fn rejects_gif_disguised_as_png() {
		let error = decode_startup_image("data:image/png;base64,R0lGODlhAQABAIAAAAAAAP///ywAAAAAAQABAAACAUwAOw==").expect_err("GIF content must not be accepted");
		assert!(error.to_string().contains("Only PNG, JPG, JPEG, and BMP"));
	}
}
