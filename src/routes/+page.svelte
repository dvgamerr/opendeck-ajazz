<script lang="ts">
	import type { DeviceInfo } from "$lib/DeviceInfo";
	import type { Profile } from "$lib/Profile";

	import { initPortBase } from "$lib/ports";
	import { inspectedInstance, inspectedParentAction } from "$lib/propertyInspector";
	import { actionList, deviceSelector, profileManager } from "$lib/singletons";

	import ActionList from "../components/ActionList.svelte";
	import DeviceSelector from "../components/DeviceSelector.svelte";
	import DeviceStartupImage from "../components/DeviceStartupImage.svelte";
	import DeviceView from "../components/DeviceView.svelte";
	import NoDevicesDetected from "../components/NoDevicesDetected.svelte";
	import ParentActionView from "../components/ParentActionView.svelte";
	import PluginManager from "../components/PluginManager.svelte";
	import ProfileManager from "../components/ProfileManager.svelte";
	import PropertyInspectorView from "../components/PropertyInspectorView.svelte";
	import SettingsView from "../components/SettingsView.svelte";

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

<main class="relative flex min-h-0 min-w-0 flex-col bg-base-200">
	<header class="navbar min-h-20 shrink-0 border-b border-base-300 bg-base-100 px-8">
		<div class="min-w-0">
			<div class="flex items-center gap-2">
				<h1 class="truncate text-lg font-semibold">{activeDevice?.name ?? "OpenDeck"}</h1>
				{#if activeDevice}
					<span class="badge badge-success badge-sm gap-1">
						<span class="status status-success"></span>
						Connected
					</span>
				{/if}
			</div>
			<p class="mt-0.5 truncate text-xs text-base-content/60">{activeProfile?.id ?? "Select a device to begin"}</p>
		</div>
		<div class="ml-auto flex shrink-0 items-center gap-2">
			<PluginManager />
			<SettingsView />
		</div>
	</header>

	<section class="device-workspace relative flex min-h-0 flex-1 items-center justify-center overflow-visible">
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
		{#if $inspectedInstance}
			<section class="absolute inset-x-0 bottom-0 z-20 flex h-96 min-h-0 overflow-hidden border-t border-base-300 bg-base-100 shadow-2xl">
				<PropertyInspectorView bind:device={activeDevice} bind:profile={activeProfile} />
			</section>
		{/if}
	{/if}
</main>

<aside class="flex h-full min-h-0 w-full flex-col border-l border-base-300 bg-base-100">
	<div class="shrink-0 space-y-2 border-b border-base-300 p-4">
		<p class="text-[10px] font-bold tracking-[0.18em] text-base-content/45">DEVICE</p>
		{#if !$inspectedParentAction}
			<DeviceSelector bind:devices bind:value={selectedDevice} bind:selectedProfiles bind:this={$deviceSelector} />
			{#key selectedDevice}
				{#if selectedDevice && devices[selectedDevice]}
					<ProfileManager bind:device={devices[selectedDevice]} bind:profile={selectedProfiles[selectedDevice]} bind:this={$profileManager} />
					<DeviceStartupImage device={devices[selectedDevice]} />
				{/if}
			{/key}
		{/if}
	</div>
	<ActionList bind:this={$actionList} />
</aside>
