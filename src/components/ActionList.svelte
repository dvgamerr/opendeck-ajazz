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

<div class="flex min-h-0 flex-1 flex-col">
	<div class="shrink-0 space-y-3 border-b border-base-300 p-4">
		<label class="input input-sm w-full bg-base-200 focus-within:input-primary">
			<MagnifyingGlass size="15" class="shrink-0 opacity-60" color="currentColor" />
			<input bind:value={query} class="grow" placeholder="Search actions" type="search" spellcheck="false" />
		</label>
		<div class="tabs tabs-box grid grid-cols-2" role="tablist" aria-label="Action controller">
			<button type="button" role="tab" class="tab gap-2" class:tab-active={controller == "Keypad"} on:click={() => (controller = "Keypad")}>
				<span class="h-3.5 w-3.5 rounded-[4px] border-2 border-current"></span>
				Keys
			</button>
			<button type="button" role="tab" class="tab gap-2" class:tab-active={controller == "Encoder"} on:click={() => (controller = "Encoder")}>
				<span class="h-3.5 w-3.5 rounded-full border-2 border-current"></span>
				Dials
			</button>
		</div>
	</div>

	<ul class="menu min-h-0 flex-1 flex-nowrap overflow-y-auto px-3 py-2 select-none">
		{#each filteredCategories as [name, { icon, actions }]}
			<li>
				<details open>
					<summary class="text-xs font-semibold">
						{#if icon || (actions[0] && plugins.find((x) => x.id == actions[0].plugin) && categories[name].actions.every((x) => x.plugin == actions[0].plugin))}
							<img
								src={icon ? (!icon.startsWith("opendeck/") ? getWebserverUrl(icon) : icon.replace("opendeck", "")) : getWebserverUrl(plugins.find((x) => x.id == actions[0].plugin).icon)}
								alt=""
								class="size-5 rounded"
							/>
						{/if}
						<span class="min-w-0 flex-1 truncate">{name}</span>
					</summary>
					<ul>
						{#each actions as action}
							<li>
								<button
									type="button"
									class="cursor-grab gap-3 py-1.5 active:cursor-grabbing"
									draggable="true"
									title={$localisations?.[action.plugin]?.[action.uuid]?.Tooltip ?? action.tooltip}
									on:dragstart={(event) => handleActionDragStart(event, action)}
								>
									<img
										src={!action.icon.startsWith("opendeck/") ? getWebserverUrl(action.icon) : action.icon.replace("opendeck", "")}
										alt=""
										class="pointer-events-none size-8 shrink-0 rounded-field object-cover"
									/>
									<span class="min-w-0 truncate">{$localisations?.[action.plugin]?.[action.uuid]?.Name ?? action.name}</span>
								</button>
							</li>
						{/each}
					</ul>
				</details>
			</li>
		{/each}
	</ul>
</div>
