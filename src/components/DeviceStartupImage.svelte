<script lang="ts">
	import type { DeviceInfo } from "$lib/DeviceInfo";

	import ArrowClockwise from "phosphor-svelte/lib/ArrowClockwise";
	import UploadSimple from "phosphor-svelte/lib/UploadSimple";

	import { invoke } from "@tauri-apps/api/core";
	import { onMount } from "svelte";

	export let device: DeviceInfo;

	const MAX_FILE_SIZE = 10 * 1024 * 1024;
	const PREVIEW_PADDING = 16;
	const PREVIEW_PANEL_HORIZONTAL_PADDING = 32;
	const PREVIEW_PANEL_VERTICAL_PADDING = 72;
	const ALLOWED_EXTENSIONS = new Set(["png", "jpg", "jpeg", "bmp"]);
	const ALLOWED_MIME_TYPES = new Set(["image/png", "image/jpeg", "image/bmp", "image/x-ms-bmp"]);
	// AKP05E_552A mask and startup image share one 810 × 470 coordinate space.
	// The visible apertures are 130 × 130 even though the device protocol also
	// accepts larger per-surface payloads. The touch display is one continuous
	// 810 × 130 strip; its four action zones are not separate physical screens.
	const AKP05_MASK = {
		width: 810,
		height: 470,
		keySize: 130,
		keyX: [0, 170, 340, 510, 680],
		keyY: [0, 170],
		touchStrip: { x: 0, y: 340, width: 810, height: 130 },
	};
	const RESIZE_HANDLES = [
		{ label: "Resize from top left", placement: "-top-1.5 -left-1.5 size-3.5 cursor-nwse-resize" },
		{ label: "Resize from top right", placement: "-top-1.5 -right-1.5 size-3.5 cursor-nesw-resize" },
		{ label: "Resize from bottom left", placement: "-bottom-1.5 -left-1.5 size-3.5 cursor-nesw-resize" },
		{ label: "Resize from bottom right", placement: "-right-1.5 -bottom-1.5 size-3.5 cursor-nwse-resize" },
		{ label: "Resize from top", placement: "-top-1 left-1/2 h-2 w-5 -translate-x-1/2 cursor-ns-resize" },
		{ label: "Resize from bottom", placement: "-bottom-1 left-1/2 h-2 w-5 -translate-x-1/2 cursor-ns-resize" },
		{ label: "Resize from left", placement: "top-1/2 -left-1 h-5 w-2 -translate-y-1/2 cursor-ew-resize" },
		{ label: "Resize from right", placement: "top-1/2 -right-1 h-5 w-2 -translate-y-1/2 cursor-ew-resize" },
	] as const;

	let fileInput: HTMLInputElement;
	let previewCanvas: HTMLCanvasElement;
	let editorViewport: HTMLDivElement;
	let previewPanel: HTMLElement;
	let previewDisplayWidth = 0;
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
	$: showAkp05Mask = device.type == 7 && startupImage.width == AKP05_MASK.width && startupImage.height == AKP05_MASK.height;
	$: previewContentWidth = showAkp05Mask ? AKP05_MASK.width : startupImage.width;
	$: previewContentHeight = showAkp05Mask ? AKP05_MASK.height : startupImage.height;
	$: previewFrameWidth = previewContentWidth + PREVIEW_PADDING * 2;
	$: previewFrameHeight = previewContentHeight + PREVIEW_PADDING * 2;
	$: transformBounds = getTransformBounds(decodedImage, startupImage.width, startupImage.height, zoom, offsetX, offsetY);
	$: if (previewPanel && previewFrameWidth && previewFrameHeight) updatePreviewDisplaySize();

	function updatePreviewDisplaySize() {
		if (!previewPanel || !previewFrameWidth || !previewFrameHeight) return;
		const availableWidth = Math.max(0, previewPanel.clientWidth - PREVIEW_PANEL_HORIZONTAL_PADDING);
		const availableHeight = Math.max(0, previewPanel.clientHeight - PREVIEW_PANEL_VERTICAL_PADDING);
		previewDisplayWidth = Math.max(0, Math.min(previewFrameWidth, availableWidth, availableHeight * (previewFrameWidth / previewFrameHeight)));
	}

	onMount(() => {
		const resizeObserver = new ResizeObserver(updatePreviewDisplaySize);
		resizeObserver.observe(previewPanel);
		updatePreviewDisplaySize();
		return () => resizeObserver.disconnect();
	});

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

		const fittedImage = getFittedImageSize(image, output, scale);
		context.save();
		context.translate(canvas.width / 2 + positionX, canvas.height / 2 + positionY);
		context.rotate((rotationDegrees * Math.PI) / 180);
		context.drawImage(image, -fittedImage.width / 2, -fittedImage.height / 2, fittedImage.width, fittedImage.height);
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

	function getTransformBounds(image: HTMLImageElement | undefined, outputWidth: number, outputHeight: number, scale: number, positionX: number, positionY: number) {
		if (!image || !outputWidth || !outputHeight) {
			return { left: 0, top: 0, width: 0, height: 0 };
		}
		const fittedImage = getFittedImageSize(image, { width: outputWidth, height: outputHeight }, scale);
		return {
			left: ((outputWidth - fittedImage.width) / 2 + positionX) / outputWidth,
			top: ((outputHeight - fittedImage.height) / 2 + positionY) / outputHeight,
			width: fittedImage.width / outputWidth,
			height: fittedImage.height / outputHeight,
		};
	}

	function getFittedImageSize(image: HTMLImageElement, output: { width: number; height: number }, scale: number) {
		const containScale = Math.min(output.width / image.naturalWidth, output.height / image.naturalHeight);
		return {
			width: image.naturalWidth * containScale * scale,
			height: image.naturalHeight * containScale * scale,
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

<div class="relative flex h-full min-h-0 min-w-0 max-w-full flex-1 flex-col">
	<input bind:this={fileInput} type="file" class="hidden" accept=".png,.jpg,.jpeg,.bmp,image/png,image/jpeg,image/bmp" on:change={() => selectFile(fileInput.files?.[0])} />
	<div class="absolute top-[-3.75rem] right-0 z-10 flex max-w-full flex-wrap items-center justify-end gap-2">
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
	<section bind:this={previewPanel} class="mx-4 mt-4 flex min-h-0 min-w-0 flex-1 items-center justify-center rounded-xl border border-base-300 bg-base-200 p-4 pb-14">
		<div class="relative shrink-0 overflow-visible rounded-lg bg-black shadow-2xl" style={`width: ${previewDisplayWidth}px; aspect-ratio: ${previewFrameWidth} / ${previewFrameHeight};`}>
			<!-- svelte-ignore a11y-no-static-element-interactions -->
			<div
				bind:this={editorViewport}
				class="absolute touch-none overflow-visible bg-black"
				class:cursor-grab={decodedImage && dragPointerId == undefined && resizePointerId == undefined && rotatePointerId == undefined}
				class:cursor-grabbing={dragPointerId != undefined}
				style={`left: ${(PREVIEW_PADDING / previewFrameWidth) * 100}%; top: ${(PREVIEW_PADDING / previewFrameHeight) * 100}%; width: ${(startupImage.width / previewFrameWidth) * 100}%; height: ${(startupImage.height / previewFrameHeight) * 100}%;`}
				on:pointerdown={beginDrag}
				on:pointermove={moveImage}
				on:pointerup={endDrag}
				on:pointercancel={endDrag}
			>
				{#if previewUrl}
					<canvas bind:this={previewCanvas} class="absolute inset-0 h-full w-full rounded-lg"></canvas>
				{/if}

				{#if showAkp05Mask}
					<svg class="pointer-events-none absolute inset-0 h-full w-full" viewBox={`0 0 ${AKP05_MASK.width} ${AKP05_MASK.height}`} preserveAspectRatio="none" aria-hidden="true">
						<defs>
							<mask id="akp05-startup-layout-mask">
								<rect width={AKP05_MASK.width} height={AKP05_MASK.height} fill="white" />
								{#each AKP05_MASK.keyY as y}
									{#each AKP05_MASK.keyX as x}
										<rect {x} {y} width={AKP05_MASK.keySize} height={AKP05_MASK.keySize} rx="11" fill="black" />
									{/each}
								{/each}
								<rect x={AKP05_MASK.touchStrip.x} y={AKP05_MASK.touchStrip.y} width={AKP05_MASK.touchStrip.width} height={AKP05_MASK.touchStrip.height} fill="black" />
							</mask>
						</defs>
						<rect width={AKP05_MASK.width} height={AKP05_MASK.height} fill="rgba(0, 0, 0, 0.82)" mask="url(#akp05-startup-layout-mask)" />
						<g fill="none" stroke="rgba(255, 255, 255, 0.38)" stroke-width="2">
							{#each AKP05_MASK.keyY as y}
								{#each AKP05_MASK.keyX as x}
									<rect {x} {y} width={Math.min(AKP05_MASK.keySize, AKP05_MASK.width - x - 1)} height={AKP05_MASK.keySize} rx="11" />
								{/each}
							{/each}
							<rect
								x={AKP05_MASK.touchStrip.x}
								y={AKP05_MASK.touchStrip.y}
								width={Math.min(AKP05_MASK.touchStrip.width, AKP05_MASK.width - AKP05_MASK.touchStrip.x - 1)}
								height={AKP05_MASK.touchStrip.height}
							/>
						</g>
					</svg>
				{/if}

				{#if decodedImage}
					<div
						class="pointer-events-none absolute z-20 border border-primary shadow-[0_0_0_1px_rgba(255,255,255,0.45)]"
						style={`left: ${transformBounds.left * 100}%; top: ${transformBounds.top * 100}%; width: ${transformBounds.width * 100}%; height: ${transformBounds.height * 100}%; transform: rotate(${rotation}deg);`}
					>
						{#each RESIZE_HANDLES as handle}
							<button
								type="button"
								class={`pointer-events-auto absolute rounded-full border border-primary bg-white shadow-sm ${handle.placement}`}
								aria-label={handle.label}
								on:pointerdown|stopPropagation={beginResize}
							></button>
						{/each}
						<span class="absolute top-full left-1/2 h-6 w-px -translate-x-1/2 bg-primary"></span>
						<button
							type="button"
							class="pointer-events-auto absolute top-[calc(100%+1.25rem)] left-1/2 z-30 flex size-7 -translate-x-1/2 cursor-grab items-center justify-center rounded-full border border-primary-content/20 bg-primary text-primary-content shadow-md active:cursor-grabbing"
							aria-label="Rotate image"
							on:pointerdown|stopPropagation={beginRotate}
						>
							<ArrowClockwise size="16" weight="bold" />
						</button>
					</div>
				{/if}
			</div>
		</div>
	</section>
</div>
