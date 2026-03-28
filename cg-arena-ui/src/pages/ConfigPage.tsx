import { Button, Card } from "react-bootstrap";

export default function ConfigPage() {
  return (
    <Card>
      <Card.Header>Edit config</Card.Header>
      <Card.Body>Coming soon</Card.Body>
      <Card.Footer>
        <div className="d-flex justify-content-between">
          <Button variant="secondary" onClick={() => {}}>
            Cancel
          </Button>
          <Button variant="primary" onClick={() => {}}>
            Save
          </Button>
        </div>
      </Card.Footer>
    </Card>
  );
}
