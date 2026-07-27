<script lang="ts">
	import type { DeviceInfo } from "$lib/DeviceInfo";

	import ArrowClockwise from "phosphor-svelte/lib/ArrowClockwise";
	import ImageSquare from "phosphor-svelte/lib/ImageSquare";
	import UploadSimple from "phosphor-svelte/lib/UploadSimple";

	import { invoke } from "@tauri-apps/api/core";

	export let device: DeviceInfo;

	const MAX_FILE_SIZE = 10 * 1024 * 1024;
	const ALLOWED_EXTENSIONS = new Set(["png", "jpg", "jpeg", "bmp"]);
	const ALLOWED_MIME_TYPES = new Set(["image/png", "image/jpeg", "image/bmp", "image/x-ms-bmp"]);
	const AKP05_MASK = {
		keySize: 100,
		keyX: [28, 189, 350, 511, 672],
		keyY: [42, 176],
		touchStrip: { x: 0, y: 332, width: 800, height: 100 },
	};

	let fileInput: HTMLInputElement;
	let previewCanvas: HTMLCanvasElement;
	let editorViewport: HTMLDivElement;
	let previewUrl = "";
	let decodedImage: HTMLImageElement | undefined;
	let zoom = 1;
	let offsetX = 0;
	let offsetY = 0;
	let rotation = 0;
	let successMessage = "";
	let applying = false;
	let lastDeviceId = "";
	let dragPointerId: number | undefined;
	let dragStartX = 0;
	let dragStartY = 0;
	let dragStartOffsetX = 0;
	let dragStartOffsetY = 0;
	let resizePointerId: number | undefined;
	let resizeCenterX = 0;
	let resizeCenterY = 0;
	let resizeStartDistance = 1;
	let resizeStartZoom = 1;
	let rotatePointerId: number | undefined;
	let rotateCenterX = 0;
	let rotateCenterY = 0;
	let rotateStartAngle = 0;
	let rotateStartValue = 0;
	$: startupImage = device.startup_image ?? { width: 0, height: 0 };
	$: showAkp05Mask = device.type == 7 && startupImage.width == 800 && startupImage.height == 480;
	$: transformBounds = getTransformBounds();

	function clearSelection() {
		previewUrl = "";
		decodedImage = undefined;
		zoom = 1;
		offsetX = 0;
		offsetY = 0;
		rotation = 0;
		successMessage = "";
		applying = false;
		if (fileInput) fileInput.value = "";
	}

	$: if (device.id != lastDeviceId) {
		lastDeviceId = device.id;
		clearSelection();
	}

	function drawComposedImage(
		canvas: HTMLCanvasElement,
		image: HTMLImageElement,
		output: { width: number; height: number },
		scale: number,
		positionX: number,
		positionY: number,
		rotationDegrees: number,
	) {
		if (!output.width || !output.height) return;
		if (canvas.width != output.width) canvas.width = output.width;
		if (canvas.height != output.height) canvas.height = output.height;

		const context = canvas.getContext("2d");
		if (!context) return;
		context.fillStyle = "#000000";
		context.fillRect(0, 0, canvas.width, canvas.height);
		context.imageSmoothingEnabled = true;
		context.imageSmoothingQuality = "high";

		const fitScale = Math.max(canvas.width / image.naturalWidth, canvas.height / image.naturalHeight);
		const drawWidth = image.naturalWidth * fitScale * scale;
		const drawHeight = image.naturalHeight * fitScale * scale;
		context.save();
		context.translate(canvas.width / 2 + positionX, canvas.height / 2 + positionY);
		context.rotate((rotationDegrees * Math.PI) / 180);
		context.drawImage(image, -drawWidth / 2, -drawHeight / 2, drawWidth, drawHeight);
		context.restore();
	}

	$: if (previewCanvas && decodedImage) drawComposedImage(previewCanvas, decodedImage, startupImage, zoom, offsetX, offsetY, rotation);

	function loadPreviewImage(source: string) {
		const image = new Image();
		image.onload = () => {
			decodedImage = image;
			zoom = 1;
			offsetX = 0;
			offsetY = 0;
			rotation = 0;
			if (previewCanvas) drawComposedImage(previewCanvas, image, startupImage, zoom, offsetX, offsetY, rotation);
		};
		image.onerror = () => {
			previewUrl = "";
			decodedImage = undefined;
		};
		image.src = source;
	}

	function selectFile(file: File | undefined) {
		successMessage = "";
		if (!file) return;

		const extension = file.name.split(".").pop()?.toLowerCase() ?? "";
		if (!ALLOWED_EXTENSIONS.has(extension) || (file.type && !ALLOWED_MIME_TYPES.has(file.type))) {
			clearSelectionAfterError();
			return;
		}
		if (file.size > MAX_FILE_SIZE) {
			clearSelectionAfterError();
			return;
		}

		const reader = new FileReader();
		reader.onload = () => {
			if (typeof reader.result != "string") {
				clearSelectionAfterError();
				return;
			}
			const mimeType = extension == "png" ? "image/png" : extension == "bmp" ? "image/bmp" : "image/jpeg";
			previewUrl = reader.result.replace(/^data:[^;,]+;base64,/, `data:${mimeType};base64,`);
			loadPreviewImage(previewUrl);
		};
		reader.onerror = () => {
			clearSelectionAfterError();
		};
		reader.readAsDataURL(file);
	}

	function clearSelectionAfterError() {
		previewUrl = "";
		decodedImage = undefined;
		if (fileInput) fileInput.value = "";
	}

	function openFilePicker() {
		fileInput.value = "";
		fileInput.click();
	}

	function resetPlacement() {
		zoom = 1;
		offsetX = 0;
		offsetY = 0;
		rotation = 0;
		successMessage = "";
	}

	function getTransformBounds() {
		if (!decodedImage || !startupImage.width || !startupImage.height) {
			return { left: 0, top: 0, width: 0, height: 0 };
		}
		const fitScale = Math.max(startupImage.width / decodedImage.naturalWidth, startupImage.height / decodedImage.naturalHeight);
		const width = decodedImage.naturalWidth * fitScale * zoom;
		const height = decodedImage.naturalHeight * fitScale * zoom;
		return {
			left: ((startupImage.width - width) / 2 + offsetX) / startupImage.width,
			top: ((startupImage.height - height) / 2 + offsetY) / startupImage.height,
			width: width / startupImage.width,
			height: height / startupImage.height,
		};
	}

	function beginDrag(event: PointerEvent) {
		if (!decodedImage || !editorViewport || resizePointerId != undefined || rotatePointerId != undefined) return;
		dragPointerId = event.pointerId;
		dragStartX = event.clientX;
		dragStartY = event.clientY;
		dragStartOffsetX = offsetX;
		dragStartOffsetY = offsetY;
		editorViewport.setPointerCapture(event.pointerId);
	}

	function moveImage(event: PointerEvent) {
		if (dragPointerId != event.pointerId || !editorViewport) return;
		const bounds = editorViewport.getBoundingClientRect();
		if (!bounds.width || !bounds.height) return;
		offsetX = dragStartOffsetX + (event.clientX - dragStartX) * (startupImage.width / bounds.width);
		offsetY = dragStartOffsetY + (event.clientY - dragStartY) * (startupImage.height / bounds.height);
		successMessage = "";
	}

	function endDrag(event: PointerEvent) {
		if (dragPointerId != event.pointerId || !editorViewport) return;
		if (editorViewport.hasPointerCapture(event.pointerId)) editorViewport.releasePointerCapture(event.pointerId);
		dragPointerId = undefined;
	}

	function beginResize(event: PointerEvent) {
		if (!decodedImage || !editorViewport || rotatePointerId != undefined) return;
		event.preventDefault();
		const bounds = editorViewport.getBoundingClientRect();
		resizePointerId = event.pointerId;
		resizeCenterX = bounds.left + (0.5 + offsetX / startupImage.width) * bounds.width;
		resizeCenterY = bounds.top + (0.5 + offsetY / startupImage.height) * bounds.height;
		resizeStartDistance = Math.max(1, Math.hypot(event.clientX - resizeCenterX, event.clientY - resizeCenterY));
		resizeStartZoom = zoom;
	}

	function resizeImage(event: PointerEvent) {
		if (resizePointerId != event.pointerId) return;
		const distance = Math.max(1, Math.hypot(event.clientX - resizeCenterX, event.clientY - resizeCenterY));
		zoom = Math.max(0.25, Math.min(3, resizeStartZoom * (distance / resizeStartDistance)));
		successMessage = "";
	}

	function endResize(event: PointerEvent) {
		if (resizePointerId == event.pointerId) resizePointerId = undefined;
	}

	function beginRotate(event: PointerEvent) {
		if (!decodedImage || !editorViewport || resizePointerId != undefined) return;
		event.preventDefault();
		const bounds = editorViewport.getBoundingClientRect();
		rotatePointerId = event.pointerId;
		rotateCenterX = bounds.left + (0.5 + offsetX / startupImage.width) * bounds.width;
		rotateCenterY = bounds.top + (0.5 + offsetY / startupImage.height) * bounds.height;
		rotateStartAngle = Math.atan2(event.clientY - rotateCenterY, event.clientX - rotateCenterX);
		rotateStartValue = rotation;
		(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
	}

	function rotateImage(event: PointerEvent) {
		if (rotatePointerId != event.pointerId) return;
		const angle = Math.atan2(event.clientY - rotateCenterY, event.clientX - rotateCenterX);
		rotation = rotateStartValue + ((angle - rotateStartAngle) * 180) / Math.PI;
		successMessage = "";
	}

	function endRotate(event: PointerEvent) {
		if (rotatePointerId == event.pointerId) rotatePointerId = undefined;
	}

	function moveTransform(event: PointerEvent) {
		resizeImage(event);
		rotateImage(event);
	}

	function endTransform(event: PointerEvent) {
		endResize(event);
		endRotate(event);
	}

	async function applyImage() {
		if (!decodedImage || !device.startup_image || applying) return;
		applying = true;
		successMessage = "";
		try {
			const output = document.createElement("canvas");
			drawComposedImage(output, decodedImage, startupImage, zoom, offsetX, offsetY, rotation);
			await invoke("set_startup_image", { device: device.id, image: output.toDataURL("image/jpeg", 0.92) });
			successMessage = `Startup image applied to ${device.name}.`;
		} catch {
			successMessage = "";
		} finally {
			applying = false;
		}
	}
</script>

<svelte:window on:pointermove={moveTransform} on:pointerup={endTransform} on:pointercancel={endTransform} />

<div class="relative min-h-0">
	<input bind:this={fileInput} type="file" class="hidden" accept=".png,.jpg,.jpeg,.bmp,image/png,image/jpeg,image/bmp" on:change={() => selectFile(fileInput.files?.[0])} />
	<div class="absolute top-[-3.75rem] right-0 z-10 flex items-center gap-2">
		{#if successMessage}
			<span class="badge badge-success">Applied</span>
		{:else}
			<span class="badge badge-outline">Not applied yet</span>
		{/if}
		<button type="button" class="btn btn-sm" on:click={openFilePicker}>
			<UploadSimple size="16" weight="bold" />
			{previewUrl ? "Upload new" : "Upload"}
		</button>
		<button type="button" class="btn btn-ghost btn-sm" disabled={!decodedImage} on:click={resetPlacement}>Reset</button>
		<button type="button" class="btn btn-primary btn-sm min-w-32" disabled={!decodedImage || applying} on:click={applyImage}>
			{#if applying}
				<span class="loading loading-spinner loading-sm"></span>
				Applying…
			{:else}
				Apply to device
			{/if}
		</button>
	</div>
	<section class="card border border-base-300 bg-base-200">
		<div class="card-body p-4">
			<div class="mb-2">
				<h3 class="card-title text-base">Device preview</h3>
				<p class="mt-1 text-sm text-base-content/60">Dark areas are hidden by the device layout.</p>
			</div>

			<div class="flex min-h-0 flex-1 items-center justify-center rounded-box border border-base-300 bg-neutral px-4 pt-4 pb-12">
				<!-- svelte-ignore a11y-no-static-element-interactions -->
				<div
					bind:this={editorViewport}
					class="relative w-full max-w-2xl touch-none overflow-visible rounded-lg border border-white/10 bg-black shadow-2xl"
					class:cursor-grab={decodedImage && dragPointerId == undefined && resizePointerId == undefined && rotatePointerId == undefined}
					class:cursor-grabbing={dragPointerId != undefined}
					style:aspect-ratio={`${startupImage.width} / ${startupImage.height}`}
					on:pointerdown={beginDrag}
					on:pointermove={moveImage}
					on:pointerup={endDrag}
					on:pointercancel={endDrag}
				>
					{#if previewUrl}
						<canvas bind:this={previewCanvas} class="absolute inset-0 h-full w-full rounded-lg"></canvas>
					{:else}
						<button
							type="button"
							class="absolute inset-0 z-30 flex cursor-pointer flex-col items-center justify-center gap-3 text-neutral-content/55 transition-colors hover:bg-white/5 hover:text-neutral-content focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-[-4px] focus-visible:outline-primary"
							on:pointerdown|stopPropagation
							on:click={openFilePicker}
						>
							<ImageSquare size="48" weight="thin" />
							<span class="flex items-center gap-2 text-sm">
								<UploadSimple size="16" weight="bold" />
								Click to upload an image
							</span>
						</button>
					{/if}

					{#if showAkp05Mask}
						<svg class="pointer-events-none absolute inset-0 h-full w-full" viewBox="0 0 800 480" preserveAspectRatio="none" aria-hidden="true">
							<defs>
								<mask id="akp05-startup-layout-mask">
									<rect width="800" height="480" fill="white" />
									{#each AKP05_MASK.keyY as y}
										{#each AKP05_MASK.keyX as x}
											<rect {x} {y} width={AKP05_MASK.keySize} height={AKP05_MASK.keySize} rx="8" fill="black" />
										{/each}
									{/each}
									<rect x={AKP05_MASK.touchStrip.x} y={AKP05_MASK.touchStrip.y} width={AKP05_MASK.touchStrip.width} height={AKP05_MASK.touchStrip.height} fill="black" />
								</mask>
							</defs>
							<rect width="800" height="480" fill="rgba(0, 0, 0, 0.82)" mask="url(#akp05-startup-layout-mask)" />
							<g fill="none" stroke="rgba(255, 255, 255, 0.38)" stroke-width="2">
								{#each AKP05_MASK.keyY as y}
									{#each AKP05_MASK.keyX as x}
										<rect {x} {y} width={AKP05_MASK.keySize} height={AKP05_MASK.keySize} rx="8" />
									{/each}
								{/each}
								<rect x={AKP05_MASK.touchStrip.x} y={AKP05_MASK.touchStrip.y} width={AKP05_MASK.touchStrip.width} height={AKP05_MASK.touchStrip.height} />
							</g>
						</svg>
					{/if}

					{#if decodedImage}
						<div
							class="pointer-events-none absolute z-20 border-2 border-primary shadow-[0_0_0_1px_rgba(255,255,255,0.7)]"
							style={`left: ${transformBounds.left * 100}%; top: ${transformBounds.top * 100}%; width: ${transformBounds.width * 100}%; height: ${transformBounds.height * 100}%; transform: rotate(${rotation}deg);`}
						>
							<button
								type="button"
								class="pointer-events-auto absolute -top-2.5 -left-2.5 size-5 cursor-nwse-resize rounded-full border-2 border-primary bg-white shadow-md"
								aria-label="Resize from top left"
								on:pointerdown|stopPropagation={beginResize}
							></button>
							<button
								type="button"
								class="pointer-events-auto absolute -top-2.5 -right-2.5 size-5 cursor-nesw-resize rounded-full border-2 border-primary bg-white shadow-md"
								aria-label="Resize from top right"
								on:pointerdown|stopPropagation={beginResize}
							></button>
							<button
								type="button"
								class="pointer-events-auto absolute -bottom-2.5 -left-2.5 size-5 cursor-nesw-resize rounded-full border-2 border-primary bg-white shadow-md"
								aria-label="Resize from bottom left"
								on:pointerdown|stopPropagation={beginResize}
							></button>
							<button
								type="button"
								class="pointer-events-auto absolute -right-2.5 -bottom-2.5 size-5 cursor-nwse-resize rounded-full border-2 border-primary bg-white shadow-md"
								aria-label="Resize from bottom right"
								on:pointerdown|stopPropagation={beginResize}
							></button>
							<button
								type="button"
								class="pointer-events-auto absolute -top-1.5 left-1/2 h-3 w-7 -translate-x-1/2 cursor-ns-resize rounded-full border border-primary bg-white shadow"
								aria-label="Resize from top"
								on:pointerdown|stopPropagation={beginResize}
							></button>
							<button
								type="button"
								class="pointer-events-auto absolute -bottom-1.5 left-1/2 h-3 w-7 -translate-x-1/2 cursor-ns-resize rounded-full border border-primary bg-white shadow"
								aria-label="Resize from bottom"
								on:pointerdown|stopPropagation={beginResize}
							></button>
							<button
								type="button"
								class="pointer-events-auto absolute top-1/2 -left-1.5 h-7 w-3 -translate-y-1/2 cursor-ew-resize rounded-full border border-primary bg-white shadow"
								aria-label="Resize from left"
								on:pointerdown|stopPropagation={beginResize}
							></button>
							<button
								type="button"
								class="pointer-events-auto absolute top-1/2 -right-1.5 h-7 w-3 -translate-y-1/2 cursor-ew-resize rounded-full border border-primary bg-white shadow"
								aria-label="Resize from right"
								on:pointerdown|stopPropagation={beginResize}
							></button>
							<span class="absolute top-full left-1/2 h-8 w-0.5 -translate-x-1/2 bg-primary"></span>
							<button
								type="button"
								class="btn btn-circle btn-primary pointer-events-auto absolute top-[calc(100%+1.75rem)] left-1/2 z-30 size-9 min-h-0 -translate-x-1/2 cursor-grab p-0 shadow-lg active:cursor-grabbing"
								aria-label="Rotate image"
								on:pointerdown|stopPropagation={beginRotate}
							>
								<ArrowClockwise size="20" weight="bold" />
							</button>
						</div>
					{/if}
				</div>
			</div>
		</div>
	</section>
</div>
