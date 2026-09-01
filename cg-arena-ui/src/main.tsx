import "./index.css";

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "bootstrap/dist/css/bootstrap.min.css";
import App from "./App.tsx";
import ThemeProvider from "@/components/ThemeProvider.tsx";
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
import { z } from "zod";

const rootRoute = createRootRoute({
  component: () => <App />,
});

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  component: () => <HomePage />,
  validateSearch: z.object({
    selectedBotId: z.number().optional(),
  }),
});

const matchesRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/matches",
  component: () => <MatchesPage />,
  validateSearch: z.object({
    filter: z.string().catch("").default(""),
    withBots: z.array(z.int()).catch([]).default([]),
    page: z.int().min(1).catch(1).default(1),
    pageSize: z.int().min(1).max(100).catch(10).default(10),
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
