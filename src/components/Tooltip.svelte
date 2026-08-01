<script>
	import Question from "phosphor-svelte/lib/Question";
</script>

<div class="tooltip tooltip-right z-50" data-tip="More information">
	<div class="dropdown dropdown-hover dropdown-end">
		<button type="button" tabindex="0" class="btn btn-circle btn-ghost btn-xs text-base-content/55" aria-label="More information">
			<Question size="18" />
		</button>
		<div role="tooltip" class="dropdown-content ui-caption z-50 mt-1 w-80 rounded-box border border-base-300 bg-base-100 p-3 text-left font-normal text-base-content shadow-lg">
			<slot />
		</div>
	</div>
</div>
