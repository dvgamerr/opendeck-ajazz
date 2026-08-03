<script lang="ts">
	export let folders: Record<string, string[]>;
</script>

{#each Object.entries(folders) as [folder, profiles]}
	{#if folder && profiles.length}
		<optgroup label={folder}>
			{#each profiles as profile}
				<option value={profile}>{profile.slice(folder.length + 1)}</option>
			{/each}
		</optgroup>
	{:else}
		{#each profiles as profile}
			<option value={profile}>{profile}</option>
		{/each}
	{/if}
{/each}
