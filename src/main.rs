use pest::Parser;
use tag::{
    parsers::query::{construct_query_ast, QueryParser, Rule},
    search::get_tags_from_files,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tagged_files = get_tags_from_files("testfiles")?;

    for file in tagged_files.iter() {
        println!("File {} contains {:?}", file.path.display(), file.tags);
    }

    let ast = construct_query_ast(
        QueryParser::parse(Rule::tagsearch, "#a & #b | (#c & #d)")
            .unwrap()
            .next()
            .unwrap()
            .into_inner(),
        vec!["#a", "#c", "#d"],
    );

    println!("{:#?}", ast);

    Ok(())
}
