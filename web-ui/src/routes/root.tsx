import { faCircle } from "@fortawesome/free-solid-svg-icons";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { Col, Container, Nav, Navbar, Row } from "react-bootstrap";
import { Outlet } from "react-router-dom";
import CandidateVsSubmitted from "../components/candidate_vs_submitted";
import WorkersView from "../components/workers_view";

export default function Root() {
    return (
        <>
            <Navbar className="bg-body-tertiary mb-3">
                <Container>
                    <Navbar.Brand href="#">
                        <img
                            alt=""
                            src="/logo.png"
                            width="32"
                            height="32"
                            className="d-inline-block align-top"
                        />{' '}
                        CG Arena
                    </Navbar.Brand>
                    <Nav className="me-auto">
                        <Nav.Link href="/bots/add">Add bot</Nav.Link>
                        <Nav.Link href="/bots">Bots</Nav.Link>
                        <Nav.Link href="/matches">Matches</Nav.Link>
                        <Nav.Link href="/jobs">Jobs</Nav.Link>
                    </Nav>
                    <Navbar.Text>
                        Connected&nbsp;
                        <FontAwesomeIcon icon={faCircle} size="xs" style={{color: "#0f0"}}/>
                    </Navbar.Text>
                </Container>
            </Navbar> 
            <Container>
                <Row>
                    <Col xs={9}>
                        <Outlet />
                    </Col>
                    <Col xs={3}>
                        <CandidateVsSubmitted />
                        <div className="mb-3" />
                        <WorkersView />
                    </Col>
                </Row>
            </Container>
        </>
    );
}
