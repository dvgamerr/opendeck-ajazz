// @ts-nocheck -- Bun provides these Node test modules at runtime.
import assert from "node:assert/strict";
import { describe, test } from "node:test";

import { getPausedProfileRenderingDevices, pauseProfileRendering, pausedProfileRenderingDevices, resumeProfileRendering } from "../src/lib/profileRendering.ts";

describe("profile rendering pause state", () => {
	test("pauses each device once and publishes new snapshots", () => {
		const snapshots = [];
		const unsubscribe = pausedProfileRenderingDevices.subscribe((devices) => snapshots.push(devices));

		assert.equal(pauseProfileRendering("device-a"), true);
		assert.equal(pauseProfileRendering("device-a"), false);
		assert.deepEqual(getPausedProfileRenderingDevices(), ["device-a"]);
		assert.notEqual(snapshots[0], snapshots.at(-1));

		resumeProfileRendering("device-a");
		assert.deepEqual(getPausedProfileRenderingDevices(), []);
		unsubscribe();
	});
});
