use sqlquack_diagnostics::classification::models::{Construct, ConstructFamily};
use sqlquack_diagnostics::refusals::models::Disposition;
use sqlquack_diagnostics::refusals::types::ConstructPolicy;

use crate::catalogue::main::builtin_catalogue::builtin_catalogue;
use crate::catalogue::models::{NotExpressibleDeclaration, RuleCatalogue};
use crate::catalogue::tests::test_types::CatalogueTestCase;

#[test]
fn given_catalogue_when_asked_about_construct_then_reports_its_disposition() {
    let declaring = RuleCatalogue {
        not_expressible: vec![NotExpressibleDeclaration {
            construct: "fn:EXAMPLE",
            reason: "an example declaration",
        }],
    };
    let test_cases = [
        CatalogueTestCase {
            description: "the built-in catalogue has no rule for a node",
            catalogue: builtin_catalogue(),
            construct: Construct::new(ConstructFamily::Node, "select"),
            expected_disposition: Disposition::Missing,
        },
        CatalogueTestCase {
            description: "the built-in catalogue has no rule for a function",
            catalogue: builtin_catalogue(),
            construct: Construct::new(ConstructFamily::Function, "CONCAT"),
            expected_disposition: Disposition::Missing,
        },
        CatalogueTestCase {
            description: "a declared construct reports its reason",
            catalogue: declaring.clone(),
            construct: Construct::new(ConstructFamily::Function, "EXAMPLE"),
            expected_disposition: Disposition::NotExpressible {
                reason: "an example declaration".to_owned(),
            },
        },
        CatalogueTestCase {
            description: "an undeclared construct is still missing",
            catalogue: declaring,
            construct: Construct::new(ConstructFamily::Cast, "VARCHAR"),
            expected_disposition: Disposition::Missing,
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            test_case.catalogue.disposition(&test_case.construct),
            test_case.expected_disposition,
            "{}",
            test_case.description
        );
    }
}
