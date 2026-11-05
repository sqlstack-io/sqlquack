// The playground's starting schema, fixture rows and examples. Every example that is meant to
// translate does so cleanly (tests/playground.test.mjs checks each one's stages).

export const SCHEMA = `{
  "ORDERS": [
    ["ORDER_ID", "NUMBER(38,0)"],
    ["CUSTOMER_ID", "NUMBER(38,0)"],
    ["STATUS", "VARCHAR"],
    ["TOTAL", "NUMBER(12,2)"],
    ["ORDERED_AT", "TIMESTAMP_NTZ"]
  ]
}`;

export const ROWS = `INSERT INTO "ORDERS" VALUES
  (1, 10, 'shipped', 120.50, TIMESTAMP '2026-10-01 09:30:00'),
  (2, 10, 'shipped', 80.00, TIMESTAMP '2026-10-03 14:00:00'),
  (3, 11, 'pending', 45.25, TIMESTAMP '2026-10-04 11:15:00'),
  (4, 12, 'shipped', 310.00, TIMESTAMP '2026-10-05 16:45:00'),
  (5, 12, NULL, 12.99, TIMESTAMP '2026-10-06 08:05:00');`;

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
