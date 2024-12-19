import { useState } from "react";
import { CreateBotRequest } from "@models";
import { Alert, Button, Form, Modal } from "react-bootstrap";
import { DialogProps } from "@hooks/useDialog";

interface Data {
  onSubmit: (req: CreateBotRequest) => Promise<void>;
}

const SubmitBotDialog = (dialog: DialogProps<Data>) => {
  const [name, setName] = useState("");
  const [language, setLanguage] = useState("");
  const [sourceCode, setSourceCode] = useState("");
  const [error, setError] = useState("");

  const canSubmit = name.length > 0 && sourceCode.length > 0 && language.length > 0;

  const closeDialog = () => {
    setName("");
    setLanguage("");
    setSourceCode("");
    setError("");
    dialog.hide();
  };

  const handleSubmit = async () => {
    const req: CreateBotRequest = {
      name,
      language,
      source_code: sourceCode,
    };
    try {
      await dialog.data.onSubmit(req);
      closeDialog();
    } catch (e) {
      if (e instanceof Error) {
        setError(e.message);
      } else {
        setError(String(e));
      }
    }
  };

  const handleSourceFileChanged = (
    e: React.ChangeEvent<HTMLInputElement>
  ): void => {
    const files = e.target.files ? Array.from(e.target.files) : [];
    if (files.length == 0) {
      setSourceCode("");
      return;
    }
    files[0].text().then((content) => setSourceCode(content));
  };

  return (
    <Modal show={dialog.isOpen} onHide={closeDialog} centered>
      <Modal.Header closeButton>
        <Modal.Title>Submit a new bot</Modal.Title>
      </Modal.Header>
      <Modal.Body>
        <Form.Group controlId="formName" className="mb-3">
          <Form.Label>Name</Form.Label>
          <Form.Control
            placeholder="Bot's name"
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
          <Form.Text className="text-muted">
            Non-empty string up to 32 characters long.
          </Form.Text>
        </Form.Group>

        <Form.Group controlId="formFile" className="mb-3">
          <Form.Label>Default file input example</Form.Label>
          <Form.Control type="file" onChange={handleSourceFileChanged} />
          <Form.Text className="text-muted">
            Up to 100k characters, same as CG.
          </Form.Text>
        </Form.Group>

        <Form.Group controlId="formLanguage" className="mb-3">
          <Form.Label>Bot's language</Form.Label>
          <Form.Control
            placeholder="Bot's language"
            value={language}
            onChange={(e) => setLanguage(e.target.value)}
          />
          <Form.Text className="text-muted">
            e.g. "c++", "rust", "python", etc. This value would be passed to
            worker "cmd_*" commands as-is.
          </Form.Text>
        </Form.Group>

        {error && <Alert variant="danger">{error}</Alert>}
      </Modal.Body>
      <Modal.Footer>
        <Button variant="secondary" onClick={closeDialog}>
          Cancel
        </Button>
        <Button variant="primary" onClick={handleSubmit} disabled={!canSubmit}>
          Submit
        </Button>
      </Modal.Footer>
    </Modal>
  );
};

export default SubmitBotDialog;
