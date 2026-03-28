import { getRouteApi, useNavigate } from "@tanstack/react-router";
import { useEffect } from "react";
import { useAppStore } from "./useAppStore";

const routeApi = getRouteApi("/");

const useEnsureValidSelectedBot = () => {
  const bots = useAppStore((s) => s.bots);
  const initialFetchCompleted = useAppStore((s) => s.initialFetchCompleted);
  const { selectedBotId } = routeApi.useSearch();
  const navigate = useNavigate();

  useEffect(() => {
    if (!initialFetchCompleted) return;

    // initial selection
    if (selectedBotId === undefined && bots.length > 0) {
      navigate({
        to: "/",
        replace: true,
        search: (prev) => ({
          ...prev,
          selectedBotId: Math.max(...bots.map((b) => b.id)),
        }),
      });
      return;
    }

    // invalid selection
    if (selectedBotId && !bots.some((b) => b.id === selectedBotId)) {
      navigate({
        to: "/",
        replace: true,
        search: (prev) => ({
          ...prev,
          selectedBotId:
            bots.length > 0 ? Math.max(...bots.map((b) => b.id)) : undefined,
        }),
      });
    }
  }, [selectedBotId, bots, initialFetchCompleted, navigate]);
};

export default useEnsureValidSelectedBot;
