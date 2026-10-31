use crate::lineage::models::{SchemaColumn, SourceColumn};
use crate::lineage::tests::test_types::ShopSchema;
use crate::lineage::types::LineageSchema;

impl LineageSchema for ShopSchema {
    fn columns(&self, relation: &str) -> Option<&[SchemaColumn]> {
        self.0.get(relation).map(Vec::as_slice)
    }
}

/// Orders, customers and products, as a Snowflake catalog spells unquoted names.
pub(crate) fn shop_schema() -> ShopSchema {
    let relations: [(&str, &[&str]); 4] = [
        (
            "ORDERS",
            &["ID", "CUSTOMER_ID", "AMOUNT", "ITEMS", "STATUS"],
        ),
        ("CUSTOMERS", &["ID", "NAME", "COUNTRY"]),
        ("SHOP.PRODUCTS", &["SKU", "PRICE"]),
        ("EXPORTS", &["\"listingId\"", "LISTING_STATUS"]),
    ];
    ShopSchema(
        relations
            .into_iter()
            .map(|(name, columns)| {
                (
                    name.to_owned(),
                    columns
                        .iter()
                        .map(|column| SchemaColumn {
                            name: column.trim_matches('"').to_owned(),
                            quoted: column.starts_with('"'),
                        })
                        .collect(),
                )
            })
            .collect(),
    )
}

/// A source column as `RELATION.column`, a quoted column in quotes.
pub(crate) fn written(source: &SourceColumn) -> String {
    let quote: &str = ["", "\""][usize::from(source.quoted)];
    format!("{}.{quote}{}{quote}", source.relation, source.column)
}
