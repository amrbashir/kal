import { Component, createSignal, For, onMount } from "solid-js";
import { IconType, IPCEvent, SearchResultItem } from "../../common_types";

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
                <div
                  class="search-results_item_left"
                  innerHTML={
                    result.icon.type === IconType.Svg
                      ? result.icon.data
                      : result.icon.type === IconType.Path
                      ? `<img src="${convertFileSrc(
                          "kalasset",
                          result.icon.data
                        )}" >`
                      : //  TODO: use a default icon
                        ""
                  }
                ></div>
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
              --primary: rgba(31, 31, 31, 0.8);
              --accent: rgba(72, 141, 210, 0.5);
              --text-primary: rgba(255, 255, 255);
              --text-secondary: rgb(107, 107, 107);
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
              border-radius: 10px 10px
                ${results().length === 0 ? "10px 10px" : "0 0"};
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
          `
        }
      </style>
    </main>
  );
};

export default MainWindow;

function convertFileSrc(protocol: string, filePath: string): string {
  const path = encodeURIComponent(filePath);
  return navigator.userAgent.includes("Windows")
    ? `https://${protocol}.localhost/${path}`
    : `${protocol}://${path}`;
}
