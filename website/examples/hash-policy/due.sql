SELECT order_id,
       HASH(customer_id) AS bucket
FROM orders
