import {
  BotMinimalResponse,
  CreateBotRequest,
  FetchLeaderboardResponse,
  RenameBotRequest,
} from "@models";
import { useCallback, useEffect, useState } from "react";
import * as api from "@api";

export const useAppLogic = () => {
  const [loading, setLoading] = useState(false);
  const [selectedBotId, setSelectedBotId] = useState<string | undefined>();
  const [bots, setBots] = useState<BotMinimalResponse[]>([]);
  const [leaderboardData, setLeaderboardData] = useState<
    FetchLeaderboardResponse | undefined
  >();
  const [autoRefresh, setAutoRefresh] = useState(true);

  const fetchInitialBots = useCallback(async () => {
    setLoading(true);
    try {
      const res = await api.fetchBots();
      setBots(res);
    } finally {
      setLoading(false);
    }
  }, [setLoading, setBots]);

  const fetchLeaderboard = useCallback(
    async (botId: string) => {
      setLoading(true);
      try {
        const res = await api.fetchLeaderboard(botId);
        setLeaderboardData(res);
      } finally {
        setLoading(false);
      }
    },
    [setLoading, setLeaderboardData]
  );

  const refreshLeaderboard = useCallback(() => {
    if (selectedBotId && !loading) {
      fetchLeaderboard(selectedBotId);
    }
  }, [selectedBotId, loading, fetchLeaderboard]);

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
    fetchInitialBots();
  }, [fetchInitialBots]);

  // handle selection change
  useEffect(() => {
    setLeaderboardData(undefined);
    if (selectedBotId) {
      fetchLeaderboard(selectedBotId);
    }
  }, [selectedBotId, fetchLeaderboard]);

  const selectBot = useCallback(
    (botId: string) => {
      setSelectedBotId(botId);
    },
    [setSelectedBotId]
  );

  // exported functions

  const submitNewBot = useCallback(
    async (req: CreateBotRequest) => {
      const bot = await api.submitNewBot(req);
      setBots((cur) => [bot, ...cur]);
      setSelectedBotId(bot.id);
    },
    [setBots, setSelectedBotId]
  );

  const renameBot = useCallback(
    async (id: string, req: RenameBotRequest) => {
      const newBot = await api.renameBot(id, req);
      setBots((bots) => {
        const existingBot = bots.find((b) => b.id == newBot.id);
        if (existingBot) {
          existingBot.name = newBot.name;
          return bots;
        } else {
          throw new Error("Bot does not exist anymore");
        }
      });
    },
    [setBots]
  );

  const deleteBot = useCallback(
    async (botId: string) => {
      setBots((bots) => bots.filter((b) => b.id != botId));
      if (selectedBotId == botId) setSelectedBotId(undefined);

      await api.deleteBot(botId);
    },
    [setBots, selectedBotId, setSelectedBotId]
  );

  return {
    selectedBotId,
    bots,
    leaderboardData,
    loading,
    autoRefresh,
    setAutoRefresh,
    selectBot,
    submitNewBot,
    deleteBot,
    renameBot,
  };
};
