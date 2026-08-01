<script lang="ts">
	import type { ActionInstance } from "$lib/ActionInstance";
	import type { DeviceInfo } from "$lib/DeviceInfo";
	import type { Profile } from "$lib/Profile";

	import { getWebserverUrl, getWebSocketPort } from "$lib/ports";
	import { inspectedInstance } from "$lib/propertyInspector";

	import { invoke } from "@tauri-apps/api/core";
	import { onMount } from "svelte";

	let iframes: { [context: string]: HTMLIFrameElement } = {};
	let iframeContainer: HTMLDivElement;
	let iframeClosePopup: HTMLButtonElement;
	let iframePopupsOpen: string[] = [];

	export let device: DeviceInfo;
	export let profile: Profile;

	async function iframeOnLoad(instance: ActionInstance) {
		const iframe = iframes[instance.context];
		const split = instance.context.split(".");

		const position = parseInt(split[3]);
		let coordinates: { row: number; column: number };
		if (split[2] == "Encoder") {
			coordinates = { row: 0, column: position };
		} else {
			coordinates = { row: Math.floor(position / device.columns), column: position % device.columns };
		}

		if (instance == null || !iframe.src || !iframe.src.startsWith(getWebserverUrl())) return;
		const info = JSON.stringify(await invoke("make_info", { plugin: instance.action.plugin }));

		iframe?.contentWindow?.postMessage(
			{
				event: "connect",
				theme: document.documentElement.classList.contains("dark") ? "dark" : "light",
				payload: [
					getWebSocketPort(),
					instance.context,
					"registerPropertyInspector",
					info,
					JSON.stringify({
						action: instance.action.uuid,
						context: instance.context,
						device: split[0],
						payload: {
							settings: instance.settings,
							coordinates,
							controller: split[2],
							state: instance.current_state,
							isInMultiAction: parseInt(split[4]) != 0,
						},
					}),
				],
			},
			getWebserverUrl(),
		);
	}

	const closePopup = (context: string) => {
		const iframe = iframes[context];
		if (iframe) {
			iframe.style.position = "";
			iframe.style.left = "";
			iframe.style.top = "";
			iframe.style.width = "100%";
			iframe.style.height = "100%";
			iframe.style.display = $inspectedInstance == context ? "block" : "none";
			iframe.contentWindow?.postMessage({ event: "windowClosed" }, getWebserverUrl());
		}

		iframePopupsOpen = iframePopupsOpen.filter((e) => e != context);

		if (iframePopupsOpen.length == 0) {
			iframeContainer.style.position = "";
			iframeContainer.style.width = "";
			iframeContainer.style.height = "";
			iframeContainer.style.padding = "";
			iframeContainer.style.zIndex = "0";

			iframeClosePopup.style.display = "none";
		}
	};

	function combineUint8Arrays(arrays: Uint8Array[]): Uint8Array {
		const totalLength = arrays.reduce((sum, item) => sum + item.length, 0);
		const mergedArray = new Uint8Array(totalLength);
		let offset = 0;
		for (const item of arrays) {
			mergedArray.set(item, offset);
			offset += item.length;
		}
		return mergedArray;
	}

	function handleMessage({ data }: MessageEvent) {
		if (data.event == "windowOpened") {
			const iframe = iframes[data.payload];
			if (!iframe) return;
			iframe.style.position = "absolute";
			iframe.style.left = "24px";
			iframe.style.top = "24px";
			iframe.style.width = "calc(100% - 48px)";
			iframe.style.height = "calc(100% - 48px)";
			iframe.style.display = "block";

			iframePopupsOpen.push(data.payload);

			iframeContainer.style.position = "absolute";
			iframeContainer.style.width = "100%";
			iframeContainer.style.height = "100%";
			iframeContainer.style.padding = "24px";
			iframeContainer.style.zIndex = "20";

			iframeClosePopup.style.display = "block";
		} else if (data.event == "windowClosed") {
			closePopup(data.payload);
		} else if (data.event == "openUrl") {
			invoke("open_url", { url: data.payload });
		} else if (data.event == "fetch") {
			const fetchCORS = (window as typeof window & { fetchCORS: (...args: any[]) => Promise<Response> }).fetchCORS;
			fetchCORS(...data.payload.args)
				.then(async (response: Response) => {
					const chunks = [];
					if (response.body) {
						const reader = response.body.getReader();
						while (true) {
							const { done, value } = await reader.read();
							if (done) break;
							chunks.push(value);
						}
					}
					const body = combineUint8Arrays(chunks);

					iframes[data.payload.context]?.contentWindow?.postMessage(
						{
							event: "fetchResponse",
							payload: {
								id: data.payload.id,
								response: {
									url: response.url,
									body,
									headers: response.headers.entries().toArray(),
									status: response.status,
									statusText: response.statusText,
								},
							},
						},
						getWebserverUrl(),
					);
				})
				.catch((error: any) => {
					iframes[data.payload.context]?.contentWindow?.postMessage({ event: "fetchError", payload: { id: data.payload.id, error } }, getWebserverUrl());
				});
		}
	}

	onMount(() => {
		window.addEventListener("message", handleMessage);
		return () => window.removeEventListener("message", handleMessage);
	});

	const nonNull = <T,>(o: T | null): o is T => o != null;
	$: instances = profile.keys
		.filter(nonNull)
		.reduce((prev, current) => prev.concat(current.children ? [current, ...current.children] : current), [] as ActionInstance[])
		.concat(profile.sliders.filter(nonNull));
</script>

<svelte:window
	on:keydown={(event) => {
		if (event.key == "Escape" && iframePopupsOpen.length > 0) {
			closePopup(iframePopupsOpen[iframePopupsOpen.length - 1]);
		}
	}}
/>

<div class="h-full min-h-0 w-full overflow-auto bg-base-100" bind:this={iframeContainer}>
	<button
		type="button"
		bind:this={iframeClosePopup}
		on:click={() => closePopup(iframePopupsOpen[iframePopupsOpen.length - 1])}
		class="btn btn-circle btn-ghost btn-sm absolute top-2 right-2 hidden"
		aria-label="Close property inspector popup"
	>
		✕
	</button>
	{#each instances as instance (instance.context)}
		{#if instance.action.property_inspector}
			<iframe
				title="Property inspector"
				class="hidden h-full w-full"
				class:block!={$inspectedInstance == instance.context}
				src={getWebserverUrl(instance.action.property_inspector + "|opendeck_property_inspector")}
				name={instance.context}
				bind:this={iframes[instance.context]}
				on:load={() => iframeOnLoad(instance)}
			></iframe>
		{/if}
	{/each}
</div>
