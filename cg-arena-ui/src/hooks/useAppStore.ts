import {
  BotOverviewResponse,
  BotId,
  LeaderboardOverviewResponse,
  CreateBotRequest,
  RenameBotRequest,
  CreateLeaderboardRequest,
  LeaderboardId,
  PatchLeaderboardRequest,
} from "@/models";
import * as api from "@/api";
import { create } from "zustand";

type Status = "connected" | "connecting";

type AppState = {
  // state
  loading: boolean;
  fetchingStatus: boolean;
  status: Status;
  evaluationSchedulingEnabled: boolean;
  initialFetchCompleted: boolean;

  bots: BotOverviewResponse[];

  leaderboards: LeaderboardOverviewResponse[];

  // actions
  fetchStatus: () => Promise<void>;
  refreshLeaderboard: () => void;

  submitNewBot: (req: CreateBotRequest) => Promise<BotOverviewResponse>;
  renameBot: (id: BotId, req: RenameBotRequest) => Promise<void>;
  deleteBot: (id: BotId) => Promise<void>;
  promoteCandidate: (id: BotId) => Promise<void>;
  archiveBenchmark: (id: BotId) => Promise<void>;

  createLeaderboard: (req: CreateLeaderboardRequest) => Promise<void>;
  patchLeaderboard: (
    id: LeaderboardId,
    req: PatchLeaderboardRequest,
  ) => Promise<void>;
  deleteLeaderboard: (id: LeaderboardId) => Promise<void>;

  setEvaluationScheduling: (enabled: boolean) => Promise<void>;
};

export const useAppStore = create<AppState>((set, get) => ({
  // ---------------- state ----------------
  loading: false,
  fetchingStatus: false,
  status: "connected",
  evaluationSchedulingEnabled: true,
  initialFetchCompleted: false,

  bots: [],

  leaderboards: [],

  // ---------------- core ----------------
  fetchStatus: async () => {
    try {
      set({ fetchingStatus: true });

      const res = await api.fetchStatus();

      set({
        bots: res.bots,
        leaderboards: res.leaderboards,
        evaluationSchedulingEnabled: res.evaluation_scheduling_enabled,
        status: "connected",
        initialFetchCompleted: true,
      });
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
  submitNewBot: async (req) => {
    set({ loading: true });

    const bot = await api.submitNewBot(req);

    set((state) => ({
      bots: [bot, ...state.bots],
      loading: false,
    }));

    // fire and forget
    void get().fetchStatus();
    return bot;
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
      bots: state.bots.filter((bot) => bot.id !== id),
      leaderboards: state.leaderboards.map((leaderboard) => ({
        ...leaderboard,
        status: "computing",
      })),
      loading: false,
    }));
    void get().fetchStatus();
  },

  promoteCandidate: async (id) => {
    set({ loading: true });
    await api.promoteCandidate(id);
    set({ loading: false });
    await get().fetchStatus();
  },

  archiveBenchmark: async (id) => {
    set({ loading: true });
    await api.archiveBenchmark(id);
    set({ loading: false });
    await get().fetchStatus();
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
  setEvaluationScheduling: async (enabled) => {
    set({ evaluationSchedulingEnabled: enabled });
    await api.setEvaluationScheduling(enabled);
  },
}));
