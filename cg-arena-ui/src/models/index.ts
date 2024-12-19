export interface CreateBotRequest {
  name: string;
  source_code: string;
  language: string;
}

export interface BotMinimalResponse {
  id: string;
  name: string;
}

export interface FetchLeaderboardResponse {
  bot_overview: LeaderboardBotOverviewResponse;
  items: LeaderboardItemResponse[];
}

export interface LeaderboardBotOverviewResponse {
  id: string;
  name: string;
  language: string;
  rating_mu: number;
  rating_sigma: number;
  matches_played: number;
  matches_with_error: number;
  builds: BuildResponse[];
}

export interface LeaderboardItemResponse {
  id: string;
  rank: number;
  name: string;
  rating_mu: number;
  rating_sigma: number;
  wins: number;
  loses: number;
  draws: number;
  created_at: string;
}

export interface BuildResponse {
  worker_name: string;
  status: string;
  stderr?: string;
}

export function rating_score(item: {
  rating_mu: number;
  rating_sigma: number;
}): number {
  return Number((item.rating_mu - item.rating_sigma * 3).toFixed(2));
}
