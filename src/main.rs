use tag::search::get_tags_from_files;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tagged_files = get_tags_from_files("testfiles")?;

    for file in tagged_files.iter() {
        println!("File {} contains {:?}", file.path.display(), file.tags);
    }

    Ok(())
}
