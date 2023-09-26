import { Button, Card, Spinner } from "react-bootstrap";

export default function CandidateVsSubmitted() {
    return (
        <Card>
            <Card.Header>
                <Spinner animation="border" role="status" size="sm">
                    <span className="visually-hidden">Matches are running...</span>
                </Spinner>
                &nbsp;Candidate vs Submitted
            </Card.Header>
            <Card.Body>
                <Card.Text>Games: <b>123/200</b></Card.Text>
                <Card.Text>Winrate: <b>65%</b></Card.Text>
            </Card.Body>
        </Card>
    );
}
