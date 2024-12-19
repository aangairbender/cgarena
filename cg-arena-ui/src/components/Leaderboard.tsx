import Identicon from "@components/Identicon";
import { useTheme } from "@hooks/useTheme";
import {
  FetchLeaderboardResponse,
  LeaderboardItemResponse,
  rating_score,
} from "@models";
import { OverlayTrigger, Stack, Table, Tooltip } from "react-bootstrap";

interface LeaderboardProps {
  data: FetchLeaderboardResponse;
  selectBot: (botId: string) => void;
}

const Leaderboard = ({ data, selectBot }: LeaderboardProps) => {
  return (
    <Table bordered hover>
      <thead>
        <tr>
          <th style={{ width: "4%" }}>Rank</th>
          <th>Name</th>
          <th style={{ width: "6%" }}>Rating</th>
          <th style={{ width: "15%" }}>Winrate</th>
          <th style={{ width: "15%" }}>Wins / Loses / Draws</th>
          <th style={{ width: "7%" }}>Total</th>
          <th style={{ width: "16%" }}>Submitted</th>
        </tr>
      </thead>
      <tbody>
        {data.items.map((item) => (
          <Row
            key={item.id}
            item={item}
            selected={item.id == data.bot_overview.id}
            select={() => selectBot(item.id)}
          />
        ))}
      </tbody>
    </Table>
  );
};

interface RowProps {
  item: LeaderboardItemResponse;
  selected: boolean;
  select: () => void;
}

const Row = ({ item, selected, select }: RowProps) => {
  return (
    <tr className={selected ? "highlighted-row" : ""}>
      <td>{item.rank}</td>
      <td>
        <Stack direction="horizontal">
          <Identicon input={item.id + ""} size={24} />
          <a href="#" style={{ marginLeft: "8px" }} onClick={select}>
            {item.name}
          </a>
        </Stack>
      </td>
      <RatingCell item={item} />
      {/* <td>{rating_score(item)}</td> */}
      {<WinrateCell item={item} />}
      {selected ? (
        <td></td>
      ) : (
        <td>{`${item.wins} / ${item.loses} / ${item.draws}`}</td>
      )}
      {selected ? <td></td> : <td>{item.wins + item.loses + item.draws}</td>}
      <td>{item.created_at}</td>
    </tr>
  );
};

interface WinrateCellProps {
  item: LeaderboardItemResponse;
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

const WinrateCell = ({ item }: WinrateCellProps) => {
  const { theme } = useTheme();

  if (item.wins + item.loses + item.draws == 0) {
    return <td></td>;
  }

  const wr = Number(
    (
      (100 * (item.wins + item.draws * 0.5)) /
      (item.wins + item.loses + item.draws)
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
