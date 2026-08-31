import { fetchMatches } from "@/api";
import { MatchCard } from "@/components/MatchCard";
import { BotId, MatchOverviewResponse } from "@/models";
import { useQuery } from "@tanstack/react-query";
import { getRouteApi } from "@tanstack/react-router";
import { useState } from "react";
import { Button, Card, Form, Pagination } from "react-bootstrap";

const routeApi = getRouteApi("/matches");

const PAGE_SIZE = 10;

export default function MatchesPage() {
  const { filter: initialFilter, withBots } = routeApi.useSearch();
  const [page, setPage] = useState(1);
  const [criteria, setCriteria] = useState({
    filter: initialFilter ?? "",
    includingBots: withBots,
  });

  const matchesQuery = useQuery({
    queryKey: ["matches", criteria.filter, criteria.includingBots, page],
    queryFn: () =>
      fetchMatches({
        ...criteria,
        offset: (page - 1) * PAGE_SIZE,
        limit: PAGE_SIZE,
      }),
  });

  const handleSearch = (filter: string, includingBots: number[]) => {
    setPage(1);
    setCriteria({ filter, includingBots });
  };

  return (
    <div className="d-flex flex-column gap-3">
      <Card>
        <Card.Header>Matches</Card.Header>
        <Card.Body>
          <MatchesFilters
            initialFilter={initialFilter}
            initialWithBots={withBots}
            onSearch={handleSearch}
          />
          {matchesQuery.error && (
            <span>Error: {String(matchesQuery.error)}</span>
          )}
          {matchesQuery.isFetching && <span>Loading...</span>}
        </Card.Body>
      </Card>

      <MatchList matches={matchesQuery.data?.matches ?? []} />

      <div className="d-flex justify-content-center">
        <Pagination>
          <Pagination.Prev
            disabled={page === 1}
            onClick={() => setPage((currentPage) => currentPage - 1)}
          >
            Prev
          </Pagination.Prev>
          <Pagination.Item active disabled>
            {page}
          </Pagination.Item>
          <Pagination.Next
            onClick={() => setPage((currentPage) => currentPage + 1)}
          >
            Next
          </Pagination.Next>
        </Pagination>
      </div>
    </div>
  );
}

type MatchesFiltersProps = {
  initialFilter?: string;
  initialWithBots?: BotId[];
  onSearch: (filter: string, includingBots: number[]) => void;
};

function MatchesFilters({
  initialFilter,
  initialWithBots,
  onSearch,
}: MatchesFiltersProps) {
  const [filter, setFilter] = useState(initialFilter ?? "");
  const withBots = initialWithBots ?? [];

  return (
    <>
      <Form.Group controlId="formName" className="mb-3">
        <Form.Label>Match filter</Form.Label>
        <div className="d-flex gap-3">
          <Form.Control
            placeholder=""
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
          />
          <Button variant="primary" onClick={() => onSearch(filter, withBots)}>
            Search
          </Button>
        </div>
        <Form.Text className="text-muted">
          e.g. match.player_count == 2
        </Form.Text>
      </Form.Group>

      {/* {error && <Alert variant="danger">{error}</Alert>} */}
    </>
  );
}

interface MatchListProps {
  matches: MatchOverviewResponse[];
}

export function MatchList({ matches }: MatchListProps) {
  if (matches.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center rounded-lg border border-dashed py-12">
        <p className="text-muted-foreground">No matches found</p>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-3">
      {matches.map((match) => (
        <MatchCard key={match.id} match={match} />
      ))}
    </div>
  );
}
