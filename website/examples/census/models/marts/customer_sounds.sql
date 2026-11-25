SELECT
    customer_id,
    SOUNDEX(name) AS name_sound
FROM __ref("stg_customers")
