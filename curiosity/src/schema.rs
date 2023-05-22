use tantivy::schema::*;

pub fn build_schema() -> Schema {
    let mut schema_builder = Schema::builder();

    schema_builder.add_u64_field("episode_id", STORED | FAST);
    schema_builder.add_u64_field("season", INDEXED | FAST);
    schema_builder.add_text_field("title", TEXT);
    schema_builder.add_text_field("body", TEXT);
    schema_builder.build()
}
