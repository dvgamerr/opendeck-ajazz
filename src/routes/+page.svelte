<script lang="ts">
	import type { PageProps } from "./$types";
	import type { DeviceInfo } from "$lib/DeviceInfo";
	import type { Profile } from "$lib/Profile";

	import { initPortBase } from "$lib/ports";
	import { inspectedInstance, inspectedParentAction } from "$lib/propertyInspector";
	import { actionList, deviceSelector, profileManager } from "$lib/singletons";

	import ActionList from "../components/ActionList.svelte";
	import DeviceSelector from "../components/DeviceSelector.svelte";
	import DeviceView from "../components/DeviceView.svelte";
	import NoDevicesDetected from "../components/NoDevicesDetected.svelte";
	import ParentActionView from "../components/ParentActionView.svelte";
	import PluginManager from "../components/PluginManager.svelte";
	import ProfileManager from "../components/ProfileManager.svelte";
	import PropertyInspectorView from "../components/PropertyInspectorView.svelte";
	import SettingsView from "../components/SettingsView.svelte";

	export let params: PageProps["params"];
	void params;

	let devices: { [id: string]: DeviceInfo } = {};
	let selectedDevice: string;
	let selectedProfiles: { [id: string]: Profile } = {};
	let activeDevice: DeviceInfo | undefined;
	let activeProfile: Profile | undefined;
	$: activeDevice = devices[selectedDevice];
	$: activeProfile = selectedProfiles[selectedDevice];

	initPortBase();
</script>

<svelte:window on:contextmenu|preventDefault on:dragover={(event) => event.preventDefault()} on:drop={(event) => event.preventDefault()} />

<main class="workspace-shell">
	<header class="workspace-toolbar">
		<div class="min-w-0">
			<div class="flex items-center gap-2">
				<h1 class="truncate text-lg font-semibold text-neutral-900 dark:text-neutral-100">{activeDevice?.name ?? "OpenDeck"}</h1>
				{#if activeDevice}
					<span class="device-status">Connected</span>
				{/if}
			</div>
			<p class="mt-0.5 truncate text-xs text-neutral-500 dark:text-neutral-400">{activeProfile?.id ?? "Select a device to begin"}</p>
		</div>
		<div class="flex shrink-0 items-center gap-2">
			<PluginManager />
			<SettingsView />
		</div>
	</header>

	<section class="device-workspace">
		{#if Object.keys(devices).length > 0 && selectedProfiles}
			{#if $inspectedParentAction}
				<ParentActionView bind:profile={selectedProfiles[selectedDevice]} />
			{:else}
				{#each Object.entries(devices) as [id, device]}
					{#if device && selectedProfiles[id]}
						<DeviceView bind:device bind:profile={selectedProfiles[id]} bind:selectedDevice />
					{/if}
				{/each}
			{/if}
		{:else}
			<NoDevicesDetected />
		{/if}
	</section>

	{#if activeDevice && activeProfile}
		<section class="options-panel" class:options-panel-visible={Boolean($inspectedInstance)}>
			<PropertyInspectorView bind:device={activeDevice} bind:profile={activeProfile} />
		</section>
	{/if}
</main>

<aside class="action-sidebar">
	<div class="sidebar-controls">
		<p class="sidebar-eyebrow">DEVICE</p>
		{#if !$inspectedParentAction}
			<DeviceSelector bind:devices bind:value={selectedDevice} bind:selectedProfiles bind:this={$deviceSelector} />
			{#key selectedDevice}
				{#if selectedDevice && devices[selectedDevice]}
					<ProfileManager bind:device={devices[selectedDevice]} bind:profile={selectedProfiles[selectedDevice]} bind:this={$profileManager} />
				{/if}
			{/key}
		{/if}
	</div>
	<ActionList bind:this={$actionList} />
</aside>
