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
  FetchMatchesResponse,
  FetchMatchesRequest,
  MatchId,
  WatchReplayResponse,
} from "@/models";

const host = import.meta.env.DEV ? "http://127.0.0.1:1234" : "";

export const fetchStatus = async (): Promise<FetchStatusResponse> => {
  const response = await fetch(`${host}/api/status`);
  return await parseResponse<FetchStatusResponse>(response);
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

export const fetchMatches = async (
  req: FetchMatchesRequest,
): Promise<FetchMatchesResponse> => {
  const query_args = [
    `filter=${req.filter}`,
    `including_bots=${req.includingBots.join(",")}`,
    `offset=${req.offset}`,
    `limit=${req.limit}`,
  ];
  const response = await fetch(`${host}/api/matches?${query_args.join("&")}`);
  return await parseResponse<FetchMatchesResponse>(response);
};

export const validateFilter = async (filter: string): Promise<boolean> => {
  const response = await fetch(`${host}/api/validate-filter?filter=${filter}`);
  if (response.status >= 500) {
    throw new Error("Internal server error");
  } else if (response.ok) {
    return true;
  } else {
    return false;
  }
};

export const watchReplay = async (
  id: MatchId,
): Promise<WatchReplayResponse> => {
  const response = await fetch(`${host}/api/matches/${id}/replay`);
  return await parseResponse<WatchReplayResponse>(response);
};

export const closeReplay = async (id: MatchId) => {
  const req = new Request(`${host}/api/matches/${id}/replay`, {
    method: "DELETE",
  });
  const response = await fetch(req);
  await checkForErrors(response);
};

async function checkForErrors(response: Response) {
  if (response.status >= 500) {
    throw new Error("Internal server error");
  } else if (!response.ok) {
    const body = (await response.json()) as ApiErrorResponse;
    throw new Error(body.message ?? body.error_code);
  }
}

async function parseResponse<T>(response: Response): Promise<T> {
  await checkForErrors(response);
  return (await response.json()) as T;
}

interface ApiErrorResponse {
  error_code: string;
  message?: string;
}
