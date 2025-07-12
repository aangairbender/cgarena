import {
  BotOverviewResponse,
  CreateBotRequest,
  FetchStatusResponse,
  RenameBotRequest,
} from "@models";

const host = import.meta.env.DEV ? "http://127.0.0.1:1234" : "";

export const fetchStatus = async (): Promise<FetchStatusResponse> => {
  const response = await fetch(`${host}/api/status`);
  return await parseResponse<FetchStatusResponse>(response);
};

export const submitNewBot = async (
  payload: CreateBotRequest
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

export const renameBot = async (
  id: string,
  payload: RenameBotRequest
) => {
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

export const deleteBot = async (id: string) => {
  const req = new Request(`${host}/api/bots/${id}`, {
      method: "DELETE"
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
  error_code: string,
  message?: string,
}
