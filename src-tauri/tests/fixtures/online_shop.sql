-- The `shop` database used by tests/online_postgres.rs. Run as postgres:
--   psql -U postgres -c 'CREATE DATABASE shop' && psql -U postgres -d shop -f tests/fixtures/online_shop.sql
CREATE TABLE customers (id serial PRIMARY KEY, name text NOT NULL, email varchar(80) UNIQUE, active boolean DEFAULT true, joined timestamptz DEFAULT now(), balance numeric(10,2) DEFAULT 0, prefs jsonb, token uuid);
CREATE TABLE orders (id int GENERATED ALWAYS AS IDENTITY PRIMARY KEY, customer_id int REFERENCES customers(id) ON DELETE CASCADE, total numeric(10,2) NOT NULL, status varchar(20) NOT NULL DEFAULT 'new', placed date DEFAULT current_date, total_with_tax numeric GENERATED ALWAYS AS (total * 1.2) STORED);
CREATE SCHEMA sales;
CREATE TABLE sales.leads (code text PRIMARY KEY, "Full Name" text, score int);
CREATE TABLE log (at timestamptz, message text);
CREATE VIEW big_orders AS SELECT o.id, c.name, o.total FROM orders o JOIN customers c ON c.id = o.customer_id WHERE o.total > 100;
INSERT INTO customers (name, email, balance, prefs, token) VALUES ('Ada', 'ada@x.io', 12.50, '{"theme":"dark"}', gen_random_uuid()), ('Grace', 'grace@x.io', 99.99, NULL, NULL), ('Linus', NULL, 0, NULL, NULL);
INSERT INTO orders (customer_id, total, status, placed) VALUES (1, 150.00, 'paid', '2026-01-05'), (1, 20.00, 'new', '2026-02-10'), (2, 300.50, 'paid', '2026-02-11');
INSERT INTO sales.leads VALUES ('L1', 'Zed Shaw', 5);
INSERT INTO log VALUES (now(), 'hello');
ANALYZE;
