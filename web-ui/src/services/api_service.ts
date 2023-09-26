export interface CreateBotRequest {
    name: string,
    language: string,
    source_code: string,
}

const HOST = "http://127.0.0.1:12345";

export async function createBot(data: CreateBotRequest): Promise<boolean> {
    const requestOptions = {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(data)
    };
    const response = await fetch(`${HOST}/api/bots`, requestOptions);
    return response.status === 200;
}
