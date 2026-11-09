use crate::functions::typed::main::canonical_typed_function_name_upper::canonical_typed_function_name_upper;
use crate::functions::typed::main::typed_dispatch_group_by_name_upper::typed_dispatch_group_by_name_upper;
use crate::functions::typed::models::TypedDispatchGroup;
use crate::functions::typed::models::TypedParseKind;
use crate::functions::typed::tests::helpers::dispatch_group_names;
use crate::functions::typed::tests::helpers::misgrouped_names;
use crate::functions::typed::tests::helpers::repeated_names;
use crate::functions::typed::tests::helpers::resolved_spec;
use crate::functions::typed::tests::helpers::stray_aliases;
use crate::functions::typed::tests::helpers::unresolved_names;
use crate::functions::typed::tests::test_types::CanonicalNameTestCase;
use crate::functions::typed::tests::test_types::DispatchGroupTestCase;
use crate::functions::typed::tests::test_types::HeavyFamilyTestCase;
use crate::functions::typed::tests::test_types::RegistryCheckTestCase;
use crate::functions::typed::tests::test_types::TypedSpecTestCase;

#[test]
fn given_the_typed_registry_when_checking_then_it_is_self_consistent() {
    let test_cases = [
        RegistryCheckTestCase {
            description: "no canonical name or alias appears twice",
            violations: repeated_names,
            expected_violations: vec![],
        },
        RegistryCheckTestCase {
            description: "every canonical name and alias resolves to its spec",
            violations: unresolved_names,
            expected_violations: vec![],
        },
        RegistryCheckTestCase {
            description: "every alias map entry is listed by its canonical spec",
            violations: stray_aliases,
            expected_violations: vec![],
        },
        RegistryCheckTestCase {
            description: "every dispatch group names a typed spec and resolves to its group",
            violations: misgrouped_names,
            expected_violations: vec![],
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            (test_case.violations)(),
            test_case.expected_violations,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_typed_function_name_when_resolving_then_its_canonical_spec_and_parse_kind_follow() {
    let test_cases = [
        TypedSpecTestCase {
            description: "COUNT_IF parses like an aggregate",
            name: "COUNT_IF",
            expected_spec: Some(("COUNT_IF", TypedParseKind::AggregateLike)),
        },
        TypedSpecTestCase {
            description: "IIF resolves to the IF conditional",
            name: "IIF",
            expected_spec: Some(("IF", TypedParseKind::Conditional)),
        },
        TypedSpecTestCase {
            description: "CONTAINS is binary",
            name: "CONTAINS",
            expected_spec: Some(("CONTAINS", TypedParseKind::Binary)),
        },
        TypedSpecTestCase {
            description: "SQRT is unary",
            name: "SQRT",
            expected_spec: Some(("SQRT", TypedParseKind::Unary)),
        },
        TypedSpecTestCase {
            description: "TIMEADD resolves to the variadic DATEADD",
            name: "TIMEADD",
            expected_spec: Some(("DATEADD", TypedParseKind::Variadic)),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            resolved_spec(test_case.name),
            test_case.expected_spec,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_an_alias_when_canonicalizing_then_it_becomes_the_dispatch_key() {
    let test_cases = [
        CanonicalNameTestCase {
            description: "COUNTIF becomes COUNT_IF",
            name: "COUNTIF",
            expected_canonical: "COUNT_IF",
        },
        CanonicalNameTestCase {
            description: "DAY_OF_MONTH becomes DAYOFMONTH",
            name: "DAY_OF_MONTH",
            expected_canonical: "DAYOFMONTH",
        },
        CanonicalNameTestCase {
            description: "SUBSTR becomes SUBSTRING",
            name: "SUBSTR",
            expected_canonical: "SUBSTRING",
        },
        CanonicalNameTestCase {
            description: "TRYCAST becomes TRY_CAST",
            name: "TRYCAST",
            expected_canonical: "TRY_CAST",
        },
        CanonicalNameTestCase {
            description: "LEN becomes LENGTH",
            name: "LEN",
            expected_canonical: "LENGTH",
        },
        CanonicalNameTestCase {
            description: "LCASE becomes LOWER",
            name: "LCASE",
            expected_canonical: "LOWER",
        },
        CanonicalNameTestCase {
            description: "UCASE becomes UPPER",
            name: "UCASE",
            expected_canonical: "UPPER",
        },
        CanonicalNameTestCase {
            description: "CEILING becomes CEIL",
            name: "CEILING",
            expected_canonical: "CEIL",
        },
        CanonicalNameTestCase {
            description: "TO_DECIMAL becomes TO_NUMBER",
            name: "TO_DECIMAL",
            expected_canonical: "TO_NUMBER",
        },
        CanonicalNameTestCase {
            description: "TRY_TO_NUMERIC becomes TRY_TO_NUMBER",
            name: "TRY_TO_NUMERIC",
            expected_canonical: "TRY_TO_NUMBER",
        },
        CanonicalNameTestCase {
            description: "DATEFROMPARTS becomes DATE_FROM_PARTS",
            name: "DATEFROMPARTS",
            expected_canonical: "DATE_FROM_PARTS",
        },
        CanonicalNameTestCase {
            description: "TIMESTAMPTZFROMPARTS becomes TIMESTAMP_TZ_FROM_PARTS",
            name: "TIMESTAMPTZFROMPARTS",
            expected_canonical: "TIMESTAMP_TZ_FROM_PARTS",
        },
        CanonicalNameTestCase {
            description: "TOTYPENAME becomes TYPEOF",
            name: "TOTYPENAME",
            expected_canonical: "TYPEOF",
        },
        CanonicalNameTestCase {
            description: "IIF becomes IF",
            name: "IIF",
            expected_canonical: "IF",
        },
        CanonicalNameTestCase {
            description: "IFF becomes IF",
            name: "IFF",
            expected_canonical: "IF",
        },
        CanonicalNameTestCase {
            description: "a name without an alias stays as written",
            name: "NOT_AN_ALIAS",
            expected_canonical: "NOT_AN_ALIAS",
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            canonical_typed_function_name_upper(test_case.name),
            test_case.expected_canonical,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_function_name_when_looking_up_its_dispatch_group_then_heavy_families_are_grouped() {
    let test_cases = [
        DispatchGroupTestCase {
            description: "COUNT has its family group",
            name: "COUNT",
            expected_group: Some(TypedDispatchGroup::Aggregate),
        },
        DispatchGroupTestCase {
            description: "ROW_NUMBER has its family group",
            name: "ROW_NUMBER",
            expected_group: Some(TypedDispatchGroup::Window),
        },
        DispatchGroupTestCase {
            description: "JSON_TABLE has its family group",
            name: "JSON_TABLE",
            expected_group: Some(TypedDispatchGroup::Json),
        },
        DispatchGroupTestCase {
            description: "TRANSLATE has its family group",
            name: "TRANSLATE",
            expected_group: Some(TypedDispatchGroup::TranslateTeradata),
        },
        DispatchGroupTestCase {
            description: "an unknown name has no group",
            name: "NOT_A_TYPED_FN",
            expected_group: None,
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            typed_dispatch_group_by_name_upper(test_case.name),
            test_case.expected_group,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_dispatch_group_specs_when_listing_names_then_they_are_exactly_the_heavy_families() {
    let test_cases = [HeavyFamilyTestCase {
        description: "the dispatch groups cover exactly the intended heavy families",
        expected_names: [
            "COUNT",
            "LIST",
            "MAP",
            "ARRAY",
            "SUM",
            "AVG",
            "MIN",
            "MAX",
            "ARRAY_AGG",
            "ARRAY_CONCAT_AGG",
            "STDDEV",
            "STDDEV_POP",
            "STDDEV_SAMP",
            "VARIANCE",
            "VAR_POP",
            "VAR_SAMP",
            "BOOL_AND",
            "BOOL_OR",
            "MEDIAN",
            "MODE",
            "FIRST",
            "LAST",
            "ANY_VALUE",
            "APPROX_DISTINCT",
            "APPROX_COUNT_DISTINCT",
            "BIT_AND",
            "BIT_OR",
            "BIT_XOR",
            "STRING_AGG",
            "GROUP_CONCAT",
            "LISTAGG",
            "ROW_NUMBER",
            "RANK",
            "DENSE_RANK",
            "PERCENT_RANK",
            "CUME_DIST",
            "NTILE",
            "LEAD",
            "LAG",
            "FIRST_VALUE",
            "LAST_VALUE",
            "NTH_VALUE",
            "JSON_EXTRACT",
            "JSON_EXTRACT_SCALAR",
            "JSON_QUERY",
            "JSON_VALUE",
            "JSON_ARRAY_LENGTH",
            "JSON_KEYS",
            "JSON_TYPE",
            "TO_JSON",
            "PARSE_JSON",
            "JSON_OBJECT",
            "JSON_ARRAY",
            "JSON_ARRAYAGG",
            "JSON_OBJECTAGG",
            "JSON_TABLE",
            "TRANSLATE",
        ]
        .into_iter()
        .collect(),
    }];

    for test_case in test_cases {
        assert_eq!(
            dispatch_group_names(),
            test_case.expected_names,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_written_call_name_when_canonicalizing_then_its_last_part_is_checked_in_lower_case() {
    let test_cases = [
        CanonicalNameTestCase {
            description: "an alias becomes its canonical name",
            name: "DATE_DIFF",
            expected_canonical: "datediff",
        },
        CanonicalNameTestCase {
            description: "a qualified name keeps its last part",
            name: "shop.analytics.Substr",
            expected_canonical: "substring",
        },
        CanonicalNameTestCase {
            description: "a name the registry does not know stays as written, in lower case",
            name: "Order_Score",
            expected_canonical: "order_score",
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            crate::canonical_call_name(test_case.name),
            test_case.expected_canonical,
            "{}",
            test_case.description
        );
    }
}
