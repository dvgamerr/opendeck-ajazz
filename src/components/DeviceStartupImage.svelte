<script lang="ts">
	import type { DeviceInfo } from "$lib/DeviceInfo";

	import ImageSquare from "phosphor-svelte/lib/ImageSquare";
	import UploadSimple from "phosphor-svelte/lib/UploadSimple";
	import Popup from "./Popup.svelte";

	import { invoke } from "@tauri-apps/api/core";

	export let device: DeviceInfo;

	const MAX_FILE_SIZE = 10 * 1024 * 1024;
	const ALLOWED_EXTENSIONS = new Set(["png", "jpg", "jpeg", "bmp"]);
	const ALLOWED_MIME_TYPES = new Set(["image/png", "image/jpeg", "image/bmp", "image/x-ms-bmp"]);

	let showPopup = false;
	let fileInput: HTMLInputElement;
	let previewUrl = "";
	let fileName = "";
	let sourceWidth = 0;
	let sourceHeight = 0;
	let errorMessage = "";
	let successMessage = "";
	let applying = false;
	let lastDeviceId = "";
	$: startupImage = device.startup_image ?? { width: 0, height: 0 };

	function clearSelection() {
		previewUrl = "";
		fileName = "";
		sourceWidth = 0;
		sourceHeight = 0;
		errorMessage = "";
		successMessage = "";
		applying = false;
		if (fileInput) fileInput.value = "";
	}

	$: if (device.id != lastDeviceId) {
		lastDeviceId = device.id;
		clearSelection();
	}

	function describeError(error: unknown): string {
		if (typeof error == "string") return error;
		if (error instanceof Error) return error.message;
		return "Unable to apply the startup image";
	}

	function loadPreviewDimensions(source: string) {
		const image = new Image();
		image.onload = () => {
			sourceWidth = image.naturalWidth;
			sourceHeight = image.naturalHeight;
		};
		image.onerror = () => {
			errorMessage = "The selected file could not be read as an image";
			previewUrl = "";
		};
		image.src = source;
	}

	function selectFile(file: File | undefined) {
		errorMessage = "";
		successMessage = "";
		if (!file) return;

		const extension = file.name.split(".").pop()?.toLowerCase() ?? "";
		if (!ALLOWED_EXTENSIONS.has(extension) || (file.type && !ALLOWED_MIME_TYPES.has(file.type))) {
			errorMessage = extension == "gif" || file.type == "image/gif" ? "GIF images are not supported. Choose a PNG, JPG, JPEG, or BMP file." : "Choose a PNG, JPG, JPEG, or BMP file.";
			clearSelectionAfterError();
			return;
		}
		if (file.size > MAX_FILE_SIZE) {
			errorMessage = "The selected image must be 10 MB or smaller.";
			clearSelectionAfterError();
			return;
		}

		const reader = new FileReader();
		reader.onload = () => {
			if (typeof reader.result != "string") {
				errorMessage = "The selected file could not be read.";
				return;
			}
			const mimeType = extension == "png" ? "image/png" : extension == "bmp" ? "image/bmp" : "image/jpeg";
			previewUrl = reader.result.replace(/^data:[^;,]+;base64,/, `data:${mimeType};base64,`);
			fileName = file.name;
			loadPreviewDimensions(previewUrl);
		};
		reader.onerror = () => {
			errorMessage = "The selected file could not be read.";
			clearSelectionAfterError();
		};
		reader.readAsDataURL(file);
	}

	function clearSelectionAfterError() {
		previewUrl = "";
		fileName = "";
		sourceWidth = 0;
		sourceHeight = 0;
		if (fileInput) fileInput.value = "";
	}

	async function applyImage() {
		if (!previewUrl || !device.startup_image || applying) return;
		applying = true;
		errorMessage = "";
		successMessage = "";
		try {
			await invoke("set_startup_image", { device: device.id, image: previewUrl });
			successMessage = `Startup image applied to ${device.name}.`;
		} catch (error) {
			errorMessage = describeError(error);
		} finally {
			applying = false;
		}
	}
</script>

