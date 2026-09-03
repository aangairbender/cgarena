import { MatchCard } from "@/components/MatchCard";
import { useAppStore } from "@/hooks/useAppStore";
import {
  matchBrowsing,
  MatchBrowseSearch,
  MatchOverview,
} from "@/match-browsing";
import { BotId, BotOverviewResponse } from "@/models";
import { getRouteApi, useNavigate } from "@tanstack/react-router";
import { FormEvent, useCallback, useState } from "react";
import {
  Alert,
  Badge,
  Button,
  Card,
  Form,
  Pagination,
  Spinner,
} from "react-bootstrap";
import { FaXmark } from "react-icons/fa6";

const routeApi = getRouteApi("/matches");

const PAGE_SIZE_OPTIONS = [10, 25, 50, 100];

export default function MatchesPage() {
  const routeSearch = routeApi.useSearch();
  const navigate = useNavigate();
  const navigateMatches = useCallback(
    (search: MatchBrowseSearch) => {
      void navigate({ to: "/matches", search });
    },
    [navigate],
  );
  const browsing = matchBrowsing.useMatches(routeSearch, navigateMatches);
  const { filter, withBots, page, pageSize } = browsing.search;
  const bots = useAppStore((state) => state.bots);

  const handleSearch = (
    nextFilter: string,
    nextWithBots: BotId[],
    nextPageSize: number,
  ) => {
    browsing.submit({
      filter: nextFilter,
      withBots: nextWithBots,
      pageSize: nextPageSize,
    });
  };

  return (
    <div className="d-flex flex-column gap-3">
      <Card>
        <Card.Header>Matches</Card.Header>
        <Card.Body className="d-flex flex-column gap-3">
          <MatchesFilters
            key={`${filter}\u0000${withBots.join(",")}\u0000${pageSize}`}
            initialFilter={filter}
            initialWithBots={withBots}
            initialPageSize={pageSize}
            bots={bots}
            onSearch={handleSearch}
          />

          {browsing.isFetching && (
            <div
              className="d-flex align-items-center gap-2 text-body-secondary"
              role="status"
            >
              <Spinner animation="border" size="sm" />
              <span>Searching matches...</span>
            </div>
          )}

          {browsing.errorMessage !== undefined && (
            <Alert variant="danger" className="mb-0">
              <Alert.Heading>Could not search matches</Alert.Heading>
              <p>{browsing.errorMessage}</p>
              <p className="mb-0">
                Fix the filter or required bots, then search again.
                {browsing.showingRetainedResults &&
                  " Your last successful results are still shown below."}
              </p>
            </Alert>
          )}
        </Card.Body>
      </Card>

      {browsing.results !== undefined && (
        <MatchList matches={browsing.results.matches} />
      )}

      <div className="d-flex justify-content-center">
        <Pagination className="mb-0">
          <Pagination.Prev
            disabled={!browsing.canGoPrevious}
            onClick={() => browsing.goToPage(page - 1)}
          >
            Prev
          </Pagination.Prev>
          <Pagination.Item active disabled>
            {page}
          </Pagination.Item>
          <Pagination.Next
            disabled={!browsing.canGoNext}
            onClick={() => browsing.goToPage(page + 1)}
          >
            Next
          </Pagination.Next>
        </Pagination>
      </div>
    </div>
  );
}

type MatchesFiltersProps = {
  initialFilter: string;
  initialWithBots: BotId[];
  initialPageSize: number;
  bots: BotOverviewResponse[];
  onSearch: (filter: string, includingBots: BotId[], pageSize: number) => void;
};

function MatchesFilters({
  initialFilter,
  initialWithBots,
  initialPageSize,
  bots,
  onSearch,
}: MatchesFiltersProps) {
  const [filter, setFilter] = useState(initialFilter);
  const [withBots, setWithBots] = useState(initialWithBots);
  const [pageSize, setPageSize] = useState(initialPageSize);

  const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    onSearch(filter, withBots, pageSize);
  };

  const removeRequiredBot = (botId: BotId) => {
    setWithBots((current) => current.filter((id) => id !== botId));
  };

  return (
    <Form onSubmit={handleSubmit}>
      <Form.Group className="mb-3">
        <Form.Label>Required bots</Form.Label>
        {withBots.length > 0 ? (
          <div className="d-flex flex-wrap gap-2" aria-label="Required bots">
            {withBots.map((botId, index) => {
              const botName = bots.find((bot) => bot.id === botId)?.name;
              const label = botName
                ? `${botName} (#${botId})`
                : `Bot #${botId}`;

              return (
                <Badge
                  key={`${botId}-${index}`}
                  bg="secondary"
                  className="d-inline-flex align-items-center gap-2 px-3 py-2"
                >
                  <span>{label}</span>
                  <Button
                    type="button"
                    variant="link"
                    className="p-0 text-reset lh-1"
                    aria-label={`Remove ${label} from required bots`}
                    title={`Remove ${label}`}
                    onClick={() => removeRequiredBot(botId)}
                  >
                    <FaXmark className="bi" />
                  </Button>
                </Badge>
              );
            })}
          </div>
        ) : (
          <Form.Text className="d-block text-muted">
            No bots are required. Matches with any participants can be shown.
          </Form.Text>
        )}
      </Form.Group>

      <div className="d-flex flex-column flex-sm-row align-items-sm-end gap-3">
        <Form.Group controlId="matchFilter" className="flex-grow-1">
          <Form.Label>Match filter</Form.Label>
          <Form.Control
            value={filter}
            onChange={(event) => setFilter(event.target.value)}
          />
          <Form.Text className="text-muted">
            e.g. match.player_count == 2
          </Form.Text>
        </Form.Group>

        <Form.Group controlId="matchesPageSize">
          <Form.Label>Per page</Form.Label>
          <Form.Select
            className="w-auto"
            value={pageSize}
            onChange={(event) => setPageSize(Number(event.target.value))}
          >
            {!PAGE_SIZE_OPTIONS.includes(pageSize) && (
              <option value={pageSize}>{pageSize}</option>
            )}
            {PAGE_SIZE_OPTIONS.map((option) => (
              <option key={option} value={option}>
                {option}
              </option>
            ))}
          </Form.Select>
        </Form.Group>

        <Button type="submit" variant="primary">
          Search
        </Button>
      </div>
    </Form>
  );
}

interface MatchListProps {
  matches: MatchOverview[];
}

export function MatchList({ matches }: MatchListProps) {
  if (matches.length === 0) {
    return (
      <div className="border rounded p-5 text-center">
        <p className="mb-1 fw-semibold">No matches match these criteria.</p>
        <p className="mb-0 text-body-secondary">
          Adjust the filter or remove a required bot, then search again.
        </p>
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
