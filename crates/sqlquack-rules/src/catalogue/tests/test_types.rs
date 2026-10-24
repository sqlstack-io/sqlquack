use sqlquack_core::classification::models::Construct;
use sqlquack_core::refusals::models::Disposition;

use crate::catalogue::models::RuleCatalogue;

pub(crate) struct CatalogueTestCase {
    pub(crate) description: &'static str,
    pub(crate) catalogue: RuleCatalogue,
    pub(crate) construct: Construct,
    pub(crate) expected_disposition: Disposition,
}
