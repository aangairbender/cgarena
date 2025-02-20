import { useState } from "react";
import { RenameBotRequest } from "@models";
import { Alert, Button, Form, Modal } from "react-bootstrap";
import { DialogProps } from "@hooks/useDialog";

interface Data {
  botId: string;
  currentName: string;
  onSubmit: (id: string, req: RenameBotRequest) => Promise<void>;
}

const RenameBotDialog = (dialog: DialogProps<Data>) => {
  const [name, setName] = useState("");
  const [error, setError] = useState("");

  const canSubmit = name.length > 0 && name != dialog.data.currentName;

  const closeDialog = () => {
    setName("");
    setError("");
    dialog.hide();
  };

  const handleSubmit = async () => {
    const req: RenameBotRequest = { name };
    try {
      await dialog.data.onSubmit(dialog.data.botId, req);
      closeDialog();
    } catch (e) {
      if (e instanceof Error) {
        setError(e.message);
      } else {
        setError(String(e));
      }
    }
  };

  return (
    <Modal show={dialog.isOpen} onHide={closeDialog} centered>
      <Modal.Header closeButton>
        <Modal.Title>Rename bot</Modal.Title>
      </Modal.Header>
      <Modal.Body>
        <Form.Group controlId="formName" className="mb-3">
          <Form.Label>Name</Form.Label>
          <Form.Control
            placeholder="Bot's name"
            defaultValue={dialog.data.currentName}
            onChange={(e) => setName(e.target.value)}
          />
          <Form.Text className="text-muted">
            Non-empty string up to 32 characters long.
          </Form.Text>
        </Form.Group>

        {error && <Alert variant="danger">{error}</Alert>}
      </Modal.Body>
      <Modal.Footer>
        <Button variant="secondary" onClick={closeDialog}>
          Cancel
        </Button>
        <Button variant="primary" onClick={handleSubmit} disabled={!canSubmit}>
          Rename
        </Button>
      </Modal.Footer>
    </Modal>
  );
};

export default RenameBotDialog;
