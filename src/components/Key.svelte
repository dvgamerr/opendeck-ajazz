<script lang="ts">
	import type { ActionInstance } from "$lib/ActionInstance";
	import type { ActionState } from "$lib/ActionState";
	import type { Context } from "$lib/Context";

	import Clipboard from "phosphor-svelte/lib/Clipboard";
	import Copy from "phosphor-svelte/lib/Copy";
	import Pencil from "phosphor-svelte/lib/Pencil";
	import Trash from "phosphor-svelte/lib/Trash";
	import InstanceEditor from "./InstanceEditor.svelte";

	import { copiedContext, inspectedInstance, inspectedParentAction, openContextMenu } from "$lib/propertyInspector";
	import { CanvasLock, renderImage } from "$lib/rendererHelper";

	import { invoke } from "@tauri-apps/api/core";
	import { listen, type UnlistenFn } from "@tauri-apps/api/event";
	import { onMount } from "svelte";

	export let context: Context | null;

	// One-way binding for slot data.
	export let inslot: ActionInstance | null;
	let slot: ActionInstance | null;
	const update = (inslot: ActionInstance | null) => {
		if (inslot && context && inslot.context.split(".")[0] != context.device) return;
		slot = inslot;
	};
	$: update(inslot);

	export let active: boolean = true;
	export let scale: number = 1;
	export let appearance: "auto" | "key" | "encoder" | "touch" = "auto";
	export let renderWidth: number | undefined = undefined;
	export let renderHeight: number | undefined = undefined;
	let pressed: boolean = false;
	$: resolvedAppearance = appearance == "auto" ? (context?.controller == "Encoder" ? "encoder" : "key") : appearance;

	let state: ActionState | undefined;
	$: {
		if (!slot) {
			state = undefined;
		} else {
			state = slot.states[slot.current_state];
		}
	}

	function select(event: MouseEvent | KeyboardEvent) {
		if (event instanceof MouseEvent && event.ctrlKey) return;
		if (!slot) return;
		if (slot.action.uuid == "opendeck.multiaction" || slot.action.uuid == "opendeck.toggleaction") {
			inspectedParentAction.set(context);
		} else {
			inspectedInstance.set(slot.context);
		}
	}

	async function contextMenu(event: MouseEvent) {
		event.preventDefault();
		if (!active || !context) return;
		const width = 128;
		const height = slot ? 120 : 44;
		$openContextMenu = {
			context,
			x: Math.max(8, Math.min(event.clientX, window.innerWidth - width - 8)),
			y: Math.max(8, Math.min(event.clientY, window.innerHeight - height - 8)),
		};
	}

	let showEditor = false;
	function edit() {
		showEditor = true;
	}

	export let handlePaste: ((source: Context, destination: Context) => void) | undefined = undefined;
	async function paste() {
		if (!$copiedContext || !context) return;
		if (handlePaste) handlePaste($copiedContext, context);
	}

	async function clear() {
		if (!slot) return;
		await invoke("remove_instance", { context: slot.context });
		if ($inspectedInstance == slot.context) inspectedInstance.set(null);
		showEditor = false;
		slot = null;
		inslot = slot;
	}

	let showAlert: boolean = false;
	let showOk: boolean = false;
	let timeouts: number[] = [];
	onMount(() => {
		let disposed = false;
		const unlisteners: UnlistenFn[] = [];
		const keep = (promise: Promise<UnlistenFn>) => {
			void promise.then((unlisten) => (disposed ? unlisten() : unlisteners.push(unlisten)));
		};

		keep(
			listen("update_state", ({ payload }: { payload: { context: string; contents: ActionInstance | null } }) => {
				if (payload.context == slot?.context) slot = payload.contents;
			}),
		);
		keep(
			listen("key_moved", ({ payload }: { payload: { context: Context; pressed: boolean } }) => {
				if (JSON.stringify(context) == JSON.stringify(payload.context)) pressed = payload.pressed;
			}),
		);
		keep(
			listen("show_alert", ({ payload }: { payload: string }) => {
				if (!slot || payload != slot.context) return;
				timeouts.forEach(clearTimeout);
				timeouts = [];
				showOk = false;
				showAlert = true;
				timeouts.push(window.setTimeout(() => (showAlert = false), 1.5e3));
			}),
		);
		keep(
			listen("show_ok", ({ payload }: { payload: string }) => {
				if (!slot || payload != slot.context) return;
				timeouts.forEach(clearTimeout);
				timeouts = [];
				showAlert = false;
				showOk = true;
				timeouts.push(window.setTimeout(() => (showOk = false), 1.5e3));
			}),
		);

		return () => {
			disposed = true;
			unlisteners.forEach((unlisten) => unlisten());
			timeouts.forEach(clearTimeout);
			timeouts = [];
		};
	});

	let canvas: HTMLCanvasElement;
	let lock = new CanvasLock();
	export let size = 144;
	$: canvasWidth = renderWidth ?? size;
	$: canvasHeight = renderHeight ?? size;
	$: canvasStyle = resolvedAppearance == "touch" ? "width: 160px; height: 100px;" : `transform: scale(${(112 / size) * scale});`;
	$: (async () => {
		const sl = structuredClone(slot);
		if (!sl) {
			if (canvas) {
				let context = canvas.getContext("2d");
				if (context) context.clearRect(0, 0, canvas.width, canvas.height);
			}
		} else {
			const unlock = await lock.lock();
			try {
				let fallback = sl.action.states[sl.current_state]?.image ?? sl.action.icon;
				if (state) await renderImage(canvas, context, state, fallback, showOk, showAlert, true, active, pressed);
			} finally {
				unlock();
			}
		}
	})();
