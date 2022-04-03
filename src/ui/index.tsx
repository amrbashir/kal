import { render } from "solid-js/web";
import { Router, Route, Routes } from "solid-app-router";
import { lazy } from "solid-js";
import "solid-styled-jsx";
import "./style.css";

const SearchInput = lazy(() => import("./routes/SearchInput"));
const SearchResults = lazy(() => import("./routes/SearchResults"));

render(
  () => (
    <Router>
      <Routes>
        <Route path="/SearchInput" element={<SearchInput />} />
        <Route path="/SearchResults" element={<SearchResults />} />
      </Routes>
    </Router>
  ),
  document.getElementById("root") as HTMLElement
);
