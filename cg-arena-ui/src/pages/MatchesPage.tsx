import { fetchMatches } from "@api";
import { MatchOverviewResponse, ParticipantOverviewResponse } from "@models";
import { getRouteApi } from "@tanstack/react-router";
import { useCallback, useEffect, useState } from "react";
import { Button, Card, Form, Pagination, Table } from "react-bootstrap";

const routeApi = getRouteApi("/matches");

const PAGE_SIZE = 10;

export default function MatchesPage() {
  const { filter: initialFilter } = routeApi.useSearch();
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
    <Card>
      <Card.Header>Matches</Card.Header>
      <Card.Body>
        <MatchesFilters initialFilter={initialFilter} onSearch={search} />
        {error && <span>Error: {error}</span>}
        {loading && <span>Loading...</span>}
        <MatchesTable matches={matches} />
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
      </Card.Body>
    </Card>
  );
}

type MatchesFiltersProps = {
  initialFilter?: string;
  onSearch: (filter: string, includingBots: number[]) => void;
};

function MatchesFilters({ initialFilter, onSearch }: MatchesFiltersProps) {
  const [filter, setFilter] = useState(initialFilter ?? "");
  const [withBots, setWithBots] = useState<number[]>([]);

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

type MatchesTableProps = {
  matches: MatchOverviewResponse[];
};

function MatchesTable({ matches }: MatchesTableProps) {
  return (
    <Table>
      <thead>
        <tr>
          <th>ID</th>
          <th>Participants</th>
          <th>Seed</th>
          <th>Actions</th>
        </tr>
      </thead>
      <tbody>
        {matches.map((m) => (
          <Row key={m.id} match={m} />
        ))}
      </tbody>
    </Table>
  );
}

type RowProps = {
  match: MatchOverviewResponse;
};

function Row({ match }: RowProps) {
  return (
    <tr>
      <td>{match.id}</td>
      <td>
        <ParticipantsCell participants={match.participants} />
      </td>
      <td>{match.seed}</td>
      <td>
        <Button variant="outline-warning">Watch replay</Button>
      </td>
    </tr>
  );
}

type ParticipantsCellProps = {
  participants: ParticipantOverviewResponse[];
};

function ParticipantsCell({ participants }: ParticipantsCellProps) {
  return (
    <div className="container">
      {participants.map((p) => (
        <div className="row" key={p.bot_id}>
          <Participant data={p} />
        </div>
      ))}
    </div>
  );
}

type ParticipantProps = {
  data: ParticipantOverviewResponse;
};

function Participant({ data }: ParticipantProps) {
  return (
    <div className="d-flex">{`#${data.rank} - ${data.bot_name} (player ${data.index + 1}) ${data.error ? "err" : ""}`}</div>
  );
}
