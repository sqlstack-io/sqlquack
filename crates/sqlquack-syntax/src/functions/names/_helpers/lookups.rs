//! Functions names: lookups.

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::LazyLock;

use crate::functions::names::constants::AGGREGATE_FUNCTION_NAME_LIST;
use crate::functions::names::constants::DUCKDB_AGGREGATE_FUNCTION_NAME_LIST;
use crate::functions::names::constants::NO_PAREN_FUNCTION_NAME_LIST;
use crate::functions::names::models::FunctionSpec;

/// Consolidated metadata map keyed by uppercased function name.
pub(crate) static FUNCTION_SPECS: LazyLock<HashMap<&'static str, FunctionSpec>> =
    LazyLock::new(|| {
        let mut specs: HashMap<&str, FunctionSpec> = HashMap::new();

        for &name in NO_PAREN_FUNCTION_NAME_LIST {
            specs.insert(
                name,
                FunctionSpec {
                    name,
                    no_paren: true,
                    aggregate: false,
                },
            );
        }

        for &name in AGGREGATE_FUNCTION_NAME_LIST {
            specs
                .entry(name)
                .and_modify(|spec| spec.aggregate = true)
                .or_insert(FunctionSpec {
                    name,
                    no_paren: false,
                    aggregate: true,
                });
        }

        specs
    });

/// Set of names that can be no-paren functions.
pub(crate) static NO_PAREN_FUNCTION_NAME_SET: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| {
        FUNCTION_SPECS
            .iter()
            .filter_map(|(name, spec)| spec.no_paren.then_some(*name))
            .collect()
    });

/// Set of names that should use aggregate parsing behavior.
pub(crate) static AGGREGATE_FUNCTION_NAME_SET: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| {
        FUNCTION_SPECS
            .iter()
            .filter_map(|(name, spec)| spec.aggregate.then_some(*name))
            .collect()
    });

/// DuckDB-specific names that should use aggregate parsing behavior.
pub(crate) static DUCKDB_AGGREGATE_FUNCTION_NAME_SET: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| {
        DUCKDB_AGGREGATE_FUNCTION_NAME_LIST
            .iter()
            .copied()
            .collect()
    });
