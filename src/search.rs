use std::{
    fs,
    io::{BufRead, BufReader},
    path::Path,
};

use pest::Parser;

use crate::parsers::tagline::{self, TaglineParser};

/// TaggedFile is a file that contains tags.
pub struct TaggedFile<'a> {
    path: &'a Path,
    tags: Vec<&'a str>,
}

/// get_tags_from_file() returns a list of tags found in a file.
/// It will return an error if a file has no parsable tags.
fn get_tags_from_file(file: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let file = fs::File::open(file)?;
    let mut buffer = BufReader::new(file);
    let mut tagline = String::new();
    let _ = buffer.read_line(&mut tagline)?;

    let mut parsed = TaglineParser::parse(tagline::Rule::tagline, &tagline)?;

    let mut tags = Vec::new();

    for tag in parsed.next().unwrap().into_inner() {
        match tag.as_rule() {
            tagline::Rule::tag => tags.push(tag.as_str().to_string()),
            _ => unreachable!(),
        }
    }

    Ok(tags)
}

/// get_tags_from_files() recursively retrieves the tags of all files
/// in a given directory.
pub fn get_tags_from_files(
    directory: &Path,
) -> Result<Vec<TaggedFile>, Box<dyn std::error::Error>> {
    todo!()
}
