-- Auto-generated rollup table for ChartEvent
CREATE TABLE core_chart_events_rollup (
  id UUID PRIMARY KEY,
  last_sequence INT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL,
  modified_at TIMESTAMPTZ NOT NULL,
  -- Flattened fields from the event JSON
  code JSONB,
  name VARCHAR,
  reference VARCHAR,

  -- Collection rollups
  audit_entry_ids BIGINT[],
  ledger_account_set_ids UUID[],
  manual_ledger_account_ids UUID[],
  node_specs JSONB

);

-- Auto-generated trigger function for ChartEvent
CREATE OR REPLACE FUNCTION core_chart_events_rollup_trigger()
RETURNS TRIGGER AS $$
DECLARE
  event_type TEXT;
  current_row core_chart_events_rollup%ROWTYPE;
  new_row core_chart_events_rollup%ROWTYPE;
BEGIN
  event_type := NEW.event_type;

  -- Load the current rollup state
  SELECT * INTO current_row
  FROM core_chart_events_rollup
  WHERE id = NEW.id;

  -- Early return if event is older than current state
  IF current_row.id IS NOT NULL AND NEW.sequence <= current_row.last_sequence THEN
    RETURN NEW;
  END IF;

  -- Validate event type is known
  IF event_type NOT IN ('initialized', 'node_added', 'manual_transaction_account_added') THEN
    RAISE EXCEPTION 'Unknown event type: %', event_type;
  END IF;

  -- Construct the new row based on event type
  new_row.id := NEW.id;
  new_row.last_sequence := NEW.sequence;
  new_row.created_at := COALESCE(current_row.created_at, NEW.recorded_at);
  new_row.modified_at := NEW.recorded_at;

  -- Initialize fields with default values if this is a new record
  IF current_row.id IS NULL THEN
    new_row.audit_entry_ids := CASE
       WHEN NEW.event ? 'audit_entry_ids' THEN
         ARRAY(SELECT value::text::BIGINT FROM jsonb_array_elements_text(NEW.event -> 'audit_entry_ids'))
       ELSE ARRAY[]::BIGINT[]
     END
;
    new_row.code := (NEW.event -> 'code');
    new_row.ledger_account_set_ids := CASE
       WHEN NEW.event ? 'ledger_account_set_ids' THEN
         ARRAY(SELECT value::text::UUID FROM jsonb_array_elements_text(NEW.event -> 'ledger_account_set_ids'))
       ELSE ARRAY[]::UUID[]
     END
;
    new_row.manual_ledger_account_ids := CASE
       WHEN NEW.event ? 'manual_ledger_account_ids' THEN
         ARRAY(SELECT value::text::UUID FROM jsonb_array_elements_text(NEW.event -> 'manual_ledger_account_ids'))
       ELSE ARRAY[]::UUID[]
     END
;
    new_row.name := (NEW.event ->> 'name');
    new_row.node_specs := CASE
       WHEN NEW.event ? 'node_specs' THEN
         (NEW.event -> 'node_specs')
       ELSE '[]'::JSONB
     END
;
    new_row.reference := (NEW.event ->> 'reference');
  ELSE
    -- Default all fields to current values
    new_row.audit_entry_ids := current_row.audit_entry_ids;
    new_row.code := current_row.code;
    new_row.ledger_account_set_ids := current_row.ledger_account_set_ids;
    new_row.manual_ledger_account_ids := current_row.manual_ledger_account_ids;
    new_row.name := current_row.name;
    new_row.node_specs := current_row.node_specs;
    new_row.reference := current_row.reference;
  END IF;

  -- Update only the fields that are modified by the specific event
  CASE event_type
    WHEN 'initialized' THEN
      new_row.audit_entry_ids := array_append(COALESCE(current_row.audit_entry_ids, ARRAY[]::BIGINT[]), (NEW.event -> 'audit_info' ->> 'audit_entry_id')::BIGINT);
      new_row.name := (NEW.event ->> 'name');
      new_row.reference := (NEW.event ->> 'reference');
    WHEN 'node_added' THEN
      new_row.audit_entry_ids := array_append(COALESCE(current_row.audit_entry_ids, ARRAY[]::BIGINT[]), (NEW.event -> 'audit_info' ->> 'audit_entry_id')::BIGINT);
      new_row.ledger_account_set_ids := array_append(COALESCE(current_row.ledger_account_set_ids, ARRAY[]::UUID[]), (NEW.event ->> 'ledger_account_set_id')::UUID);
      new_row.node_specs := COALESCE(current_row.node_specs, '[]'::JSONB) || jsonb_build_array(NEW.event -> 'spec');
    WHEN 'manual_transaction_account_added' THEN
      new_row.audit_entry_ids := array_append(COALESCE(current_row.audit_entry_ids, ARRAY[]::BIGINT[]), (NEW.event -> 'audit_info' ->> 'audit_entry_id')::BIGINT);
      new_row.code := (NEW.event -> 'code');
      new_row.manual_ledger_account_ids := array_append(COALESCE(current_row.manual_ledger_account_ids, ARRAY[]::UUID[]), (NEW.event ->> 'ledger_account_id')::UUID);
  END CASE;

  INSERT INTO core_chart_events_rollup (
    id,
    last_sequence,
    created_at,
    modified_at,
    audit_entry_ids,
    code,
    ledger_account_set_ids,
    manual_ledger_account_ids,
    name,
    node_specs,
    reference
  )
  VALUES (
    new_row.id,
    new_row.last_sequence,
    new_row.created_at,
    new_row.modified_at,
    new_row.audit_entry_ids,
    new_row.code,
    new_row.ledger_account_set_ids,
    new_row.manual_ledger_account_ids,
    new_row.name,
    new_row.node_specs,
    new_row.reference
  )
  ON CONFLICT (id) DO UPDATE SET
    last_sequence = EXCLUDED.last_sequence,
    modified_at = EXCLUDED.modified_at,
    audit_entry_ids = EXCLUDED.audit_entry_ids,
    code = EXCLUDED.code,
    ledger_account_set_ids = EXCLUDED.ledger_account_set_ids,
    manual_ledger_account_ids = EXCLUDED.manual_ledger_account_ids,
    name = EXCLUDED.name,
    node_specs = EXCLUDED.node_specs,
    reference = EXCLUDED.reference;

  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Auto-generated trigger for ChartEvent
CREATE TRIGGER core_chart_events_rollup_trigger
  AFTER INSERT ON core_chart_events
  FOR EACH ROW
  EXECUTE FUNCTION core_chart_events_rollup_trigger();
