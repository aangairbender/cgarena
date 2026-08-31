import { DialogProps } from "@/hooks/useDialog";
import { MatchId } from "@/models";
import { useEffect, useState } from "react";
import { Modal } from "react-bootstrap";
import * as api from "@/api";

export interface ReplayDialogData {
  match_id: MatchId;
}

const ReplayDialog = (dialog: DialogProps<ReplayDialogData>) => {
  const data = dialog.data;

  const [url, setUrl] = useState<string>("");
  const [error, setError] = useState<string>("");

  useEffect(() => {
    if (data?.match_id === undefined) return;

    api
      .watchReplay(data.match_id)
      .then((d) => setUrl(d.viewer_url))
      .catch((e) => setError(String(e)));
  }, [data]);

  const onHide = () => {
    if (data?.match_id) {
      api.closeReplay(data?.match_id);
    }
    dialog.hide();
  };

  if (data === undefined) return null;

  return (
    <Modal show={dialog.isOpen} onHide={onHide} fullscreen>
      <Modal.Header closeButton>
        <Modal.Title>Replay</Modal.Title>
      </Modal.Header>
      <Modal.Body className="p-0">
        {error && <span>Error: {error}</span>}
        {url ? (
          <iframe width="100%" height="100%" src={url}></iframe>
        ) : (
          <span>Loading</span>
        )}
      </Modal.Body>
    </Modal>
  );
};

export default ReplayDialog;
