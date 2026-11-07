// The playground's starting schema, fixture rows and examples. Every example that is meant to
// translate does so cleanly (tests/playground.test.mjs checks each one's stages).

export const SCHEMA = `{
  "ORDERS": [
    ["ORDER_ID", "NUMBER(38,0)"],
    ["CUSTOMER_ID", "NUMBER(38,0)"],
    ["STATUS", "VARCHAR"],
    ["TOTAL", "NUMBER(12,2)"],
    ["ORDERED_AT", "TIMESTAMP_NTZ"]
  ],
  "CUSTOMERS": [
    ["CUSTOMER_ID", "NUMBER(38,0)"],
    ["EMAIL", "VARCHAR"],
    ["FIRST_NAME", "VARCHAR"],
    ["COUNTRY", "VARCHAR"],
    ["SIGNUP_DATE", "DATE"]
  ],
  "PRODUCTS": [
    ["PRODUCT_ID", "NUMBER(38,0)"],
    ["NAME", "VARCHAR"],
    ["CATEGORY", "VARCHAR"],
    ["PRICE", "NUMBER(10,2)"]
  ],
  "ORDER_ITEMS": [
    ["ORDER_ID", "NUMBER(38,0)"],
    ["PRODUCT_ID", "NUMBER(38,0)"],
    ["QUANTITY", "NUMBER(38,0)"],
    ["UNIT_PRICE", "NUMBER(10,2)"]
  ],
  "EVENTS": [
    ["EVENT_ID", "NUMBER(38,0)"],
    ["CUSTOMER_ID", "NUMBER(38,0)"],
    ["EVENT_TYPE", "VARCHAR"],
    ["OCCURRED_AT", "TIMESTAMP_NTZ"],
    ["PAYLOAD", "VARIANT"]
  ]
}`;

export const ROWS = `INSERT INTO "ORDERS" VALUES
  (1, 10, 'shipped', 120.50, TIMESTAMP '2026-10-01 09:30:00'),
  (2, 10, 'shipped', 80.00, TIMESTAMP '2026-10-03 14:00:00'),
  (3, 11, 'pending', 45.25, TIMESTAMP '2026-10-04 11:15:00'),
  (4, 12, 'shipped', 310.00, TIMESTAMP '2026-10-05 16:45:00'),
  (5, 12, NULL, 12.99, TIMESTAMP '2026-10-06 08:05:00');
INSERT INTO "CUSTOMERS" VALUES
  (10, 'ada@example.com', 'Ada', 'GB', DATE '2026-01-15'),
  (11, 'grace@example.com', 'Grace', 'US', DATE '2026-03-02'),
  (12, 'linus@example.com', 'Linus', 'FI', DATE '2026-06-20');
INSERT INTO "PRODUCTS" VALUES
  (100, 'Duck mug', 'kitchen', 12.50),
  (101, 'Rubber duck', 'toys', 4.99),
  (102, 'Pond map', 'books', 30.00);
INSERT INTO "ORDER_ITEMS" VALUES
  (1, 100, 2, 12.50), (1, 101, 1, 4.99), (2, 102, 2, 30.00),
  (3, 101, 5, 4.99), (4, 102, 10, 30.00), (5, 101, 1, 4.99);
INSERT INTO "EVENTS" VALUES
  (1000, 10, 'view', TIMESTAMP '2026-10-01 09:00:00', '{"page": "/mugs", "ms": 820}'),
  (1001, 10, 'purchase', TIMESTAMP '2026-10-01 09:30:00', '{"order_id": 1, "items": [100, 101]}'),
  (1002, 11, 'view', TIMESTAMP '2026-10-04 11:00:00', '{"page": "/ducks", "ms": 310}'),
  (1003, 12, 'purchase', TIMESTAMP '2026-10-05 16:45:00', '{"order_id": 4, "items": [102]}');`;

export const EXAMPLES = {
	valid: `SELECT customer_id,
       SUM(total) AS revenue,
       IFF(COUNT(*) > 1, 'repeat', 'new') AS segment
FROM orders
WHERE status = 'shipped'
GROUP BY customer_id
ORDER BY revenue DESC`,
	decimal: `SELECT order_id, total / 3 AS third
FROM orders
ORDER BY order_id`,
	syntax: `SELECT customer_id, SUM(total) AS revenue
FROM orders
WHERE status = 'shipped' AND
GROUP BY customer_id`,
	rejected: `SELECT customer_id, SUM(total) AS revenue
FROM orders
GROUP BY customer_id
QUALIFY revenue > 100`,
	unknown: `SELECT order_id, discount
FROM orders`,
	refused: `SELECT order_id,
       HASH(customer_id) AS bucket
FROM orders`,
};

// Shown under the editor while an example is loaded and unedited.
export const CAPTIONS = {
	refused:
		'Valid Snowflake SQL that SQLQuack will not translate. DuckDB computes HASH differently, so translated SQL would run and return other values. SQLQuack refuses and says why rather than guess; a policy file can allow a known divergence explicitly.',
};
