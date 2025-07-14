import { DialogProps } from "@hooks/useDialog";
import { useState } from "react";
import { Button, Modal, Table } from "react-bootstrap";
import { FaCopy } from "react-icons/fa6";

export interface ExampleSeedsDialogData {
  example_seeds: number[];
}

const ExampleSeedsDialog = (dialog: DialogProps<ExampleSeedsDialogData>) => {
  const [copied, setCopied] = useState<number>();

  const data = dialog.data;
  if (data === undefined) return null;

  const copy = async (seed: number) => {
    await copyToClipboard(seed);
    setCopied(seed);
    setTimeout(() => {
      setCopied(c => c == seed ? undefined : c);
    }, 3000)
  };

  return (
    <Modal show={dialog.isOpen} onHide={dialog.hide} centered>
      <Modal.Header closeButton>
        <Modal.Title>Example seeds</Modal.Title>
      </Modal.Header>
      <Modal.Body>
        <Table hover>
          <thead>
            <tr>
              <th>#</th>
              <th>Seed</th>
              <th style={{ width: "80px" }}></th>
            </tr>
          </thead>
          <tbody>
            {data.example_seeds.map((seed, index) => (
              <tr key={index}>
                <td>{index + 1}</td>
                <td>{seed}</td>
                <td style={{ textAlign: "right" }}>
                  <Button
                    variant="outline-secondary"
                    size="sm"
                    onClick={() => copy(seed)}
                  >
                    {copied === seed ? "Copied" : <FaCopy className="bi" />}
                  </Button>
                </td>
              </tr>
            ))}
          </tbody>
        </Table>
      </Modal.Body>
      <Modal.Footer>
        <Button variant="secondary" onClick={dialog.hide}>
          Close
        </Button>
      </Modal.Footer>
    </Modal>
  );
};

const copyToClipboard = async (seed: number) => {
  try {
    await navigator.clipboard.writeText(seed.toString());
  } catch (err) {
    console.error("Failed to copy text: " + err);
  }
};

export default ExampleSeedsDialog;
