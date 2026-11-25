SELECT customer_id, SUM(total) AS revenue
FROM orders
WHERE status = 'shipped'
GROUP BY customer_id
