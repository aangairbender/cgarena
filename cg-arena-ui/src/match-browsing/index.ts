import {
  keepPreviousData,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import { useCallback, useEffect, useState } from "react";
import { z } from "zod";

import { BotId } from "@/models";

const DEFAULT_PAGE = 1;
const DEFAULT_PAGE_SIZE = 10;
const MATCH_QUERY_KEY = ["match-browsing"] as const;

const routeSearchSchema = z.object({
  filter: z.string().catch("").default(""),
  withBots: z.array(z.int()).catch([]).default([]),
  page: z.int().min(1).catch(DEFAULT_PAGE).default(DEFAULT_PAGE),
  pageSize: z
    .int()
    .min(1)
    .max(100)
    .catch(DEFAULT_PAGE_SIZE)
    .default(DEFAULT_PAGE_SIZE),
});

const matchResultsSchema = z.object({
  matches: z.array(
    z.object({
      id: z.int(),
      participants: z.array(
        z.object({
          rank: z.int(),
          index: z.int(),
          bot_id: z.int(),
          bot_name: z.string(),
          error: z.boolean(),
        }),
      ),
      seed: z.string(),
      attributes: z.array(
        z.object({
          name: z.string(),
          bot_id: z.int().nullable(),
          turn: z.int().nullable(),
          value: z.string(),
        }),
      ),
    }),
  ),
  has_more: z.boolean(),
});

export type MatchBrowseSearch = z.output<typeof routeSearchSchema>;
export type MatchOverview = z.output<
  (typeof matchResultsSchema)["shape"]["matches"]["element"]
>;
export type MatchResults = z.output<typeof matchResultsSchema>;

export interface MatchSearchRequest {
  filter: string;
  includingBots: BotId[];
  offset: number;
  limit: number;
}

/** Port implemented by the production HTTP adapter and by interface tests in memory. */
export interface MatchBrowsingAdapter {
  search(request: MatchSearchRequest): Promise<unknown>;
}

export interface MatchSearchDraft {
  filter: string;
  withBots: BotId[];
  pageSize: number;
}

export interface HeadToHeadMatchLink {
  leaderboardFilter: string;
  selectedBotId: BotId;
  opponentBotId: BotId;
  result: "win" | "loss" | "draw" | "all";
}

export interface MatchBrowsingView {
  search: MatchBrowseSearch;
  results: MatchResults | undefined;
  errorMessage: string | undefined;
  isFetching: boolean;
  showingRetainedResults: boolean;
  canGoPrevious: boolean;
  canGoNext: boolean;
  submit(draft: MatchSearchDraft): void;
  goToPage(page: number): void;
}

export type NavigateMatches = (search: MatchBrowseSearch) => void;

/**
 * The complete match-browsing interface. Callers supply user intent and render
 * the returned view; URL policy, filter composition, paging, requests,
 * validation, refreshes, errors, and retained results remain behind this seam.
 */
export interface MatchBrowsing {
  readonly routeSearch: typeof routeSearchSchema;
  leaderboardSearch(link: HeadToHeadMatchLink): MatchBrowseSearch;
  useMatches(
    search: MatchBrowseSearch,
    navigate: NavigateMatches,
  ): MatchBrowsingView;
}

export function createMatchBrowsing(
  adapter: MatchBrowsingAdapter,
): MatchBrowsing {
  return {
    routeSearch: routeSearchSchema,
    leaderboardSearch,
    useMatches(search, navigate) {
      return useMatchBrowsing(adapter, search, navigate);
    },
  };
}

function leaderboardSearch({
  leaderboardFilter,
  selectedBotId,
  opponentBotId,
  result,
}: HeadToHeadMatchLink): MatchBrowseSearch {
  let resultFilter = "";
  if (result !== "all") {
    const operator = { win: "<", loss: ">", draw: "==" }[result];
    resultFilter = `bot(${selectedBotId}).rank ${operator} bot(${opponentBotId}).rank`;
  }

  const filter =
    leaderboardFilter && resultFilter
      ? `(${leaderboardFilter}) AND (${resultFilter})`
      : leaderboardFilter || resultFilter;

  return routeSearchSchema.parse({
    filter,
    withBots: [selectedBotId, opponentBotId],
    page: DEFAULT_PAGE,
    pageSize: DEFAULT_PAGE_SIZE,
  });
}

function useMatchBrowsing(
  adapter: MatchBrowsingAdapter,
  unvalidatedSearch: MatchBrowseSearch,
  navigate: NavigateMatches,
): MatchBrowsingView {
  const search = routeSearchSchema.parse(unvalidatedSearch);
  const queryClient = useQueryClient();
  const [lastSuccessfulResults, setLastSuccessfulResults] =
    useState<MatchResults>();
  const query = useQuery({
    queryKey: [
      ...MATCH_QUERY_KEY,
      search.filter,
      search.withBots,
      search.page,
      search.pageSize,
    ],
    queryFn: async () => {
      const response = await adapter.search({
        filter: search.filter,
        includingBots: search.withBots,
        offset: (search.page - 1) * search.pageSize,
        limit: search.pageSize,
      });
      const parsed = matchResultsSchema.safeParse(response);
      if (!parsed.success) {
        const details = parsed.error.issues
          .map((issue) => {
            const location =
              issue.path.length > 0 ? issue.path.join(".") : "response";
            return `${location}: ${issue.message}`;
          })
          .join("; ");
        throw new Error(`Invalid match response: ${details}`);
      }
      return parsed.data;
    },
    placeholderData: keepPreviousData,
    retry: false,
  });

  useEffect(
    () =>
      queryClient.getQueryCache().subscribe((event) => {
        if (
          event.type === "updated" &&
          event.query.queryKey[0] === MATCH_QUERY_KEY[0] &&
          event.query.state.status === "success"
        ) {
          setLastSuccessfulResults(event.query.state.data as MatchResults);
        }
      }),
    [queryClient],
  );

  const rememberCurrentResults = useCallback(() => {
    if (
      query.data !== undefined &&
      !query.isPlaceholderData &&
      !query.isError
    ) {
      setLastSuccessfulResults(query.data);
    }
  }, [query.data, query.isError, query.isPlaceholderData]);

  const displayedResults = query.data ?? lastSuccessfulResults;
  const canGoNext =
    !query.isError && !query.isFetching && displayedResults?.has_more === true;

  const submit = useCallback(
    (draft: MatchSearchDraft) => {
      rememberCurrentResults();
      const nextSearch = routeSearchSchema.parse({
        ...draft,
        page: DEFAULT_PAGE,
      });

      if (searchesMatch(search, nextSearch)) {
        void query.refetch();
      } else {
        navigate(nextSearch);
      }
    },
    [navigate, query, rememberCurrentResults, search],
  );

  const goToPage = useCallback(
    (page: number) => {
      rememberCurrentResults();
      if (
        page < DEFAULT_PAGE ||
        (page > search.page &&
          (query.isError ||
            query.isFetching ||
            displayedResults?.has_more !== true))
      ) {
        return;
      }
      navigate({ ...search, page });
    },
    [
      displayedResults?.has_more,
      navigate,
      query.isError,
      query.isFetching,
      rememberCurrentResults,
      search,
    ],
  );

  const errorMessage = query.isError
    ? query.error instanceof Error
      ? query.error.message
      : String(query.error)
    : undefined;

  return {
    search,
    results: displayedResults,
    errorMessage,
    isFetching: query.isFetching,
    showingRetainedResults:
      query.isError && lastSuccessfulResults !== undefined,
    canGoPrevious: search.page > DEFAULT_PAGE && !query.isFetching,
    canGoNext,
    submit,
    goToPage,
  };
}

function searchesMatch(left: MatchBrowseSearch, right: MatchBrowseSearch) {
  return (
    left.filter === right.filter &&
    left.page === right.page &&
    left.pageSize === right.pageSize &&
    left.withBots.length === right.withBots.length &&
    left.withBots.every((botId, index) => botId === right.withBots[index])
  );
}

const apiHost = import.meta.env.DEV ? "http://127.0.0.1:1234" : "";

const httpAdapter: MatchBrowsingAdapter = {
  async search(request) {
    const query = new URLSearchParams({
      filter: request.filter,
      including_bots: request.includingBots.join(","),
      offset: String(request.offset),
      limit: String(request.limit),
    });
    const response = await fetch(`${apiHost}/api/matches?${query}`);
    await checkForErrors(response);
    return await response.json();
  },
};

async function checkForErrors(response: Response) {
  if (response.status >= 500) {
    throw new Error("Internal server error");
  }
  if (!response.ok) {
    const body = (await response.json()) as {
      error_code?: string;
      message?: string;
    };
    throw new Error(body.message ?? body.error_code ?? "Request failed");
  }
}

export const matchBrowsing = createMatchBrowsing(httpAdapter);
