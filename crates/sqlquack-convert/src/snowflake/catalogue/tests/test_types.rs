use sqlquack_diagnostics::classification::models::Construct;
use sqlquack_diagnostics::refusals::models::Disposition;

use crate::snowflake::catalogue::models::RuleCatalogue;

pub(crate) struct CatalogueTestCase {
    pub(crate) description: &'static str,
    pub(crate) catalogue: RuleCatalogue,
    pub(crate) construct: Construct,
    pub(crate) expected_disposition: Disposition,
}
