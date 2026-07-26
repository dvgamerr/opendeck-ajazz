<script lang="ts">
	import type { DeviceInfo } from "$lib/DeviceInfo";
	import type { Profile } from "$lib/Profile";

	import Browsers from "phosphor-svelte/lib/Browsers";
	import Trash from "phosphor-svelte/lib/Trash";
	import Popup from "./Popup.svelte";

	import { invoke } from "@tauri-apps/api/core";
	import { listen, type UnlistenFn } from "@tauri-apps/api/event";
	import { onMount } from "svelte";

	let folders: { [name: string]: string[] } = {};
	let value: string;
	let disposed = false;
	let profileRequest = 0;
	async function getProfiles(device: DeviceInfo) {
		const request = ++profileRequest;
		const deviceId = device.id;
		try {
			const profiles: string[] = await invoke("get_profiles", { device: deviceId });
			const selected: Profile = await invoke("get_selected_profile", { device: deviceId });
			if (disposed || request != profileRequest || device.id != deviceId) return;

			const nextFolders: { [name: string]: string[] } = {};
			for (const id of profiles) {
				const folder = id.includes("/") ? id.split("/")[0] : "";
				if (nextFolders[folder]) nextFolders[folder].push(id);
				else nextFolders[folder] = [id];
			}
			folders = nextFolders;
			profile = selected;
			value = profile.id;
			oldValue = value;
		} catch {
			// The selected device may disconnect while profiles are loading.
		}
	}

	export let device: DeviceInfo;

	export let profile: Profile;
	export async function setProfile(id: string) {
		if (!device || !id) return;
		if (value != id) {
			value = id;
			return;
		}
		const deviceId = device.id;
		try {
			await invoke("set_selected_profile", { device: deviceId, id });
			const selected: Profile = await invoke("get_selected_profile", { device: deviceId });
			if (disposed || device.id != deviceId) return;
			profile = selected;
		} catch {
			return;
		}

		let folder = id.includes("/") ? id.split("/")[0] : "";
		if (folders[folder]) {
			if (!folders[folder].includes(id)) folders[folder].push(id);
		} else folders[folder] = [id];
		folders = folders;
	}

	async function deleteProfile(id: string) {
		for (const devices of Object.values(applicationProfiles)) {
			if (devices[device.id] == id) {
				delete devices[device.id];
				applicationProfiles = applicationProfiles;
			}
		}
		await invoke("delete_profile", { device: device.id, profile: id });
		let folder = id.includes("/") ? id.split("/")[0] : "";
		folders[folder].splice(folders[folder].indexOf(id), 1);
		folders = folders;
	}

	let oldValue: string;
	$: {
		if (value == "opendeck_edit_profiles") {
			if (oldValue) showPopup = true;
			value = oldValue;
		} else if (value && value != oldValue && (!profile || profile.id != value)) {
			setProfile(value);
			oldValue = value;
		}
	}

	let showPopup: boolean = false;
	let nameInput: HTMLInputElement;

	let showApplicationManager: boolean = false;
	let applications: string[] = [];
	let applicationProfiles: { [appName: string]: { [device: string]: string } } = {};
	let applicationProfilesLoaded = false;
	let lastSavedApplicationProfiles = "";

	function cleanApplicationProfiles(value: { [appName: string]: { [device: string]: string } }) {
		return Object.fromEntries(Object.entries(value).filter(([_, devices]) => Object.values(devices).some((profile) => profile)));
	}

	onMount(() => {
		const unlisteners: UnlistenFn[] = [];
		const keep = (promise: Promise<UnlistenFn>) => {
			void promise.then((unlisten) => (disposed ? unlisten() : unlisteners.push(unlisten)));
		};

		void getProfiles(device);
		void Promise.all([invoke<string[]>("get_applications"), invoke<{ [appName: string]: { [device: string]: string } }>("get_application_profiles")])
			.then(([loadedApplications, loadedProfiles]) => {
				if (disposed) return;
				applications = loadedApplications;
				applicationProfiles = cleanApplicationProfiles(loadedProfiles);
				lastSavedApplicationProfiles = JSON.stringify(applicationProfiles);
				applicationProfilesLoaded = true;
			})
			.catch(() => {});

		keep(
			listen("rerender_images", async () => {
				const deviceId = device.id;
				try {
					const selected: Profile = await invoke("get_selected_profile", { device: deviceId });
					if (!disposed && device.id == deviceId) profile = selected;
				} catch {
					// The device or profile can disappear while the event is in flight.
				}
			}),
		);
		keep(listen("applications", ({ payload }: { payload: string[] }) => (applications = payload)));

		return () => {
			disposed = true;
			profileRequest += 1;
			unlisteners.forEach((unlisten) => unlisten());
		};
	});

	let applicationsAddAppName: string = "opendeck_select_application";
	let applicationsAddProfile: string = "opendeck_select_profile";
	$: {
		if (applicationsAddAppName != "opendeck_select_application" && applicationsAddProfile != "opendeck_select_profile") {
			applicationProfiles[applicationsAddAppName] ||= {};
			applicationProfiles[applicationsAddAppName][device.id] = applicationsAddProfile;
			applicationsAddAppName = "opendeck_select_application";
			applicationsAddProfile = "opendeck_select_profile";
		}
	}
	$: {
		if (applicationProfilesLoaded) {
			const cleaned = cleanApplicationProfiles(applicationProfiles);
			const serialized = JSON.stringify(cleaned);
			if (serialized != lastSavedApplicationProfiles) {
				lastSavedApplicationProfiles = serialized;
				applicationProfiles = cleaned;
				void invoke("set_application_profiles", { value: applicationProfiles }).catch(() => {});
			}
		}
	}
</script>

