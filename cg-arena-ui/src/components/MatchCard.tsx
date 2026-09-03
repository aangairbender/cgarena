import { MatchOverview } from "@/match-browsing";
import { FaHashtag, FaTrophy } from "react-icons/fa6";
import { cn } from "@/lib/utils";
import { LuCircleAlert } from "react-icons/lu";
import { Badge, Button, Card, Table } from "react-bootstrap";
import Identicon from "./Identicon";
import { useDialogs } from "@/hooks/useDialogs";

interface MatchCardProps {
  match: MatchOverview;
}

const excludedAttrs = ["score", "index", "rank", "error"];

function trophyColorByRank(rank: number): string {
  switch (rank) {
    case 0:
      return "text-cg-gold";
    case 1:
      return "text-cg-silver";
    default:
      return "text-cg-bronze";
  }
}

export function MatchCard({ match }: MatchCardProps) {
  const { replayDialog } = useDialogs();

  const hasErrors = match.participants.some((p) => p.error);
  const sortedParticipants = [...match.participants].sort(
    (a, b) => a.index - b.index,
  );

  // Separate match-level and participant-level attributes
  const matchAttributes = match.attributes.filter(
    (attr) => attr.bot_id === null,
  );
  const participantAttributes = match.attributes.filter(
    (attr) => attr.bot_id !== null,
  );

  // Get unique attribute names for participant columns (excluding Score which is shown prominently)
  const participantAttrNames = [
    ...new Set(
      participantAttributes
        .filter((attr) => !excludedAttrs.includes(attr.name))
        .map((attr) => attr.name),
    ),
  ];

  // Helper to get attribute value for a specific participant
  const getParticipantAttr = (botId: number, attrName: string) => {
    return participantAttributes.find(
      (attr) => attr.bot_id === botId && attr.name === attrName,
    )?.value;
  };

  const getParticipantScore = (botId: number) => {
    return participantAttributes.find(
      (attr) => attr.bot_id === botId && attr.name.toLowerCase() === "score",
    )?.value;
  };

  return (
    <Card
      className={cn("overflow-hidden", hasErrors && "border-destructive/30")}
    >
      {/* Header with Match ID, Seed, and Match-level attributes */}
      <Card.Header>
        <div className="flex justify-between">
          <div className="flex flex-wrap items-center gap-x-4 gap-y-1">
            <div className="flex items-center gap-1.5">
              <FaHashtag className="h-4 w-4 text-muted-foreground" />
              <span className="font-mono text-sm font-semibold text-foreground">
                {match.id}
              </span>
            </div>
            {matchAttributes.map((attr, idx) => (
              <div
                key={idx}
                className="flex items-center gap-1 text-xs text-muted-foreground"
              >
                <div className="text-base">
                  <Badge bg="secondary">
                    {attr.name}: {attr.value}
                  </Badge>
                </div>
              </div>
            ))}
          </div>
          <div className="flex items-center gap-1.5">
            <Button
              variant="outline-warning"
              size="sm"
              onClick={() => replayDialog.show({ match_id: match.id })}
            >
              Watch replay
            </Button>
          </div>
        </div>
      </Card.Header>

      <Table hover className="mb-0">
        <thead>
          <tr className="border-t bg-muted/30 text-xs uppercase tracking-wider">
            <th
              style={{ width: "4%" }}
              className="px-4 py-2 text-left font-medium text-muted"
            >
              Rank
            </th>
            <th className="px-4 py-2 text-left font-medium text-muted">
              Participant
            </th>
            <th
              style={{ width: "4%" }}
              className="px-4 py-2 text-right font-medium text-muted"
            >
              Score
            </th>
            {participantAttrNames.map((name) => (
              <th
                style={{ width: "4%" }}
                key={name}
                className="px-4 py-2 text-right font-medium text-muted"
              >
                {name}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {sortedParticipants.map((participant) => {
            const score = getParticipantScore(participant.bot_id);
            const trophyColor = trophyColorByRank(participant.rank);

            return (
              <tr
                key={participant.bot_id}
                className={cn("border-t transition-colors")}
              >
                {/* Rank */}
                <td className="px-4">
                  <div className="flex items-center gap-1.5">
                    <FaTrophy className={cn(trophyColor)} />
                    {participant.rank + 1}
                  </div>
                </td>

                {/* Participant Name */}
                <td className="px-4">
                  <div className="flex items-center gap-2">
                    <Identicon input={participant.bot_id + ""} size={24} />
                    <span>{participant.bot_name}</span>
                    {participant.error && (
                      <LuCircleAlert className="h-4 w-4 text-red-400" />
                    )}
                  </div>
                </td>

                {/* Score */}
                <td className="px-4 text-right">
                  <span className={cn("font-mono text-sm font-semibold")}>
                    {score ?? "—"}
                  </span>
                </td>

                {/* Dynamic Participant Attributes */}
                {participantAttrNames.map((attrName) => {
                  const value = getParticipantAttr(
                    participant.bot_id,
                    attrName,
                  );
                  return (
                    <td key={attrName} className="px-4 text-right">
                      <span
                        className={cn(
                          "font-mono text-sm",
                          "text-muted-foreground",
                        )}
                      >
                        {value?.toString() ?? "—"}
                      </span>
                    </td>
                  );
                })}
              </tr>
            );
          })}
        </tbody>
      </Table>
    </Card>
  );
}
