ALTER TABLE core_customers ADD COLUMN last_activity TIMESTAMPTZ;

CREATE INDEX idx_core_customers_last_activity ON core_customers(last_activity); 