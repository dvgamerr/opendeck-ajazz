/**
 * @param {unknown[]} profiles
 * @returns {string[]}
 */
export function normaliseProfiles(profiles) {
	/** @type {Set<string>} */
	const unique = new Set();
	for (const profile of profiles) {
		if (typeof profile === "string" && profile.length > 0) unique.add(profile);
	}
	return [...unique];
}

/**
 * @param {unknown[]} profiles
 * @param {string[]} availableProfiles
 * @returns {string[]}
 */
export function normaliseSelection(profiles, availableProfiles) {
	const available = new Set(availableProfiles);
	return normaliseProfiles(profiles).filter((profile) => available.has(profile));
}

/**
 * @param {string[]} profiles
 * @param {number} from
 * @param {number} to
 * @returns {string[]}
 */
export function reorderProfiles(profiles, from, to) {
	if (from < 0 || from >= profiles.length || to < 0 || to >= profiles.length || from === to) return profiles;
	const reordered = [...profiles];
	[reordered[from], reordered[to]] = [reordered[to], reordered[from]];
	return reordered;
}
