import {
  BotMinimalResponse,
  CreateBotRequest,
  FetchLeaderboardResponse,
} from "@models";

const host = import.meta.env.DEV ? "http://127.0.0.1:1234" : "";

export const fetchBots = async (): Promise<BotMinimalResponse[]> => {
  const response = await fetch(`${host}/api/bots`);
  return (await response.json()) as BotMinimalResponse[];
};

export const fetchLeaderboard = async (
  id: string
): Promise<FetchLeaderboardResponse | undefined> => {
  const response = await fetch(`${host}/api/bots/${id}`);
  if (response.status == 404) return undefined;
  return (await response.json()) as FetchLeaderboardResponse;
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
  if (response.status == 409) {
    throw new Error("Bot with the same name already exists");
  }
  return (await response.json()) as BotMinimalResponse;
};

export const deleteBot = async (id: string) => {
    const req = new Request(`${host}/api/bots/${id}`, {
        method: "DELETE"
    });
    await fetch(req);
};
