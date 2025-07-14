import {
  BotId,
  BotOverviewResponse,
  CreateBotRequest,
  CreateLeaderboardRequest,
  LeaderboardId,
  LeaderboardOverviewResponse,
  RenameBotRequest,
  PatchLeaderboardRequest,
} from "@models";
import { useCallback, useEffect, useState } from "react";
import * as api from "@api";

export const useAppLogic = () => {
  const [loading, setLoading] = useState(false);
  const [selectedBotId, setSelectedBotId] = useState<BotId | undefined>();
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
    (botId: BotId) => {
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

      // not awaiting intentionally to not block dialog
      fetchStatus();
    },
    [setBots, setSelectedBotId, setLoading, fetchStatus]
  );

  const renameBot = useCallback(
    async (id: BotId, req: RenameBotRequest) => {
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
    async (botId: BotId) => {
      setLoading(true);
      await api.deleteBot(botId);
      setLoading(false);

      setBots((bots) => bots.filter((b) => b.id != botId));
      if (selectedBotId == botId) setSelectedBotId(undefined);
      setLeaderboards(leaderboards => leaderboards.map(lb => ({ ...lb, status: "computing" })));

      // not awaiting intentionally to not block dialog
      fetchStatus();
    },
    [setBots, selectedBotId, setSelectedBotId, setLoading, fetchStatus]
  );

  
  const createLeaderboard = useCallback(
    async (req: CreateLeaderboardRequest) => {
      setLoading(true);
      const leaderboard = await api.createLeaderboard(req);
      setLeaderboards((cur) => [...cur, leaderboard]);
      setLoading(false);
    },
    [setLeaderboards, setLoading]
  );

  const patchLeaderboard = useCallback(
    async (id: LeaderboardId, req: PatchLeaderboardRequest) => {
      setLoading(true);
      await api.patchLeaderboard(id, req);
      setLeaderboards((leaderboards) => {
        const existing = leaderboards.find((lb) => lb.id == id);
        if (existing) {
          existing.name = req.name;
          if (existing.filter != req.filter) {
            existing.status = "computing";
          }
          existing.filter = req.filter;
          return leaderboards;
        } else {
          throw new Error("Bot does not exist anymore");
        }
      });
      setLoading(false);
    },
    [setLeaderboards]
  );

  const deleteLeaderboard = useCallback(
    async (leaderboardId: LeaderboardId) => {
      setLoading(true);
      setLeaderboards((leaderboards) => leaderboards.filter((lb) => lb.id != leaderboardId));
      await api.deleteLeaderboard(leaderboardId);
      setLoading(false);
    },
    [setLeaderboards, setLoading]
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
    createLeaderboard,
    patchLeaderboard,
    deleteLeaderboard,
  };
};
