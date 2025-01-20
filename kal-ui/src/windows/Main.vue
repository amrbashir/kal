<script lang="ts" setup>
import { computed, onMounted, ref, useTemplateRef } from "vue";
import { isEventForHotkey as isEventForAccelerator, isVScrollable } from "../utils";
import { watchDebounced } from "@vueuse/core";
import { ResultItem, Action } from "../result_item";
import ReloadIcon from "../components/ReloadIcon.vue";
import ResultItemComponent from "../components/ResultItem.vue";
import { IpcCommand, IpcEvent } from "../ipc";
import { useConfig } from "../composables/config";
import { useSystemAccentColor } from "../composables/systemAccentColor";

const config = useConfig();
const systemAccentColor = useSystemAccentColor();

const inputRef = useTemplateRef<HTMLElement>("input-ref");
onMounted(() =>
  window.KAL.ipc.on(IpcEvent.FocusInput, () => {
    const shadowRoot = inputRef.value?.shadowRoot;
    const input = shadowRoot?.querySelector<HTMLInputElement>("#control");
    input?.focus();
    input?.select();
  }),
);

const resultItemRefs = useTemplateRef("result-item-refs");
const itemsContainerRef = useTemplateRef<HTMLElement>("items-container-ref");

const results = ref<ResultItem[]>([]);

const reloading = ref(false);

const currentQuery = ref("");
const currentSelection = ref(0);
const currentSelectedItem = computed(() => results.value[currentSelection.value]);
const currentSelectedAction = ref(0);

watchDebounced(currentQuery, (query) => runQuery(query), {
  debounce: 200,
});

async function runQuery(query: string) {
  if (query) {
    console.log(query);
    const response: ResultItem[] = await window.KAL.ipc.invoke(IpcCommand.Query, query);
    resetSelection();
    results.value = response;
  } else {
    results.value = [];
    await window.KAL.ipc.invoke(IpcCommand.ClearResults);
  }
}

function resetSelection() {
  currentSelection.value = 0;
  currentSelectedAction.value = 0;
}

function resetQuery() {
  currentQuery.value = "";
  resetSelection();
}

async function hideMainWindow() {
  await window.KAL.ipc.invoke(IpcCommand.HideMainWindow);
}

async function runAction(action: Action) {
  const payload = `${action.id}#${currentSelectedItem.value.id}`;
  await window.KAL.ipc.invoke(IpcCommand.RunAction, payload);
}

function selectNextItem() {
  const current = currentSelection.value;
  currentSelection.value = current === results.value.length - 1 ? 0 : current + 1;
  currentSelectedAction.value = 0;
}

function selectPrevItem() {
  const current = currentSelection.value;
  currentSelection.value = current === 0 ? results.value.length - 1 : current - 1;
  currentSelectedAction.value = 0;
}

function updateSelection(e: KeyboardEvent) {
  if (e.key === "ArrowDown") {
    selectNextItem();
  }

  if (e.key === "ArrowUp") {
    selectPrevItem();
  }

  if (e.key === "Tab" && !e.shiftKey) {
    if (!config.value.general.tab_through_context_buttons) {
      selectNextItem();
    } else if (currentSelectedAction.value == currentSelectedItem.value.actions.length - 1) {
      selectNextItem();
    } else {
      currentSelectedAction.value = currentSelectedAction.value + 1;
    }
  }

  if (e.key === "Tab" && e.shiftKey) {
    if (!config.value.general.tab_through_context_buttons) {
      selectPrevItem();
    } else if (currentSelectedAction.value == 0) {
      selectPrevItem();
      currentSelectedAction.value = currentSelectedItem.value.actions.length - 1;
    } else {
      currentSelectedAction.value = currentSelectedAction.value - 1;
    }
  }
}

function scrollSelected() {
  // avoid scrolling if container is not scrollable atm
  if (!isVScrollable(itemsContainerRef.value)) return;

  const current = currentSelection.value;
  const block = current === 0 ? "end" : current === results.value.length - 1 ? "start" : "nearest";
  resultItemRefs.value?.[current]?.$el.scrollIntoView({
    behavior: "instant",
    block,
  });
}

async function reload() {
  reloading.value = true;
  await window.KAL.ipc.invoke(IpcCommand.Reload);
  reloading.value = false;
  runQuery(currentQuery.value);
}

function onInputChange(e: InputEvent) {
  if (e.target && "value" in e.target && e.target.value === "") {
    resetQuery();
  }
}

async function onInputKeyDown(e: KeyboardEvent) {
  if (e.key === "Escape") {
    e.preventDefault();
    await hideMainWindow();
  }

  if (e.ctrlKey && e.key === "r") {
    e.preventDefault();
    await reload();
  }

  if (results.value.length === 0) return;

  if (e.key === "Enter" && currentSelectedAction.value > 0) {
    e.preventDefault();
    const action = currentSelectedItem.value.actions[currentSelectedAction.value];
    await runAction(action);
    return;
  }

  for (const action of currentSelectedItem.value.actions) {
    if (action.accelerator && isEventForAccelerator(e, action.accelerator)) {
      e.preventDefault();
      await runAction(action);
      return;
    }
  }

  if (["ArrowDown", "Tab", "ArrowUp"].includes(e.key)) {
    e.preventDefault();
    updateSelection(e);
    scrollSelected();
  }
}

const isTransparent = computed(() => config.value.appearance.transparent);
const bgPrimaryColor = computed(() =>
  isTransparent.value ? "bg-transparent" : "bg-[rgba(21,_20,_20,_0.75)]",
);
const inputHeight = computed(() => `${config.value.appearance.input_height}px`);
const itemHeight = computed(() => `${config.value.appearance.item_height}px`);
const itemsContainerHeight = computed(() => `calc(100% - ${inputHeight.value})`);
</script>

<template>
  <main class="w-full h-full" :class="bgPrimaryColor">
    <fluent-search
      ref="input-ref"
      :style="{ height: inputHeight }"
      :class="[
        'part:root:h-full w-full px-4 text-1rem',
        'part:root:bg-none part:start:scale-120 after:hidden',
        results.length === 0 ? 'rd-2' : 'rd-[0.5rem_0.5rem_0_0]',
      ]"
      placeholder="Start typing..."
      v-model="currentQuery"
      @keydown="onInputKeyDown"
      @change="onInputChange"
    >
      <div slot="clear-button" class="flex justify-center items-center">
        <Transition name="fade">
          <ReloadIcon v-if="reloading" class="animate-spin" />
          <span v-else></span>
        </Transition>
      </div>
    </fluent-search>

    <fluent-divider />

    <ul
      ref="items-container-ref"
      :style="{ height: itemsContainerHeight }"
      class="overflow-x-hidden overflow-y-auto p-4 children:mb-1"
      tabindex="-1"
    >
      <ResultItemComponent
        :itemHeight
        v-for="(item, index) in results"
        ref="result-item-refs"
        :item
        :selected="index === currentSelection"
        :selectedAction="currentSelectedAction"
      />
    </ul>
  </main>
</template>

<style>
* {
  --accent-fill-rest: v-bind(systemAccentColor);
}
</style>
