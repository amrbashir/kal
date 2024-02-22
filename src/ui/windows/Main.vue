<script lang="ts" setup>
import { watch, onMounted, ref } from "vue";
import { SearchResultItem, IPCEvent } from "../../common";
import { getIconHtml } from "../utils";
import { neutralForegroundHover } from "@fluentui/web-components";
import RestartIcon from "../../common/icons/windows/restart.svg";

const neutralForegroundHover10percent = `${neutralForegroundHover
  .getValueFor(document.documentElement)
  .toColorString()}1A`;

const primaryColor = window.KAL.config?.appearance.transparent
  ? "rgba(0, 0, 0, 0)"
  : "rgba(21, 20, 20, 0.75)";

let results = ref<SearchResultItem[]>([]);
let currentQuery = ref("");
let currentSelection = ref(0);
let refreshingIndex = ref(false);
let gettingConfirmation = ref(false);

function search(query: string) {
  if (query) {
    window.KAL.ipc.send(IPCEvent.Search, query);
  } else {
    results.value = [];
    window.KAL.ipc.send(IPCEvent.ClearResults);
  }
}

watch(currentQuery, (query) => search(query));

function onChange(e: InputEvent) {
  if (e.target && "value" in e.target && e.target.value === "") {
    gettingConfirmation.value = false;
    currentQuery.value = "";
    currentSelection.value = 0;
  }
}

function onkeydown(e: KeyboardEvent) {
  if (gettingConfirmation.value && e.key !== "Enter") {
    gettingConfirmation.value = false;
  }

  if (e.key === "Escape") {
    e.preventDefault();
    window.KAL.ipc.send(IPCEvent.HideMainWindow);
  }

  if (["ArrowDown", "ArrowUp"].includes(e.key)) {
    e.preventDefault();
    if (e.key === "ArrowDown") {
      currentSelection.value =
        currentSelection.value === results.value.length - 1
          ? 0
          : currentSelection.value + 1;
    } else {
      currentSelection.value =
        currentSelection.value === 0
          ? results.value.length - 1
          : currentSelection.value - 1;
    }

    const block: ScrollLogicalPosition =
      currentSelection.value === 0
        ? "end"
        : currentSelection.value === results.value.length - 1
          ? "start"
          : "nearest";

    document
      .getElementById(`search-results_item_#${currentSelection.value}`)
      ?.scrollIntoView({ behavior: "smooth", block });
  }

  if (e.key === "Enter") {
    e.preventDefault();

    const confirm = results.value[currentSelection.value].needs_confirmation;
    if (!gettingConfirmation.value && confirm) {
      gettingConfirmation.value = true;
      return;
    } else {
      gettingConfirmation.value = false;
      window.KAL.ipc.send(IPCEvent.Execute, currentSelection.value, e.shiftKey);
    }
  }

  if (e.ctrlKey && e.key === "o") {
    e.preventDefault();
    window.KAL.ipc.send(IPCEvent.OpenLocation, currentSelection.value);
  }

  if (e.ctrlKey && e.key === "r") {
    e.preventDefault();
    window.KAL.ipc.send(IPCEvent.RefreshIndex);
    refreshingIndex.value = true;
  }
}

onMounted(() => {
  window.KAL.ipc.on(IPCEvent.FocusInput, () => {
    let input = document.getElementById("search-input");
    input?.focus();
    (input as HTMLInputElement).select();
  });

  window.KAL.ipc.on(IPCEvent.Results, (payload: SearchResultItem[]) => {
    currentSelection.value = 0;
    results.value = payload;
  });

  window.KAL.ipc.on(IPCEvent.RefreshingIndexFinished, () => {
    setTimeout(() => (refreshingIndex.value = false), 500); // artifical delay for transition purposes
    search(currentQuery.value);
  });
});
</script>

