import { Component, createSignal, For, onMount } from "solid-js";

const SearchResults: Component = () => {
  const [results, setResults] = createSignal([]);

  onMount(() => {
    window.KAL.ipc.on("results", (res: string[]) => setResults(res));
  });

  return (
    <>
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
      <style jsx dynamic>
        {`
          #search_results {
            background-color: var(--bg-color);
            width: 100vw;
            height: 100vh;
            border-radius: 10px;
            display: grid;
            place-items: center;
            color: #6b6b6b;
            font-size: 3rem;
          }
        `}
      </style>
    </>
  );
};

export default SearchResults;
