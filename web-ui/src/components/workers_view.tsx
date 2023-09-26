import { Badge, Card, Table } from "react-bootstrap";

export default function WorkersView() {
    return (
        <Card>
            <Card.Header>
                Worker view
            </Card.Header>
            <Card.Body>
                <Table>
                    <thead>
                        <tr>
                            <th>Name</th>
                            <th>Status</th>
                            <th>Queue</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr>
                            <td>Embedded</td>
                            <td><Badge bg="success">OK</Badge></td>
                            <td>90</td>
                        </tr>
                        <tr>
                            <td>Laptop</td>
                            <td><Badge bg="success">OK</Badge></td>
                            <td>65</td>
                        </tr>
                        <tr>
                            <td>AWS 1</td>
                            <td><Badge bg="danger">Down</Badge></td>
                            <td>0</td>
                        </tr>
                    </tbody>
                </Table>
            </Card.Body>
        </Card>
    );
}
