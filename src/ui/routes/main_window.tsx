import { Component, createSignal, For, onMount } from "solid-js";
import { IPCEvent, SearchResultItem } from "../../common_types";

const MainWindow: Component = () => {
  const [currentSelection, setCurrentSelection] = createSignal(0);
  const [results, setResults] = createSignal<SearchResultItem[]>([]);

  function search(query: string) {
    if (query) {
      window.KAL.ipc.send(IPCEvent.Search, query);
    } else {
      setResults([]);
      window.KAL.ipc.send(IPCEvent.ClearResults);
    }
  }

  function onkeydown(e: KeyboardEvent) {
    if (["ArrowDown", "ArrowUp"].includes(e.key)) {
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
      window.KAL.ipc.send(IPCEvent.Execute, currentSelection());
    }
  }

  onMount(() => {
    window.KAL.ipc.on(IPCEvent.FocusInput, () => {
      document.getElementById("search-input").focus();
    });

    window.KAL.ipc.on(IPCEvent.Results, (payload: SearchResultItem[]) =>
      setResults(payload)
    );
  });

  return (
    <main>
      <div id="search-input_container">
        <div id="search-input_icon-container">
          <svg
            id="search-input_icon"
            xmlns="http://www.w3.org/2000/svg"
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
        </div>
        <input
          id="search-input"
          placeholder="Search..."
          onkeydown={(e) => onkeydown(e)}
          onInput={(e) => search(e.currentTarget.value)}
        />
      </div>

      <div id="search-results_container">
        <ul>
          <For each={results()}>
            {(result, index) => (
              <li
                id={`search-results_item_#${index()}`}
                class="search-results_item"
                classList={{ selected: index() == currentSelection() }}
              >
                <div class="search-results_item_left">
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
                <div class="search-results_item_right">
                  <span class="text-primary">{result.primary_text}</span>
                  <span class="text-secondary">{result.secondary_text}</span>
                </div>
              </li>
            )}
          </For>
        </ul>
      </div>

      <style jsx dynamic>
        {
          /* css */ `
            :root {
              --bg: #1f1f1f;
              --accent: red;
              --text-primary: #ffffff;
              --text-secondary: #6b6b6b;
            }

            main {
              overflow: hidden;
              width: 100vw;
              height: 100vh;
            }

            #search-input_container {
              appearance: none;
              background-color: var(--bg);
              width: 100%;
              height: 60px;
              border-radius: 10px 10px 0 0;
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

            #search-input_icon-container {
              display: grid;
              place-items: center;
              height: 100%;
              width: 50px;
              color: var(--text-primary);
            }

            #search-results_container {
              overflow-x: hidden;
              background-color: var(--bg);
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
          `
        }
      </style>
    </main>
  );
};

export default MainWindow;
