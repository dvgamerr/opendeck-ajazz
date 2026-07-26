<script lang="ts">
	import type { Action } from "$lib/Action";

	import MagnifyingGlass from "phosphor-svelte/lib/MagnifyingGlass";

	import { getWebserverUrl } from "$lib/ports";
	import { openContextMenu } from "$lib/propertyInspector";
	import { localisations } from "$lib/settings";
	import { PRODUCT_NAME } from "$lib/singletons";

	import { invoke } from "@tauri-apps/api/core";

	let categories: { [name: string]: { icon?: string; actions: Action[] } } = {};
	let plugins: any[] = [];
	export async function reload() {
		categories = await invoke("get_categories");
		plugins = await invoke("list_plugins");
	}
	reload();

	let query: string = "";
	let controller: "Keypad" | "Encoder" = "Keypad";
	const actionMime = "application/x-opendeck-action";

	function handleActionDragStart(event: DragEvent, action: Action) {
		if (!event.dataTransfer) return;
		openContextMenu.set(null);
		const serialized = JSON.stringify(action);
		event.dataTransfer.effectAllowed = "copy";
		event.dataTransfer.setData(actionMime, serialized);
		// Retain the original payload for compatibility with existing drop targets.
		event.dataTransfer.setData("action", serialized);
	}

	let filteredCategories: [string, { icon?: string; actions: Action[] }][] = [];
	$: {
		let lowerCaseQuery = query.toLowerCase().trim();
		filteredCategories = Object.entries(categories)
			.sort((a, b) => (a[0] == PRODUCT_NAME ? -1 : b[0] == PRODUCT_NAME ? 1 : a[0].localeCompare(b[0])))
			.map(([categoryName, { icon, actions }]): [string, { icon?: string; actions: Action[] }] => {
				actions = actions.filter((action) => action.controllers.includes(controller));
				if (!categoryName.toLowerCase().includes(lowerCaseQuery)) {
					actions = actions.filter((action) => action.name.toLowerCase().includes(lowerCaseQuery));
				}
				return [categoryName, { icon, actions }];
			})
			.filter(([_, { actions }]) => actions.length > 0);
	}
</script>

<div class="action-library">
	<div class="action-library-header">
		<div class="action-search">
			<MagnifyingGlass size="15" class="ml-3 shrink-0" color="currentColor" />
			<input
				bind:value={query}
				class="min-w-0 flex-1 bg-transparent px-2 py-1.5 text-sm text-neutral-700 outline-hidden dark:text-neutral-200"
				placeholder="Search actions"
				type="search"
				spellcheck="false"
			/>
		</div>
		<div class="action-tabs" aria-label="Action controller">
			<button class:action-tab-active={controller == "Keypad"} class="action-tab" on:click={() => (controller = "Keypad")}>
				<span class="h-3.5 w-3.5 rounded-[4px] border-2 border-current"></span>
				Keys
			</button>
			<button class:action-tab-active={controller == "Encoder"} class="action-tab" on:click={() => (controller = "Encoder")}>
				<span class="h-3.5 w-3.5 rounded-full border-2 border-current"></span>
				Dials
			</button>
		</div>
	</div>

	<div class="action-list-scroll">
		{#each filteredCategories as [name, { icon, actions }]}
			<details open class="action-category">
				<summary>
					{#if icon || (actions[0] && plugins.find((x) => x.id == actions[0].plugin) && categories[name].actions.every((x) => x.plugin == actions[0].plugin))}
						<img
							src={icon ? (!icon.startsWith("opendeck/") ? getWebserverUrl(icon) : icon.replace("opendeck", "")) : getWebserverUrl(plugins.find((x) => x.id == actions[0].plugin).icon)}
							alt=""
							class="h-5 w-5 rounded"
						/>
					{/if}
					<span class="min-w-0 flex-1 truncate">{name}</span>
				</summary>
				{#each actions as action}
					<div
						class="action-row"
						role="group"
						draggable="true"
						title={$localisations?.[action.plugin]?.[action.uuid]?.Tooltip ?? action.tooltip}
						on:dragstart={(event) => handleActionDragStart(event, action)}
					>
						<img
							src={!action.icon.startsWith("opendeck/") ? getWebserverUrl(action.icon) : action.icon.replace("opendeck", "")}
							alt={$localisations?.[action.plugin]?.[action.uuid]?.Tooltip ?? action.tooltip}
							class="h-8 w-8 shrink-0 rounded-md object-cover pointer-events-none"
						/>
						<span class="min-w-0 truncate">{$localisations?.[action.plugin]?.[action.uuid]?.Name ?? action.name}</span>
					</div>
				{/each}
			</details>
		{/each}
	</div>
</div>
