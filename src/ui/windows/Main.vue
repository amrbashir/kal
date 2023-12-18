<script lang="ts" setup>
import { watch, onMounted, ref } from "vue";
import { SearchResultItem, IPCEvent, Icon, IconType } from "../../common";
import SearchIcon from "../icons/SearchIcon.vue";
import RefreshingIcon from "../icons/RefreshingIcon.vue";

let results = ref<SearchResultItem[]>([]);
let currentQuery = ref("");
let currentSelection = ref(0);
let refreshingIndex = ref(false);

// handlers
function search(query: string) {
  if (query) {
    window.KAL.ipc.send(IPCEvent.Search, query);
  } else {
    results.value = [];
    window.KAL.ipc.send(IPCEvent.ClearResults);
  }
}

watch(currentQuery, (query) => search(query));

function onkeydown(e: KeyboardEvent) {
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

    document
      .getElementById(`search-results_item_#${currentSelection}`)
      ?.scrollIntoView({ behavior: "smooth", block: "nearest" });
  }

  if (e.key === "Enter") {
    e.preventDefault();
    window.KAL.ipc.send(IPCEvent.Execute, currentSelection.value, e.shiftKey);
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

// utils
function convertFileSrc(protocol: string, filePath: string): string {
  const path = encodeURIComponent(filePath);
  return navigator.userAgent.includes("Windows")
    ? `http://${protocol}.localhost/${path}`
    : `${protocol}://${path}`;
}

function getIconHtml(icon: Icon): string {
  switch (icon.type) {
    case IconType.Default:
    case IconType.Path:
      return `<img src="${convertFileSrc("kalasset", icon.data)}" />`;
    case IconType.Svg:
      return icon.data;
    default:
      return "<span>TODO: empty icon</span>";
  }
}
</script>

<template>
  <main>
    <div
      id="search-input_container"
      :style="{
        'border-radius': results.length === 0 ? '10px 10px' : '10px 10px 0 0',
      }"
    >
      <div id="search-input_icon-container">
        <SearchIcon id="search-input_icon" />
      </div>

      <input
        id="search-input"
        placeholder="Search..."
        v-model="currentQuery"
        @keydown="onkeydown"
      />

      <Transition name="slide-fade">
        <div v-if="refreshingIndex" id="refreshing_icon-container">
          <RefreshingIcon id="refreshing_icon" />
        </div>
      </Transition>
    </div>

    <div id="search-results_container">
      <ul>
        <li
          v-for="(item, index) in results"
          :id="`search-results_item_#${index}`"
          class="search-results_item"
          :class="{ selected: index === currentSelection }"
        >
          <div
            class="search-results_item_left"
            v-html="getIconHtml(item.icon)"
          ></div>
          <div class="search-results_item_right">
            <span class="text-primary">{{ item.primary_text }}</span>
            <span class="text-secondary">{{ item.secondary_text }}</span>
          </div>
        </li>
      </ul>
    </div>
  </main>
</template>

<style>
:root {
  --primary: rgba(21, 20, 20, 0.75);
  --accent: rgba(70, 140, 210, 0.5);
  --accent-lighter: rgba(90, 163, 235, 0.5);
  --text-primary: rgba(180, 180, 180);
  --text-secondary: rgb(100, 100, 100);
  --text-primary-on-accent: rgba(255, 255, 255);
  --text-secondary-on-accent: rgb(160, 160, 160);
}

main {
  overflow: hidden;
  width: 100vw;
  height: 100vh;
}

#search-input_container {
  appearance: none;
  background-color: var(--primary);
  width: 100%;
  height: 60px;
  display: flex;
}

#search-input {
  flex-grow: 1;
  background: transparent;
  height: 100%;
  outline: none;
  border: none;
  font-size: larger;
  color: var(--text-primary);
  padding: 1rem;
}

#search-input::placeholder {
  color: var(--text-secondary);
}

#search-input_icon-container,
#refreshing_icon-container {
  display: grid;
  place-items: center;
  height: 100%;
  width: 50px;
  color: var(--text-primary);
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

#refreshing_icon {
  animation: rotation 1s infinite linear;
}

@keyframes rotation {
  from {
    transform: rotate(0deg);
  }

  to {
    transform: rotate(359deg);
  }
}

#search-results_container {
  overflow-x: hidden;
  background-color: var(--primary);
  width: 100%;
  height: 400px;
  border-radius: 0 0 10px 10px;
}

.search-results_item {
  overflow: hidden;
  padding: 0 10px;
  list-style: none;
  display: flex;
  width: 100%;
  height: 60px;
}

.search-results_item:hover,
.search-results_item.selected {
  background-color: var(--accent);
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

.search-results_item_right {
  overflow: hidden;
  height: 100%;
  display: flex;
  flex-direction: column;
  justify-content: center;
}

.search-results_item_right span {
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}

.text-primary {
  color: var(--text-primary);
}

.text-secondary {
  color: var(--text-secondary);
}

.search-results_item.selected .text-primary {
  color: var(--text-primary-on-accent);
}

.search-results_item.selected .text-secondary {
  color: var(--text-secondary-on-accent);
}
</style>
