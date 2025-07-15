import {
  BuildResponse,
  BotOverviewResponse,
} from "@models";
import React from "react";
import { Badge, Button, Stack, Table } from "react-bootstrap";
import { FaCode, FaPencil, FaTrash } from "react-icons/fa6";
import { ViewCodeDialogData } from "./ViewCodeDialog";
import { fetchBotSourceCode } from "@api";

interface BotOverviewProps {
  bot: BotOverviewResponse;
  showCodeDialog: (data: ViewCodeDialogData) => void;
  deleteBot: () => void;
  renameBot: () => void;
}

const BotOverview: React.FC<BotOverviewProps> = ({
  bot,
  showCodeDialog,
  deleteBot,
  renameBot,
}) => {
  const showSourceCode = async () => {
    const data = await fetchBotSourceCode(bot.id);
    showCodeDialog({
      title: `Source code of ${bot.name}`,
      content: data.source_code,
    });
  };

  return (
    <Table hover className="mb-0">
      <thead>
        <tr>
          <th>Id</th>
          <th>Name</th>
          <th>Language</th>
          <th>Matches played</th>
          <th>Matches with error</th>
          <th>Build</th>
          <th>Submitted</th>
          <th>Actions</th>
        </tr>
      </thead>
      <tbody>
        <tr key={bot.id}>
          <td>{bot.id}</td>
          <td>{bot.name}</td>
          <td>{bot.language}</td>
          <td>{bot.matches_played}</td>
          <td>{bot.matches_with_error}</td>
          <td>
            <Builds builds={bot.builds} showCodeDialog={showCodeDialog} />
          </td>
          <td>{bot.created_at}</td>
          <td>
            <Stack direction="horizontal" gap={2}>
              <Button variant="outline-info" size="sm" onClick={showSourceCode}>
                <FaCode className="bi"/>
              </Button>
              <Button variant="outline-warning" size="sm" onClick={renameBot}>
                <FaPencil className="bi"/>
              </Button>
              <Button variant="outline-danger" size="sm" onClick={deleteBot}>
                <FaTrash className="bi"/>
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
  showCodeDialog: (data: ViewCodeDialogData) => void;
}

const Builds: React.FC<BuildsProps> = ({ builds, showCodeDialog }) => {
  return (
    <Stack>
      {builds.map((build) => (
        <Stack key={build.worker_name} direction="horizontal" gap={1}>
          {buildBadge(build)}
          {build.stderr && (
            <a
              href="#"
              onClick={() =>
                showCodeDialog({
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
