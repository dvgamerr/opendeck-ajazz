// @ts-nocheck -- Bun provides these Node test modules at runtime.
import assert from "node:assert/strict";
import { describe, test } from "node:test";

import { normaliseProfiles, normaliseSelection, reorderProfiles } from "../plugins/com.amansprojects.starterpack.sdPlugin/assets/propertyInspector/profilePaginationState.js";

describe("Profile Pagination inspector state", () => {
	test("keeps a unique ordered list of valid profile names", () => {
		assert.deepEqual(normaliseProfiles(["Work", "Gaming", "Work", "", null]), ["Work", "Gaming"]);
		assert.deepEqual(normaliseSelection(["Missing", "Gaming", "Work"], ["Work", "Gaming"]), ["Gaming", "Work"]);
	});

	test("reorders without mutating settings and ignores invalid moves", () => {
		const profiles = ["One", "Two", "Three"];
		assert.deepEqual(reorderProfiles(profiles, 1, 0), ["Two", "One", "Three"]);
		assert.deepEqual(profiles, ["One", "Two", "Three"]);
		assert.equal(reorderProfiles(profiles, 0, -1), profiles);
	});
});
