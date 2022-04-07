import { Component, createSignal, For, onMount } from "solid-js";
import { IPCEvent, SearchResultItem } from "../../common_types";

const SearchResults: Component = () => {
  const [currentSelection, setCurrentSelection] = createSignal(0);
  const [results, setResults] = createSignal<SearchResultItem[]>([]);

  onMount(() => {
    window.KAL.ipc.on(IPCEvent.Results, (payload: SearchResultItem[]) =>
      setResults(payload)
    );

    window.KAL.ipc.on(IPCEvent.ClearResults, () => setResults([]));

    window.KAL.ipc.on(IPCEvent.SelectNextResult, (i: number) => {
      setCurrentSelection(i);
      document
        .getElementById(`search_results_item_#${currentSelection()}`)
        .scrollIntoView({ behavior: "smooth", block: "nearest" });
    });

    window.KAL.ipc.on(IPCEvent.SelectPreviousResult, (i: number) => {
      setCurrentSelection(i);
      document
        .getElementById(`search_results_item_#${currentSelection()}`)
        .scrollIntoView({ behavior: "smooth", block: "nearest" });
    });
  });

  return (
    <>
      <div id="search_results">
        <div>
          <For each={results()}>
            {(result, index) => (
              <li
                id={`search_results_item_#${index()}`}
                class="search_results_item"
                classList={{ selected: index() == currentSelection() }}
              >
                <div class="search_results_item_left">
                  {/* TODO: replace with search result icon */}
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    aria-hidden="true"
                    class="iconify iconify--bi"
                    width="32"
                    height="32"
                    preserveAspectRatio="xMidYMid meet"
                    viewBox="0 0 16 16"
                  >
                    <path
                      fill="currentColor"
                      fill-rule="evenodd"
                      d="M14 4.5V14a2 2 0 0 1-2 2h-1v-1h1a1 1 0 0 0 1-1V4.5h-2A1.5 1.5 0 0 1 9.5 3V1H4a1 1 0 0 0-1 1v9H2V2a2 2 0 0 1 2-2h5.5L14 4.5ZM2.575 15.202H.785v-1.073H2.47v-.606H.785v-1.025h1.79v-.648H0v3.999h2.575v-.647ZM6.31 11.85h-.893l-.823 1.439h-.036l-.832-1.439h-.931l1.227 1.983l-1.239 2.016h.861l.853-1.415h.035l.85 1.415h.908l-1.254-1.992L6.31 11.85Zm1.025 3.352h1.79v.647H6.548V11.85h2.576v.648h-1.79v1.025h1.684v.606H7.334v1.073Z"
                    ></path>
                  </svg>
                </div>
                <div class="search_results_item_right">
                  <span class="primary">{result.primary_text}</span>
                  <span class="secondary">{result.secondary_text}</span>
                </div>
              </li>
            )}
          </For>
        </div>
      </div>
      <style jsx dynamic>
        {`
          #search_results {
            overflow-x: hidden;
            background-color: var(--bg-color);
            width: 100vw;
            height: 100vh;
            border-radius: 10px;
            color: #6b6b6b;
          }

          li.search_results_item {
            overflow: hidden;
            padding: 0 10px;
            list-style: none;
            display: flex;
            width: 100%;
            height: 60px;
          }
          li.search_results_item:hover,
          li.search_results_item.selected {
            background-color: var(--search-result-item-bg-hover);
          }

          div.search_results_item_left {
            flex-shrink: 0;
            width: 60px;
            height: 100%;
            display: grid;
            place-items: center;
          }

          div.search_results_item_left > * {
            width: 50%;
            height: 50%;
          }

          div.search_results_item_right {
            overflow: hidden;
            height: 100%;
            display: flex;
            flex-direction: column;
            justify-content: center;
          }

          div.search_results_item_right span {
            overflow: hidden;
            white-space: nowrap;
            text-overflow: ellipsis;
          }
          div.search_results_item_right span.primary {
            color: var(--search-result-item-primary-text);
          }

          div.search_results_item_right span.secondary {
            color: var(--search-result-item-secondary-text);
          }
        `}
      </style>
    </>
  );
};

export default SearchResults;
