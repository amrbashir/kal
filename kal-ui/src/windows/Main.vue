<script lang="ts" setup>
import { computed, onMounted, ref, useTemplateRef } from "vue";
import { isVScrollable } from "../utils";
import { watchDebounced } from "@vueuse/core";
import { ResultItem } from "../result_item";
import { IpcEvent, IpcAction } from "../ipc";
import { accentFillRest } from "@fluentui/web-components";
import RefreshIcon from "../components/RefreshIcon.vue";
import ResultItemComponent from "../components/ResultItem.vue";

const inputRef = useTemplateRef<HTMLElement>("input-ref");
onMounted(() =>
  window.KAL.ipc.on(IpcEvent.FocusInput, () => {
    const shadowRoot = inputRef.value?.shadowRoot;
    const input = shadowRoot?.querySelector<HTMLInputElement>("#control");
    input?.focus();
    input?.select();
  }),
);

const refreshingIndex = ref(false);

const gettingConfirmation = ref(false);
const gettingConfirmationIndex = ref(0);
function resetConfirm() {
  gettingConfirmation.value = false;
  gettingConfirmationIndex.value = 0;
}

const currentSelection = ref(0);
function updateSelection(e: KeyboardEvent) {
  const current = currentSelection.value;
  if (e.key === "ArrowUp") {
    currentSelection.value = current === 0 ? results.value.length - 1 : current - 1;
  } else {
    currentSelection.value = current === results.value.length - 1 ? 0 : current + 1;
  }
}

const itemsContainerRef = useTemplateRef<HTMLElement>("items-container-ref");
function scrollSelected() {
  // avoid scrolling if container is not scrollable atm
  if (!isVScrollable(itemsContainerRef.value)) return;

  const current = currentSelection.value;
  const block = current === 0 ? "end" : current === results.value.length - 1 ? "start" : "nearest";
  console.log(resultItemRefs.value?.[current]);
  resultItemRefs.value?.[current]?.$el.scrollIntoView({
    behavior: "smooth",
    block,
  });
}

const results = ref<ResultItem[]>([]);
const resultItemRefs = useTemplateRef("result-item-refs");

const currentQuery = ref("");
watchDebounced(currentQuery, (query) => runQuery(query), {
  debounce: 200,
  maxWait: 1000,
});

async function runQuery(query: string) {
  if (query) {
    const response: ResultItem[] = await window.KAL.ipc.invoke(IpcAction.Query, query);
    currentSelection.value = 0;
    results.value = response;
  } else {
    results.value = [];
    await window.KAL.ipc.invoke(IpcAction.ClearResults);
  }
}

function resetQuery() {
  currentQuery.value = "";
  currentSelection.value = 0;
}

async function executeItem(e: { shiftKey: boolean }, index: number) {
  let item = results.value[index];
  const confirm = item.needs_confirmation;

  if (
    (!confirm && gettingConfirmation.value) ||
    (confirm && gettingConfirmation.value && gettingConfirmationIndex.value !== index)
  ) {
    resetConfirm();
  }

  if (confirm && !gettingConfirmation.value) {
    gettingConfirmation.value = true;
    gettingConfirmationIndex.value = index;
  } else {
    resetConfirm();
    await window.KAL.ipc.invoke(IpcAction.Execute, e.shiftKey, item.id);
  }
}

async function showItemInDir(index: number) {
  let item = results.value[index];
  await window.KAL.ipc.invoke(IpcAction.ShowItemInDir, item.id);
}

function onChange(e: InputEvent) {
  if (e.target && "value" in e.target && e.target.value === "") {
    resetConfirm();
    resetQuery();
  }
}

async function onkeydown(e: KeyboardEvent) {
  if (gettingConfirmation.value && e.key !== "Enter") {
    resetConfirm();
  }

  if (e.key === "Escape") {
    e.preventDefault();
    await window.KAL.ipc.invoke(IpcAction.HideMainWindow);
  }

  if (["ArrowDown", "Tab", "ArrowUp"].includes(e.key)) {
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
    refreshingIndex.value = true;
    await window.KAL.ipc.invoke(IpcAction.RefreshIndex);
    setTimeout(() => (refreshingIndex.value = false), 500); // artifical delay for nicer animation
    runQuery(currentQuery.value);
  }
}

const config = ref<KalConfig>(window.KAL.config);
onMounted(() =>
  window.KAL.ipc.on<KalConfig>(IpcEvent.UpdateConfig, (newConfig) => {
    config.value = newConfig;
    window.KAL.config = newConfig;
  }),
);

const accentColor = accentFillRest.getValueFor(document.documentElement).toColorString();
const systemAccentColor = ref<string>(window.KAL.systemAccentColor ?? accentColor);
onMounted(() =>
  window.KAL.ipc.on<string>(IpcEvent.UpdateSystemAccentColor, (newColor) => {
    systemAccentColor.value = newColor;
    window.KAL.systemAccentColor = newColor;
  }),
);

const isTransparent = computed(() => config.value.appearance.transparent);
const bgPrimaryColor = computed(() =>
  isTransparent.value ? "bg-transparent" : "bg-[rgba(21,_20,_20,_0.75)]",
);
const inputHeight = computed(() => `${config.value.appearance.input_height}px`);
const itemHeight = computed(() => `${config.value.appearance.item_height}px`);
const itemsContainerHeight = computed(() => `calc(100% - ${inputHeight})`);
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
      @keydown="onkeydown"
      @change="onChange"
    >
      <div slot="clear-button" class="flex justify-center items-center">
        <Transition name="fade">
          <RefreshIcon v-if="refreshingIndex" class="animate-spin" />
          <span v-else></span>
        </Transition>
      </div>
    </fluent-search>

    <fluent-divider />

    <ul
      ref="items-container-ref"
      :style="{ height: itemsContainerHeight }"
      class="overflow-x-hidden overflow-y-auto px-4 pt-4 children:mb-1"
      tabindex="-1"
    >
      <ResultItemComponent
        :itemHeight
        v-for="(item, index) in results"
        ref="result-item-refs"
        :item
        :selected="index === currentSelection"
        :showConfirm="gettingConfirmation && gettingConfirmationIndex == index"
        @click="(e: MouseEvent) => executeItem(e, index)"
      />
    </ul>
  </main>
</template>

<style scoped>
* {
  --accent-fill-rest: v-bind(systemAccentColor);
}
</style>
