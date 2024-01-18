use std::{
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use pest::Parser;
use walkdir::WalkDir;

use crate::parsers::tagline::{self, TaglineParser};

/// TaggedFile is a file that contains tags.
#[derive(Clone)]
pub struct TaggedFile {
    path: PathBuf,
    tags: Vec<String>,
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
pub fn get_tags_from_files(directory: &str) -> Result<Vec<TaggedFile>, Box<dyn std::error::Error>> {
    let mut tagged_files = Vec::new();

    for entry in WalkDir::new(directory).follow_links(true) {
        let entry = entry?;
        let tags = get_tags_from_file(entry.path())?;
        tagged_files.push(TaggedFile {
            path: entry.path().to_owned(),
            tags,
        })
    }

    Ok(tagged_files.clone())
}