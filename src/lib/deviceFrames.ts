import type { Context } from "./Context";
import { DeviceFrameCoordinator } from "./DeviceFrameCoordinator";

import { invoke } from "@tauri-apps/api/core";

const coordinator = new DeviceFrameCoordinator((frames) => invoke("update_images", { frames }));

export function beginInitialDeviceRender(device: string, profile: string, expectedFrames: number) {
	coordinator.beginInitialRender(device, profile, expectedFrames);
}

export function queueDeviceFrame(context: Context, image: string | null) {
	coordinator.queue({ context: { ...context }, image });
}

export function cancelDeviceFrames(device: string) {
	coordinator.cancel(device);
}
