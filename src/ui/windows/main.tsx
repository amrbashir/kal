import { Component, createSignal, For, JSX, onMount } from "solid-js";
import { Icon, IconType, IPCEvent, SearchResultItem } from "../../common";
import "./main.css";

const MainWindow: Component = () => {
  // state
  const [currentSelection, setCurrentSelection] = createSignal(0);
  const [results, setResults] = createSignal<SearchResultItem[]>([]);
  const [refreshingIndex, setRefreshingIndex] = createSignal(false);

  // handlers
  function search(query: string) {
    if (query) {
      window.KAL.ipc.send(IPCEvent.Search, query);
    } else {
      setResults([]);
      window.KAL.ipc.send(IPCEvent.ClearResults);
    }
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      window.KAL.ipc.send(IPCEvent.HideMainWindow);
    }

    if (["ArrowDown", "ArrowUp"].includes(e.key)) {
      e.preventDefault();
      if (e.key === "ArrowDown") {
        setCurrentSelection(
          currentSelection() === results().length - 1
            ? 0
            : currentSelection() + 1
        );
      } else {
        setCurrentSelection(
          currentSelection() === 0
            ? results().length - 1
            : currentSelection() - 1
        );
      }

      document
        .getElementById(`search-results_item_#${currentSelection()}`)
        .scrollIntoView({ behavior: "smooth", block: "nearest" });
    }

    if (e.key === "Enter") {
      e.preventDefault();
      window.KAL.ipc.send(IPCEvent.Execute, currentSelection(), e.shiftKey);
    }

    if (e.key === "o" && e.ctrlKey) {
      e.preventDefault();
      window.KAL.ipc.send(IPCEvent.OpenLocation, currentSelection());
    }

    if (e.key === "r" && e.ctrlKey) {
      e.preventDefault();
      window.KAL.ipc.send(IPCEvent.Refresh);
      setRefreshingIndex(true);
    }
  }

  onMount(() => {
    window.KAL.ipc.on(IPCEvent.FocusInput, () => {
      let input = document.getElementById("search-input");
      input.focus();
      (input as HTMLInputElement).select();
    });

    window.KAL.ipc.on(IPCEvent.Results, (payload: SearchResultItem[]) => {
      setCurrentSelection(0);
      setResults(payload);
    });

    window.KAL.ipc.on(IPCEvent.RefreshingIndexFinished, () => {
      setRefreshingIndex(false);
    });
  });

  return (
    <main>
      <div
        id="search-input_container"
        style={{
          "border-radius": results().length === 0 ? "10px 10px" : "0 0",
        }}
      >
        <div id="search-input_icon-container">
          <SearchIcon id="search-input_icon" />
        </div>
        <input
          id="search-input"
          placeholder="Search..."
          onkeydown={(e) => onkeydown(e)}
          onInput={(e) => search(e.currentTarget.value)}
        />
        <div
          id="refreshing_icon-container"
          style={{
            opacity: refreshingIndex() ? 1 : 0,
          }}
        >
          <RefreshingIcon id="refreshing_icon" />
        </div>
      </div>

      <div id="search-results_container">
        <ul>
          <For each={results()}>
            {(item, index) => (
              <li
                id={`search-results_item_#${index()}`}
                class="search-results_item"
                classList={{ selected: index() == currentSelection() }}
              >
                <div
                  class="search-results_item_left"
                  innerHTML={getIconHtml(item.icon)}
                ></div>
                <div class="search-results_item_right">
                  <span class="text-primary">{item.primary_text}</span>
                  <span class="text-secondary">{item.secondary_text}</span>
                </div>
              </li>
            )}
          </For>
        </ul>
      </div>
    </main>
  );
};

export default MainWindow;

// utils
function convertFileSrc(protocol: string, filePath: string): string {
  const path = encodeURIComponent(filePath);
  return navigator.userAgent.includes("Windows")
    ? `https://${protocol}.localhost/${path}`
    : `${protocol}://${path}`;
}

function getIconHtml(icon: Icon): string {
  switch (icon.type) {
    case IconType.Path:
      return `<img src="${convertFileSrc("kalasset", icon.data)}" />`;
    case IconType.Svg:
      return icon.data;
    default:
      return ""; // TODO use a default icon
  }
}

const SearchIcon: Component<JSX.HTMLAttributes<HTMLElement>> = (props) => (
  <svg
    id={props.id}
    style={props.style}
    width="32px"
    height="32px"
    viewBox="0 0 24 24"
  >
    <g
      fill="none"
      stroke="currentColor"
      stroke-linecap="round"
      stroke-linejoin="round"
      stroke-width="2"
    >
      <circle cx="10" cy="10" r="7"></circle>
      <path d="m21 21l-6-6"></path>
    </g>
  </svg>
);

const RefreshingIcon: Component<JSX.HTMLAttributes<HTMLElement>> = (props) => (
  <svg
    id={props.id}
    style={props.style}
    width="32"
    height="32"
    preserveAspectRatio="xMidYMid meet"
    viewBox="0 0 24 24"
  >
    <path
      fill="none"
      stroke="currentColor"
      stroke-linecap="round"
      stroke-linejoin="round"
      stroke-width="2"
      d="M4 4v5h.582m15.356 2A8.001 8.001 0 0 0 4.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 0 1-15.357-2m15.357 2H15"
    ></path>
  </svg>
);
