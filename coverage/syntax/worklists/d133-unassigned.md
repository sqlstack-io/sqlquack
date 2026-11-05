# D133 disagreement worklist: owner unassigned

Settled-accept (D133) cases on catalogue e85aeb3e (evidence 83452441) that the parser at main c793813c
rejects or leaves opaque. Snowflake parsed every one of them. Construct: the most common failure,
with the token the parser stopped at, or the head of the opaque statement.

## classes: 35 forms, 35 cases (9 rejected, 26 opaque)

| form | cases | rejected | opaque | most common construct |
|---|---:|---:|---:|---|
| `sf.classes.anomaly-detection.commands.show-anomaly-detection` | 1 | 1 | 0 | rejected at `. ANOMALY_DETECTION`: Invalid expression / Unexpected token (1) |
| `sf.classes.anomaly-insights.methods.add-notification-integration` | 1 | 0 | 1 | opaque: CALL SNOWFLAKE . (1) |
| `sf.classes.anomaly-insights.methods.adhoc-calculate-anomalies-from-config` | 1 | 0 | 1 | opaque: CALL SNOWFLAKE . (1) |
| `sf.classes.anomaly-insights.methods.create-monitor` | 1 | 0 | 1 | opaque: CALL SNOWFLAKE . (1) |
| `sf.classes.anomaly-insights.methods.drop-monitor` | 1 | 0 | 1 | opaque: CALL SNOWFLAKE . (1) |
| `sf.classes.anomaly-insights.methods.get-account-anomalies-in-credits` | 1 | 0 | 1 | opaque: CALL SNOWFLAKE . (1) |
| `sf.classes.anomaly-insights.methods.get-account-notification-emails` | 1 | 0 | 1 | opaque: CALL SNOWFLAKE . (1) |
| `sf.classes.anomaly-insights.methods.get-daily-consumption-anomaly-data` | 1 | 0 | 1 | opaque: CALL SNOWFLAKE . (1) |
| `sf.classes.anomaly-insights.methods.get-hourly-consumption-by-service-type` | 1 | 0 | 1 | opaque: CALL SNOWFLAKE . (1) |
| `sf.classes.anomaly-insights.methods.get-hourly-spend-for-anomaly` | 1 | 0 | 1 | opaque: CALL SNOWFLAKE . (1) |
| `sf.classes.anomaly-insights.methods.get-monitor-anomalies` | 1 | 0 | 1 | opaque: CALL SNOWFLAKE . (1) |
| `sf.classes.anomaly-insights.methods.get-monitor-config` | 1 | 0 | 1 | opaque: CALL SNOWFLAKE . (1) |
| `sf.classes.anomaly-insights.methods.get-monitor-notification-emails` | 1 | 0 | 1 | opaque: CALL SNOWFLAKE . (1) |
| `sf.classes.anomaly-insights.methods.get-monitor-notification-log` | 1 | 0 | 1 | opaque: CALL SNOWFLAKE . (1) |
| `sf.classes.anomaly-insights.methods.get-notification-integrations` | 1 | 0 | 1 | opaque: CALL SNOWFLAKE . (1) |
| `sf.classes.anomaly-insights.methods.get-org-notification-emails` | 1 | 0 | 1 | opaque: CALL SNOWFLAKE . (1) |
| `sf.classes.anomaly-insights.methods.get-top-accounts-by-consumption` | 1 | 0 | 1 | opaque: CALL SNOWFLAKE . (1) |
| `sf.classes.anomaly-insights.methods.get-top-queries-from-warehouse` | 1 | 0 | 1 | opaque: CALL SNOWFLAKE . (1) |
| `sf.classes.anomaly-insights.methods.get-top-warehouses-on-date` | 1 | 0 | 1 | opaque: CALL SNOWFLAKE . (1) |
| `sf.classes.anomaly-insights.methods.list-monitors` | 1 | 0 | 1 | opaque: CALL SNOWFLAKE . (1) |
| `sf.classes.anomaly-insights.methods.recalculate-anomalies` | 1 | 0 | 1 | opaque: CALL SNOWFLAKE . (1) |
| `sf.classes.anomaly-insights.methods.remove-notification-integration` | 1 | 0 | 1 | opaque: CALL SNOWFLAKE . (1) |
| `sf.classes.anomaly-insights.methods.rename-monitor` | 1 | 0 | 1 | opaque: CALL SNOWFLAKE . (1) |
| `sf.classes.anomaly-insights.methods.set-account-notification-emails` | 1 | 0 | 1 | opaque: CALL SNOWFLAKE . (1) |
| `sf.classes.anomaly-insights.methods.set-monitor-notification-emails` | 1 | 0 | 1 | opaque: CALL SNOWFLAKE . (1) |
| `sf.classes.anomaly-insights.methods.set-org-notification-emails` | 1 | 0 | 1 | opaque: CALL SNOWFLAKE . (1) |
| `sf.classes.anomaly-insights.methods.update-monitor-config` | 1 | 0 | 1 | opaque: CALL SNOWFLAKE . (1) |
| `sf.classes.budget.commands.show-budget` | 1 | 1 | 0 | rejected at `. BUDGET`: Invalid expression / Unexpected token (1) |
| `sf.classes.classification-profile.commands.show-classification-profile` | 1 | 1 | 0 | rejected at `. CLASSIFICATION_PROFILE`: Invalid expression / Unexpected token (1) |
| `sf.classes.classification-profile.methods.describe` | 1 | 1 | 0 | rejected at `DESCRIBE (`: Unexpected token: Exclamation (1) |
| `sf.classes.classification.commands.show-classification` | 1 | 1 | 0 | rejected at `. CLASSIFICATION`: Invalid expression / Unexpected token (1) |
| `sf.classes.custom-classifier.commands.show-custom-classifiers` | 1 | 1 | 0 | rejected at `. CUSTOM_CLASSIFIER`: Invalid expression / Unexpected token (1) |
| `sf.classes.custom-classifier.methods.list` | 1 | 1 | 0 | rejected at `LIST (`: Unexpected token: Exclamation (1) |
| `sf.classes.forecast.commands.show-forecast` | 1 | 1 | 0 | rejected at `. FORECAST`: Invalid expression / Unexpected token (1) |
| `sf.classes.top-insights.commands.show-top-insights` | 1 | 1 | 0 | rejected at `. TOP_INSIGHTS`: Invalid expression / Unexpected token (1) |

