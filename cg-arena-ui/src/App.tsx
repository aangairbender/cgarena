import "./App.css";

import AppNavbar from "@components/AppNavbar";
import DialogsProvider from "@components/DialogsProvider";
import { Outlet } from "@tanstack/react-router";
import { Container } from "react-bootstrap";

function App() {
  return (
    <DialogsProvider>
      <AppNavbar />
      <Container className="py-4">
        <Outlet />
      </Container>
    </DialogsProvider>
  );
}

export default App;
