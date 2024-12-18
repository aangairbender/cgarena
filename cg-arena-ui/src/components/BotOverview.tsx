import { LeaderboardBotOverviewResponse, rating_score } from "@models";
import { Table } from "react-bootstrap";

interface BotOverviewProps {
  bot: LeaderboardBotOverviewResponse;
}

const BotOverview: React.FC<BotOverviewProps> = ({ bot }) => {
  return (
    <Table bordered hover>
      <thead>
        <tr>
          <th>Name</th>
          <th>Language</th>
          <th>Rating</th>
          <th>Matches played</th>
          <th>Matches with error</th>
        </tr>
      </thead>
      <tbody>
        <tr key={bot.id}>
          <td>{bot.name}</td>
          <td>{bot.language}</td>
          <td>{rating_score(bot)}</td>
          <td>{bot.matches_played}</td>
          <td>{bot.matches_with_error}</td>
        </tr>
      </tbody>
    </Table>
  );
};

export default BotOverview;