## transaction_session: 4 forms, 6 cases (6 rejected, 0 opaque)

| form | cases | rejected | opaque | most common construct |
|---|---:|---:|---:|---|
| `sf.transaction_session.desc-result` | 2 | 2 | 0 | rejected at `;`: Expected a query ID or LAST_QUERY_ID() (1) |
| `sf.transaction_session.use-schema` | 2 | 2 | 0 | rejected at ``: Expected identifier, got "Semicolon" (1) |
| `sf.transaction_session.use-role` | 1 | 1 | 0 | rejected at ``: Expected identifier, got "Semicolon" (1) |
| `sf.transaction_session.use-warehouse` | 1 | 1 | 0 | rejected at ``: Expected identifier, got "Semicolon" (1) |

## utility: 16 forms, 76 cases (70 rejected, 6 opaque)

| form | cases | rejected | opaque | most common construct |
|---|---:|---:|---:|---|
| `sf.utility.execute-notebook-project` | 11 | 11 | 0 | rejected at `. v1`: Invalid expression / Unexpected token (9) |
| `sf.utility.execute-job-service` | 10 | 10 | 0 | rejected at `COMPUTE POOL`: Invalid expression / Unexpected token (10) |
| `sf.utility.call-with.table-language-java-runtime-version.java` | 8 | 8 | 0 | rejected at `RETURNS INT`: Expected identifier, got "RParen" (2) |
| `sf.utility.call-with.table-language-scala-runtime-version.scala` | 8 | 8 | 0 | rejected at `RETURNS INT`: Expected identifier, got "RParen" (2) |
| `sf.utility.execute-dbt-project.project` | 8 | 8 | 0 | rejected at `EXTERNAL_ACCESS_INTEGRATIONS =`: Invalid expression / Unexpected token (2) |
| `sf.utility.execute-inference-job-service` | 7 | 7 | 0 | rejected at `IN COMPUTE`: Invalid expression / Unexpected token (7) |
| `sf.utility.execute-immediate-from` | 6 | 0 | 6 | opaque: EXECUTE IMMEDIATE FROM (6) |
| `sf.utility.call-with.table-language-sql-as` | 5 | 5 | 0 | rejected at `RETURNS INT`: Expected identifier, got "RParen" (2) |
| `sf.utility.call-with.language-javascript-as-call` | 3 | 3 | 0 | rejected at `RETURNS INT`: Expected identifier, got "RParen" (2) |
| `sf.utility.execute-code-bundle` | 3 | 3 | 0 | rejected at `ENTRYPOINT =`: Invalid expression / Unexpected token (3) |
| `sf.utility.call-with.table-language-python-runtime-version` | 2 | 2 | 0 | rejected at `) HANDLER`: Expected string, got "Var" (1) |
| `sf.utility.call-with.table-language-python-runtime-version.python` | 1 | 1 | 0 | rejected at `) HANDLER`: Expected string, got "Var" (1) |
| `sf.utility.execute-notebook` | 1 | 1 | 0 | rejected at `m1 )`: Invalid expression / Unexpected token (1) |
| `sf.utility.execute-task-graph.graph-retry-graph-run` | 1 | 1 | 0 | rejected at `RUN GROUP`: Invalid expression / Unexpected token (1) |
| `sf.utility.execute-task-graph.graph-retry-last` | 1 | 1 | 0 | rejected at `;`: Invalid expression / Unexpected token (1) |
| `sf.utility.execute-task-graph.graph` | 1 | 1 | 0 | rejected at ``: Expected identifier, got "Semicolon" (1) |
