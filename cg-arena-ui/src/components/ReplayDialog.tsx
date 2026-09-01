import { DialogProps } from "@/hooks/useDialog";
import { MatchId } from "@/models";
import { useEffect, useRef, useState } from "react";
import { Modal } from "react-bootstrap";
import * as api from "@/api";

export interface ReplayDialogData {
  match_id: MatchId;
}
interface ReplayState {
  matchId: MatchId;
  url: string;
  error: string;
}


const ReplayDialog = (dialog: DialogProps<ReplayDialogData>) => {
  const data = dialog.data;
  const [replayState, setReplayState] = useState<ReplayState | null>(null);
  const sessionId = useRef<string | null>(null);
  const startup = useRef<AbortController | null>(null);

  useEffect(() => {
    if (data?.match_id === undefined) return;

    const controller = new AbortController();
    startup.current = controller;
    sessionId.current = null;

    api
      .watchReplay(data.match_id, controller.signal)
      .then(async (replay) => {
        if (controller.signal.aborted) {
          await api.closeReplay(replay.session_id);
          return;
        }
        sessionId.current = replay.session_id;
        setReplayState({
          matchId: data.match_id,
          url: replay.viewer_url,
          error: "",
        });
      })
      .catch((cause: unknown) => {
        if (!controller.signal.aborted) {
          setReplayState({
            matchId: data.match_id,
            url: "",
            error: String(cause),
          });
        }
      });

    return () => {
      controller.abort();
      startup.current = null;
      const currentSession = sessionId.current;
      sessionId.current = null;
      if (currentSession !== null) {
        void api.closeReplay(currentSession).catch(console.error);
      }
    };
  }, [data?.match_id]);

  const onHide = async () => {
    startup.current?.abort();
    const currentSession = sessionId.current;
    if (currentSession !== null) {
      try {
        await api.closeReplay(currentSession);
        sessionId.current = null;
      } catch (cause) {
        if (data !== undefined) {
          setReplayState({
            matchId: data.match_id,
            url: "",
            error: `Failed to close replay: ${String(cause)}`,
          });
        }
        return;
      }
    }
    dialog.hide();
  };

  if (data === undefined) return null;
  const currentReplay =
    replayState?.matchId === data.match_id ? replayState : null;

  return (
    <Modal show={dialog.isOpen} onHide={() => void onHide()} fullscreen>
      <Modal.Header closeButton>
        <Modal.Title>Replay</Modal.Title>
      </Modal.Header>
      <Modal.Body className="p-0">
        {currentReplay?.error ? (
          <span>Error: {currentReplay.error}</span>
        ) : currentReplay?.url ? (
          <iframe
            title="Match replay"
            width="100%"
            height="100%"
            src={currentReplay.url}
          />
        ) : (
          <span>Loading</span>
        )}
      </Modal.Body>
    </Modal>
  );
};

export default ReplayDialog;