<div class="select-wrapper">
	<select bind:value class="my-1 w-full">
		{#each Object.entries(folders) as [id, profiles]}
			{#if id && profiles.length}
				<optgroup label={id}>
					{#each profiles as profile}
						<option value={profile}>{profile.split("/")[1]}</option>
					{/each}
				</optgroup>
			{:else}
				{#each profiles as profile}
					<option value={profile}>{profile}</option>
				{/each}
			{/if}
		{/each}
		<option value="opendeck_edit_profiles">Edit...</option>
	</select>
</div>

<svelte:window
	on:keydown={(event) => {
		if (event.key == "Escape") {
			if (showApplicationManager) showApplicationManager = false;
			else showPopup = false;
		}
	}}
/>

<Popup show={showPopup}>
	<button class="mr-1 float-right text-xl dark:text-neutral-300" on:click={() => (showPopup = false)}>✕</button>
	<h2 class="text-xl font-semibold dark:text-neutral-300">{device.name}</h2>

	<div class="flex flex-row mt-2 mb-1">
		<input
			bind:this={nameInput}
			pattern="[a-zA-Z0-9_ ]+(\/[a-zA-Z0-9_ ]+)?"
			class="grow p-2 dark:text-neutral-300 invalid:text-red-400 dark:bg-neutral-700 rounded-l-md outline-hidden"
			placeholder="Profile ID (e.g. &quot;folder/profile&quot;)"
		/>

		<button
			on:click={async () => {
				if (!nameInput.checkValidity() || !nameInput.value) return;
				await setProfile(nameInput.value);
				value = nameInput.value;
				nameInput.value = "";
				showPopup = false;
			}}
			class="px-4 dark:text-neutral-300 bg-neutral-200 dark:bg-neutral-900 rounded-r-md"
		>
			Create
		</button>

		<button class="ml-2 px-4 flex items-center dark:text-neutral-300 bg-neutral-200 dark:bg-neutral-900 rounded-md outline-hidden" on:click={() => (showApplicationManager = true)}>
			<Browsers size={24} />
		</button>
	</div>

	<div class="divide-y">
		{#each Object.entries(folders) as [id, profiles]}
			{#if id && profiles.length}
				<h4 class="py-2 font-bold text-lg dark:text-neutral-300">{id}</h4>
			{/if}
			{#each profiles as profile}
				<div class="py-2" class:ml-6={id} class:pl-2={id}>
					<input type="radio" bind:group={value} value={profile} />
					<span class="dark:text-neutral-400"> {id ? profile.split("/")[1] : profile} </span>
					{#if profile != value}
						<button on:click={() => deleteProfile(profile)} class="float-right">
							<Trash size="20" color={document.documentElement.classList.contains("dark") ? "#C0BFBC" : "#77767B"} />
						</button>
					{/if}
				</div>
			{/each}
		{/each}
	</div>
</Popup>

<Popup show={showApplicationManager}>
	<button class="mr-1 float-right text-xl dark:text-neutral-300" on:click={() => (showApplicationManager = false)}>✕</button>
	<h2 class="text-xl font-semibold dark:text-neutral-300">{device.name}</h2>
	<span class="text-sm dark:text-neutral-400">If your application isn't listed, try switching to it and back again.</span>
	<span class="text-sm dark:text-neutral-400">The 'default profile' will activate when the focussed application has no profile associated with it.</span>

	<table class="w-full dark:text-neutral-300 divide-y">
		{#each Object.entries(applicationProfiles).sort((a, b) => (a[0] == "opendeck_default" ? -1 : b[0] == "opendeck_default" ? 1 : a[0].localeCompare(b[0]))) as [appName, devices]}
			{#if devices[device.id]}
				<tr class="h-12">
					<td>{appName == "opendeck_default" ? "Default profile" : appName}:</td>
					<td class="select-wrapper">
						<select bind:value={applicationProfiles[appName][device.id]} class="w-full">
							{#each Object.entries(folders) as [id, profiles]}
								{#if id && profiles.length}
									<optgroup label={id}>
										{#each profiles as profile}
											<option value={profile}>{profile.split("/")[1]}</option>
										{/each}
									</optgroup>
								{:else}
									{#each profiles as profile}
										<option value={profile}>{profile}</option>
									{/each}
								{/if}
							{/each}
							<option disabled>──────────</option>
							<option value={undefined}>Remove application</option>
						</select>
					</td>
				</tr>
			{/if}
		{/each}
		<tr class="h-12">
			<td class="w-48 select-wrapper">
				<select bind:value={applicationsAddAppName} class="w-full">
					<option selected disabled value="opendeck_select_application">Select application...</option>
					{#if !applicationProfiles["opendeck_default"] || !applicationProfiles["opendeck_default"][device.id]}
						<option value="opendeck_default">Default profile</option>
						{#if applications.filter((appName) => !applicationProfiles[appName] || !applicationProfiles[appName][device.id]).length > 0}
							<option disabled>──────────</option>
						{/if}
					{/if}
					{#each applications as appName}
						{#if !applicationProfiles[appName] || !applicationProfiles[appName][device.id]}
							<option value={appName}>{appName}</option>
						{/if}
					{/each}
				</select>
			</td>
			<td class="w-96 select-wrapper">
				<select bind:value={applicationsAddProfile} class="w-full">
					<option selected disabled value="opendeck_select_profile">Select profile...</option>
					{#each Object.entries(folders) as [id, profiles]}
						{#if id && profiles.length}
							<optgroup label={id}>
								{#each profiles as profile}
									<option value={profile}>{profile.split("/")[1]}</option>
								{/each}
							</optgroup>
						{:else}
							{#each profiles as profile}
								<option value={profile}>{profile}</option>
							{/each}
						{/if}
					{/each}
				</select>
			</td>
		</tr>
	</table>
</Popup>
