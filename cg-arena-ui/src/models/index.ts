export interface CreateBotRequest {
  name: string;
  source_code: string;
  language: string;
}

export interface RenameBotRequest {
  name: string;
}

export interface CreateLeaderboardRequest {
  name: string;
  filter: string;
}

export interface PatchLeaderboardRequest {
  name: string;
  filter: string,
}

export interface FetchStatusResponse {
  bots: BotOverviewResponse[];
  leaderboards: LeaderboardOverviewResponse[];
}

export interface LeaderboardOverviewResponse {
  id: LeaderboardId;
  name: string;
  filter: string;
  status: "live" | "computing";
  items: LeaderboardItemResponse[];
  winrate_stats: WinrateStatsResponse[];
  total_matches: number;
  example_seeds: number[];
}

export interface WinrateStatsResponse {
  bot_id: BotId;
  opponent_bot_id: BotId;
  wins: number;
  loses: number;
  draws: number;
}

export interface BotOverviewResponse {
  id: BotId;
  name: string;
  language: string;
  matches_played: number;
  matches_with_error: number;
  builds: BuildResponse[];
  created_at: string;
}

export interface LeaderboardItemResponse {
  id: BotId;
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

export type BotId = number;
export type LeaderboardId = number;

export const GLOBAL_LEADERBOARD_ID = 0 as LeaderboardId;