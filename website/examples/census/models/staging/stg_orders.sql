SELECT order_id, customer_id, WEEKISO(ordered_at) AS order_week FROM orders
