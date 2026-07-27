<script lang="ts">
	import type { ActionInstance } from "$lib/ActionInstance";

	import { getImage, resizeImage } from "$lib/rendererHelper";

	import { invoke } from "@tauri-apps/api/core";

	export let instance: ActionInstance;
	export let showEditor: boolean;

	let state: number = 0;
	let bold: boolean;
	let italic: boolean;

	let fileInput: HTMLInputElement;
	let colourInput: HTMLInputElement;

	function portalToPreviewDock(node: HTMLElement) {
		const previewDock = document.querySelector<HTMLElement>(".device-workspace");
		previewDock?.appendChild(node);

		return {
			destroy() {
				node.remove();
			},
		};
	}

	function update(instance: ActionInstance) {
		bold = instance.states[state].style.includes("Bold");
		italic = instance.states[state].style.includes("Italic");
	}
	$: update(instance);
	$: invoke("set_state", { instance, state });
</script>

<svelte:window
	on:keydown={(event) => {
		if (event.key == "Escape") showEditor = false;
	}}
/>

<div use:portalToPreviewDock class="absolute inset-0 z-[120]">
	<button type="button" class="absolute inset-0 cursor-default bg-black/60 backdrop-blur-sm" aria-label="Close key editor" on:click={() => (showEditor = false)}></button>

	<div
		class="absolute inset-4 overflow-y-auto rounded-xl border border-neutral-300 bg-neutral-100 p-5 text-neutral-700 shadow-2xl dark:border-[#484848] dark:bg-[#252525] dark:text-neutral-300"
		role="dialog"
		aria-modal="true"
		aria-label="Edit key appearance"
	>
		<div class="mb-5 flex flex-row items-center border-b border-neutral-200 pb-4 dark:border-[#424242]">
			<h2 class="mr-5 text-lg font-semibold text-neutral-900 dark:text-neutral-100">Edit key appearance</h2>
			<div class="select-wrapper ml-auto w-48">
				<select class="w-full" bind:value={state}>
					{#each instance.states as _, i}
						<option value={i}>State {i + 1}</option>
					{/each}
				</select>
			</div>
			<button class="ml-3 rounded-lg p-2 text-xl leading-none hover:bg-neutral-200 dark:text-neutral-300 dark:hover:bg-[#383838]" on:click={() => (showEditor = false)}>✕</button>
		</div>

		<div class="flex flex-row">
			<div class="flex flex-col justify-center items-center">
				<button
					on:click={() => fileInput.click()}
					on:contextmenu={(event) => {
						event.preventDefault();
						instance.states[state].image = instance.action.states[state].image;
					}}
				>
					<img
						src={getImage(instance.states[state].image, instance.action.states[state]?.image ?? instance.action.icon)}
						class="my-auto p-1 w-32 h-min aspect-square rounded-xl cursor-pointer"
						alt="State {state}"
					/>
				</button>
				<button on:click={() => colourInput.click()} class="mt-0.5 px-0.5 text-sm text-neutral-700 dark:text-neutral-400 bg-neutral-200 dark:bg-neutral-600 rounded-md outline-hidden">
					Solid colour
				</button>
			</div>
			<input
				bind:this={fileInput}
				type="file"
				class="hidden"
				accept="image/*"
				on:change={async () => {
					if (!fileInput.files || fileInput.files.length == 0) return;
					const reader = new FileReader();

					reader.onload = async () => {
						let result = reader.result?.toString();
						if (result) {
							let resized = await resizeImage(result);
							if (resized) instance.states[state].image = resized;
							else instance.states[state].image = result;
						}
					};

					reader.readAsDataURL(fileInput.files[0]);
				}}
			/>
			<input
				bind:this={colourInput}
				type="color"
				class="invisible w-0 h-0"
				value="#FFFFFE"
				on:change={() => {
					const canvas = document.createElement("canvas");
					canvas.width = 1;
					canvas.height = 1;
					const context = canvas.getContext("2d");
					if (!context) return;
					context.fillStyle = colourInput.value;
					context.fillRect(0, 0, canvas.width, canvas.height);
					instance.states[state].image = canvas.toDataURL("image/png");
				}}
			/>

			<div class="flex flex-col pl-2 pr-1 pt-4 pb-2 space-y-2">
				<div class="flex flex-row space-x-2">
					<span> Text </span>
					<textarea bind:value={instance.states[state].text} rows="1" class="w-full px-1 dark:text-neutral-300 bg-neutral-200 dark:bg-neutral-600 rounded-md outline-hidden resize-none"></textarea>
				</div>
				<div class="flex flex-row items-center">
					<span class="mr-2"> Colour </span>
					<input type="color" bind:value={instance.states[state].colour} class="mr-2 px-0.5 bg-neutral-200 dark:bg-neutral-600 rounded-md outline-hidden" />
					<span class="mr-2"> Show </span>
					<input type="checkbox" bind:checked={instance.states[state].show} class="mr-4 mt-1 scale-125" />
					<select bind:value={instance.states[state].alignment} class="px-1! py-0.5!">
						<option value="top">Top</option>
						<option value="middle">Middle</option>
						<option value="bottom">Bottom</option>
					</select>
				</div>
				<div class="flex flex-row">
					<span class="mr-2"> Font </span>
					<input
						list="families"
						bind:value={instance.states[state].family}
						placeholder="Font family"
						class="w-full px-1 dark:text-neutral-300 bg-neutral-200 dark:bg-neutral-600 rounded-md outline-hidden"
					/>
					<datalist id="families">
						<option value="Liberation Sans">Liberation Sans</option>
						<option value="Archivo Black">Archivo Black</option>
						<option value="Comic Neue">Comic Neue</option>
						<option value="Courier Prime">Courier Prime</option>
						<option value="Tinos">Tinos</option>
						<option value="Anton">Anton</option>
						<option value="Liberation Serif">Liberation Serif</option>
						<option value="Open Sans">Open Sans</option>
						<option value="Fira Sans">Fira Sans</option>
					</datalist>
				</div>
				<div class="flex flex-row">
					<span class="mr-3 font-bold"> B </span>
					<input
						type="checkbox"
						bind:checked={bold}
						on:change={() => (instance.states[state].style = bold && italic ? "Bold Italic" : bold ? "Bold" : italic ? "Italic" : "Regular")}
						class="mr-4 mt-1 scale-125"
					/>
					<span class="mr-3 italic"> I </span>
					<input
						type="checkbox"
						bind:checked={italic}
						on:change={() => (instance.states[state].style = bold && italic ? "Bold Italic" : bold ? "Bold" : italic ? "Italic" : "Regular")}
						class="mr-4 mt-1 scale-125"
					/>
					<span class="mr-3 underline"> U </span>
					<input type="checkbox" bind:checked={instance.states[state].underline} class="mr-4 mt-1 scale-125" />
					<span class="mr-2"> Size </span>
					<input type="number" bind:value={instance.states[state].size} class="px-0.5 w-14 dark:text-neutral-300 bg-neutral-200 dark:bg-neutral-600 rounded-md outline-hidden" />
				</div>
			</div>
		</div>
	</div>
</div>
