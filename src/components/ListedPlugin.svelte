<script lang="ts">
	export let icon: string;
	export let name: string;
	export let subtitle: string;
	export let hidden: boolean = false;
	export let disconnected: boolean = false;
	export let action: () => void;
	export let secondaryAction: (() => void) | undefined = undefined;
</script>

<article class="card card-side m-2 min-w-0 border border-base-300 bg-base-200 shadow-sm" class:hidden>
	<figure class="shrink-0 p-2 pr-0">
		<img src={icon} class="size-20 rounded-box object-cover" class:opacity-50={disconnected} alt={name} loading="lazy" />
	</figure>
	<div class="card-body min-w-0 gap-1 p-3" class:opacity-50={disconnected}>
		<h3 class="truncate font-semibold">{name}</h3>
		<p class="truncate text-xs text-base-content/60">{subtitle}</p>
	</div>

	<div class="flex shrink-0 flex-col justify-center gap-1 pr-3">
		{#if secondaryAction}
			<button type="button" class="btn btn-circle btn-ghost btn-sm" aria-label="Plugin settings" on:click={secondaryAction}>
				<slot name="secondary" />
			</button>
		{/if}
		<button type="button" class="btn btn-circle btn-ghost btn-sm" aria-label="Plugin action" on:click={action}>
			<slot />
		</button>
	</div>
</article>
