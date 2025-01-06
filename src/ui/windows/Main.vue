<script lang="ts" setup>
import { watch, onMounted, ref } from "vue";
import { SearchResultItem, IPCEvent } from "../../common";
import { getIconHtml, isVScrollable } from "../utils";
import { neutralForegroundHover } from "@fluentui/web-components";
import { watchDebounced } from "@vueuse/core";

const inputRef = ref<Element | null>(null);
onMounted(() =>
  window.KAL.ipc.on(IPCEvent.FocusInput, () => {
    const shadowRoot = inputRef.value?.shadowRoot;
    const input = shadowRoot?.querySelector<HTMLInputElement>("#control");
    input?.focus();
    input?.select();
  }),
);

const refreshingIndex = ref(false);
onMounted(() =>
  window.KAL.ipc.on(IPCEvent.RefreshingIndexFinished, () => {
    setTimeout(() => (refreshingIndex.value = false), 500); // artifical delay for nicer animation
    search(currentQuery.value);
  }),
);

const gettingConfirmation = ref(false);
const gettingConfirmationIndex = ref(0);
function resetConfirm() {
  gettingConfirmation.value = false;
  gettingConfirmationIndex.value = 0;
}

const currentSelection = ref(0);
function updateSelection(e: KeyboardEvent) {
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
}

const resultsContainerRef = ref<Element | null>(null);
function scrollSelected() {
  // avoid scrolling if container is not scrollable atm
  if (!isVScrollable(resultsContainerRef.value)) return;

  const block: ScrollLogicalPosition =
    currentSelection.value === 0
      ? "end"
      : currentSelection.value === results.value.length - 1
        ? "start"
        : "nearest";

  resultItemRefs.value[currentSelection.value]?.scrollIntoView({
    behavior: "smooth",
    block,
  });
}

const results = ref<SearchResultItem[]>([]);
const resultItemRefs = ref<(HTMLElement | null)[]>([]);
onMounted(() =>
  window.KAL.ipc.on(IPCEvent.Results, (payload: SearchResultItem[]) => {
    currentSelection.value = 0;
    results.value = payload;
  }),
);

const currentQuery = ref("");
watchDebounced(currentQuery, (query) => search(query), {
  debounce: 300,
  maxWait: 3000,
});

function search(query: string) {
  if (query) {
    window.KAL.ipc.send(IPCEvent.Search, query);
  } else {
    results.value = [];
    window.KAL.ipc.send(IPCEvent.ClearResults);
  }
}

function resetQuery() {
  currentQuery.value = "";
  currentSelection.value = 0;
}

function executeItem(e: { shiftKey: boolean }, index: number) {
  let item = results.value[index];
  const confirm = item.needs_confirmation;

  if (
    (!confirm && gettingConfirmation.value) ||
    (confirm &&
      gettingConfirmation.value &&
      gettingConfirmationIndex.value !== index)
  ) {
    resetConfirm();
  }

  if (confirm && !gettingConfirmation.value) {
    gettingConfirmation.value = true;
    gettingConfirmationIndex.value = index;
  } else {
    resetConfirm();
    console.log(item);
    window.KAL.ipc.send(IPCEvent.Execute, item.id, e.shiftKey);
  }
}

function showItemInDir(index: number) {
  let item = results.value[index];
  window.KAL.ipc.send(IPCEvent.OpenLocation, item.id);
}

function onChange(e: InputEvent) {
  if (e.target && "value" in e.target && e.target.value === "") {
    resetConfirm();
    resetQuery();
  }
}

function onkeydown(e: KeyboardEvent) {
  if (gettingConfirmation.value && e.key !== "Enter") {
    resetConfirm();
  }

  if (e.key === "Escape") {
    e.preventDefault();
    window.KAL.ipc.send(IPCEvent.HideMainWindow);
  }

  if (["ArrowDown", "ArrowUp"].includes(e.key)) {
    e.preventDefault();
    updateSelection(e);
    scrollSelected();
  }

  if (e.key === "Enter") {
    e.preventDefault();
    executeItem(e, currentSelection.value);
  }

  if (e.ctrlKey && e.key === "o") {
    e.preventDefault();
    showItemInDir(currentSelection.value);
  }

  if (e.ctrlKey && e.key === "r") {
    e.preventDefault();
    window.KAL.ipc.send(IPCEvent.RefreshIndex);
    refreshingIndex.value = true;
  }
}

