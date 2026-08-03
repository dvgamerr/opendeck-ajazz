import type { Context } from "./Context";

export type DeviceFrame = {
	context: Context;
	image: string | null;
};

type BatchSender = (frames: DeviceFrame[]) => Promise<void>;

type DeviceRenderState = {
	profile: string;
	expectedInitialFrames: number;
	collectingInitialFrames: boolean;
	initialFrames: Map<string, DeviceFrame>;
	liveFrames: Map<string, DeviceFrame>;
	pendingFrames: Map<string, DeviceFrame>;
	initialTimer?: ReturnType<typeof setTimeout>;
	liveTimer?: ReturnType<typeof setTimeout>;
	drainPromise?: Promise<void>;
	cancelled: boolean;
};

const frameKey = (context: Context) => `${context.controller}:${context.position}`;

export class DeviceFrameCoordinator {
	private states = new Map<string, DeviceRenderState>();

	constructor(
		private readonly sendBatch: BatchSender,
		private readonly initialTimeoutMs = 750,
		private readonly liveWindowMs = 16,
		private readonly reportError: (error: unknown) => void = (error) => console.warn("Failed to update device images", error),
	) {}

	beginInitialRender(device: string, profile: string, expectedFrames: number) {
		this.cancel(device);
		const state: DeviceRenderState = {
			profile,
			expectedInitialFrames: Math.max(0, expectedFrames),
			collectingInitialFrames: expectedFrames > 0,
			initialFrames: new Map(),
			liveFrames: new Map(),
			pendingFrames: new Map(),
			cancelled: false,
		};
		this.states.set(device, state);

		if (state.collectingInitialFrames) {
			state.initialTimer = setTimeout(() => this.flushInitial(device, state), this.initialTimeoutMs);
		}
	}

	queue(frame: DeviceFrame) {
		let state = this.states.get(frame.context.device);
		if (state && state.profile != frame.context.profile) return;
		if (!state) {
			state = {
				profile: frame.context.profile,
				expectedInitialFrames: 0,
				collectingInitialFrames: false,
				initialFrames: new Map(),
				liveFrames: new Map(),
				pendingFrames: new Map(),
				cancelled: false,
			};
			this.states.set(frame.context.device, state);
		}

		if (state.collectingInitialFrames) {
			state.initialFrames.set(frameKey(frame.context), frame);
			if (state.initialFrames.size >= state.expectedInitialFrames) {
				this.flushInitial(frame.context.device, state);
			}
			return;
		}

		state.liveFrames.set(frameKey(frame.context), frame);
		if (state.liveTimer === undefined) {
			state.liveTimer = setTimeout(() => this.flushLive(frame.context.device, state), this.liveWindowMs);
		}
	}

	cancel(device: string) {
		const state = this.states.get(device);
		if (!state) return;
		if (state.initialTimer !== undefined) clearTimeout(state.initialTimer);
		if (state.liveTimer !== undefined) clearTimeout(state.liveTimer);
		state.cancelled = true;
		state.initialFrames.clear();
		state.liveFrames.clear();
		state.pendingFrames.clear();
		this.states.delete(device);
	}

	async flushPending(device: string) {
		const state = this.states.get(device);
		if (!state) return;
		if (state.collectingInitialFrames) this.flushInitial(device, state);
		else this.flushLive(device, state);
		while (state.drainPromise !== undefined) await state.drainPromise;
	}

	private flushInitial(device: string, state: DeviceRenderState) {
		if (this.states.get(device) !== state || !state.collectingInitialFrames) return;
		if (state.initialTimer !== undefined) clearTimeout(state.initialTimer);
		state.initialTimer = undefined;
		state.collectingInitialFrames = false;
		const frames = [...state.initialFrames.values()];
		state.initialFrames.clear();
		this.enqueue(state, frames);
	}

	private flushLive(device: string, state: DeviceRenderState) {
		if (this.states.get(device) !== state || state.collectingInitialFrames) return;
		if (state.liveTimer !== undefined) clearTimeout(state.liveTimer);
		state.liveTimer = undefined;
		const frames = [...state.liveFrames.values()];
		state.liveFrames.clear();
		this.enqueue(state, frames);
	}

	private enqueue(state: DeviceRenderState, frames: DeviceFrame[]) {
		if (state.cancelled || frames.length == 0) return;
		for (const frame of frames) state.pendingFrames.set(frameKey(frame.context), frame);
		if (state.drainPromise === undefined) state.drainPromise = this.drain(state);
	}

	private async drain(state: DeviceRenderState) {
		try {
			while (!state.cancelled && state.pendingFrames.size > 0) {
				const frames = [...state.pendingFrames.values()];
				state.pendingFrames.clear();
				try {
					await this.sendBatch(frames);
				} catch (error) {
					this.reportError(error);
				}
			}
		} finally {
			state.drainPromise = undefined;
			if (!state.cancelled && state.pendingFrames.size > 0) state.drainPromise = this.drain(state);
		}
	}
}
