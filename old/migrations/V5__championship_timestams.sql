CREATE OR REPLACE FUNCTION update_timestamp_championship()
RETURNS TRIGGER AS '
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
' LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_timestamp_championship
BEFORE UPDATE ON championship
FOR EACH ROW
EXECUTE FUNCTION update_timestamp_championship(); -- Corrected function name