{#if device.startup_image}
	<button type="button" class="btn btn-outline btn-sm w-full justify-start" on:click={() => (showPopup = true)}>
		<ImageSquare size="16" weight="bold" />
		Startup image
	</button>
{/if}

<svelte:window
	on:keydown={(event) => {
		if (event.key == "Escape") showPopup = false;
	}}
/>

<Popup show={showPopup} fullscreen onClose={() => (showPopup = false)}>
	<header class="flex items-center border-b border-base-300 pb-4">
		<div>
			<p class="text-xs font-semibold tracking-widest text-base-content/50">DEVICE STARTUP</p>
			<h2 class="text-2xl font-semibold">Startup image</h2>
			<p class="mt-1 text-sm text-base-content/60">{device.name}</p>
		</div>
		<button type="button" class="btn btn-circle btn-ghost ml-auto" aria-label="Close startup image editor" on:click={() => (showPopup = false)}>✕</button>
	</header>

	<div class="mt-6 grid min-h-0 gap-6 xl:grid-cols-[minmax(20rem,0.8fr)_minmax(32rem,1.2fr)]">
		<section class="card border border-base-300 bg-base-200">
			<div class="card-body gap-5">
				<div>
					<h3 class="card-title text-base">Choose an image</h3>
					<p class="mt-1 text-sm text-base-content/60">PNG, JPG, JPEG, or BMP · maximum 10 MB</p>
				</div>

				<input bind:this={fileInput} type="file" class="hidden" accept=".png,.jpg,.jpeg,.bmp,image/png,image/jpeg,image/bmp" on:change={() => selectFile(fileInput.files?.[0])} />

				<button type="button" class="btn btn-primary justify-start" on:click={() => fileInput.click()}>
					<UploadSimple size="18" weight="bold" />
					{previewUrl ? "Choose another image" : "Upload image"}
				</button>

				<div class="rounded-box border border-base-300 bg-base-100 p-4 text-sm">
					<div class="flex justify-between gap-3">
						<span class="text-base-content/60">Device output</span>
						<span class="font-medium">{startupImage.width} × {startupImage.height} JPEG</span>
					</div>
					{#if sourceWidth && sourceHeight}
						<div class="mt-2 flex justify-between gap-3">
							<span class="text-base-content/60">Source image</span>
							<span class="font-medium">{sourceWidth} × {sourceHeight}</span>
						</div>
					{/if}
					{#if fileName}
						<div class="mt-2 flex justify-between gap-3">
							<span class="text-base-content/60">File</span>
							<span class="max-w-56 truncate font-medium" title={fileName}>{fileName}</span>
						</div>
					{/if}
				</div>

				<div role="alert" class="alert py-3 text-sm">
					<span>The device stores a static JPEG. Animated GIF files are not accepted.</span>
				</div>

				{#if errorMessage}
					<div role="alert" class="alert alert-error py-3 text-sm"><span>{errorMessage}</span></div>
				{/if}
				{#if successMessage}
					<div role="status" class="alert alert-success py-3 text-sm"><span>{successMessage}</span></div>
				{/if}
			</div>
		</section>

		<section class="card border border-base-300 bg-base-200">
			<div class="card-body">
				<div class="mb-3 flex items-start justify-between gap-4">
					<div>
						<h3 class="card-title text-base">Device preview</h3>
						<p class="mt-1 text-sm text-base-content/60">This preview matches the device aspect ratio. The image is resized to fill the full display.</p>
					</div>
					{#if successMessage}
						<span class="badge badge-success shrink-0">Applied</span>
					{:else}
						<span class="badge badge-outline shrink-0">Not applied yet</span>
					{/if}
				</div>

				<div class="flex min-h-72 flex-1 items-center justify-center rounded-box border border-base-300 bg-neutral p-5">
					<div class="relative w-full max-w-4xl overflow-hidden rounded-lg border border-white/10 bg-black shadow-2xl" style:aspect-ratio={`${startupImage.width} / ${startupImage.height}`}>
						{#if previewUrl}
							<img src={previewUrl} alt="Startup preview" class="absolute inset-0 h-full w-full object-fill" />
						{:else}
							<div class="absolute inset-0 flex flex-col items-center justify-center gap-3 text-neutral-content/55">
								<ImageSquare size="48" weight="thin" />
								<span class="text-sm">Upload an image to preview it here</span>
							</div>
						{/if}
					</div>
				</div>
			</div>
		</section>
	</div>

	<footer class="mt-6 flex items-center gap-3 border-t border-base-300 pt-4">
		<p class="text-sm text-base-content/55">Nothing is sent to the device until you apply.</p>
		<div class="ml-auto flex gap-2">
			<button type="button" class="btn" on:click={() => (showPopup = false)}>Cancel</button>
			<button type="button" class="btn btn-primary min-w-40" disabled={!previewUrl || applying} on:click={applyImage}>
				{#if applying}
					<span class="loading loading-spinner loading-sm"></span>
					Applying…
				{:else}
					Apply to device
				{/if}
			</button>
		</div>
	</footer>
</Popup>
