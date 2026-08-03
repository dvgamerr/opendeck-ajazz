import { normaliseProfiles, normaliseSelection, reorderProfiles } from "./profilePaginationState.js";

let websocket;
let actionContext;
let availableProfiles = [];
let selectedProfiles = [];
let lastSavedProfiles = "";

function save() {
	const serialized = JSON.stringify(selectedProfiles);
	if (serialized === lastSavedProfiles || websocket?.readyState !== WebSocket.OPEN) return;

	websocket.send(
		JSON.stringify({
			event: "setSettings",
			context: actionContext,
			payload: { profiles: selectedProfiles },
		}),
	);
	lastSavedProfiles = serialized;
}

function profileRow(profile, pageIndex = -1) {
	const row = document.createElement("div");
	row.className = "pagination-row";

	const main = document.createElement("div");
	main.className = "pagination-row-main";

	if (pageIndex >= 0) {
		const page = document.createElement("span");
		page.className = "badge badge-primary badge-sm";
		page.textContent = String(pageIndex + 1);
		page.setAttribute("aria-label", `Page ${pageIndex + 1}`);
		main.append(page);
	}

	const name = document.createElement("span");
	name.className = "pagination-name";
	name.textContent = profile;
	name.title = profile;
	main.append(name);
	row.append(main);

	return row;
}

function actionButton(label, symbol, onClick, disabled = false, style = "btn-ghost") {
	const button = document.createElement("button");
	button.type = "button";
	button.className = `btn btn-xs ${style}`;
	button.textContent = symbol;
	button.title = label;
	button.setAttribute("aria-label", label);
	button.disabled = disabled;
	button.addEventListener("click", onClick);
	return button;
}

function updateSelection(nextProfiles) {
	selectedProfiles = normaliseSelection(nextProfiles, availableProfiles);
	render();
	save();
}

function moveProfile(from, to) {
	updateSelection(reorderProfiles(selectedProfiles, from, to));
}

function renderSelected() {
	const list = document.getElementById("selected-profiles");
	list.replaceChildren();

	selectedProfiles.forEach((profile, index) => {
		const row = profileRow(profile, index);
		const actions = document.createElement("div");
		actions.className = "pagination-actions";
		actions.append(
			actionButton(`Move ${profile} to the previous page`, "↑", () => moveProfile(index, index - 1), index === 0),
			actionButton(`Move ${profile} to the next page`, "↓", () => moveProfile(index, index + 1), index === selectedProfiles.length - 1),
			actionButton(`Remove ${profile} from page order`, "×", () => updateSelection(selectedProfiles.filter((selected) => selected !== profile)), false, "btn-ghost text-error"),
		);
		row.append(actions);
		list.append(row);
	});

	document.getElementById("selected-empty").classList.toggle("pagination-hidden", selectedProfiles.length > 0);
	document.getElementById("clear-all").disabled = selectedProfiles.length === 0;
}

function renderAvailable() {
	const list = document.getElementById("available-profiles");
	const query = document.getElementById("profile-search").value.trim().toLocaleLowerCase();
	const remaining = availableProfiles.filter((profile) => !selectedProfiles.includes(profile));
	const visible = remaining.filter((profile) => profile.toLocaleLowerCase().includes(query));
	list.replaceChildren();

	for (const profile of visible) {
		const row = profileRow(profile);
		row.append(actionButton(`Add ${profile} to page order`, "+", () => updateSelection([...selectedProfiles, profile]), false, "btn-primary"));
		list.append(row);
	}

	const empty = document.getElementById("available-empty");
	empty.textContent =
		availableProfiles.length === 0 ? "No profiles are available for this device." : remaining.length === 0 ? "Every profile is already in the page order." : "No profiles match your search.";
	empty.classList.toggle("pagination-hidden", visible.length > 0);
	document.getElementById("add-all").disabled = remaining.length === 0;
}

function render() {
	renderSelected();
	renderAvailable();
	document.getElementById("selection-status").textContent = `${selectedProfiles.length} of ${availableProfiles.length} selected`;
}

function applySettings(settings) {
	const configured = Array.isArray(settings?.profiles) ? settings.profiles : [];
	selectedProfiles = normaliseSelection(configured, availableProfiles);
	lastSavedProfiles = JSON.stringify(selectedProfiles);
	render();
}

function connectElgatoStreamDeckSocket(inPort, inPropertyInspectorUUID, inRegisterEvent, _inInfo, inActionInfo) {
	const actionInfo = JSON.parse(inActionInfo);
	actionContext = actionInfo.context;
	availableProfiles = normaliseProfiles(Array.isArray(actionInfo.payload.profiles) ? actionInfo.payload.profiles : []);
	applySettings(actionInfo.payload.settings);

	websocket = new WebSocket(`ws://localhost:${inPort}`);
	websocket.addEventListener("open", () => {
		websocket.send(JSON.stringify({ event: inRegisterEvent, uuid: inPropertyInspectorUUID }));
		save();
	});
	websocket.addEventListener("message", ({ data }) => {
		const message = JSON.parse(data);
		if (message.event === "didReceiveSettings" && message.context === actionContext) applySettings(message.payload.settings);
	});
}

window.connectElgatoStreamDeckSocket = connectElgatoStreamDeckSocket;
window.addEventListener("DOMContentLoaded", () => {
	document.getElementById("profile-search").addEventListener("input", renderAvailable);
	document.getElementById("add-all").addEventListener("click", () => updateSelection([...selectedProfiles, ...availableProfiles]));
	document.getElementById("clear-all").addEventListener("click", () => updateSelection([]));
	render();
});
