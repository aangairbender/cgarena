import {
  BotId,
  BotOverviewResponse,
  CreateBotRequest,
  CreateLeaderboardRequest,
  FetchStatusResponse,
  LeaderboardId,
  LeaderboardOverviewResponse,
  RenameBotRequest,
  PatchLeaderboardRequest,
  ChartRequest,
  ChartOverviewResponse,
  BotSourceCode,
  EnableMatchmakingRequest,
  MatchId,
  WatchReplayResponse,
  ArenaConfiguration,
  ConfigurationState,
} from "@/models";

const host = import.meta.env.DEV ? "http://127.0.0.1:1234" : "";

export const fetchStatus = async (): Promise<FetchStatusResponse> => {
  const response = await fetch(`${host}/api/status`);
  return await parseResponse<FetchStatusResponse>(response);
};

export const fetchConfiguration = async (): Promise<ConfigurationState> => {
  const response = await fetch(`${host}/api/configuration`);
  return await parseResponse<ConfigurationState>(response);
};

export const applyConfiguration = async (
  payload: ArenaConfiguration,
): Promise<ConfigurationState> => {
  const response = await fetch(
    new Request(`${host}/api/configuration`, {
      method: "PUT",
      body: JSON.stringify(payload),
      headers: { "Content-Type": "application/json" },
    }),
  );
  return await parseResponse<ConfigurationState>(response);
};

export const submitNewBot = async (
  payload: CreateBotRequest,
): Promise<BotOverviewResponse> => {
  const req = new Request(`${host}/api/bots`, {
    method: "POST",
    body: JSON.stringify(payload),
    headers: {
      "Content-Type": "application/json",
    },
  });

  const response = await fetch(req);
  return await parseResponse<BotOverviewResponse>(response);
};

export const renameBot = async (id: BotId, payload: RenameBotRequest) => {
  const req = new Request(`${host}/api/bots/${id}`, {
    method: "PATCH",
    body: JSON.stringify(payload),
    headers: {
      "Content-Type": "application/json",
    },
  });

  const response = await fetch(req);
  await checkForErrors(response);
};

export const deleteBot = async (id: BotId) => {
  const req = new Request(`${host}/api/bots/${id}`, {
    method: "DELETE",
  });
  const response = await fetch(req);
  await checkForErrors(response);
};

export const createLeaderboard = async (
  payload: CreateLeaderboardRequest,
): Promise<LeaderboardOverviewResponse> => {
  const req = new Request(`${host}/api/leaderboards`, {
    method: "POST",
    body: JSON.stringify(payload),
    headers: {
      "Content-Type": "application/json",
    },
  });

  const response = await fetch(req);
  return await parseResponse<LeaderboardOverviewResponse>(response);
};

export const patchLeaderboard = async (
  id: LeaderboardId,
  payload: PatchLeaderboardRequest,
) => {
  const req = new Request(`${host}/api/leaderboards/${id}`, {
    method: "PATCH",
    body: JSON.stringify(payload),
    headers: {
      "Content-Type": "application/json",
    },
  });

  const response = await fetch(req);
  await checkForErrors(response);
};

export const deleteLeaderboard = async (id: LeaderboardId) => {
  const req = new Request(`${host}/api/leaderboards/${id}`, {
    method: "DELETE",
  });
  const response = await fetch(req);
  await checkForErrors(response);
};

export const chart = async (
  payload: ChartRequest,
): Promise<ChartOverviewResponse> => {
  const req = new Request(`${host}/api/chart`, {
    method: "POST",
    body: JSON.stringify(payload),
    headers: {
      "Content-Type": "application/json",
    },
  });

  const response = await fetch(req);
  return await parseResponse<ChartOverviewResponse>(response);
};

export const fetchBotSourceCode = async (id: BotId): Promise<BotSourceCode> => {
  const response = await fetch(`${host}/api/bots/${id}/source`);
  return await parseResponse<BotSourceCode>(response);
};

export const enableMatchmaking = async (enabled: boolean): Promise<void> => {
  const payload: EnableMatchmakingRequest = { enabled };

  const req = new Request(`${host}/api/matchmaking`, {
    method: "PUT",
    body: JSON.stringify(payload),
    headers: {
      "Content-Type": "application/json",
    },
  });

  const response = await fetch(req);
  await checkForErrors(response);
};

export const watchReplay = async (
  id: MatchId,
  signal?: AbortSignal,
): Promise<WatchReplayResponse> => {
  const response = await fetch(`${host}/api/matches/${id}/replay`, { signal });
  return await parseResponse<WatchReplayResponse>(response);
};

export const closeReplay = async (sessionId: string) => {
  const req = new Request(`${host}/api/replays/${sessionId}`, {
    method: "DELETE",
  });
  const response = await fetch(req);
  await checkForErrors(response);
};

async function checkForErrors(response: Response) {
  if (response.ok) {
    return;
  }

  let message = response.statusText || `HTTP ${response.status}`;
  try {
    const body = (await response.json()) as ApiErrorResponse;
    message = body.message ?? body.error_code ?? message;
  } catch {
    // Keep the HTTP status when the server does not return its JSON error shape.
  }
  throw new Error(message);
}

async function parseResponse<T>(response: Response): Promise<T> {
  await checkForErrors(response);
  return (await response.json()) as T;
}

interface ApiErrorResponse {
  error_code: string;
  message?: string;
}
