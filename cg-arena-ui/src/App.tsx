import "./App.css";

import * as api from "@/api";
import AppNavbar from "@/components/AppNavbar";
import DialogsProvider from "@/components/DialogsProvider";
import { configurationQueryKey } from "@/configuration";
import ConfigPage from "@/pages/ConfigPage";
import { useQuery } from "@tanstack/react-query";
import { Outlet } from "@tanstack/react-router";
import { Alert, Container, Spinner } from "react-bootstrap";

function App() {
  const configuration = useQuery({
    queryKey: configurationQueryKey,
    queryFn: api.fetchConfiguration,
  });
  const runtimeAvailable = configuration.data?.runtime_available === true;

  let content;
  if (configuration.isPending) {
    content = <Spinner animation="border" aria-label="Loading application" />;
  } else if (configuration.error) {
    content = <Alert variant="danger">{configuration.error.message}</Alert>;
  } else if (!runtimeAvailable) {
    content = <ConfigPage />;
  } else {
    content = <Outlet />;
  }

  return (
    <DialogsProvider>
      <AppNavbar runtimeAvailable={runtimeAvailable} />
      <Container className="py-4">{content}</Container>
    </DialogsProvider>
  );
}

export default App;
