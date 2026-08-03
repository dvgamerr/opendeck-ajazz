<script lang="ts">
	import type { DeviceInfo } from "$lib/DeviceInfo";
	import type { Profile } from "$lib/Profile";

	import Browsers from "phosphor-svelte/lib/Browsers";
	import Check from "phosphor-svelte/lib/Check";
	import Pencil from "phosphor-svelte/lib/Pencil";
	import Trash from "phosphor-svelte/lib/Trash";
	import X from "phosphor-svelte/lib/X";
	import Popup from "./Popup.svelte";
	import ProfileOptions from "./ProfileOptions.svelte";

	import { invoke } from "@tauri-apps/api/core";
	import { listen, type UnlistenFn } from "@tauri-apps/api/event";
	import { onMount } from "svelte";
	import { copiedContext, inspectedInstance, inspectedParentAction, openContextMenu } from "$lib/propertyInspector";

	let folders: { [name: string]: string[] } = {};
	let value: string;
	let disposed = false;
	let profileRequest = 0;
	let profileManagerError = "";
	let renamingProfile = "";
	let renameValue = "";

	function makeFolders(profiles: string[]) {
		const nextFolders: { [name: string]: string[] } = {};
		for (const id of profiles) {
			const folder = id.includes("/") ? id.split("/")[0] : "";
			if (nextFolders[folder]) nextFolders[folder].push(id);
			else nextFolders[folder] = [id];
		}
		return nextFolders;
	}

	async function getProfiles(device: DeviceInfo) {
		const request = ++profileRequest;
		const deviceId = device.id;
		try {
			const [profiles, selected] = await Promise.all([invoke<string[]>("get_profiles", { device: deviceId }), invoke<Profile>("get_selected_profile", { device: deviceId })]);
			if (disposed || request != profileRequest || device.id != deviceId) return;

			folders = makeFolders(profiles);
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
		profileManagerError = "";
		try {
			await invoke("delete_profile", { device: device.id, profile: id });
			for (const devices of Object.values(applicationProfiles)) {
				if (devices[device.id] == id) delete devices[device.id];
			}
			applicationProfiles = cleanApplicationProfiles(applicationProfiles);
			lastSavedApplicationProfiles = JSON.stringify(applicationProfiles);
			await getProfiles(device);
		} catch (error) {
			profileManagerError = `Unable to delete profile: ${String(error)}`;
		}
	}

	function beginRename(id: string) {
		renamingProfile = id;
		renameValue = id;
		profileManagerError = "";
	}

	async function renameProfile(id: string) {
		if (!renameValue || renameValue == id) {
			renamingProfile = "";
			return;
		}
		if (!/^[a-zA-Z0-9_ ]+(\/[a-zA-Z0-9_ ]+)?$/.test(renameValue)) {
			profileManagerError = "Profile names may contain letters, numbers, spaces, underscores, and one folder separator.";
			return;
		}
		profileManagerError = "";
		try {
			const selected = await invoke<Profile>("rename_profile", { device: device.id, profile: id, newId: renameValue });
			inspectedInstance.set(null);
			inspectedParentAction.set(null);
			openContextMenu.set(null);
			copiedContext.set(null);
			profile = selected;
			value = selected.id;
			oldValue = selected.id;
			renamingProfile = "";
			const loadedProfiles = await invoke<{ [appName: string]: { [device: string]: string } }>("get_application_profiles");
			applicationProfiles = cleanApplicationProfiles(loadedProfiles);
			lastSavedApplicationProfiles = JSON.stringify(applicationProfiles);
			await getProfiles(device);
		} catch (error) {
			profileManagerError = `Unable to rename profile: ${String(error)}`;
		}
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

<select bind:value class="select select-sm w-full" aria-label="Profile">
	<ProfileOptions {folders} />
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
	<header class="mb-3 flex items-center">
		<div>
			<p class="ui-eyebrow">Profiles</p>
			<h2 class="ui-page-title">{device.name}</h2>
		</div>
		<button type="button" class="btn btn-circle btn-ghost btn-sm ml-auto" aria-label="Close profile manager" on:click={() => (showPopup = false)}>✕</button>
	</header>

	<div class="join mb-3 flex w-full">
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

	{#if profileManagerError}
		<div role="alert" class="alert alert-error mb-3 py-2 text-sm"><span>{profileManagerError}</span></div>
	{/if}

	<div class="divide-y divide-base-300 rounded-box border border-base-300 bg-base-200 px-3">
		{#each Object.entries(folders) as [id, profiles]}
			{#if id && profiles.length}
				<h4 class="ui-eyebrow py-2">{id}</h4>
			{/if}
			{#each profiles as profile}
				<div class="flex items-center gap-3 py-2" class:ml-6={id} class:pl-2={id}>
					{#if renamingProfile == profile}
						<input
							class="input input-bordered input-sm min-w-0 flex-1"
							pattern="[a-zA-Z0-9_ ]+(\/[a-zA-Z0-9_ ]+)?"
							bind:value={renameValue}
							aria-label={`Rename profile ${profile}`}
							on:keydown={(event) => {
								if (event.key == "Enter") void renameProfile(profile);
								if (event.key == "Escape") renamingProfile = "";
							}}
						/>
						<button type="button" class="btn btn-circle btn-primary btn-xs" aria-label="Save profile name" on:click={() => renameProfile(profile)}>
							<Check size="15" weight="bold" />
						</button>
						<button type="button" class="btn btn-circle btn-ghost btn-xs" aria-label="Cancel rename" on:click={() => (renamingProfile = "")}>
							<X size="15" weight="bold" />
						</button>
					{:else}
						<label class="flex min-w-0 flex-1 cursor-pointer items-center gap-3">
							<input type="radio" bind:group={value} value={profile} class="radio radio-primary radio-sm" />
							<span class="truncate">{id ? profile.split("/")[1] : profile}</span>
						</label>
						<button type="button" class="btn btn-circle btn-ghost btn-xs" aria-label={`Rename profile ${profile}`} on:click={() => beginRename(profile)}>
							<Pencil size="16" />
						</button>
					{/if}
					{#if profile != value && renamingProfile != profile}
						<button type="button" on:click={() => deleteProfile(profile)} class="btn btn-circle btn-ghost btn-xs text-error" aria-label="Delete profile">
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
			<p class="ui-eyebrow">Application mapping</p>
			<h2 class="ui-page-title">{device.name}</h2>
		</div>
		<button type="button" class="btn btn-circle btn-ghost btn-sm ml-auto" aria-label="Close application mapping" on:click={() => (showApplicationManager = false)}>✕</button>
	</header>
	<div role="alert" class="alert mb-3">
		<span>If an application is missing, switch to it and back. The previous profile is restored when a mapped application becomes inactive.</span>
	</div>
	{#if applicationProfilesError}
		<div role="alert" class="alert alert-error mb-3"><span>{applicationProfilesError}</span></div>
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
									<ProfileOptions {folders} />
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
							<ProfileOptions {folders} />
						</select>
					</td>
				</tr>
			</tbody>
		</table>
	</div>
</Popup>
