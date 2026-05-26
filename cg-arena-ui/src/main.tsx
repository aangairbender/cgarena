import "./index.css";

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "bootstrap/dist/css/bootstrap.min.css";
import App from "./App.tsx";
import ThemeProvider from "@components/ThemeProvider.tsx";
import {
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import HomePage from "./pages/HomePage.tsx";
import ConfigPage from "./pages/ConfigPage.tsx";
import MatchesPage from "./pages/MatchesPage.tsx";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

const rootRoute = createRootRoute({
  component: () => <App />,
});

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  component: () => <HomePage />,
  validateSearch: (search) => ({
    selectedBotId: search.selectedBotId
      ? Number(search.selectedBotId)
      : undefined,
  }),
});

const matchesRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/matches",
  component: () => <MatchesPage />,
  validateSearch: (search) => ({
    filter: typeof search.filter === "string" ? search.filter : "",
    withBots: Array.isArray(search.withBots)
      ? search.withBots.map(Number)
      : typeof search.withBots === "string"
        ? search.withBots.split(",").map(Number)
        : [],
    page: search.page ? Number(search.page) : undefined,
    pageSize: search.pageSize ? Number(search.pageSize) : undefined,
  }),
});

const configRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/config",
  component: () => <ConfigPage />,
});

const routeTree = rootRoute.addChildren([
  indexRoute,
  matchesRoute,
  configRoute,
]);

const router = createRouter({
  routeTree,
  defaultPreload: "intent",
});

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}

const queryClient = new QueryClient();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <ThemeProvider>
        <RouterProvider router={router} />
      </ThemeProvider>
    </QueryClientProvider>
  </StrictMode>,
);
