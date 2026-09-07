export interface ConfigurationState {
  active: ArenaConfiguration | null;
  runtime_available: boolean;
  runtime_error: string | null;
}

export interface ArenaConfiguration {
  game: GameConfiguration;
  evaluation: EvaluationConfiguration;
  ranking: RankingConfiguration;
  leaderboards: LeaderboardsConfiguration;
  workers: EmbeddedWorkerConfiguration[];
}

export interface GameConfiguration {
  min_players: number;
  max_players: number;
  symmetric: boolean;
}

export interface EvaluationConfiguration {
  enabled_on_start: boolean | null;
  seed_sequence_key: number;
  stages: EvaluationStageConfiguration[];
}

export interface EvaluationStageConfiguration {
  name: string;
  seed_source: EvaluationSeedSource;
  coverage: EvaluationCoveragePolicy;
  min_players: number | null;
  max_players: number | null;
}

export type EvaluationSeedSource =
  | { type: "generated" }
  | { type: "curated"; seeds: number[] }
  | { type: "fresh_random" };

export type EvaluationCoveragePolicy =
  | { type: "per_benchmark"; target: number }
  | { type: "total"; target: number }
  | {
      type: "weighted_total";
      target: number;
      weights: Record<string, number>;
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
      type: "managed_codingame";
      repository_url: string;
      branch: string | null;
      java: string | null;
      maven: string | null;
    }
  | {
      type: "command";
      play_match: string;
      watch_replay: string;
    };
export type RefereeAction = "install" | "check" | "rebuild" | "update";

export interface RefereeOperationStatus {
  action: RefereeAction | null;
  phase: string | null;
  diagnostic: string | null;
}

export interface ManagedRefereeStatus {
  selected: Extract<RefereeConfiguration, { type: "managed_codingame" }> | null;
  installed: boolean;
  replacement_required: boolean;
  checkout_path: string;
  artifact_path: string;
  installed_repository_url: string | null;
  branch: string | null;
  upstream_commit: string | null;
  adaptation_commit: string | null;
  committed_ahead: number | null;
  committed_behind: number | null;
  staged: boolean;
  unstaged: boolean;
  untracked: boolean;
  update_status: "up_to_date" | "update_available" | "unavailable";
  last_successful_check: string | null;
  observed_remote_commit: string | null;
  operation: RefereeOperationStatus;
}

export interface CreateBotRequest {
  name: string;
  source_code: string;
  language: string;
  role: BotRole;
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
  evaluation_scheduling_enabled: boolean;
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
  role: BotRole;
  evaluation_plan_revision_id: number | null;
  evaluation: CandidateEvaluationResponse | null;
  matches_played: number;
  matches_with_error: number;
  builds: BuildResponse[];
  created_at: string;
}

export type BotRole = "candidate" | "benchmark" | "archived_benchmark";

export interface CandidateEvaluationResponse {
  complete: boolean;
  stages: EvaluationStageResponse[];
}

export interface EvaluationStageResponse {
  id: number;
  name: string;
  coverage: "per_benchmark" | "total" | "weighted_total";
  target: number;
  matches: number;
  candidate_errors: number;
  complete: boolean;
  benchmark_encounters: Record<string, number>;
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

export interface EvaluationSchedulingRequest {
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
