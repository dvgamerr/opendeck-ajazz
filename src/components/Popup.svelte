<script lang="ts">
	export let show = false;
	export let fullscreen = false;
	export let onClose: (() => void) | undefined = undefined;
</script>

{#if show}
	{#if fullscreen}
		<button
			type="button"
			class="fixed inset-0 z-[200] cursor-default bg-black/65 backdrop-blur-sm"
			aria-label="Close modal"
			on:click={() => onClose?.()}
		></button>
		<section
			class="fixed inset-3 z-[210] overflow-y-auto rounded-xl border border-neutral-300 bg-neutral-100 p-6 shadow-2xl sm:inset-5 dark:border-[#484848] dark:bg-[#252525]"
			role="dialog"
			aria-modal="true"
		>
			<slot />
		</section>
	{:else}
		<div class="absolute top-0 left-0 m-2 p-4 w-[calc(100%-1rem)] h-[calc(100%-1rem)] bg-neutral-100 dark:bg-neutral-800 border-2 dark:border-neutral-700 rounded-lg overflow-scroll z-30">
			<slot />
		</div>
	{/if}
{/if}
