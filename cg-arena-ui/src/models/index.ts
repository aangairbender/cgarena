export interface ConfigurationState {
  active: ArenaConfiguration | null;
  runtime_available: boolean;
}

export interface ArenaConfiguration {
  game: GameConfiguration;
  matchmaking: MatchmakingConfiguration;
  ranking: RankingConfiguration;
  leaderboards: LeaderboardsConfiguration;
  workers: EmbeddedWorkerConfiguration[];
}

export interface GameConfiguration {
  min_players: number;
  max_players: number;
  symmetric: boolean;
  referee_git_url: string | null;
}

export type MatchmakingConfiguration =
  | {
      algorithm: "v1";
      min_matches: number;
      min_matches_preference: number;
      enabled_on_start: boolean | null;
    }
  | {
      algorithm: "v2";
      min_matches_against_best: number | null;
      min_matches_per_pair: number;
      max_matches: number | null;
      enabled_on_start: boolean | null;
    };

export type RankingConfiguration =
  | {
      algorithm: "OpenSkill";
      beta: number | null;
      uncertainty_tolerance: number | null;
    }
  | {
      algorithm: "TrueSkill";
      draw_probability: number | null;
      beta: number | null;
      default_dynamics: number | null;
    }
  | { algorithm: "Elo"; k: number | null }
  | { algorithm: "BradleyTerry"; max_iter: number | null };

export interface LeaderboardsConfiguration {
  uncertainty_coefficient: number | null;
}

export interface EmbeddedWorkerConfiguration {
  type: "embedded";
  threads: number;
  referee: RefereeConfiguration;
  cmd_build: string;
  cmd_run: string;
}

export type RefereeConfiguration =
  | {
      type: "codingame_jar";
      path: string;
      java: string | null;
      league: number | null;
    }
  | {
      type: "command";
      play_match: string;
      watch_replay: string;
    };

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
  filter: string;
}

export interface FetchStatusResponse {
  bots: BotOverviewResponse[];
  leaderboards: LeaderboardOverviewResponse[];
  matchmaking_enabled: boolean;
}

export interface LeaderboardOverviewResponse {
  id: LeaderboardId;
  name: string;
  filter: string;
  status: "live" | "computing";
  items: LeaderboardItemResponse[];
  winrate_stats: WinrateStatsResponse[];
  total_matches: number;
  example_seeds: string[];
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
  rank: number;
  rating: number;
  rating_mu: number;
  rating_sigma: number;
}

export interface BuildResponse {
  worker_name: string;
  status: string;
  stderr?: string;
}

export interface ChartRequest {
  filter: string;
  attribute_name: string;
}

export interface ChartOverviewResponse {
  items: ChartItemResponse[];
  total_matches: number;
}

export interface ChartItemResponse {
  bot_id: BotId;
  data: ChartTurnDataResponse[];
}

export interface ChartTurnDataResponse {
  turn: number;
  avg: number;
  min: number;
  max: number;
}

export interface BotSourceCode {
  language: string;
  source_code: string;
}

export interface EnableMatchmakingRequest {
  enabled: boolean;
}

export interface WatchReplayResponse {
  session_id: string;
  viewer_url: string;
}

export type BotId = number;
export type MatchId = number;
export type LeaderboardId = number;

export const GLOBAL_LEADERBOARD_ID = 0 as LeaderboardId;
