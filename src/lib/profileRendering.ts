import { writable } from "svelte/store";

let pausedDevices = new Set<string>();
const store = writable<ReadonlySet<string>>(pausedDevices);

export const pausedProfileRenderingDevices = {
	subscribe: store.subscribe,
};

export function pauseProfileRendering(device: string): boolean {
	if (pausedDevices.has(device)) return false;
	pausedDevices = new Set(pausedDevices).add(device);
	store.set(pausedDevices);
	return true;
}

export function resumeProfileRendering(device: string) {
	if (!pausedDevices.has(device)) return;
	pausedDevices = new Set(pausedDevices);
	pausedDevices.delete(device);
	store.set(pausedDevices);
}

export function getPausedProfileRenderingDevices(): string[] {
	return [...pausedDevices];
}
