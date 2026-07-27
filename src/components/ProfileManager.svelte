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
	export function applySelectedProfile(selected: Profile) {
		profile = selected;
		value = selected.id;
		oldValue = selected.id;

		const folder = selected.id.includes("/") ? selected.id.split("/")[0] : "";
		if (folders[folder]) {
			if (!folders[folder].includes(selected.id)) folders[folder].push(selected.id);
		} else {
			folders[folder] = [selected.id];
		}
		folders = folders;
	}

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
	let applicationProfilesSaving = false;
	let pendingApplicationProfiles: { [appName: string]: { [device: string]: string } } | undefined;
	let applicationProfilesError = "";

	function cleanApplicationProfiles(value: { [appName: string]: { [device: string]: string } }) {
		return Object.fromEntries(
			Object.entries(value)
				.map(([appName, devices]) => [appName, Object.fromEntries(Object.entries(devices).filter(([_, profile]) => profile))])
				.filter(([_, devices]) => Object.keys(devices).length),
		);
	}

	async function persistApplicationProfiles(value: { [appName: string]: { [device: string]: string } }) {
		pendingApplicationProfiles = cleanApplicationProfiles(value);
		if (applicationProfilesSaving) return;

		applicationProfilesSaving = true;
		try {
			while (pendingApplicationProfiles) {
				const next = pendingApplicationProfiles;
				pendingApplicationProfiles = undefined;
				const serialized = JSON.stringify(next);
				if (serialized == lastSavedApplicationProfiles) continue;

				try {
					await invoke("set_application_profiles", { value: next });
					lastSavedApplicationProfiles = serialized;
					applicationProfilesError = "";
				} catch (error) {
					applicationProfilesError = `Unable to save application profiles: ${String(error)}`;
					console.error(applicationProfilesError);
					break;
				}
			}
		} finally {
			applicationProfilesSaving = false;
		}
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
			.catch((error) => {
				applicationProfilesError = `Unable to load application profiles: ${String(error)}`;
				console.error(applicationProfilesError);
			});

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
			applicationProfiles = {
				...applicationProfiles,
				[applicationsAddAppName]: {
					...applicationProfiles[applicationsAddAppName],
					[device.id]: applicationsAddProfile,
				},
			};
			applicationsAddAppName = "opendeck_select_application";
			applicationsAddProfile = "opendeck_select_profile";
		}
	}
	$: {
		if (applicationProfilesLoaded) {
			const cleaned = cleanApplicationProfiles(applicationProfiles);
			const serialized = JSON.stringify(cleaned);
			if (serialized != lastSavedApplicationProfiles) {
				if (serialized != JSON.stringify(applicationProfiles)) applicationProfiles = cleaned;
				void persistApplicationProfiles(cleaned);
			}
		}
	}
</script>

<select bind:value class="select select-sm my-1 w-full" aria-label="Profile">
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

<svelte:window
	on:keydown={(event) => {
		if (event.key == "Escape") {
			if (showApplicationManager) showApplicationManager = false;
			else showPopup = false;
		}
	}}
/>

<Popup show={showPopup}>
	<header class="mb-4 flex items-center">
		<div>
			<p class="text-xs font-semibold tracking-widest text-base-content/50">PROFILES</p>
			<h2 class="text-xl font-semibold">{device.name}</h2>
		</div>
		<button type="button" class="btn btn-circle btn-ghost btn-sm ml-auto" aria-label="Close profile manager" on:click={() => (showPopup = false)}>✕</button>
	</header>

	<div class="join mb-4 flex w-full">
		<input
			bind:this={nameInput}
			pattern="[a-zA-Z0-9_ ]+(\/[a-zA-Z0-9_ ]+)?"
			class="input input-bordered join-item grow invalid:input-error"
			placeholder="Profile ID (e.g. &quot;folder/profile&quot;)"
		/>

		<button
			type="button"
			on:click={async () => {
				if (!nameInput.checkValidity() || !nameInput.value) return;
				await setProfile(nameInput.value);
				value = nameInput.value;
				nameInput.value = "";
				showPopup = false;
			}}
			class="btn btn-primary join-item"
		>
			Create
		</button>

		<button type="button" class="btn join-item" title="Manage application profiles" on:click={() => (showApplicationManager = true)}>
			<Browsers size={24} />
		</button>
	</div>

	<div class="divide-y divide-base-300 rounded-box border border-base-300 bg-base-200 px-3">
		{#each Object.entries(folders) as [id, profiles]}
			{#if id && profiles.length}
				<h4 class="py-2 text-sm font-bold uppercase tracking-wide text-base-content/55">{id}</h4>
			{/if}
			{#each profiles as profile}
				<div class="flex items-center gap-3 py-2" class:ml-6={id} class:pl-2={id}>
					<label class="flex min-w-0 flex-1 cursor-pointer items-center gap-3">
						<input type="radio" bind:group={value} value={profile} class="radio radio-primary radio-sm" />
						<span class="truncate">{id ? profile.split("/")[1] : profile}</span>
					</label>
					{#if profile != value}
						<button type="button" on:click={() => deleteProfile(profile)} class="btn btn-circle btn-ghost btn-xs ml-auto text-error" aria-label="Delete profile">
							<Trash size="18" />
						</button>
					{/if}
				</div>
			{/each}
		{/each}
	</div>
</Popup>

<Popup show={showApplicationManager}>
	<header class="mb-3 flex items-center">
		<div>
			<p class="text-xs font-semibold tracking-widest text-base-content/50">APPLICATION MAPPING</p>
			<h2 class="text-xl font-semibold">{device.name}</h2>
		</div>
		<button type="button" class="btn btn-circle btn-ghost btn-sm ml-auto" aria-label="Close application mapping" on:click={() => (showApplicationManager = false)}>✕</button>
	</header>
	<div role="alert" class="alert mb-3 py-2 text-sm">
		<span>If an application is missing, switch to it and back. The previous profile is restored when a mapped application becomes inactive.</span>
	</div>
	{#if applicationProfilesError}
		<div role="alert" class="alert alert-error mb-3 py-2 text-sm"><span>{applicationProfilesError}</span></div>
	{/if}

	<div class="overflow-x-auto rounded-box border border-base-300">
		<table class="table table-sm w-full">
			<thead>
				<tr><th>Application</th><th>Profile</th></tr>
			</thead>
			<tbody>
				{#each Object.entries(applicationProfiles).sort((a, b) => (a[0] == "opendeck_default" ? -1 : b[0] == "opendeck_default" ? 1 : a[0].localeCompare(b[0]))) as [appName, devices]}
					{#if devices[device.id]}
						<tr>
							<td>{appName == "opendeck_default" ? "Default profile" : appName}:</td>
							<td>
								<select bind:value={applicationProfiles[appName][device.id]} class="select select-sm w-full">
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
					<td class="w-48">
						<select bind:value={applicationsAddAppName} class="select select-sm w-full">
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
					<td class="w-96">
						<select bind:value={applicationsAddProfile} class="select select-sm w-full">
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
			</tbody>
		</table>
	</div>
</Popup>
