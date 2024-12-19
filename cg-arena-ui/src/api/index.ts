import {
  BotMinimalResponse,
  CreateBotRequest,
  FetchLeaderboardResponse,
  RenameBotRequest,
} from "@models";

const host = import.meta.env.DEV ? "http://127.0.0.1:1234" : "";

export const fetchBots = async (): Promise<BotMinimalResponse[]> => {
  const response = await fetch(`${host}/api/bots`);
  return await parseResponse<BotMinimalResponse[]>(response);
};

export const fetchLeaderboard = async (
  id: string
): Promise<FetchLeaderboardResponse | undefined> => {
  const response = await fetch(`${host}/api/bots/${id}`);
  if (response.status == 404) return undefined;
  return await parseResponse<FetchLeaderboardResponse>(response);
};

export const submitNewBot = async (
  payload: CreateBotRequest
): Promise<BotMinimalResponse> => {
  const req = new Request(`${host}/api/bots`, {
    method: "POST",
    body: JSON.stringify(payload),
    headers: {
      "Content-Type": "application/json",
    },
  });

  const response = await fetch(req);
  return await parseResponse<BotMinimalResponse>(response);
};

export const renameBot = async (
  id: string,
  payload: RenameBotRequest
): Promise<BotMinimalResponse> => {
  const req = new Request(`${host}/api/bots/${id}`, {
    method: "PATCH",
    body: JSON.stringify(payload),
    headers: {
      "Content-Type": "application/json",
    },
  });

  const response = await fetch(req);
  return await parseResponse<BotMinimalResponse>(response);
};

export const deleteBot = async (id: string) => {
  const req = new Request(`${host}/api/bots/${id}`, {
      method: "DELETE"
  });
  const response = await fetch(req);
  if (!response.ok) {
    throw new Error("Internal server error");
  }
};


async function parseResponse<T>(response: Response): Promise<T> {
  if (response.ok) {
    return (await response.json()) as T;
  } else if (response.status >= 500) {
    throw new Error("Internal server error");
  } else {
    const body = (await response.json()) as ApiErrorResponse;
    throw new Error(body.message ?? body.error_code);
  }
}

interface ApiErrorResponse {
  error_code: string,
  message?: string,
}
