// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, renderHook, waitFor } from "@testing-library/react";
import { PropsWithChildren } from "react";
import { afterEach, describe, expect, it } from "vitest";

import {
  createMatchBrowsing,
  MatchBrowseSearch,
  MatchBrowsingAdapter,
  MatchBrowsingView,
  MatchResults,
  MatchSearchRequest,
  NavigateMatches,
} from "./index";

class InMemoryMatchAdapter implements MatchBrowsingAdapter {
  readonly requests: MatchSearchRequest[] = [];

  constructor(private readonly responses: Array<unknown | Error>) {}

  async search(request: MatchSearchRequest): Promise<unknown> {
    this.requests.push(request);
    const response = this.responses.shift();
    if (response instanceof Error) {
      throw response;
    }
    return response;
  }
}

function results(id: number, hasMore: boolean): MatchResults {
  return {
    matches: [
      {
        id,
        participants: [],
        seed: String(id),
        attributes: [],
      },
    ],
    has_more: hasMore,
  };
}

function renderBrowsing(
  adapter: InMemoryMatchAdapter,
  initialSearch: MatchBrowseSearch,
) {
  const browsing = createMatchBrowsing(adapter);
  const transitions: MatchBrowseSearch[] = [];
  const navigate: NavigateMatches = (search) => transitions.push(search);
  const queryClient = new QueryClient({
    defaultOptions: { queries: { gcTime: 0 } },
  });
  const wrapper = ({ children }: PropsWithChildren) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
  const hook = renderHook<
    MatchBrowsingView,
    { search: MatchBrowseSearch; navigate: NavigateMatches }
  >(
    ({ search, navigate: nextNavigate }) =>
      browsing.useMatches(search, nextNavigate),
    {
      wrapper,
      initialProps: { search: initialSearch, navigate },
    },
  );

  return { browsing, transitions, navigate, ...hook };
}

afterEach(() => {
  document.body.innerHTML = "";
});

describe("match-browsing interface", () => {
  it("canonicalizes route defaults and builds complete head-to-head links", () => {
    const browsing = createMatchBrowsing(new InMemoryMatchAdapter([]));

    expect(
      browsing.routeSearch.parse({
        page: 0,
        pageSize: 101,
        withBots: "not-an-array",
      }),
    ).toEqual({ filter: "", withBots: [], page: 1, pageSize: 10 });

    const leaderboardFilter = 'match.label == "A&B+#"';
    for (const [result, operator] of [
      ["win", "<"],
      ["loss", ">"],
      ["draw", "=="],
    ] as const) {
      expect(
        browsing.leaderboardSearch({
          leaderboardFilter,
          selectedBotId: 7,
          opponentBotId: 9,
          result,
        }),
      ).toEqual({
        filter: `(${leaderboardFilter}) AND (bot(7).rank ${operator} bot(9).rank)`,
        withBots: [7, 9],
        page: 1,
        pageSize: 10,
      });
    }
    expect(
      browsing.leaderboardSearch({
        leaderboardFilter,
        selectedBotId: 7,
        opponentBotId: 9,
        result: "all",
      }).filter,
    ).toBe(leaderboardFilter);
  });

  it("owns request encoding, URL transitions, and page eligibility", async () => {
    const filter = 'match.label == "A&B+#"';
    const adapter = new InMemoryMatchAdapter([
      results(101, true),
      results(102, false),
    ]);
    const initialSearch = {
      filter,
      withBots: [7, 9],
      page: 1,
      pageSize: 25,
    };
    const { result, rerender, transitions, navigate } = renderBrowsing(
      adapter,
      initialSearch,
    );

    await waitFor(() =>
      expect(result.current.results?.matches[0].id).toBe(101),
    );
    expect(adapter.requests[0]).toEqual({
      filter,
      includingBots: [7, 9],
      offset: 0,
      limit: 25,
    });
    expect(result.current.canGoNext).toBe(true);

    act(() => result.current.goToPage(2));
    expect(transitions).toEqual([{ ...initialSearch, page: 2 }]);
    rerender({ search: transitions[0], navigate });

    await waitFor(() =>
      expect(result.current.results?.matches[0].id).toBe(102),
    );
    expect(adapter.requests[1]).toEqual({
      filter,
      includingBots: [7, 9],
      offset: 25,
      limit: 25,
    });
    expect(result.current.canGoNext).toBe(false);
    expect(result.current.canGoPrevious).toBe(true);

    act(() => result.current.goToPage(3));
    expect(transitions).toHaveLength(1);
    act(() => result.current.goToPage(1));
    expect(transitions[1]).toEqual(initialSearch);
  });

  it("refreshes an unchanged search and resets changed criteria to page one", async () => {
    const adapter = new InMemoryMatchAdapter([
      results(201, false),
      results(202, false),
    ]);
    const initialSearch = {
      filter: "match.player_count == 2",
      withBots: [1, 2],
      page: 1,
      pageSize: 10,
    };
    const { result, transitions } = renderBrowsing(adapter, initialSearch);

    await waitFor(() =>
      expect(result.current.results?.matches[0].id).toBe(201),
    );
    act(() =>
      result.current.submit({
        filter: initialSearch.filter,
        withBots: initialSearch.withBots,
        pageSize: initialSearch.pageSize,
      }),
    );
    await waitFor(() => expect(adapter.requests).toHaveLength(2));
    await waitFor(() =>
      expect(result.current.results?.matches[0].id).toBe(202),
    );
    expect(transitions).toHaveLength(0);

    act(() =>
      result.current.submit({
        filter: "match.player_count == 3",
        withBots: [1],
        pageSize: 50,
      }),
    );
    expect(transitions).toEqual([
      {
        filter: "match.player_count == 3",
        withBots: [1],
        page: 1,
        pageSize: 50,
      },
    ]);
  });

  it("retains the last success across request and response-validation errors", async () => {
    const adapter = new InMemoryMatchAdapter([
      results(301, true),
      new Error("Invalid filter expression"),
      { matches: [], has_more: "yes" },
    ]);
    const initialSearch = {
      filter: "",
      withBots: [],
      page: 1,
      pageSize: 10,
    };
    const { result, rerender, transitions, navigate } = renderBrowsing(
      adapter,
      initialSearch,
    );

    await waitFor(() =>
      expect(result.current.results?.matches[0].id).toBe(301),
    );

    act(() =>
      result.current.submit({
        filter: "invalid(",
        withBots: [],
        pageSize: 10,
      }),
    );
    rerender({ search: transitions[0], navigate });
    await waitFor(() =>
      expect(result.current.errorMessage).toBe("Invalid filter expression"),
    );
    expect(result.current.results?.matches[0].id).toBe(301);
    expect(result.current.showingRetainedResults).toBe(true);
    expect(result.current.canGoNext).toBe(false);

    act(() =>
      result.current.submit({
        filter: "match.player_count == 2",
        withBots: [],
        pageSize: 10,
      }),
    );
    rerender({ search: transitions[1], navigate });
    await waitFor(() =>
      expect(result.current.errorMessage).toMatch(/^Invalid match response:/),
    );
    expect(result.current.errorMessage).toContain("has_more");
    expect(result.current.results?.matches[0].id).toBe(301);
    expect(result.current.showingRetainedResults).toBe(true);
  });
});
