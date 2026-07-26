<script lang="ts">
	import type { ActionInstance } from "$lib/ActionInstance";
	import type { Context } from "$lib/Context";
	import type { DeviceInfo } from "$lib/DeviceInfo";
	import type { Profile } from "$lib/Profile";

	import Key from "./Key.svelte";

	import { inspectedInstance, inspectedParentAction, openContextMenu } from "$lib/propertyInspector";

	import { invoke } from "@tauri-apps/api/core";

	export let device: DeviceInfo;
	export let profile: Profile;

	export let selectedDevice: string;
	const actionMime = "application/x-opendeck-action";

	function handleDragStart({ dataTransfer }: DragEvent, controller: string, position: number) {
		if (!dataTransfer) return;
		openContextMenu.set(null);
		dataTransfer.effectAllowed = "copyMove";
		dataTransfer?.setData("controller", controller);
		dataTransfer?.setData("position", position.toString());
	}

	function handleDragOver(event: DragEvent) {
		if (!event.dataTransfer) return false;
		event.preventDefault();
		event.dataTransfer.dropEffect = event.dataTransfer.types.includes("controller") ? "move" : "copy";
		return true;
	}

	async function handleDrop(event: DragEvent, controller: string, position: number) {
		event.preventDefault();
		event.stopPropagation();
		openContextMenu.set(null);
		const { dataTransfer } = event;
		if (!dataTransfer) return;

		let context = { device: device.id, profile: profile.id, controller, position };
		let array = controller == "Encoder" ? profile.sliders : profile.keys;
		const serializedAction = dataTransfer.getData(actionMime) || dataTransfer.getData("action");
		if (serializedAction) {
			let action = JSON.parse(serializedAction);
			if (!action.controllers?.includes(controller)) return;
			if (array[position]) {
				await invoke("remove_instance", { context: array[position]!.context });
			}
			const instance: ActionInstance | null = await invoke("create_instance", { context, action });
			if (instance) {
				array[position] = instance;
				profile = profile;
			}
		} else if (dataTransfer.getData("controller")) {
			let oldArray = dataTransfer.getData("controller") == "Encoder" ? profile.sliders : profile.keys;
			let oldPosition = parseInt(dataTransfer.getData("position"));
			if (oldArray == array && oldPosition == position) return;
			let response: ActionInstance = await invoke("move_instance", {
				source: { device: device.id, profile: profile.id, controller: dataTransfer.getData("controller"), position: oldPosition },
				destination: context,
				retain: false,
			});
			if (response) {
				array[position] = response;
				oldArray[oldPosition] = null;
				profile = profile;
			}
		}
	}

	async function handlePaste(source: Context, destination: Context) {
		let response: ActionInstance = await invoke("move_instance", { source, destination, retain: true });
		if (response) {
			(destination.controller == "Encoder" ? profile.sliders : profile.keys)[destination.position] = response;
			profile = profile;
		}
	}
</script>

{#key device}
	<!-- svelte-ignore a11y-no-static-element-interactions -->
	<div class="device-chassis" class:hidden={$inspectedParentAction || selectedDevice != device.id} on:click={() => inspectedInstance.set(null)} on:keyup={() => inspectedInstance.set(null)}>
		<div class="device-key-grid">
			{#each { length: device.rows } as _, r}
				<div class="flex flex-row">
					{#each { length: device.columns } as _, c}
						<Key
							context={{ device: device.id, profile: profile.id, controller: "Keypad", position: r * device.columns + c }}
							bind:inslot={profile.keys[r * device.columns + c]}
							on:dragover={handleDragOver}
							on:drop={(event) => handleDrop(event, "Keypad", r * device.columns + c)}
							on:dragstart={(event) => handleDragStart(event, "Keypad", r * device.columns + c)}
							{handlePaste}
							size={device.id.startsWith("sd-") && device.rows == 4 && device.columns == 8 ? 192 : 144}
						/>
					{/each}
				</div>
			{/each}
		</div>

		{#if device.type == 7}
			<div class="touch-strip">
				{#each { length: device.encoders } as _, i}
					<Key
						context={{ device: device.id, profile: profile.id, controller: "Encoder", position: i }}
						bind:inslot={profile.sliders[i]}
						on:dragover={handleDragOver}
						on:drop={(event) => handleDrop(event, "Encoder", i)}
						on:dragstart={(event) => handleDragStart(event, "Encoder", i)}
						{handlePaste}
						appearance="touch"
						renderWidth={176}
						renderHeight={112}
					/>
				{/each}
			</div>
			<div class="knob-row" aria-hidden="true">
				{#each { length: device.encoders } as _}
					<div class="knob-slot">
						<div class="device-knob"></div>
					</div>
				{/each}
			</div>
		{:else}
			<div class="flex flex-row">
				{#each { length: device.encoders } as _, i}
					<Key
						context={{ device: device.id, profile: profile.id, controller: "Encoder", position: i }}
						bind:inslot={profile.sliders[i]}
						on:dragover={handleDragOver}
						on:drop={(event) => handleDrop(event, "Encoder", i)}
						on:dragstart={(event) => handleDragStart(event, "Encoder", i)}
						{handlePaste}
						size={device.id.startsWith("sd-") && device.rows == 4 && device.columns == 8 ? 192 : 144}
					/>
				{/each}
			</div>
		{/if}
	</div>
{/key}
