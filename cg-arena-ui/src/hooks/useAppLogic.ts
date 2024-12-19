import {
  BotMinimalResponse,
  CreateBotRequest,
  FetchLeaderboardResponse,
} from "@models";
import { useEffect, useState } from "react";
import * as api from "@api";

export const useAppLogic = () => {
  const [loading, setLoading] = useState(false);
  const [selectedBotId, setSelectedBotId] = useState<string | undefined>();
  const [bots, setBots] = useState<BotMinimalResponse[]>([]);
  const [leaderboardData, setLeaderboardData] = useState<
    FetchLeaderboardResponse | undefined
  >();

  const fetchInitialBots = async () => {
    setLoading(true);
    try {
      const res = await api.fetchBots()
      setBots(res);
      if (res.length != 0) {
        setSelectedBotId(res[0].id);
      }
    } finally {
      setLoading(false);
    }
  };

  const fetchLeaderboard = async (botId: string) => {
    setLoading(true);
    try {
      const res = await api.fetchLeaderboard(botId);
      setLeaderboardData(res);
    } finally {
      setLoading(false);
    }
  };

  const refreshLeaderboard = () => {
    if (selectedBotId && !loading) {
      fetchLeaderboard(selectedBotId);
    }
  };

  const selectBot = (botId: string) => {
    setSelectedBotId(botId);
  };

  const submitNewBot = async (req: CreateBotRequest) => {
    const bot = await api.submitNewBot(req);
    setBots((cur) => [bot, ...cur]);
    setSelectedBotId(bot.id);
  };

  // load bots initially
  useEffect(() => {
    fetchInitialBots();
  }, []);

  // handle selection change
  useEffect(() => {
    setLeaderboardData(undefined);
    if (selectedBotId) {
      fetchLeaderboard(selectedBotId);
    }
  }, [selectedBotId]);

  return {
    selectedBotId,
    bots,
    leaderboardData,
    loading,
    selectBot,
    submitNewBot,
    refreshLeaderboard,
  };
};