import { Component, createSignal } from "solid-js";
import "./style.css";

const SearchInput: Component = () => {
  const [searchQuery, setSearchQuery] = createSignal("");

  function search(query: string) {
    window.KAL.ipc.send("search", query);
  }

  return (
    <div>
      {/* TODO: Add search svg icon */}
      <input
        id="search_input"
        placeholder="Search..."
        onInput={(e) => search(e.currentTarget.value)}
      />
      {/* TODO: Add an empty dev for indicators */}
    </div>
  );
};

export default SearchInput;
