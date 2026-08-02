// @ts-nocheck -- Bun provides these Node test modules at runtime.
import assert from "node:assert/strict";
import { describe, test } from "node:test";

import { isGifImageSource } from "../src/lib/imageFormat.ts";
import { resizeImage } from "../src/lib/rendererHelper.ts";

describe("isGifImageSource", () => {
	test("recognises GIF data URLs", () => {
		assert.equal(isGifImageSource("data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///w=="), true);
		assert.equal(isGifImageSource("DATA:IMAGE/GIF;BASE64,R0lGODlhAQABAIAAAAAAAP///w=="), true);
	});

	test("recognises GIF file paths and URLs", () => {
		assert.equal(isGifImageSource("animations/status.gif"), true);
		assert.equal(isGifImageSource("http://localhost:57118/status.GIF?version=2#preview"), true);
	});

	test("does not classify static images as GIF", () => {
		assert.equal(isGifImageSource("data:image/png;base64,iVBORw0KGgo="), false);
		assert.equal(isGifImageSource("status.webp"), false);
		assert.equal(isGifImageSource(undefined), false);
	});

	test("keeps the original GIF data instead of flattening it through canvas", async () => {
		const gif = "data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///ywAAAAAAQABAAACAUwAOw==";
		assert.equal(await resizeImage(gif), gif);
	});
});
