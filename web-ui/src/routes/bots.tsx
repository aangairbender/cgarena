import { faPencil, faTrash } from "@fortawesome/free-solid-svg-icons";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { Badge, Button, OverlayTrigger, Spinner, Table, Tooltip } from "react-bootstrap";

export default function Bots() {
    const candidateTooltip = (props: any) => (
        <Tooltip id="title-tooltip" {...props}>Candidate</Tooltip>
    );
    const submittedTooltip = (props: any) => (
        <Tooltip id="title-tooltip" {...props}>Submitted</Tooltip>
    );

    return (
        <Table>
            <thead>
                <tr>
                    <th></th>
                    <th>Bot name</th>
                    <th>Language</th>
                    <th>Status</th>
                    <th>Games</th>
                    <th>Rating</th>
                    <th></th>
                </tr>
            </thead>
            <tbody>
                <tr>
                    <td>
                        <OverlayTrigger placement="left" overlay={candidateTooltip}>
                            <Badge bg="success">C</Badge>
                        </OverlayTrigger>
                    </td>
                    <td>
                        Bot 1&nbsp;
                        <FontAwesomeIcon icon={faPencil} size="2xs" />
                    </td>
                    <td>Rust</td>
                    <td>
                        <Badge bg="primary">
                            Building
                        </Badge>
                    </td>
                    <td>123</td>
                    <td>1200 (± 25)</td>
                    <td>
                        <Button variant="danger" size="sm">
                            <FontAwesomeIcon icon={faTrash} />
                        </Button>
                    </td>
                </tr>
                <tr>
                    <td>
                        <OverlayTrigger placement="left" overlay={submittedTooltip}>
                            <Badge bg="warning" text="dark">S</Badge>
                        </OverlayTrigger>
                    </td>
                    <td>Bot 2</td>
                    <td>Cpp</td>
                    <td><Badge bg="danger">Error</Badge></td>
                    <td>40</td>
                    <td>1400 (± 100)</td>
                    <td>
                        <Button variant="danger" size="sm">
                            <FontAwesomeIcon icon={faTrash} />
                        </Button>
                    </td>
                </tr>
                <tr>
                    <td></td>
                    <td>Bot 3</td>
                    <td>Cpp</td>
                    <td><Badge bg="success">Ready</Badge></td>
                    <td>50</td>
                    <td>800 (± 75)</td>
                    <td>
                        <Button variant="danger" size="sm">
                            <FontAwesomeIcon icon={faTrash} />
                        </Button>
                    </td>
                </tr>
            </tbody>
        </Table>
    );
}