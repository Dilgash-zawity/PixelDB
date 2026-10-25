-- The `shop` database used by tests/online_servers.rs for MySQL/MariaDB. Run as root:
--   mysql -u root < tests/fixtures/online_shop_mysql.sql
DROP DATABASE IF EXISTS shop;
CREATE DATABASE shop;
USE shop;
CREATE TABLE customers (id int AUTO_INCREMENT PRIMARY KEY, name varchar(100) NOT NULL, email varchar(80) UNIQUE, active tinyint(1) DEFAULT 1, joined datetime DEFAULT CURRENT_TIMESTAMP, balance decimal(10,2) DEFAULT 0, notes text, kind enum('person','company') DEFAULT 'person') ENGINE=InnoDB;
CREATE TABLE orders (id int AUTO_INCREMENT PRIMARY KEY, customer_id int, total decimal(10,2) NOT NULL, status varchar(20) NOT NULL DEFAULT 'new', placed date, total_with_tax decimal(12,2) AS (total * 1.2) STORED, FOREIGN KEY (customer_id) REFERENCES customers(id) ON DELETE CASCADE) ENGINE=InnoDB;
CREATE TABLE log (at datetime, message text);
CREATE VIEW big_orders AS SELECT o.id, c.name, o.total FROM orders o JOIN customers c ON c.id = o.customer_id WHERE o.total > 100;
INSERT INTO customers (name, email, balance) VALUES ('Ada', 'ada@x.io', 12.50), ('Grace', 'grace@x.io', 99.99), ('Linus', NULL, 0);
INSERT INTO orders (customer_id, total, status, placed) VALUES (1, 150.00, 'paid', '2026-01-05'), (1, 20.00, 'new', '2026-02-10'), (2, 300.50, 'paid', '2026-02-11');
INSERT INTO log VALUES (NOW(), 'hello');
