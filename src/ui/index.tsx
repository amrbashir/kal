import { render } from "solid-js/web";
import { Router, Route, Routes } from "solid-app-router";
import { lazy } from "solid-js";
import "solid-styled-jsx";
import "./style.css";

const MainWindow = lazy(() => import("./windows/main"));
// const SettingsWindow = lazy(() => import("./routes/settings_window"));

render(
  () => (
    <Router>
      <Routes>
        <Route path="/main-window" element={<MainWindow />} />
        {/* <Route path="/settings-window" element={<SettingsWindow />} /> */}
      </Routes>
    </Router>
  ),
  document.getElementById("root") as HTMLElement
);
