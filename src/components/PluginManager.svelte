<script lang="ts">
	import ArrowClockwise from "phosphor-svelte/lib/ArrowClockwise";
	import ArrowSquareOut from "phosphor-svelte/lib/ArrowSquareOut";
	import CloudArrowDown from "phosphor-svelte/lib/CloudArrowDown";
	import FileArrowUp from "phosphor-svelte/lib/FileArrowUp";
	import Gear from "phosphor-svelte/lib/Gear";
	import MagnifyingGlass from "phosphor-svelte/lib/MagnifyingGlass";
	import PuzzlePiece from "phosphor-svelte/lib/PuzzlePiece";
	import Trash from "phosphor-svelte/lib/Trash";
	import WarningCircle from "phosphor-svelte/lib/WarningCircle";
	import ListedPlugin from "./ListedPlugin.svelte";
	import PluginDetails from "./PluginDetails.svelte";
	import Popup from "./Popup.svelte";
	import Tooltip from "./Tooltip.svelte";

	import { getWebserverUrl } from "$lib/ports";
	import { localisations, settings } from "$lib/settings";
	import { actionList, deviceSelector } from "$lib/singletons";

	import { invoke } from "@tauri-apps/api/core";
	import { onOpenUrl } from "@tauri-apps/plugin-deep-link";
	import { ask, message, open } from "@tauri-apps/plugin-dialog";
	import { onMount } from "svelte";

	// @ts-expect-error
	const fetch = window.fetchNative ?? window.fetch;

	let showPopup: boolean;
	onMount(() => {
		let disposed = false;
		let unlistenOpenUrl: (() => void) | undefined;
		const refreshInterval = window.setInterval(async () => {
			if (showPopup) installed = await invoke("list_plugins");
		}, 1e3);
		void onOpenUrl((urls: string[]) => {
			if (!urls[0].includes("installPlugin/")) return;
			const id = urls[0].split("installPlugin/")[1];
			if (plugins?.[id]) void installPluginGitHub(id, plugins[id]);
		}).then((unlisten) => {
			if (disposed) unlisten();
			else unlistenOpenUrl = unlisten;
		});

		return () => {
			disposed = true;
			window.clearInterval(refreshInterval);
			unlistenOpenUrl?.();
		};
	});

	async function installPlugin(name: string, url: string | null, file: string | null, fallback_id: string | null) {
		if (!file && !(await ask(`It may take a while to install the plugin.`, { title: `Install "${name}"?` }))) return;
		try {
			await invoke("install_plugin", { url, file, fallback_id });
			message(`Successfully installed "${name}".`, { title: `Installed "${name}"` });
			$actionList?.reload();
			installed = await invoke("list_plugins");
		} catch (error: any) {
			message(error, { title: `Failed to install "${name}"` });
		}
	}

	let choices: any[] | undefined;
	let choice: number;
	let finishChoice = (_: unknown) => {};
	let cancelChoice = () => {};
	async function chooseAsset(assets: any[]): Promise<any> {
		choices = assets;
		try {
			await new Promise((resolve, reject) => {
				finishChoice = resolve;
				cancelChoice = reject;
			});
		} finally {
			choices = undefined;
			finishChoice = (_: unknown) => {};
			cancelChoice = () => {};
		}
		return assets[choice];
	}

	let openDetailsView: string | null = null;
	type GitHubPlugin = {
		name: string;
		author: string;
		repository: string;
		download_url: string | undefined;
	};
	async function installPluginGitHub(id: string, plugin: GitHubPlugin) {
		if (plugin.download_url) {
			await installPlugin(plugin.name, plugin.download_url, null, id);
			return;
		}

		let endpoint = new URL(plugin.repository);
		endpoint.hostname = "api." + endpoint.hostname;
		endpoint.pathname = "/repos" + endpoint.pathname + "/releases";

		let res;
		try {
			res = await (await fetch(endpoint)).json();
		} catch (error: any) {
			message(error, { title: `Failed to install "${plugin.name}"` });
			return;
		}

		let assets = [];
		for (const asset of res[0].assets) {
			if (asset.name.toLowerCase().endsWith(".streamdeckplugin") || asset.name.toLowerCase().endsWith(".zip")) {
				assets.push(asset);
			}
		}
		let selected;
		if (assets.length == 1) selected = assets[0];
		else {
			try {
				selected = await chooseAsset(assets);
			} catch {
				return;
			}
		}

		await installPlugin(plugin.name, selected.browser_download_url, null, id);
	}

	async function installPluginElgato(plugin: any) {
		await installPlugin(plugin.name, `https://plugins.amankhanna.me/rezipped/${plugin.id}.zip`, null, plugin.id);
	}

	async function installPluginFile() {
		const path = await open({ multiple: false, directory: false });
		if (!path) return;
		await installPlugin(path.replaceAll("\\", "/").split("/").at(-1) ?? path, null, path, null);
	}

	async function removePlugin(plugin: any) {
		if (!(await ask(`Are you sure you want to remove "${plugin.name}"?`, { title: `Remove "${plugin.name}"?` }))) return;
		try {
			await invoke("remove_plugin", { id: plugin.id });
			message(`Successfully removed "${plugin.name}".`, { title: `Removed "${plugin.name}"` });
			$actionList?.reload();
			$deviceSelector?.reloadProfiles();
			installed = await invoke("list_plugins");
		} catch (error: any) {
			message(error, { title: `Failed to remove "${plugin.name}"` });
		}
	}

	let installed: any[] = [];
	(async () => (installed = await invoke("list_plugins")))();

	let plugins: { [id: string]: GitHubPlugin };
	(async () => (plugins = await (await fetch("https://openactionapi.github.io/plugins/catalogue.json")).json()))();

	let query: string = "";
