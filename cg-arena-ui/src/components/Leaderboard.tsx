import Identicon from "@/components/Identicon";
import { useTheme } from "@/hooks/useTheme";
import {
  BotId,
  BotOverviewResponse,
  GLOBAL_LEADERBOARD_ID,
  LeaderboardItemResponse,
  LeaderboardOverviewResponse,
  PatchLeaderboardRequest,
} from "@/models";
import {
  Button,
  Card,
  OverlayTrigger,
  Spinner,
  Stack,
  Table,
  Tooltip,
} from "react-bootstrap";
import {
  FaChartLine,
  FaSeedling,
  FaPencil,
  FaTrash,
  FaCaretDown,
  FaCaretRight,
} from "react-icons/fa6";
import { useState } from "react";
import { useDialogs } from "@/hooks/useDialogs";
import { Link } from "@tanstack/react-router";
import { matchBrowsing } from "@/match-browsing";

interface LeaderboardProps {
  lb: LeaderboardOverviewResponse;
  bots: BotOverviewResponse[];
  selectedBotId: number | undefined;
  patchLeaderboard: (id: number, req: PatchLeaderboardRequest) => Promise<void>;
  deleteLeaderboard: (leaderboardId: number) => Promise<void>;
}

const Leaderboard = ({
  lb,
  bots,
  selectedBotId,
  patchLeaderboard,
  deleteLeaderboard,
}: LeaderboardProps) => {
  const {
    chartDialog,
    exampleSeedsDialog,
    patchLeaderboardDialog,
    confirmDialog,
  } = useDialogs();
  const [expanded, setExpanded] = useState(lb.id == GLOBAL_LEADERBOARD_ID);
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const renderTooltip = (props: any) => (
    <Tooltip id={`lb-${lb.id}-tooltip`} {...props}>
      <div>
        <div>{`Matches: ${lb.total_matches}`}</div>
        {lb.id != GLOBAL_LEADERBOARD_ID && <div>{`Filter: ${lb.filter}`}</div>}
      </div>
    </Tooltip>
  );

  const headerStyle = expanded
    ? {}
    : { borderBottom: 0, borderRadius: "var(--bs-card-inner-border-radius)" };

  return (
    <Card key={lb.id}>
      <Card.Header
        className="d-flex justify-content-between"
        style={headerStyle}
      >
        <div className="d-flex align-items-center gap-2">
          <Button
            variant="link"
            size="lg"
            className="p-0 text-body text-decoration-none"
            onClick={() => setExpanded((s) => !s)}
          >
            {expanded ? (
              <FaCaretDown className="bi" />
            ) : (
              <FaCaretRight className="bi" />
            )}
          </Button>
          <OverlayTrigger overlay={renderTooltip} placement="right">
            <div>{lb.name}</div>
          </OverlayTrigger>
        </div>
        <div className="d-flex gap-2">
          <Button
            variant="outline-info"
            size="sm"
            onClick={() => chartDialog.show({ filter: lb.filter, bots })}
          >
            <FaChartLine className="bi" />
          </Button>

          {lb.id != GLOBAL_LEADERBOARD_ID && (
            <>
              <Button
                variant="outline-info"
                size="sm"
                onClick={() =>
                  exampleSeedsDialog.show({ example_seeds: lb.example_seeds })
                }
              >
                <FaSeedling className="bi" />
              </Button>

              <Button
                variant="outline-warning"
                size="sm"
                onClick={() =>
                  patchLeaderboardDialog.show({
                    leaderboard: lb,
                    onSubmit: patchLeaderboard,
                  })
                }
              >
                <FaPencil className="bi" />
              </Button>

              <Button
                variant="outline-danger"
                size="sm"
                onClick={() =>
                  confirmDialog.show({
                    prompt: `Do you really want to delete leaderboard '${lb.name}'?`,
                    action: () => deleteLeaderboard(lb.id),
                  })
                }
              >
                <FaTrash className="bi" />
              </Button>
            </>
          )}
        </div>
      </Card.Header>
      {expanded && (
        <Card.Body>
          <LeaderboardTable
            bots={bots}
            data={lb}
            selectedBotId={selectedBotId}
          />
        </Card.Body>
      )}
    </Card>
  );
};

interface LeaderboardTableProps {
  bots: BotOverviewResponse[];
  data: LeaderboardOverviewResponse;
  selectedBotId: BotId | undefined;
}

