use std::{fmt::Write, sync::LazyLock};
use ttf_parser::OutlineBuilder;

static PIXELOID: LazyLock<ttf_parser::Face<'static>> = LazyLock::new(|| {
	ttf_parser::Face::parse(include_bytes!("../assets/fonts/PixeloidSans.ttf"), 0).unwrap()
});

struct GlyphPath {
	data: String,
	x: f32,
}

impl OutlineBuilder for GlyphPath {
	fn move_to(&mut self, x: f32, y: f32) {
		let _ = write!(self.data, "M{} {}", (x + self.x) as i32, y as i32);
	}

	fn line_to(&mut self, x: f32, y: f32) {
		let _ = write!(self.data, "L{} {}", (x + self.x) as i32, y as i32);
	}

	fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
		let _ = write!(
			self.data,
			"Q{} {} {} {}",
			(x1 + self.x) as i32,
			y1 as i32,
			(x + self.x) as i32,
			y as i32
		);
	}

	fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
		let _ = write!(
			self.data,
			"C{} {} {} {} {} {}",
			(x1 + self.x) as i32,
			y1 as i32,
			(x2 + self.x) as i32,
			y2 as i32,
			(x + self.x) as i32,
			y as i32
		);
	}

	fn close(&mut self) {
		self.data.push('Z');
	}
}

pub fn text_path(text: &str, center: f32, baseline: u8, size: u8, fill: &str) -> String {
	let face = &*PIXELOID;
	let scale = f32::from(size) / f32::from(face.units_per_em());
	let mut path = GlyphPath {
		data: String::with_capacity(text.len() * 80),
		x: 0.0,
	};
	for character in text.chars() {
		if let Some(glyph) = face
			.glyph_index(character)
			.or_else(|| face.glyph_index('?'))
		{
			face.outline_glyph(glyph, &mut path);
			path.x += f32::from(face.glyph_hor_advance(glyph).unwrap_or_default());
		}
	}
	let x = center - path.x * scale / 2.0;
	format!(
		r##"<path d="{}" transform="translate({x:.2} {baseline}) scale({scale:.5} -{scale:.5})" fill="{fill}"/>"##,
		path.data
	)
}

pub fn data_uri(svg: &str) -> String {
	let mut encoded = String::with_capacity(svg.len() * 2);
	for byte in svg.bytes() {
		if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
			encoded.push(byte as char);
		} else {
			let _ = write!(encoded, "%{byte:02X}");
		}
	}
	format!("data:image/svg+xml,{encoded}")
}