<template>
  <main>
    <div
      id="search-input_container"
      :style="{
        'border-radius': results.length === 0 ? '10px 10px' : '10px 10px 0 0',
      }"
    >
      <fluent-search
        id="search-input"
        placeholder="Search..."
        v-model="currentQuery"
        @keydown="onkeydown"
        @change="onChange"
      >
        <Transition name="fade">
          <span id="refresh-icon" slot="clear-button" v-if="refreshingIndex">
            <svg width="24" height="24" viewBox="0 0 24 24">
              <path
                fill="currentColor"
                d="M5.463 4.433A9.961 9.961 0 0 1 12 2c5.523 0 10 4.477 10 10c0 2.136-.67 4.116-1.81 5.74L17 12h3A8 8 0 0 0 6.46 6.228l-.997-1.795zm13.074 15.134A9.961 9.961 0 0 1 12 22C6.477 22 2 17.523 2 12c0-2.136.67-4.116 1.81-5.74L7 12H4a8 8 0 0 0 13.54 5.772l.997 1.795z"
              ></path>
            </svg>
          </span>
        </Transition>
      </fluent-search>
    </div>

    <fluent-divider />

    <ul id="search-results_container">
      <fluent-option
        v-for="(item, index) in results"
        :id="`search-results_item_#${index}`"
        class="search-results_item"
        :class="{ selected: index === currentSelection }"
        :aria-selected="index === currentSelection"
      >
        <div
          class="search-results_item_left"
          v-html="getIconHtml(item.icon)"
        ></div>
        <div class="search-results_item_center">
          <span class="text-big">
            {{ item.primary_text }}
          </span>
          <span class="text-hint text-small">
            {{ item.secondary_text }}
          </span>
        </div>
        <div class="search-results_item_right">
          <Transition name="slide-fade">
            <span
              class="text-warning"
              v-if="gettingConfirmation && currentSelection == index"
            >
              Are your sure?
            </span>
          </Transition>
        </div>
      </fluent-option>
    </ul>

    <fluent-divider v-if="results.length > 0" />
  </main>
</template>

<style>
::-webkit-scrollbar {
  width: 5px;
}
::-webkit-scrollbar-thumb {
  border-radius: 10px;
  background-color: var(--neutral-fill-strong-rest);
}
::-webkit-scrollbar-thumb:hover {
  background-color: var(--neutral-fill-strong-hover);
}

main {
  overflow: hidden;
  width: 100vw;
  height: 100vh;
}

.text-warning {
  color: rgb(255, 181, 125);
}

.text-hint {
  color: var(--neutral-fill-strong-hover);
}

.text-big {
  font-size: medium;
}
.text-small {
  font-size: small;
}

#search-input_container {
  display: flex;
  gap: 10px;
  padding: 10px;
  height: 60px;
  background-color: v-bind(primaryColor);
}

#search-input {
  width: 100%;
  height: 100%;
  transition: all 0.3s ease-out;
}

#search-input::part(root) {
  height: 100% !important;
}

#refresh-icon svg {
  width: 100%;
  height: 100%;
  padding: 20%;
}
#refresh-icon svg {
  animation: rotate 1s infinite;
}

@keyframes rotate {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}

#search-results_container {
  overflow-x: hidden;
  padding: 10px;
  background-color: v-bind(primaryColor);
  width: 100%;
  height: calc(100% - 60px);
}

#search-results_container:empty {
  padding: 0;
}

.search-results_item,
.search-results_item::part(content) {
  overflow: hidden;
  display: flex;
  width: 100%;
  height: 60px;
}

/** Gap between items without flex */
.search-results_item {
  margin-bottom: 5px;
}
.search-results_item:last-child {
  margin-bottom: 0px;
}

/**
 Fallback rules when aria-selected="true" effects doesn't work initially
 TODO: find out why and remove these hacks, except semi-transparent background
 */
fluent-option[aria-selected="false"].search-results_item {
  background-color: transparent;
}
fluent-option[aria-selected="false"].search-results_item.selected,
.search-results_item.selected,
fluent-option[aria-selected="false"].search-results_item:hover,
.search-results_item:hover {
  background: v-bind(neutralForegroundHover10percent);
}
fluent-option[aria-selected="false"].search-results_item.selected::before,
.search-results_item.selected::before,
fluent-option[aria-selected="false"].search-results_item:hover::before,
.search-results_item:hover::before {
  background: var(--accent-fill-rest);
}

.search-results_item_left {
  flex-shrink: 0;
  width: 60px;
  height: 100%;
  display: grid;
  place-items: center;
}

.search-results_item_left > * {
  width: 50%;
  height: 50%;
}

.search-results_item_center {
  overflow: hidden;
  height: 100%;
  display: flex;
  flex: 1;
  flex-direction: column;
  justify-content: center;
}

.search-results_item_center span {
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}

.search-results_item_right {
  display: flex;
  justify-content: center;
  align-items: center;
}

.fade-leave-active,
.fade-enter-active {
  transition: opacity 0.3s ease-out;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
