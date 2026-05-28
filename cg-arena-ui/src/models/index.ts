import { z } from "zod";
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
  example_seeds: bigint[];
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

export interface FetchMatchesRequest {
  filter: string;
  includingBots: number[];
  offset: number;
  limit: number;
}

export interface FetchMatchesResponse {
  matches: MatchOverviewResponse[];
}

export interface MatchOverviewResponse {
  id: number;
  participants: ParticipantOverviewResponse[];
  seed: bigint;
  attributes: MatchAttributeResponse[];
}

export interface MatchAttributeResponse {
  name: string;
  bot_id?: BotId;
  turn?: number;
  value: string;
}

export interface ParticipantOverviewResponse {
  rank: number;
  index: number;
  bot_id: BotId;
  bot_name: string;
  error: boolean;
}

export interface WatchReplayResponse {
  viewer_url: string;
}

export type BotId = number;
export type MatchId = number;
export type LeaderboardId = number;

export const GLOBAL_LEADERBOARD_ID = 0 as LeaderboardId;

export const configSchema = z.object({
  game: z
    .object({
      min_players: z.int().min(2),
      max_players: z.int().max(8),
      symmetric: z.boolean(),
    })
    .refine((data) => data.min_players <= data.max_players, {
      error: "max_players can't be smaller than min_players",
      path: ["max_players"],
    }),
  matchmaking: z.intersection(
    z.object({
      enabled_on_start: z.boolean(),
    }),
    z.union([
      z.object({
        algorithm: z.literal("v1").optional(),
        min_matches: z.int().positive(),
        min_matches_preference: z.number().min(0).max(1),
      }),
      z.object({
        algorithm: z.literal("v2").optional(),
        min_matches_per_pair: z.int().positive(),
        min_matches_against_best: z.int().positive().optional(),
        max_matches: z.int().positive().optional(),
      }),
    ]),
  ),
  ranking: z.discriminatedUnion("algorithm", [
    z.object({
      algorithm: "OpenSkill",
    }),
    z.object({
      algorithm: "TrueSkill",
    }),
    z.object({
      algorithm: "Elo",
    }),
    z.object({
      algorithm: "BradleyTerry",
    }),
  ]),
  server: z.object({
    port: z.number().min(0).max(65535),
    expose: z.boolean(),
  }),
  log: z.object({
    level: z.enum(["INFO", "DEBUG"]),
    file: z.string(),
  }),
  leaderboards: z.object({
    uncertainty_coefficient: z.number().optional(),
  }),
  workers: z.array(
    z.discriminatedUnion("type", [
      z.object({
        type: "embedded",
        threads: z.int(),
        cmd_play_match: z.string(),
        cmd_build: z.string(),
        cmd_run: z.string(),
      }),
    ]),
  ),
});

export type Config = z.infer<typeof configSchema>;
