import "./index.css";

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "bootstrap/dist/css/bootstrap.min.css";
import App from "./App.tsx";
import ThemeProvider from "@components/ThemeProvider.tsx";
import DialogsProvider from "@components/DialogsProvider.tsx";
import {
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import HomePage from "./pages/HomePage.tsx";
import ConfigPage from "./pages/ConfigPage.tsx";

const rootRoute = createRootRoute({
  component: () => <App />,
});

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  component: () => <HomePage />,
});

const configRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/config",
  component: () => <ConfigPage />,
});

const routeTree = rootRoute.addChildren([indexRoute, configRoute]);

const router = createRouter({
  routeTree,
  defaultPreload: "intent",
});

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ThemeProvider>
      <DialogsProvider>
        <RouterProvider router={router} />
      </DialogsProvider>
    </ThemeProvider>
  </StrictMode>,
);
