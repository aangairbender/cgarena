import { Card, Col, Container, Nav, Navbar, Row } from "react-bootstrap";
import { Outlet } from "react-router-dom";

export default function Root() {
    return (
        <>
            <Navbar className="bg-body-tertiary mb-3">
                <Container>
                    <Navbar.Brand href="#">CG Arena</Navbar.Brand>
                    <Nav className="me-auto">
                        <Nav.Link href="/bots">Bots</Nav.Link>
                        <Nav.Link href="/matches">Matches</Nav.Link>
                        <Nav.Link href="/jobs">Jobs</Nav.Link>
                    </Nav> 
                </Container>
            </Navbar> 
            <Container>
                <Row>
                    <Col xs={3}>
                        <Card>Some card</Card>
                    </Col>
                    <Col xs={9}>
                        <Outlet />
                    </Col>
                </Row>
            </Container>
        </>
    );
}
