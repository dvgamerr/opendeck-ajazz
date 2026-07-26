<script lang="ts">
	import { PRODUCT_NAME } from "$lib/singletons";

	import { invoke } from "@tauri-apps/api/core";

	let buildInfo: string;
	(async () => (buildInfo = await invoke("get_build_info")))();
</script>

<div class="flex h-full w-full flex-col items-center justify-center text-center text-neutral-700 dark:text-neutral-300">
	<div class="w-80 rounded-2xl border border-neutral-200 bg-white p-7 text-sm shadow-xl dark:border-[#414141] dark:bg-[#2d2d2d]">
		<div class="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-xl bg-neutral-100 text-2xl dark:bg-[#3a3a3a]">⌁</div>
		<h2 class="mb-2 text-lg font-bold">No devices detected</h2>
		<p class="mb-2 text-neutral-500 dark:text-neutral-400">Make sure your devices are connected properly and you have permission to access them.</p>
		{#if buildInfo?.includes("linux")}
			<p class="mb-2">Ensure you have the correct udev subsystem rules installed.</p>
		{/if}
		<p class="mb-5 text-neutral-500 dark:text-neutral-400">You may need to install a plugin that adds support for your device.</p>
		<button class="rounded-lg bg-indigo-600 px-4 py-2 text-sm font-medium text-white hover:bg-indigo-500" on:click={() => invoke("restart")}>
			Restart {PRODUCT_NAME}
		</button>
	</div>
</div>