const LeaderboardTable = ({
  bots,
  data,
  selectedBotId,
}: LeaderboardTableProps) => {
  if (data.status === "computing") {
    return (
      <Stack direction="horizontal">
        <Spinner animation="border" />
        <div className="ms-3">Building the leaderboard</div>
      </Stack>
    );
  }

  return (
    <Table hover className="mb-0">
      <thead>
        <tr className="uppercase text-xs font-medium tracking-wider bg-muted/30">
          <th style={{ width: "4%" }} className="text-muted">
            Rank
          </th>
          <th className="text-muted">Name</th>
          <th style={{ width: "6%" }} className="text-muted">
            Rating
          </th>
          <th style={{ width: "15%" }} className="text-muted">
            Winrate
          </th>
          <th style={{ width: "12%" }} className="text-muted">
            W / L / D
          </th>
          <th style={{ width: "7%" }} className="text-muted">
            Total
          </th>
        </tr>
      </thead>
      <tbody>
        {data.items.map((item) => {
          const stats: WinrateStats | undefined = data.winrate_stats.find(
            (s) => s.bot_id == selectedBotId && s.opponent_bot_id == item.id,
          );

          return (
            <Row
              lb={data}
              key={item.id}
              item={item}
              bot={bots.find((b) => b.id == item.id)}
              stats={stats}
              selectedBotId={selectedBotId}
            />
          );
        })}
      </tbody>
    </Table>
  );
};

interface WinrateStats {
  wins: number;
  draws: number;
  loses: number;
}

interface RowProps {
  lb: LeaderboardOverviewResponse;
  item: LeaderboardItemResponse;
  stats: WinrateStats | undefined;
  bot: BotOverviewResponse | undefined;
  selectedBotId: number | undefined;
}

const Row = ({ lb, item, stats, bot, selectedBotId }: RowProps) => {
  if (!bot) {
    return null;
  }

  const selected = selectedBotId === bot.id;

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const renderTooltip = (props: any) => (
    <Tooltip id={`lb-${lb.id}-bot-${bot.id}-tooltip`} {...props}>
      <div>{`id: ${bot.id}`}</div>
    </Tooltip>
  );

  return (
    <tr className={selected ? "highlighted-row" : ""}>
      <td>{item.rank + 1}</td>
      <td>
        <Stack direction="horizontal">
          <Identicon input={item.id + ""} size={24} />
          <OverlayTrigger overlay={renderTooltip} placement="right">
            <Link
              to="/"
              style={{ marginLeft: "8px" }}
              search={(prev) => ({ ...prev, selectedBotId: bot.id })}
            >
              {bot.name}
            </Link>
          </OverlayTrigger>
        </Stack>
      </td>
      <RatingCell item={item} />
      {stats ? <WinrateCell stats={stats} /> : <td></td>}
      {stats && selectedBotId !== undefined ? (
        <HeadToHeadLinks
          leaderboardFilter={lb.filter}
          selectedBotId={selectedBotId}
          opponentBotId={item.id}
          stats={stats}
        />
      ) : (
        <>
          <td></td>
          <td></td>
        </>
      )}
    </tr>
  );
};

interface HeadToHeadLinksProps {
  leaderboardFilter: string;
  selectedBotId: BotId;
  opponentBotId: BotId;
  stats: WinrateStats;
}

const HeadToHeadLinks = ({
  leaderboardFilter,
  selectedBotId,
  opponentBotId,
  stats,
}: HeadToHeadLinksProps) => {
  const search = (result: "win" | "loss" | "draw" | "all") =>
    matchBrowsing.leaderboardSearch({
      leaderboardFilter,
      selectedBotId,
      opponentBotId,
      result,
    });

  return (
    <>
      <td>
        <Link to="/matches" search={search("win")}>
          {stats.wins}
        </Link>
        {" / "}
        <Link to="/matches" search={search("loss")}>
          {stats.loses}
        </Link>
        {" / "}
        <Link to="/matches" search={search("draw")}>
          {stats.draws}
        </Link>
      </td>
      <td>
        <Link to="/matches" search={search("all")}>
          {stats.wins + stats.loses + stats.draws}
        </Link>
      </td>
    </>
  );
};

interface WinrateCellProps {
  stats: WinrateStats;
}

const RatingCell = ({ item }: { item: LeaderboardItemResponse }) => {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const renderTooltip = (props: any) => (
    <Tooltip
      id={`bot-${item.id}-tooltip`}
      {...props}
    >{`mu: ${item.rating_mu.toFixed(2)} sigma: ${item.rating_sigma.toFixed(2)}`}</Tooltip>
  );

  return (
    <OverlayTrigger overlay={renderTooltip} placement="left">
      <td>{item.rating.toFixed(2)}</td>
    </OverlayTrigger>
  );
};

const WinrateCell = ({ stats }: WinrateCellProps) => {
  const { theme } = useTheme();

  if (stats.wins + stats.loses + stats.draws == 0) {
    return <td></td>;
  }

  const wr = Number(
    (
      (100 * (stats.wins + stats.draws * 0.5)) /
      (stats.wins + stats.loses + stats.draws)
    ).toFixed(),
  );

  const green = theme == "light" ? "#dff0d8" : "#3d6c2a";
  const red = theme == "light" ? "#f2dede" : "#712d2d";

  const background =
    wr > 50
      ? `linear-gradient(to right, transparent 0%, transparent 49%, ${green} 50%, ${green} ${wr}%, transparent ${
          wr + 1
        }%)`
      : `linear-gradient(to right, transparent 0%, transparent ${wr}%, ${red} ${
          wr + 1
        }%, ${red} 50%, transparent 51%)`;
  return <td style={{ background }}>{`${wr}%`}</td>;
};

export default Leaderboard;
