import { fetchMatches } from "@/api";
import { MatchCard } from "@/components/MatchCard";
import { BotId, MatchOverviewResponse } from "@/models";
import { getRouteApi } from "@tanstack/react-router";
import { useCallback, useEffect, useState } from "react";
import { Button, Card, Form, Pagination } from "react-bootstrap";

const routeApi = getRouteApi("/matches");

const PAGE_SIZE = 10;

export default function MatchesPage() {
  const { filter: initialFilter, withBots } = routeApi.useSearch();
  const [page, setPage] = useState(1);
  const [matches, setMatches] = useState<MatchOverviewResponse[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const search = useCallback(
    (filter: string, includingBots: number[]) => {
      const req = {
        filter,
        includingBots,
        offset: (page - 1) * PAGE_SIZE,
        limit: PAGE_SIZE,
      };
      setLoading(true);
      setError("");
      fetchMatches(req)
        .then((resp) => setMatches(resp.matches))
        .catch((e) => setError(e.toString()))
        .finally(() => setLoading(false));
    },
    [setLoading, setError, setMatches, page],
  );

  useEffect(() => {
    search(initialFilter ?? "", []);
  }, [initialFilter, search]);

  return (
    <div className="d-flex flex-column gap-3">
      <Card>
        <Card.Header>Matches</Card.Header>
        <Card.Body>
          <MatchesFilters
            initialFilter={initialFilter}
            initialWithBots={withBots}
            onSearch={search}
          />
          {error && <span>Error: {error}</span>}
          {loading && <span>Loading...</span>}
          {/* <MatchesTable matches={matches} /> */}
        </Card.Body>
      </Card>

      <MatchList matches={matches} />

      <div className="d-flex justify-content-center">
        <Pagination>
          <Pagination.Prev
            disabled={page == 1}
            onClick={() => setPage((p) => p - 1)}
          >
            Prev
          </Pagination.Prev>
          <Pagination.Item active disabled>
            {page}
          </Pagination.Item>
          <Pagination.Next onClick={() => setPage((p) => p + 1)}>
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
  const [withBots, setWithBots] = useState<number[]>(initialWithBots ?? []);

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
