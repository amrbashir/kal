import { Component, createSignal, For, onMount } from "solid-js";
import "./style.css";

const SearchResults: Component = () => {
  const [results, setResults] = createSignal([]);

  onMount(() => {
    window.KAL.ipc.on("results", (res: string[]) => setResults(res));
  });

  return (
    <div id="search_results">
      <span>Under construction!</span>
      <div>
        <For each={results()}>
          {(result, i) => (
            <li>
              {i} - {result}
            </li>
          )}
        </For>
      </div>
    </div>
  );
};

export default SearchResults;
