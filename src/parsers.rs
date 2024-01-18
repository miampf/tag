use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "tagline.pest"]
/// TaglineParser is responsible for parsing the taglines at the start of each searched file.
pub struct TaglineParser;
