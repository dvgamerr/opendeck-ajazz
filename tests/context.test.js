// @ts-nocheck -- Bun provides these Node test modules at runtime.
import assert from "node:assert/strict";
import { describe, test } from "node:test";

import { contextsEqual } from "../src/lib/Context.ts";

describe("contextsEqual", () => {
	test("matches separate objects for the same key", () => {
		const first = { device: "device-a", profile: "Default", controller: "Keypad", position: 2 };
		const refreshed = { ...first };

		assert.equal(contextsEqual(first, refreshed), true);
	});

	test("rejects a different key or missing context", () => {
		const context = { device: "device-a", profile: "Default", controller: "Keypad", position: 2 };

		assert.equal(contextsEqual(context, { ...context, position: 3 }), false);
		assert.equal(contextsEqual(context, null), false);
	});
});
