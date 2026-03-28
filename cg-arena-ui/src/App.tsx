import "./App.css";

import AppNavbar from "@components/AppNavbar";
import { Outlet } from "@tanstack/react-router";

function App() {
  return (
    <>
      <AppNavbar />
      <Outlet />
    </>
  );
}

export default App;
