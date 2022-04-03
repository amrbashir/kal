import { Component, createSignal } from "solid-js";

const SearchInput: Component = () => {
  const [searchQuery, setSearchQuery] = createSignal("");

  function search(query: string) {
    window.KAL.ipc.send("search", query);
  }

  return (
    <>
      <div>
        {/* TODO: Add search svg icon */}
        <input
          id="search_input"
          placeholder="Search..."
          onInput={(e) => search(e.currentTarget.value)}
        />
        {/* TODO: Add an empty dev for indicators */}
      </div>

      <style jsx dynamic>
        {`
          #search_input {
            appearance: none;
            background-color: var(--bg-color);
            width: 100vw;
            height: 100vh;
            outline: none;
            border: none;
            border-radius: 10px;
            padding: 1rem;
            font-size: larger;
            color: white;
            overflow: hidden;
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
