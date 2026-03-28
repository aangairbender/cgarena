import {
  BotOverviewResponse,
  BotId,
  LeaderboardOverviewResponse,
  CreateBotRequest,
  RenameBotRequest,
  CreateLeaderboardRequest,
  LeaderboardId,
  PatchLeaderboardRequest,
} from "@models";
import * as api from "@api";
import { create } from "zustand";

type Status = "connected" | "connecting";

type AppState = {
  // state
  loading: boolean;
  fetchingStatus: boolean;
  status: Status;
  matchmakingEnabled: boolean;

  bots: BotOverviewResponse[];
  selectedBotId?: BotId;

  leaderboards: LeaderboardOverviewResponse[];

  // actions
  fetchStatus: () => Promise<void>;
  refreshLeaderboard: () => void;

  selectBot: (id: BotId) => void;

  submitNewBot: (req: CreateBotRequest) => Promise<void>;
  renameBot: (id: BotId, req: RenameBotRequest) => Promise<void>;
  deleteBot: (id: BotId) => Promise<void>;

  createLeaderboard: (req: CreateLeaderboardRequest) => Promise<void>;
  patchLeaderboard: (
    id: LeaderboardId,
    req: PatchLeaderboardRequest,
  ) => Promise<void>;
  deleteLeaderboard: (id: LeaderboardId) => Promise<void>;

  enableMatchmaking: (enabled: boolean) => Promise<void>;
};

export const useAppStore = create<AppState>((set, get) => ({
  // ---------------- state ----------------
  loading: false,
  fetchingStatus: false,
  status: "connected",
  matchmakingEnabled: true,

  bots: [],
  selectedBotId: undefined,

  leaderboards: [],

  // ---------------- core ----------------
  fetchStatus: async () => {
    try {
      set({ fetchingStatus: true });

      const res = await api.fetchStatus();

      set({
        bots: res.bots,
        leaderboards: res.leaderboards,
        matchmakingEnabled: res.matchmaking_enabled,
        status: "connected",
      });

      // auto-select bot
      const { selectedBotId } = get();
      if (!selectedBotId && res.bots.length > 0) {
        set({
          selectedBotId: Math.max(...res.bots.map((b) => b.id)),
        });
      }
    } catch {
      set({ status: "connecting" });
    } finally {
      set({ fetchingStatus: false });
    }
  },

  refreshLeaderboard: () => {
    if (!get().fetchingStatus) {
      get().fetchStatus();
    }
  },

  // ---------------- bots ----------------
  selectBot: (id) => set({ selectedBotId: id }),

  submitNewBot: async (req) => {
    set({ loading: true });

    const bot = await api.submitNewBot(req);

    set((state) => ({
      bots: [bot, ...state.bots],
      selectedBotId: bot.id,
      loading: false,
    }));

    // fire and forget
    get().fetchStatus();
  },

  renameBot: async (id, req) => {
    set({ loading: true });

    await api.renameBot(id, req);

    set((state) => ({
      bots: state.bots.map((b) => (b.id === id ? { ...b, name: req.name } : b)),
      loading: false,
    }));
  },

  deleteBot: async (id) => {
    set({ loading: true });

    await api.deleteBot(id);

    set((state) => ({
      bots: state.bots.filter((b) => b.id !== id),
      selectedBotId:
        state.selectedBotId === id ? undefined : state.selectedBotId,
      leaderboards: state.leaderboards.map((lb) => ({
        ...lb,
        status: "computing",
      })),
      loading: false,
    }));

    get().fetchStatus();
  },

  // ---------------- leaderboards ----------------
  createLeaderboard: async (req) => {
    set({ loading: true });

    const leaderboard = await api.createLeaderboard(req);

    set((state) => ({
      leaderboards: [...state.leaderboards, leaderboard],
      loading: false,
    }));
  },

  patchLeaderboard: async (id, req) => {
    set({ loading: true });

    await api.patchLeaderboard(id, req);

    set((state) => ({
      leaderboards: state.leaderboards.map((lb) =>
        lb.id === id
          ? {
              ...lb,
              name: req.name,
              filter: req.filter,
              status: lb.filter !== req.filter ? "computing" : lb.status,
            }
          : lb,
      ),
      loading: false,
    }));
  },

  deleteLeaderboard: async (id) => {
    set({ loading: true });

    set((state) => ({
      leaderboards: state.leaderboards.filter((lb) => lb.id !== id),
    }));

    await api.deleteLeaderboard(id);

    set({ loading: false });
  },

  // ---------------- misc ----------------
  enableMatchmaking: async (enabled) => {
    set({ matchmakingEnabled: enabled });
    await api.enableMatchmaking(enabled);
  },
}));
