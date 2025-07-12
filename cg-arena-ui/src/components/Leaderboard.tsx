import Identicon from "@components/Identicon";
import { useTheme } from "@hooks/useTheme";
import {
  LeaderboardItemResponse,
  LeaderboardOverviewResponse,
  rating_score,
} from "@models";
import { OverlayTrigger, Stack, Table, Tooltip } from "react-bootstrap";

interface LeaderboardProps {
  data: LeaderboardOverviewResponse;
  selectedBotId: string | undefined;
  selectBot: (botId: string) => void;
}

const Leaderboard = ({ data, selectedBotId, selectBot }: LeaderboardProps) => {
  return (
    <Table hover className="mb-0">
      <thead>
        <tr>
          <th style={{ width: "4%" }}>Rank</th>
          <th>Name</th>
          <th style={{ width: "6%" }}>Rating</th>
          <th style={{ width: "15%" }}>Winrate</th>
          <th style={{ width: "15%" }}>Wins / Loses / Draws</th>
          <th style={{ width: "7%" }}>Total</th>
        </tr>
      </thead>
      <tbody>
        {data.items.map((item) => {
          const stats: WinrateStats | undefined = data.winrate_stats
            .find(s => s.bot_id == selectedBotId && s.opponent_bot_id == item.id);

          return (<Row
            key={item.id}
            item={item}
            stats={stats}
            selected={item.id == selectedBotId}
            select={() => selectBot(item.id)}
          />);
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
  item: LeaderboardItemResponse;
  stats: WinrateStats | undefined;
  selected: boolean;
  select: () => void;
}

const Row = ({ item, stats, selected, select }: RowProps) => {
  return (
    <tr className={selected ? "highlighted-row" : ""}>
      <td>{item.rank + 1}</td>
      <td>
        <Stack direction="horizontal">
          <Identicon input={item.id + ""} size={24} />
          <a href="#" style={{ marginLeft: "8px" }} onClick={select}>
            {item.name}
          </a>
        </Stack>
      </td>
      <RatingCell item={item} />
      {stats ? <WinrateCell stats={stats} /> : <td></td>}
      {stats ? (
        <td>{`${stats.wins} / ${stats.loses} / ${stats.draws}`}</td>
      ) : (
        <td></td>
      )}
      {stats ? <td>{stats.wins + stats.loses + stats.draws}</td> : <td></td>}
    </tr>
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
      <td>{rating_score(item)}</td>
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
    ).toFixed()
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
