<script lang="ts">
	import type { ActionInstance } from "$lib/ActionInstance";
	import type { Profile } from "$lib/Profile";

	import Trash from "phosphor-svelte/lib/Trash";
	import Key from "./Key.svelte";

	import { inspectedInstance, inspectedParentAction } from "$lib/propertyInspector";

	import { invoke } from "@tauri-apps/api/core";

	export let profile: Profile;

	let children: ActionInstance[];
	$: children = profile.keys[$inspectedParentAction!.position]!.children!;
	let parentUuid: string;
	$: parentUuid = profile.keys[$inspectedParentAction!.position]!.action.uuid;

	function handleDragOver(event: DragEvent) {
		event.preventDefault();
		return true;
	}

	async function handleDrop({ dataTransfer }: DragEvent) {
		if (dataTransfer?.getData("action")) {
			let action = JSON.parse(dataTransfer?.getData("action"));
			if (
				(parentUuid == "opendeck.multiaction" && !action.supported_in_multi_actions) ||
				(parentUuid == "opendeck.toggleaction" && (action.uuid == "opendeck.multiaction" || action.uuid == "opendeck.toggleaction"))
			) {
				return;
			}
			let response: ActionInstance | null = await invoke("create_instance", { context: $inspectedParentAction, action });
			if (response) profile.keys[$inspectedParentAction!.position]!.children = [...children, response];
		}
	}

	async function removeInstance(index: number) {
		await invoke("remove_instance", { context: children[index].context });
		children.splice(index, 1);
		profile.keys[$inspectedParentAction!.position]!.children = children;
	}
</script>

<svelte:window
	on:keydown={(event) => {
		if (event.key == "Escape") $inspectedParentAction = null;
	}}
/>

<header class="flex items-center border-b border-base-300 px-4 py-3">
	<div>
		<p class="ui-eyebrow">Action flow</p>
		<h1 class="ui-page-title">{parentUuid == "opendeck.toggleaction" ? "Toggle Action" : "Multi Action"}</h1>
	</div>
	<button type="button" class="btn btn-circle btn-ghost ml-auto" aria-label="Close action flow" on:click={() => ($inspectedParentAction = null)}>✕</button>
</header>

<!-- svelte-ignore a11y-no-static-element-interactions -->
<div class="flex h-80 flex-col gap-2 overflow-auto p-4" on:click={() => inspectedInstance.set(null)} on:keyup={() => inspectedInstance.set(null)}>
	{#each children as instance, index}
		<div class="card card-side flex-row items-center border border-base-300 bg-base-100 px-3">
			<Key inslot={instance} context={null} active={false} scale={3 / 4} />
			<p class="ml-3 font-medium">{instance.action.name}</p>
			<button type="button" class="btn btn-circle btn-ghost btn-sm ml-auto text-error" aria-label="Remove action" on:click={() => removeInstance(index)}>
				<Trash size="20" />
			</button>
		</div>
	{/each}
	<div class="card flex-row items-center border-2 border-dashed border-base-300 bg-base-100 p-3" on:dragover={handleDragOver} on:drop={handleDrop}>
		<img src="/cube.png" class="m-2 w-16 rounded-box" alt="" />
		<div class="ml-3">
			<p class="ui-label">Drop actions here</p>
			<p class="ui-muted">Drag an action from the library to add it to this flow.</p>
		</div>
	</div>
</div>
