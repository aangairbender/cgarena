import {
    BotMinimalResponse,
    CreateBotRequest,
    FetchLeaderboardResponse,
} from "@models";

export const fetchBots = async (): Promise<BotMinimalResponse[]> => {
    const response = await fetch(`/api/bots`);
    const data: BotMinimalResponse[] = await response.json();
    return data;
};

export const fetchLeaderboard = async (
    id: string
): Promise<FetchLeaderboardResponse | undefined> => {
    const response = await fetch(`/api/bots/${id}`);
    if (response.status == 404) return undefined;
    const data: FetchLeaderboardResponse = await response.json();
    return data;
};

export const submitNewBot = async (
    payload: CreateBotRequest
): Promise<BotMinimalResponse> => {
    const req = new Request(`/api/bots`, {
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
    const data: BotMinimalResponse = await response.json();
    return data;
};
