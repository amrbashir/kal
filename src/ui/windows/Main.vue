<script lang="ts" setup>
import { watch, onMounted, ref } from "vue";
import { SearchResultItem, IPCEvent } from "../../common";
import { getIconHtml } from "../utils";
import { neutralForegroundHover } from "@fluentui/web-components";

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
      />
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
          <span>
            {{ item.primary_text }}
          </span>
          <span class="text-hint">
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

    <div id="search-footer-container">
      <span></span>
      <Transition name="slide-fade">
        <p class="text-hint" v-if="refreshingIndex">Refreshing...</p>
      </Transition>
    </div>
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

#search-results_container {
  overflow-x: hidden;
  padding: 10px;
  background-color: v-bind(primaryColor);
  width: 100%;
  height: calc(100% - 60px - 45px);
}

#search-results_container:empty {
  padding: 0;
}

.search-results_item,
.search-results_item::part(content) {
  overflow: hidden;
  padding: 0 10px;
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

#search-footer-container {
  display: flex;
  justify-content: space-between;
  padding: 5px 10px;
  height: 45px;
  background-color: v-bind(primaryColor);
  border-radius: 0 0 10px 10px;
}

.slide-fade-leave-active,
.slide-fade-enter-active {
  transition: all 0.3s ease-out;
}

.slide-fade-enter-from,
.slide-fade-leave-to {
  transform: translateX(20px);
  opacity: 0;
}
</style>
