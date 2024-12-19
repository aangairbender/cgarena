import {
  BuildResponse,
  LeaderboardBotOverviewResponse,
  rating_score,
} from "@models";
import React from "react";
import { Badge, Button, Stack, Table } from "react-bootstrap";
import { FaTrash } from "react-icons/fa";

interface BotOverviewProps {
  bot: LeaderboardBotOverviewResponse;
  showContentDialog: (data: { title: string; content: string }) => void;
  deleteBot: () => void;
}

const BotOverview: React.FC<BotOverviewProps> = ({
  bot,
  showContentDialog,
  deleteBot,
}) => {
  return (
    <Table bordered hover>
      <thead>
        <tr>
          <th>Name</th>
          <th>Language</th>
          <th>Rating</th>
          <th>Matches played</th>
          <th>Matches with error</th>
          <th>Build</th>
          <th>Actions</th>
        </tr>
      </thead>
      <tbody>
        <tr key={bot.id}>
          <td>{bot.name}</td>
          <td>{bot.language}</td>
          <td>{rating_score(bot)}</td>
          <td>{bot.matches_played}</td>
          <td>{bot.matches_with_error}</td>
          <td>
            <Builds builds={bot.builds} showContentDialog={showContentDialog} />
          </td>
          <td>
            <Stack direction="horizontal">
              <Button variant="outline-danger" size="sm" onClick={deleteBot}>
                <FaTrash />
              </Button>
            </Stack>
          </td>
        </tr>
      </tbody>
    </Table>
  );
};

interface BuildsProps {
  builds: BuildResponse[];
  showContentDialog: (data: { title: string; content: string }) => void;
}

const Builds: React.FC<BuildsProps> = ({ builds, showContentDialog }) => {
  return (
    <Stack>
      {builds.map((build) => (
        <Stack key={build.worker_name} direction="horizontal" gap={1}>
          {buildBadge(build)}
          {build.stderr && (
            <a
              href="#"
              onClick={() =>
                showContentDialog({
                  title: `Build on worker ${build.worker_name}`,
                  content: build.stderr ?? "",
                })
              }
            >
              details
            </a>
          )}
        </Stack>
      ))}
    </Stack>
  );
};

const buildBadge = (build: BuildResponse) => {
  if (build.status == "pending") {
    return <Badge bg="secondary">Pending</Badge>;
  } else if (build.status == "running") {
    return <Badge bg="primary">Running</Badge>;
  } else if (build.status == "finished") {
    if (build.stderr) return <Badge bg="danger">Error</Badge>;
    else return <Badge bg="success">Success</Badge>;
  }
  throw new Error("Unexpected build status");
};

export default BotOverview;
