<script lang="ts">
	import type { DeviceInfo } from "$lib/DeviceInfo";

	import ArrowClockwise from "phosphor-svelte/lib/ArrowClockwise";
	import CaretDown from "phosphor-svelte/lib/CaretDown";
	import CaretUp from "phosphor-svelte/lib/CaretUp";
	import ImageSquare from "phosphor-svelte/lib/ImageSquare";
	import Plus from "phosphor-svelte/lib/Plus";
	import Stack from "phosphor-svelte/lib/Stack";
	import Trash from "phosphor-svelte/lib/Trash";

	import { invoke } from "@tauri-apps/api/core";
	import DOMPurify from "dompurify";
	import { onMount } from "svelte";

	export let device: DeviceInfo;

	type PersistedLayer = {
		id: string;
		name: string;
		image: string;
		zoom: number;
		offset_x: number;
		offset_y: number;
		rotation: number;
	};

	type ImageLayer = PersistedLayer & {
		decoded?: HTMLImageElement;
	};

	type StartupImageProject = {
		layers: PersistedLayer[];
	};

	const MAX_FILE_SIZE = 10 * 1024 * 1024;
	const MAX_LAYERS = 64;
	const PREVIEW_PADDING = 16;
	const PREVIEW_PANEL_HORIZONTAL_PADDING = 32;
	const PREVIEW_PANEL_VERTICAL_PADDING = 32;
	const ALLOWED_EXTENSIONS = new Set(["png", "jpg", "jpeg", "bmp", "svg"]);
	const ALLOWED_MIME_TYPES = new Set(["image/png", "image/jpeg", "image/bmp", "image/x-ms-bmp", "image/svg+xml"]);
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
		{ label: "Resize from top left", placement: "-top-1.5 -left-1.5 size-3.5 cursor-nwse-resize", x: -1, y: -1 },
		{ label: "Resize from top right", placement: "-top-1.5 -right-1.5 size-3.5 cursor-nesw-resize", x: 1, y: -1 },
		{ label: "Resize from bottom left", placement: "-bottom-1.5 -left-1.5 size-3.5 cursor-nesw-resize", x: -1, y: 1 },
		{ label: "Resize from bottom right", placement: "-right-1.5 -bottom-1.5 size-3.5 cursor-nwse-resize", x: 1, y: 1 },
		{ label: "Resize from top", placement: "-top-1 left-1/2 h-2 w-5 -translate-x-1/2 cursor-ns-resize", x: 0, y: -1 },
		{ label: "Resize from bottom", placement: "-bottom-1 left-1/2 h-2 w-5 -translate-x-1/2 cursor-ns-resize", x: 0, y: 1 },
		{ label: "Resize from left", placement: "top-1/2 -left-1 h-5 w-2 -translate-y-1/2 cursor-ew-resize", x: -1, y: 0 },
		{ label: "Resize from right", placement: "top-1/2 -right-1 h-5 w-2 -translate-y-1/2 cursor-ew-resize", x: 1, y: 0 },
	] as const;

	let fileInput: HTMLInputElement;
	let previewCanvas: HTMLCanvasElement;
	let editorViewport: HTMLDivElement;
	let previewPanel: HTMLElement;
	let previewDisplayWidth = 0;
	let layers: ImageLayer[] = [];
	let activeLayerId = "";
	let activeLayer: ImageLayer | undefined;
	let revision = 0;
	let savedRevision = 0;
	let loading = true;
	let applying = false;
	let successMessage = "";
	let errorMessage = "";
	let lastDeviceId = "";
	let loadGeneration = 0;
	let dragPointerId: number | undefined;
	let dragStartX = 0;
	let dragStartY = 0;
	let dragStartOffsetX = 0;
	let dragStartOffsetY = 0;
	let resizePointerId: number | undefined;
	let resizeAnchorX = 0;
	let resizeAnchorY = 0;
	let resizeStartVectorX = 0;
	let resizeStartVectorY = 0;
	let resizeStartVectorLengthSquared = 1;
	let resizeStartWidth = 0;
	let resizeStartHeight = 0;
	let resizeDirectionX = 0;
	let resizeDirectionY = 0;
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
	$: activeLayer = layers.find((layer) => layer.id == activeLayerId);
	$: transformBounds = getTransformBounds(activeLayer, startupImage.width, startupImage.height);
	$: isDirty = revision != savedRevision;
	$: if (previewPanel && previewFrameWidth && previewFrameHeight) updatePreviewDisplaySize();
	$: if (previewCanvas && startupImage.width && startupImage.height) {
		layers;
		drawComposedImage(previewCanvas, layers, startupImage);
	}
	$: if (device.id != lastDeviceId) {
		lastDeviceId = device.id;
		void loadProject(device.id, ++loadGeneration);
	}

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

	function clearEditor() {
		layers = [];
		activeLayerId = "";
		revision = 0;
		savedRevision = 0;
		successMessage = "";
		errorMessage = "";
		applying = false;
		if (fileInput) fileInput.value = "";
	}

	async function loadProject(deviceId: string, generation: number) {
		clearEditor();
		loading = true;
		try {
			const project = await invoke<StartupImageProject>("get_startup_image_project", { device: deviceId });
			const loadedLayers = await Promise.all(
				project.layers.map(async (layer) => ({
					...layer,
					decoded: await decodeImage(layer.image),
				})),
			);
			if (generation != loadGeneration || device.id != deviceId) return;
			layers = loadedLayers;
			activeLayerId = loadedLayers.at(-1)?.id ?? "";
			revision = 0;
			savedRevision = 0;
		} catch (error) {
			if (generation != loadGeneration || device.id != deviceId) return;
			errorMessage = `Unable to load saved startup image: ${String(error)}`;
		} finally {
			if (generation == loadGeneration && device.id == deviceId) loading = false;
		}
	}

	function decodeImage(source: string) {
		return new Promise<HTMLImageElement>((resolve, reject) => {
			const image = new Image();
			image.onload = () => resolve(image);
			image.onerror = () => reject(new Error("The image could not be decoded"));
			image.src = source;
		});
	}

	function drawComposedImage(canvas: HTMLCanvasElement, imageLayers: ImageLayer[], output: { width: number; height: number }) {
		if (!output.width || !output.height) return;
		if (canvas.width != output.width) canvas.width = output.width;
		if (canvas.height != output.height) canvas.height = output.height;

		const context = canvas.getContext("2d");
		if (!context) return;
		context.fillStyle = "#000000";
		context.fillRect(0, 0, canvas.width, canvas.height);
		context.imageSmoothingEnabled = true;
		context.imageSmoothingQuality = "high";

		for (const layer of imageLayers) {
			if (!layer.decoded) continue;
			const fittedImage = getFittedImageSize(layer.decoded, output, layer.zoom);
			context.save();
			context.translate(canvas.width / 2 + layer.offset_x, canvas.height / 2 + layer.offset_y);
			context.rotate((layer.rotation * Math.PI) / 180);
			context.drawImage(layer.decoded, -fittedImage.width / 2, -fittedImage.height / 2, fittedImage.width, fittedImage.height);
			context.restore();
		}
	}

	function readFile(file: File, extension: string) {
		return new Promise<string>((resolve, reject) => {
			const reader = new FileReader();
			reader.onload = () => {
				if (typeof reader.result != "string") {
					reject(new Error(`${file.name} could not be read`));
					return;
				}
				if (extension == "svg") {
					const sanitized = DOMPurify.sanitize(reader.result, {
						USE_PROFILES: { svg: true, svgFilters: true },
						FORBID_TAGS: ["script", "foreignObject", "iframe", "object", "embed"],
					});
					const document = new DOMParser().parseFromString(sanitized, "image/svg+xml");
					if (document.querySelector("parsererror") || document.documentElement.localName != "svg") {
						reject(new Error(`${file.name} is not a valid SVG image`));
						return;
					}
					const bytes = new TextEncoder().encode(sanitized);
					let binary = "";
					for (let index = 0; index < bytes.length; index += 8192) {
						binary += String.fromCharCode(...bytes.subarray(index, index + 8192));
					}
					resolve(`data:image/svg+xml;base64,${btoa(binary)}`);
					return;
				}
				const mimeType = extension == "png" ? "image/png" : extension == "bmp" ? "image/bmp" : "image/jpeg";
				resolve(reader.result.replace(/^data:[^;,]+;base64,/, `data:${mimeType};base64,`));
			};
			reader.onerror = () => reject(new Error(`${file.name} could not be read`));
			if (extension == "svg") reader.readAsText(file);
			else reader.readAsDataURL(file);
		});
	}

	function makeLayerId() {
		return globalThis.crypto?.randomUUID?.() ?? `layer-${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}`;
	}

	async function selectFiles(fileList: FileList | null) {
		successMessage = "";
		errorMessage = "";
		const files = Array.from(fileList ?? []);
		if (!files.length) return;
		if (layers.length + files.length > MAX_LAYERS) {
			errorMessage = `A startup image can contain at most ${MAX_LAYERS} images.`;
			return;
		}

		const validFiles: { file: File; extension: string }[] = [];
		const rejected: string[] = [];
		for (const file of files) {
			const extension = file.name.split(".").pop()?.toLowerCase() ?? "";
			if (!ALLOWED_EXTENSIONS.has(extension) || (file.type && !ALLOWED_MIME_TYPES.has(file.type))) {
				rejected.push(`${file.name} has an unsupported file type`);
			} else if (file.size > MAX_FILE_SIZE) {
				rejected.push(`${file.name} is larger than 10 MB`);
			} else {
				validFiles.push({ file, extension });
			}
		}

		try {
			const addedLayers = await Promise.all(
				validFiles.map(async ({ file, extension }) => {
					const image = await readFile(file, extension);
					return {
						id: makeLayerId(),
						name: file.name,
						image,
						zoom: 1,
						offset_x: 0,
						offset_y: 0,
						rotation: 0,
						decoded: await decodeImage(image),
					} satisfies ImageLayer;
				}),
			);
			layers = [...layers, ...addedLayers];
			activeLayerId = addedLayers.at(-1)?.id ?? activeLayerId;
			if (addedLayers.length) revision += 1;
		} catch (error) {
			rejected.push(String(error));
		}

		if (rejected.length) errorMessage = rejected.join(". ");
		if (fileInput) fileInput.value = "";
	}

	function openFilePicker() {
		fileInput.value = "";
		fileInput.click();
	}

	function updateActiveLayer(changes: Partial<PersistedLayer>) {
		if (!activeLayer) return;
		layers = layers.map((layer) => (layer.id == activeLayerId ? { ...layer, ...changes } : layer));
		revision += 1;
		successMessage = "";
		errorMessage = "";
	}

	function resetPlacement() {
		updateActiveLayer({ zoom: 1, offset_x: 0, offset_y: 0, rotation: 0 });
	}

	function removeLayer(id: string) {
		const index = layers.findIndex((layer) => layer.id == id);
		if (index < 0) return;
		layers = layers.filter((layer) => layer.id != id);
		if (activeLayerId == id) activeLayerId = layers[Math.min(index, layers.length - 1)]?.id ?? "";
		revision += 1;
		successMessage = "";
		errorMessage = "";
	}

	function moveLayer(id: string, direction: 1 | -1) {
		const index = layers.findIndex((layer) => layer.id == id);
		const destination = index + direction;
		if (index < 0 || destination < 0 || destination >= layers.length) return;
		const nextLayers = [...layers];
		[nextLayers[index], nextLayers[destination]] = [nextLayers[destination], nextLayers[index]];
		layers = nextLayers;
		revision += 1;
		successMessage = "";
		errorMessage = "";
	}

	function getTransformBounds(layer: ImageLayer | undefined, outputWidth: number, outputHeight: number) {
		if (!layer?.decoded || !outputWidth || !outputHeight) {
			return { left: 0, top: 0, width: 0, height: 0 };
		}
		const fittedImage = getFittedImageSize(layer.decoded, { width: outputWidth, height: outputHeight }, layer.zoom);
		return {
			left: ((outputWidth - fittedImage.width) / 2 + layer.offset_x) / outputWidth,
			top: ((outputHeight - fittedImage.height) / 2 + layer.offset_y) / outputHeight,
			width: fittedImage.width / outputWidth,
			height: fittedImage.height / outputHeight,
		};
	}

	function getFittedImageSize(image: HTMLImageElement, output: { width: number; height: number }, scale: number) {
		const fillScale = Math.max(output.width / image.naturalWidth, output.height / image.naturalHeight);
		return {
			width: image.naturalWidth * fillScale * scale,
			height: image.naturalHeight * fillScale * scale,
		};
	}

	function rotateVector(x: number, y: number, degrees: number) {
		const radians = (degrees * Math.PI) / 180;
		const cosine = Math.cos(radians);
		const sine = Math.sin(radians);
		return {
			x: x * cosine - y * sine,
			y: x * sine + y * cosine,
		};
	}

	function pointerToOutput(event: PointerEvent) {
		const bounds = editorViewport.getBoundingClientRect();
		return {
			x: ((event.clientX - bounds.left) * startupImage.width) / bounds.width,
			y: ((event.clientY - bounds.top) * startupImage.height) / bounds.height,
		};
	}

	function beginDrag(event: PointerEvent) {
		if (!activeLayer?.decoded || !editorViewport || resizePointerId != undefined || rotatePointerId != undefined) return;
		dragPointerId = event.pointerId;
		dragStartX = event.clientX;
		dragStartY = event.clientY;
		dragStartOffsetX = activeLayer.offset_x;
		dragStartOffsetY = activeLayer.offset_y;
		editorViewport.setPointerCapture(event.pointerId);
	}

	function moveImage(event: PointerEvent) {
		if (dragPointerId != event.pointerId || !editorViewport) return;
		const bounds = editorViewport.getBoundingClientRect();
		if (!bounds.width || !bounds.height) return;
		updateActiveLayer({
			offset_x: dragStartOffsetX + (event.clientX - dragStartX) * (startupImage.width / bounds.width),
			offset_y: dragStartOffsetY + (event.clientY - dragStartY) * (startupImage.height / bounds.height),
		});
	}

	function endDrag(event: PointerEvent) {
		if (dragPointerId != event.pointerId || !editorViewport) return;
		if (editorViewport.hasPointerCapture(event.pointerId)) editorViewport.releasePointerCapture(event.pointerId);
		dragPointerId = undefined;
	}

	function beginResize(event: PointerEvent, directionX: number, directionY: number) {
		if (!activeLayer?.decoded || !editorViewport || rotatePointerId != undefined) return;
		event.preventDefault();
		const fittedImage = getFittedImageSize(activeLayer.decoded, startupImage, activeLayer.zoom);
		const centerX = startupImage.width / 2 + activeLayer.offset_x;
		const centerY = startupImage.height / 2 + activeLayer.offset_y;
		const oppositeCorner = rotateVector((-directionX * fittedImage.width) / 2, (-directionY * fittedImage.height) / 2, activeLayer.rotation);
		const startVector = rotateVector(directionX * fittedImage.width, directionY * fittedImage.height, activeLayer.rotation);

		resizePointerId = event.pointerId;
		resizeAnchorX = centerX + oppositeCorner.x;
		resizeAnchorY = centerY + oppositeCorner.y;
		resizeStartVectorX = startVector.x;
		resizeStartVectorY = startVector.y;
		resizeStartVectorLengthSquared = Math.max(1, startVector.x ** 2 + startVector.y ** 2);
		resizeStartWidth = fittedImage.width;
		resizeStartHeight = fittedImage.height;
		resizeDirectionX = directionX;
		resizeDirectionY = directionY;
		resizeStartZoom = activeLayer.zoom;
		(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
	}

	function resizeImage(event: PointerEvent) {
		if (resizePointerId != event.pointerId || !editorViewport || !activeLayer) return;
		const pointer = pointerToOutput(event);
		const pointerVectorX = pointer.x - resizeAnchorX;
		const pointerVectorY = pointer.y - resizeAnchorY;
		const projectedScale = (pointerVectorX * resizeStartVectorX + pointerVectorY * resizeStartVectorY) / resizeStartVectorLengthSquared;
		const nextZoom = Math.max(0.25, Math.min(3, resizeStartZoom * projectedScale));
		const appliedScale = nextZoom / resizeStartZoom;
		const centerFromAnchor = rotateVector((resizeDirectionX * resizeStartWidth * appliedScale) / 2, (resizeDirectionY * resizeStartHeight * appliedScale) / 2, activeLayer.rotation);

		updateActiveLayer({
			zoom: nextZoom,
			offset_x: resizeAnchorX + centerFromAnchor.x - startupImage.width / 2,
			offset_y: resizeAnchorY + centerFromAnchor.y - startupImage.height / 2,
		});
	}

	function endResize(event: PointerEvent) {
		if (resizePointerId == event.pointerId) resizePointerId = undefined;
	}

	function beginRotate(event: PointerEvent) {
		if (!activeLayer?.decoded || !editorViewport || resizePointerId != undefined) return;
		event.preventDefault();
		const bounds = editorViewport.getBoundingClientRect();
		rotatePointerId = event.pointerId;
		rotateCenterX = bounds.left + (0.5 + activeLayer.offset_x / startupImage.width) * bounds.width;
		rotateCenterY = bounds.top + (0.5 + activeLayer.offset_y / startupImage.height) * bounds.height;
		rotateStartAngle = Math.atan2(event.clientY - rotateCenterY, event.clientX - rotateCenterX);
		rotateStartValue = activeLayer.rotation;
		(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
	}

	function rotateImage(event: PointerEvent) {
		if (rotatePointerId != event.pointerId) return;
		const angle = Math.atan2(event.clientY - rotateCenterY, event.clientX - rotateCenterX);
		updateActiveLayer({ rotation: rotateStartValue + ((angle - rotateStartAngle) * 180) / Math.PI });
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

	function persistedProject(): StartupImageProject {
		return {
			layers: layers.map(({ id, name, image, zoom, offset_x, offset_y, rotation }) => ({
				id,
				name,
				image,
				zoom,
				offset_x,
				offset_y,
				rotation,
			})),
		};
	}

	async function applyImage() {
		if (!layers.length || layers.some((layer) => !layer.decoded) || !device.startup_image || applying) return;
		applying = true;
		successMessage = "";
		errorMessage = "";
		const revisionToSave = revision;
		let projectSaved = false;
		try {
			await invoke("save_startup_image_project", { device: device.id, project: persistedProject() });
			projectSaved = true;
			if (revision == revisionToSave) savedRevision = revisionToSave;

			const output = document.createElement("canvas");
			drawComposedImage(output, layers, startupImage);
			await invoke("set_startup_image", { device: device.id, image: output.toDataURL("image/jpeg", 0.92) });
			successMessage = `Saved and applied to ${device.name}.`;
		} catch (error) {
			errorMessage = projectSaved ? `Saved, but could not apply to the device: ${String(error)}` : `Unable to save startup image: ${String(error)}`;
		} finally {
			applying = false;
		}
	}
</script>

<svelte:window on:pointermove={moveTransform} on:pointerup={endTransform} on:pointercancel={endTransform} />

<div class="flex h-full min-h-0 min-w-0 flex-1 flex-col gap-4">
	<input bind:this={fileInput} type="file" class="hidden" accept=".png,.jpg,.jpeg,.bmp,.svg,image/png,image/jpeg,image/bmp,image/svg+xml" multiple on:change={() => selectFiles(fileInput.files)} />

	<header class="flex shrink-0 flex-wrap items-start gap-3">
		<div class="min-w-0">
			<div class="flex items-center gap-2">
				<Stack size="20" weight="bold" class="text-primary" />
				<h3 class="text-lg font-semibold">Startup composition</h3>
			</div>
			<p class="mt-1 text-sm text-base-content/55">Add multiple images, select a layer, then drag, resize, or rotate it on the device layout.</p>
		</div>
		<div class="ml-auto flex flex-wrap items-center justify-end gap-2">
			{#if loading}
				<span class="badge badge-outline gap-2"><span class="loading loading-spinner loading-xs"></span>Loading</span>
			{:else if successMessage}
				<span class="badge badge-success">Applied &amp; saved</span>
			{:else if layers.length && isDirty}
				<span class="badge badge-warning badge-outline">Unsaved changes</span>
			{:else if layers.length}
				<span class="badge badge-success badge-outline">Saved</span>
			{:else}
				<span class="badge badge-outline">No images</span>
			{/if}
			<button type="button" class="btn btn-sm" disabled={loading || layers.length >= MAX_LAYERS} on:click={openFilePicker}>
				<Plus size="16" weight="bold" />
				Add
			</button>
			<button type="button" class="btn btn-ghost btn-sm" disabled={!activeLayer || applying} on:click={resetPlacement}>Reset selected</button>
			<button type="button" class="btn btn-primary btn-sm min-w-32" disabled={!layers.length || loading || applying} on:click={applyImage}>
				{#if applying}
					<span class="loading loading-spinner loading-sm"></span>
					Applying…
				{:else}
					Apply
				{/if}
			</button>
		</div>
	</header>

	{#if errorMessage}
		<div role="alert" class="alert alert-error shrink-0 py-2 text-sm">
			<span>{errorMessage}</span>
		</div>
	{/if}

	<div class="grid min-h-0 min-w-0 flex-1 grid-cols-[15rem_minmax(0,1fr)] overflow-hidden rounded-xl border border-base-300 bg-base-200">
		<aside class="flex min-h-0 flex-col border-r border-base-300 bg-base-100/60">
			<div class="flex shrink-0 items-center gap-2 border-b border-base-300 px-3 py-3">
				<ImageSquare size="17" weight="bold" />
				<h4 class="text-sm font-semibold">Images</h4>
				<span class="badge badge-sm ml-auto">{layers.length}</span>
			</div>

			{#if layers.length}
				<div class="min-h-0 flex-1 space-y-2 overflow-y-auto p-2">
					{#each [...layers].reverse() as layer (layer.id)}
						{@const layerIndex = layers.findIndex((item) => item.id == layer.id)}
						<div class={`group flex items-center gap-2 rounded-lg border p-2 transition-colors ${activeLayerId == layer.id ? "border-primary bg-primary/10" : "border-base-300 bg-base-100"}`}>
							<button type="button" class="flex min-w-0 flex-1 items-center gap-2 text-left" on:click={() => (activeLayerId = layer.id)}>
								<span class="flex size-10 shrink-0 items-center justify-center overflow-hidden rounded-md border border-base-300 bg-black">
									<img src={layer.image} alt="" class="max-h-full max-w-full object-contain" />
								</span>
								<span class="min-w-0">
									<span class="block truncate text-sm font-medium">{layer.name}</span>
									<span class="block text-[11px] text-base-content/45">Layer {layerIndex + 1}</span>
								</span>
							</button>
							<div class="flex shrink-0 flex-col">
								<button
									type="button"
									class="btn btn-ghost btn-xs h-5 min-h-5 px-1"
									aria-label={`Move ${layer.name} up`}
									disabled={layerIndex == layers.length - 1}
									on:click={() => moveLayer(layer.id, 1)}
								>
									<CaretUp size="13" weight="bold" />
								</button>
								<button type="button" class="btn btn-ghost btn-xs h-5 min-h-5 px-1" aria-label={`Move ${layer.name} down`} disabled={layerIndex == 0} on:click={() => moveLayer(layer.id, -1)}>
									<CaretDown size="13" weight="bold" />
								</button>
							</div>
							<button type="button" class="btn btn-circle btn-ghost btn-xs text-error" aria-label={`Remove ${layer.name}`} on:click={() => removeLayer(layer.id)}>
								<Trash size="15" />
							</button>
						</div>
					{/each}
				</div>
			{:else if loading}
				<div class="flex flex-1 items-center justify-center p-5">
					<span class="loading loading-spinner loading-md text-primary"></span>
				</div>
			{:else}
				<div class="flex flex-1 flex-col items-center justify-center p-5 text-center">
					<div class="mb-3 flex size-12 items-center justify-center rounded-full bg-base-200">
						<ImageSquare size="24" class="text-base-content/45" />
					</div>
					<p class="text-sm font-medium">No images added</p>
					<p class="mt-1 text-xs text-base-content/50">Add one or more PNG, JPG, JPEG, BMP, or SVG files.</p>
					<button type="button" class="btn btn-primary btn-sm mt-4" on:click={openFilePicker}>
						<Plus size="15" weight="bold" />
						Add images
					</button>
				</div>
			{/if}
		</aside>

		<section bind:this={previewPanel} class="relative flex min-h-0 min-w-0 items-center justify-center overflow-hidden p-4">
			<div class="relative shrink-0 overflow-visible rounded-lg bg-black shadow-2xl" style={`width: ${previewDisplayWidth}px; aspect-ratio: ${previewFrameWidth} / ${previewFrameHeight};`}>
				<!-- svelte-ignore a11y-no-static-element-interactions -->
				<div
					bind:this={editorViewport}
					class="absolute touch-none overflow-visible bg-black"
					class:cursor-grab={activeLayer && dragPointerId == undefined && resizePointerId == undefined && rotatePointerId == undefined}
					class:cursor-grabbing={dragPointerId != undefined}
					style={`left: ${(PREVIEW_PADDING / previewFrameWidth) * 100}%; top: ${(PREVIEW_PADDING / previewFrameHeight) * 100}%; width: ${(startupImage.width / previewFrameWidth) * 100}%; height: ${(startupImage.height / previewFrameHeight) * 100}%;`}
					on:pointerdown={beginDrag}
					on:pointermove={moveImage}
					on:pointerup={endDrag}
					on:pointercancel={endDrag}
				>
					<canvas bind:this={previewCanvas} class="absolute inset-0 h-full w-full rounded-lg"></canvas>

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

					{#if activeLayer?.decoded}
						<div
							class="pointer-events-none absolute z-20 border border-primary shadow-[0_0_0_1px_rgba(255,255,255,0.45)]"
							style={`left: ${transformBounds.left * 100}%; top: ${transformBounds.top * 100}%; width: ${transformBounds.width * 100}%; height: ${transformBounds.height * 100}%; transform: rotate(${activeLayer.rotation}deg);`}
						>
							{#each RESIZE_HANDLES as handle}
								<button
									type="button"
									class={`pointer-events-auto absolute rounded-full border border-primary bg-white shadow-sm ${handle.placement}`}
									aria-label={handle.label}
									on:pointerdown|stopPropagation={(event) => beginResize(event, handle.x, handle.y)}
								></button>
							{/each}
							<span class="absolute top-full left-1/2 h-6 w-px -translate-x-1/2 bg-primary"></span>
							<button
								type="button"
								class="pointer-events-auto absolute top-[calc(100%+1.25rem)] left-1/2 z-30 flex size-7 -translate-x-1/2 cursor-grab items-center justify-center rounded-full border border-primary-content/20 bg-primary text-primary-content shadow-md active:cursor-grabbing"
								aria-label="Rotate selected image"
								on:pointerdown|stopPropagation={beginRotate}
							>
								<ArrowClockwise size="16" weight="bold" />
							</button>
						</div>
					{/if}
				</div>
			</div>

			{#if !loading && !layers.length}
				<div class="pointer-events-none absolute inset-0 flex items-end justify-center pb-5">
					<p class="rounded-full bg-base-100/85 px-3 py-1.5 text-xs text-base-content/55 shadow-sm backdrop-blur">The black frame represents the device canvas.</p>
				</div>
			{/if}
		</section>
	</div>
</div>
