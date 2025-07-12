import {
  BotOverviewResponse,
  CreateBotRequest,
  LeaderboardOverviewResponse,
  RenameBotRequest,
} from "@models";
import { useCallback, useEffect, useState } from "react";
import * as api from "@api";

export const useAppLogic = () => {
  const [loading, setLoading] = useState(false);
  const [selectedBotId, setSelectedBotId] = useState<string | undefined>();
  const [bots, setBots] = useState<BotOverviewResponse[]>([]);
  const [leaderboards, setLeaderboards] = useState<LeaderboardOverviewResponse[]>([]);
  const [autoRefresh, setAutoRefresh] = useState(true);

  const fetchStatus = useCallback(
    async () => {
      setLoading(true);
      try {
        const res = await api.fetchStatus();
        setBots(res.bots)
        setLeaderboards(res.leaderboards);
      } finally {
        setLoading(false);
      }
    },
    [setLoading, setBots, setLeaderboards]
  );

  const refreshLeaderboard = useCallback(() => {
    if (!loading) {
      fetchStatus();
    }
  }, [loading, fetchStatus]);

  // effects

  // auto refresh
  useEffect(() => {
    if (!autoRefresh) return;

    const interval = setInterval(refreshLeaderboard, 3000); // in ms
    return () => clearInterval(interval);
  }, [refreshLeaderboard, autoRefresh]);

  // select bot from the list
  useEffect(() => {
    if (selectedBotId) return;
    if (bots.length == 0) return;
    setSelectedBotId(bots[0].id);
  }, [selectedBotId, bots]);

  // load bots initially
  useEffect(() => {
    fetchStatus();
  }, [fetchStatus]);

  const selectBot = useCallback(
    (botId: string) => {
      setSelectedBotId(botId);
    },
    [setSelectedBotId]
  );

  // exported functions

  const submitNewBot = useCallback(
    async (req: CreateBotRequest) => {
      setLoading(true);
      const bot = await api.submitNewBot(req);
      setBots((cur) => [bot, ...cur]);
      setSelectedBotId(bot.id);
      setLoading(false);
    },
    [setBots, setSelectedBotId, setLoading]
  );

  const renameBot = useCallback(
    async (id: string, req: RenameBotRequest) => {
      setLoading(true);
      await api.renameBot(id, req);
      setBots((bots) => {
        const existingBot = bots.find((b) => b.id == id);
        if (existingBot) {
          existingBot.name = req.name;
          return bots;
        } else {
          throw new Error("Bot does not exist anymore");
        }
      });
      setLoading(false);
    },
    [setBots]
  );

  const deleteBot = useCallback(
    async (botId: string) => {
      setLoading(true);
      setBots((bots) => bots.filter((b) => b.id != botId));
      if (selectedBotId == botId) setSelectedBotId(undefined);

      await api.deleteBot(botId);
      setLoading(false);
    },
    [setBots, selectedBotId, setSelectedBotId]
  );

  return {
    selectedBotId,
    bots,
    leaderboards,
    loading,
    autoRefresh,
    setAutoRefresh,
    selectBot,
    submitNewBot,
    deleteBot,
    renameBot,
  };
};