</script>

<canvas
	bind:this={canvas}
	class={`key-canvas key-canvas--${resolvedAppearance} relative block outline-none outline-offset-2 outline-indigo-500`}
	class:-m-2={resolvedAppearance != "touch"}
	class:border-2={resolvedAppearance != "touch"}
	class:dark:border-neutral-700={resolvedAppearance != "touch"}
	class:rounded-md={resolvedAppearance == "key"}
	class:outline-solid={slot && $inspectedInstance == slot.context}
	class:-m-[2.06rem]={resolvedAppearance != "touch" && size == 192}
	class:rounded-full!={resolvedAppearance == "encoder"}
	width={canvasWidth}
	height={canvasHeight}
	style={canvasStyle}
	draggable={slot != null}
	on:dragstart
	on:dragover
	on:drop
	on:click|stopPropagation={select}
	on:keyup|stopPropagation={select}
	on:contextmenu={contextMenu}
/>

{#if $openContextMenu && $openContextMenu?.context == context}
	<div
		class="fixed z-50 w-32 divide-y rounded-lg border-2 bg-neutral-100 text-sm font-semibold shadow-xl dark:border-neutral-600 dark:bg-neutral-700 dark:text-neutral-300"
		style={`left: ${$openContextMenu.x}px; top: ${$openContextMenu.y}px;`}
	>
		{#if !slot}
			<button class="flex flex-row p-2 w-full cursor-pointer items-center" on:click={paste}>
				<Clipboard size="18" color={document.documentElement.classList.contains("dark") ? "#DEDDDA" : "#77767B"} />
				<span class="ml-2"> Paste </span>
			</button>
		{:else}
			<button class="flex flex-row p-2 w-full cursor-pointer items-center" on:click={edit}>
				<Pencil size="18" color={document.documentElement.classList.contains("dark") ? "#DEDDDA" : "#77767B"} />
				<span class="ml-2"> Edit </span>
			</button>
			<button class="flex flex-row p-2 w-full cursor-pointer items-center" on:click={() => copiedContext.set(context)}>
				<Copy size="18" color={document.documentElement.classList.contains("dark") ? "#DEDDDA" : "#77767B"} />
				<span class="ml-2"> Copy </span>
			</button>
			<button class="flex flex-row p-2 w-full cursor-pointer items-center" on:click={clear}>
				<Trash size="18" color="#F66151" />
				<span class="ml-2"> Delete </span>
			</button>
		{/if}
	</div>
{/if}

{#if slot && showEditor}
	<InstanceEditor bind:instance={slot} bind:showEditor />
{/if}