</script>

<button type="button" class="btn btn-ghost btn-sm" title="Manage plugins" on:click={() => (showPopup = true)}>
	<PuzzlePiece size="16" weight="bold" />
	<span>Plugins</span>
</button>

<svelte:window
	on:keydown={(event) => {
		if (event.key == "Escape") {
			if (choices) cancelChoice();
			else if (openDetailsView) openDetailsView = null;
			else showPopup = false;
		}
	}}
/>

<Popup show={showPopup} fullscreen onClose={() => (showPopup = false)}>
	<header class="flex items-center border-b border-base-300 pb-4">
		<div>
			<p class="text-xs font-semibold tracking-widest text-base-content/50">OPENDECK</p>
			<h2 class="text-2xl font-semibold">Manage plugins</h2>
		</div>
		<button type="button" class="btn btn-circle btn-ghost ml-auto" aria-label="Close plugin manager" on:click={() => (showPopup = false)}>✕</button>
	</header>

	<div class="mt-6 flex items-center gap-2">
		<h3 class="text-lg font-semibold">Installed plugins</h3>
		<span class="badge badge-neutral badge-sm">{installed.length}</span>
	</div>
	<div class="mt-2 grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3">
		{#each installed.sort((a, b) => (a.builtin && !b.builtin ? -1 : b.builtin && !a.builtin ? 1 : a.id.localeCompare(b.id))) as plugin}
			<ListedPlugin
				icon={getWebserverUrl(plugin.icon)}
				name={$localisations && $localisations[plugin.id] && $localisations[plugin.id].Name ? $localisations[plugin.id].Name : plugin.name}
				subtitle={plugin.version}
				disconnected={!plugin.registered}
				action={() => {
					if ($settings?.developer) invoke("reload_plugin", { id: plugin.id });
					else removePlugin(plugin);
				}}
				secondaryAction={() => {
					if (!plugin.registered) invoke("open_log_directory");
					else if (plugin.has_settings_interface) invoke("show_settings_interface", { plugin: plugin.id });
				}}
			>
				<svelte:fragment slot="secondary">
					{#if !plugin.registered}
						<WarningCircle size="24" color="#E5A50A" />
					{:else if plugin.has_settings_interface}
						<Gear size="24" color="#26A269" />
					{/if}
				</svelte:fragment>

				{#if $settings?.developer}
					<ArrowClockwise size="20" />
				{:else if !plugin.builtin}
					<Trash size="20" />
				{/if}
			</ListedPlugin>
		{/each}
	</div>

	<div class="mt-8 flex items-center justify-between gap-4">
		<h3 class="text-lg font-semibold">Plugin store</h3>
		<button type="button" class="btn btn-sm" on:click={installPluginFile}>
			<FileArrowUp />
			Install from file
		</button>
	</div>
	<label class="input input-bordered mt-3 w-full bg-base-200">
		<MagnifyingGlass size="16" class="opacity-60" />
		<input bind:value={query} class="grow" placeholder="Search plugins" type="search" spellcheck="false" />
	</label>

	<div role="alert" class="alert mt-6">
		<ArrowSquareOut size="20" />
		<span>Need plugins from the Elgato Marketplace?</span>
		<button type="button" on:click={() => invoke("open_url", { url: "https://github.com/nekename/OpenDeck/wiki/0.-Elgato-Marketplace" })} class="btn btn-sm"> View instructions </button>
	</div>

	{#if !plugins}
		<div class="mt-6 space-y-2">
			<div class="skeleton h-20 w-full"></div>
			<div class="skeleton h-20 w-full"></div>
		</div>
	{:else}
		<div class="mt-6 flex items-center gap-2">
			<h3 class="font-semibold">Open-source plugins</h3>
			<Tooltip>Open-source plugins downloaded from the author's releases.</Tooltip>
		</div>
		<div class="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3">
			{#each Object.entries(plugins) as [id, plugin]}
				<ListedPlugin
					icon="https://openactionapi.github.io/plugins/icons/{id}.png"
					name={plugin.name}
					subtitle={plugin.author}
					hidden={!plugin.name.toLowerCase().includes(query.toLowerCase())}
					action={() => (openDetailsView = id)}
				>
					<ArrowSquareOut size="20" />
				</ListedPlugin>
			{/each}
		</div>
	{/if}

	{#await fetch("https://plugins.amankhanna.me/catalogue.json")}
		<div class="skeleton mt-6 h-20 w-full"></div>
	{:then archiveRes}
		<div class="mt-6 flex items-center gap-2">
			<h3 class="font-semibold">Elgato App Store archive</h3>
			<Tooltip>Plugins archived from the Elgato App Store (now replaced by the Elgato Marketplace).</Tooltip>
		</div>
		{#await archiveRes.json() then entries}
			<div class="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3">
				{#each entries as plugin}
					<ListedPlugin
						icon="https://plugins.amankhanna.me/icons/{plugin.id}.png"
						name={plugin.name}
						subtitle={plugin.author}
						hidden={!plugin.name.toLowerCase().includes(query.toLowerCase())}
						action={() => installPluginElgato(plugin)}
					>
						<CloudArrowDown size="20" />
					</ListedPlugin>
				{/each}
			</div>
		{/await}
	{/await}
</Popup>

{#if openDetailsView}
	<PluginDetails
		id={openDetailsView}
		details={plugins[openDetailsView]}
		install={() => {
			// @ts-expect-error
			installPluginGitHub(openDetailsView, plugins[openDetailsView]);
		}}
		close={() => (openDetailsView = null)}
	/>
{/if}

{#if choices}
	<div class="modal modal-open z-[300]">
		<div class="modal-box w-96 border border-base-300">
			<h3 class="text-lg font-semibold">Choose a release asset</h3>
			<select class="select select-bordered mt-4 w-full" bind:value={choice}>
				{#each choices as choice, i}
					<option value={i}>{choice.name}</option>
				{/each}
			</select>
			<div class="modal-action">
				<button type="button" class="btn" on:click={cancelChoice}>Cancel</button>
				<button type="button" class="btn btn-primary" on:click={finishChoice}>Install</button>
			</div>
		</div>
	</div>
{/if}
