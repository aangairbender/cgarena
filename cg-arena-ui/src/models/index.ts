export interface CreateBotRequest {
  name: string;
  source_code: string;
  language: string;
}

export interface RenameBotRequest {
  name: string;
}

export interface FetchStatusResponse {
  bots: BotOverviewResponse[];
  leaderboards: LeaderboardOverviewResponse[];
}

export interface LeaderboardOverviewResponse {
  id: string,
  name: string,
  items: LeaderboardItemResponse[];
  winrate_stats: WinrateStatsResponse[];
}

export interface WinrateStatsResponse {
  bot_id: string;
  opponent_bot_id: string;
  wins: number;
  loses: number;
  draws: number;
}

export interface BotOverviewResponse {
  id: string;
  name: string;
  language: string;
  matches_played: number;
  matches_with_error: number;
  builds: BuildResponse[];
  created_at: string;
}

export interface LeaderboardItemResponse {
  id: string;
  name: string;
  rank: number,
  rating_mu: number;
  rating_sigma: number;
}

export interface BuildResponse {
  worker_name: string;
  status: string;
  stderr?: string;
}

export function rating_score(item: LeaderboardItemResponse): number {
  return Number((item.rating_mu - item.rating_sigma * 3).toFixed(2));
}
