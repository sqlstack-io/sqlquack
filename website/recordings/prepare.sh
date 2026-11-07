#!/usr/bin/env bash
# Build the scratch directory for one demo. Usage: prepare.sh <demo>
set -euo pipefail
demo="$1"
here="$(cd "$(dirname "$0")" && pwd)"
dir="$here/scratch/$demo"
rm -rf "$dir"
mkdir -p "$dir"
cd "$dir"
case "$demo" in
  check-convert)
    # A query Snowflake rejects, one that converts, and one the built-in policy refuses.
    cat > orders.sql <<'SQL'
SELECT customer_id, SUM(total) AS revenue
FROM orders
WHERE status = 'shipped'
GROUP BY customer_id
QUALIFY revenue > 100
SQL
    cat > revenue.sql <<'SQL'
SELECT customer_id, SUM(total) AS revenue
FROM orders
WHERE status = 'shipped'
GROUP BY customer_id
HAVING SUM(total) > 100
SQL
    cat > bucket.sql <<'SQL'
SELECT order_id,
       HASH(customer_id) AS bucket
FROM orders
SQL
    cat > schema.json <<'JSON'
{
  "ORDERS": [
    ["ORDER_ID", "NUMBER(38,0)"],
    ["CUSTOMER_ID", "NUMBER(38,0)"],
    ["STATUS", "VARCHAR"],
    ["TOTAL", "NUMBER(12,2)"]
  ]
}
JSON
    ;;
  *)
    echo "unknown demo: $demo" >&2
    exit 2 ;;
esac