// styles
const neutralForegroundHover10percent = `${neutralForegroundHover
  .getValueFor(document.documentElement)
  .toColorString()}1A`;
const bgPrimaryColor = window.KAL.config?.appearance.transparent
  ? "bg-transparent"
  : "bg-[rgba(21,_20,_20,_0.75)]";

const inputHeight = window.KAL.config?.appearance.input_height;
const resultsRowHeight = window.KAL.config?.appearance.results_row_height;
const resultsRowHeightPx = `${resultsRowHeight}px`;
</script>

<template>
  <main class="w-100vw h-100vh overflow-hidden">
    <div
      :style="{ height: `${inputHeight}px` }"
      :class="[
        'p-2',
        results.length === 0 ? 'rd-2' : 'rd-[0.5rem_0.5rem_0_0]',
        bgPrimaryColor,
      ]"
    >
      <fluent-search
        ref="inputRef"
        class="w-full h-full part:root:h-full!"
        placeholder="Search..."
        v-model="currentQuery"
        @keydown="onkeydown"
        @change="onChange"
      >
        <Transition name="fade">
          <span slot="clear-button" v-if="refreshingIndex">
            <svg
              class="animate-spin w-full h-full p-25%"
              width="24"
              height="24"
              viewBox="0 0 24 24"
            >
              <path
                d="M5.463 4.433A9.961 9.961 0 0 1 12 2c5.523 0 10 4.477 10 10c0 2.136-.67 4.116-1.81 5.74L17 12h3A8 8 0 0 0 6.46 6.228l-.997-1.795zm13.074 15.134A9.961 9.961 0 0 1 12 22C6.477 22 2 17.523 2 12c0-2.136.67-4.116 1.81-5.74L7 12H4a8 8 0 0 0 13.54 5.772l.997 1.795z"
              ></path>
            </svg>
          </span>
        </Transition>
      </fluent-search>
    </div>

    <fluent-divider />

    <ul
      ref="resultsContainerRef"
      :style="{ height: `calc(100% - ${inputHeight}px)` }"
      class="overflow-x-hidden p-2 w-full"
      :class="bgPrimaryColor"
    >
      <fluent-option
        v-for="(item, index) in results"
        @click="(e: MouseEvent) => executeItem(e, index)"
        ref="resultItemRefs"
        :style="{ height: `${resultsRowHeight}px` }"
        class="overflow-hidden flex w-full part:content:overflow-hidden mb-1 last:mb-0 part:content:flex part:content:w-full"
        :class="{ selected: index === currentSelection }"
        :aria-selected="index === currentSelection"
      >
        <div
          :style="{ width: `${resultsRowHeight}px` }"
          class="flex-shrink-0 h-full grid place-items-center children:w-50% children:h-50%"
          v-html="getIconHtml(item.icon)"
        ></div>

        <div
          class="overflow-hidden h-full flex flex-1 flex-col justify-center children:overflow-hidden children:text-nowrap children:text-ellipsis"
        >
          <span class="text-lg">
            {{ item.primary_text }}
          </span>
          <span class="text-[var(--neutral-fill-strong-hover)] text-xs">
            {{ item.secondary_text }}
          </span>
        </div>

        <div class="flex justify-center items-center p-2">
          <Transition name="slide-fade">
            <span
              class="text-orange-300"
              v-if="gettingConfirmation && gettingConfirmationIndex == index"
            >
              Are your sure?
            </span>
          </Transition>
        </div>
      </fluent-option>
    </ul>
  </main>
</template>

<style scoped>
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

ul fluent-option[aria-selected="false"] {
  background-color: transparent;
}
ul fluent-option[aria-selected="true"],
/*
 TODO: find out if we can remove the following fallback rules
       when going from 0 results to more than one result,
       aria-selected for the first element is still "false"
       even though currentselection is 0
 */
ul fluent-option[aria-selected="false"].selected,
ul fluent-option[aria-selected="false"]:hover {
  background: v-bind(neutralForegroundHover10percent);
}
ul fluent-option[aria-selected="false"].selected::before,
ul fluent-option[aria-selected="false"]:hover::before {
  background: var(--accent-fill-rest);
}

ul fluent-option::part(content) {
  height: v-bind(resultsRowHeightPx);
}

.fade-leave-active,
.fade-enter-active {
  transition: opacity 0.3s ease-out;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
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
