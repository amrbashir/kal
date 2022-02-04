import { Route, Routes } from "solid-app-router";
import { Component, lazy } from "solid-js";
import "./style.css";

const SearchInput = lazy(() => import("./components/SearchInput"));
const SearchResults = lazy(() => import("./components/SearchResults"));

const App: Component = () => {
  return (
    <>
      <Routes>
        <Route path="SearchInput" element={<SearchInput />} />
        <Route path="SearchResults" element={<SearchResults />} />
      </Routes>
    </>
  );
};

export default App;
