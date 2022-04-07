import { Component } from "solid-js";
import { IPCEvent } from "../../common_types";

const SearchInput: Component = () => {
  function search(query: string) {
    if (query) {
      window.KAL.ipc.send(IPCEvent.Search, query);
    } else {
      window.KAL.ipc.send(IPCEvent.ClearResults, query);
    }
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "ArrowDown") {
      window.KAL.ipc.send(IPCEvent.SelectNextResult);
    }

    if (e.key === "ArrowUp") {
      window.KAL.ipc.send(IPCEvent.SelectPreviousResult);
    }

    if (e.key === "Enter") {
      window.KAL.ipc.send(IPCEvent.Execute);
    }
  }

  return (
    <>
      <div id="search_input">
        <div id="search_input_icon">
          <svg
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
          placeholder="Search..."
          onkeydown={(e) => onkeydown(e)}
          onInput={(e) => search(e.currentTarget.value)}
        />
      </div>

      <style jsx dynamic>
        {`
          #search_input {
            appearance: none;
            background-color: var(--bg-color);
            width: 100vw;
            height: 100vh;
            border-radius: 10px;
            overflow: hidden;
            display: flex;
          }

          #search_input input {
            flex-grow: 1;
            background: transparent;
            height: 100%;
            outline: none;
            border: none;
            font-size: larger;
            color: white;
            padding: 1rem;
          }

          #search_input #search_input_icon {
            display: grid;
            place-items: center;
            height: 100%;
            width: 50px;
            color: white;
          }

          #search_input ::placeholder {
            color: #6b6b6b;
          }
        `}
      </style>
    </>
  );
};

export default SearchInput;
