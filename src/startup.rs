use crate::models::{NewPurpose, Purpose};
use crate::schema::purposes;
use crate::snowflake::SnowflakeGeneratorWrapper;
use diesel::prelude::*;

pub fn upsert_purposes(
    conn: &mut PgConnection,
    snowflake_gen: &SnowflakeGeneratorWrapper,
    purpose_slugs: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    for slug in purpose_slugs {
        let existing: Option<Purpose> = purposes::table
            .filter(purposes::slug.eq(slug))
            .first(conn)
            .optional()?;

        if existing.is_none() {
            let oid = snowflake_gen.generate()?;
            let id = crate::snowflake::generate_prefixed_id("purpose", oid);

            let new_purpose = NewPurpose {
                oid,
                id,
                slug: slug.clone(),
            };

            diesel::insert_into(purposes::table)
                .values(&new_purpose)
                .execute(conn)?;

            tracing::info!("Created purpose: {}", slug);
        }
    }

    Ok(())
}
