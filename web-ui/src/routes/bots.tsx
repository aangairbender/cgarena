import { faTrash } from "@fortawesome/free-solid-svg-icons";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { Button, Table } from "react-bootstrap";

export default function Bots() {
    return (
        <Table striped bordered hover>
            <thead>
                <tr>
                    <th>Bot name</th>
                    <th>Language</th>
                    <th>Games</th>
                    <th>Rating</th>
                    <th style={{width: "auto"}}></th>
                </tr>
            </thead>
            <tbody>
                <tr>
                    <td>Bot 1</td>
                    <td>Rust</td>
                    <td>123</td>
                    <td>1200 (± 25)</td>
                    <td>
                        <Button variant="danger" size="sm">
                            <FontAwesomeIcon icon={faTrash} />
                        </Button>
                    </td>
                </tr>
                <tr>
                    <td>Bot 2</td>
                    <td>Cpp</td>
                    <td>40</td>
                    <td>1400 (± 100)</td>
                    <td>
                        <Button variant="danger" size="sm">
                            <FontAwesomeIcon icon={faTrash} />
                        </Button>
                    </td>
                </tr>
                <tr>
                    <td>Bot 3</td>
                    <td>Python3</td>
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