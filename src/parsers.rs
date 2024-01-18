use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "tagline.pest"]
/// TaglineParser is responsible for parsing the taglines at the start of each searched file.
pub struct TaglineParser;

#[cfg(test)]
mod tests {
    use pest::Parser;

    use super::*;

    #[test]
    fn test_tagline_parser() {
        struct TestCase<'a> {
            name: &'a str,
            input: &'a str,
            expected_error: bool,
        }

        let test_cases = [
            TestCase {
                name: "success_with_space",
                input: "tags: [#1 #2 #3]",
                expected_error: false,
            },
            TestCase {
                name: "success_without_space",
                input: "tags:[#1#2#3]",
                expected_error: false,
            },
            TestCase {
                name: "success_with_newline",
                input: "tags:\n[\n\t#1\n\t#2\n\t#3\n]",
                expected_error: false,
            },
            TestCase {
                name: "fail_no_brackets",
                input: "tags:#1#2#3",
                expected_error: true,
            },
            TestCase {
                name: "fail_no_tags",
                input: "[#1#2#3]",
                expected_error: true,
            },
            TestCase {
                name: "fail_wrong_tag",
                input: "tags:[##]",
                expected_error: true,
            },
        ];

        test_cases.iter().for_each(|test_case| {
            println!("Testing {}", test_case.name);

            let res = TaglineParser::parse(Rule::tagline, test_case.input);
            if res.is_err() {
                assert!(test_case.expected_error);
            } else {
                assert!(!test_case.expected_error);
            }
        })
    }
}
