CREATE TRIGGER delete_match_after_participation_deleted
    AFTER DELETE
    ON participations
BEGIN
    DELETE FROM matches WHERE id = old.match_id;
END;