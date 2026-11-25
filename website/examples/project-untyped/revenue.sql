SELECT status, SUM(total) AS revenue
FROM orders
GROUP BY status
