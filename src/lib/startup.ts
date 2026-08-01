export type StartupTask = "services" | "devices" | "actions";

const taskMessages: Record<StartupTask, string> = {
	services: "Preparing application services…",
	devices: "Detecting connected devices…",
	actions: "Loading actions and plugins…",
};

const pendingTasks = new Set<StartupTask>(Object.keys(taskMessages) as StartupTask[]);
const startedAt = Date.now();
let finishing = false;

function statusElement(): HTMLElement | null {
	if (typeof document == "undefined") return null;
	return document.getElementById("startup-status-text");
}

export function showStartupTask(task: StartupTask): void {
	if (!pendingTasks.has(task)) return;
	const status = statusElement();
	if (status) status.textContent = taskMessages[task];
}

export function completeStartupTask(task: StartupTask): void {
	pendingTasks.delete(task);
	if (finishing) return;

	if (pendingTasks.size > 0) {
		const nextTask = pendingTasks.values().next().value as StartupTask | undefined;
		if (nextTask) showStartupTask(nextTask);
		return;
	}

	finishing = true;
	const status = statusElement();
	if (status) status.textContent = "Finalizing workspace…";

	const minimumVisibleTime = 700;
	const delay = Math.max(0, minimumVisibleTime - (Date.now() - startedAt));
	window.setTimeout(() => {
		const loader = document.getElementById("startup-loader");
		if (!loader) return;
		loader.classList.add("startup-loader--leaving");
		loader.setAttribute("aria-hidden", "true");
		window.setTimeout(() => loader.remove(), 220);
	}, delay);
}
