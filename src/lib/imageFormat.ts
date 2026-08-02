export function isGifImageSource(source: string | undefined): boolean {
	if (!source) return false;

	const normalized = source.trim().toLowerCase();
	if (/^data:image\/gif(?:;[^,]*)?,/.test(normalized)) return true;

	const path = normalized.split("#", 1)[0].split("?", 1)[0];
	return path.endsWith(".gif");
}
