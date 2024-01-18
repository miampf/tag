pub mod tagline {
    use pest_derive::Parser;

    #[derive(Parser)]
    #[grammar = "tagline.pest"]
    /// TaglineParser is responsible for parsing the taglines at the start of each searched file.
    /// The relevant rule is `tagline`.
    pub struct TaglineParser;
}

pub mod query {
    use pest::{iterators::Pairs, pratt_parser::PrattParser};
    use pest_derive::Parser;

    /// Expr represents an AST for a search query.
    #[derive(Debug, PartialEq)]
    pub enum Expr {
        Bool(bool),
        Operation {
            lhs: Box<Expr>,
            op: Op,
            rhs: Box<Expr>,
        },
    }

    /// Op is an Operation that can be used in a query.
    #[derive(Debug, PartialEq)]
    pub enum Op {
        And,
        Or,
    }

    lazy_static::lazy_static! {
        static ref PRATT_PARSER: PrattParser<Rule> = {
            use pest::pratt_parser::{Assoc::*, Op};
            use Rule::*;

            PrattParser::new()
                // & and | are evaluated with the same precedence
                .op(Op::infix(and, Left) | Op::infix(or, Left))
        };
    }

    #[derive(Parser)]
    #[grammar = "query.pest"]
    /// QueryParser is responsible for parsing the search query.
    /// The relevant rule is `tagsearch`.
    pub struct QueryParser;

    /// construct_query_ast() creates an AST from a string of symbols
    /// lexed by the QueryParser and a list of tags.
    pub fn construct_query_ast(pairs: Pairs<Rule>, tags: Vec<&str>) -> Expr {
        PRATT_PARSER
            .map_primary(|primary| match primary.as_rule() {
                Rule::tag => Expr::Bool(tags.contains(&primary.as_str().trim())),
                Rule::expr => construct_query_ast(primary.into_inner(), tags.clone()),
                rule => unreachable!("Expected tag, found {:?}", rule),
            })
            .map_infix(|lhs, op, rhs| {
                let op = match op.as_rule() {
                    Rule::or => Op::Or,
                    Rule::and => Op::And,
                    rule => unreachable!("Expected operation, found {:?}", rule),
                };

                Expr::Operation {
                    lhs: Box::new(lhs),
                    op,
                    rhs: Box::new(rhs),
                }
            })
            .parse(pairs)
    }
}

#[cfg(test)]
mod tests {
    use crate::parsers::query::construct_query_ast;
    use crate::parsers::query::Expr;
    use crate::parsers::query::Op;
    use crate::parsers::query::QueryParser;

    use super::query;
    use super::tagline;

    use pest::Parser;

    #[test]
    fn test_tagline_parser() {
        struct TestCase<'a> {
            name: &'a str,
            input: &'a str,
            expected_tags: Vec<&'a str>,
            expected_error: bool,
        }

        let test_cases = [
            TestCase {
                name: "success_with_space",
                input: "tags: [#1 #2 #3]",
                expected_tags: vec!["#1", "#2", "#3"],
                expected_error: false,
            },
            TestCase {
                name: "success_without_space",
                input: "tags:[#1#asdf#something-idk]",
                expected_tags: vec!["#1", "#asdf", "#something-idk"],
                expected_error: false,
            },
            TestCase {
                name: "fail_no_brackets",
                input: "tags:#1#2#3",
                expected_tags: vec![],
                expected_error: true,
            },
            TestCase {
                name: "fail_no_tags",
                input: "[#1#2#3]",
                expected_tags: vec![],
                expected_error: true,
            },
            TestCase {
                name: "fail_wrong_tag",
                input: "tags:[##]",
                expected_tags: vec![],
                expected_error: true,
            },
        ];

        test_cases.iter().for_each(|test_case| {
            println!("test_tagline_parser: \n\t{}", test_case.name);

            let res = tagline::TaglineParser::parse(tagline::Rule::tagline, test_case.input);
            if res.is_err() {
                assert!(test_case.expected_error);
                return;
            }

            assert!(!test_case.expected_error);

            for (i, tag) in res.unwrap().enumerate() {
                if tag.as_rule() == tagline::Rule::tag {
                    assert_eq!(tag.as_str().trim(), test_case.expected_tags[i]);
                }
            }
        })
    }

    #[test]
    fn test_query_parser() {
        struct TestCase<'a> {
            name: &'a str,
            input: &'a str,
            expected_error: bool,
        }

        let test_cases = [
            TestCase {
                name: "success_with_space",
                input: "#a & #b",
                expected_error: false,
            },
            TestCase {
                name: "success_without_space",
                input: "#a&#b",
                expected_error: false,
            },
            TestCase {
                name: "success_with_newline",
                input: "#a\n|\n#b",
                expected_error: false,
            },
            TestCase {
                name: "success_nested",
                input: "#a & (#b | #c)",
                expected_error: false,
            },
            TestCase {
                name: "fail_wrong_tag",
                input: "##",
                expected_error: true,
            },
            TestCase {
                name: "fail_no_following_tag",
                input: "#a &",
                expected_error: true,
            },
            TestCase {
                name: "fail_no_open_parentheses",
                input: "#a & #b)",
                expected_error: true,
            },
            TestCase {
                name: "fail_no_closing_parentheses",
                input: "(#a & #b",
                expected_error: true,
            },
        ];

        test_cases.iter().for_each(|test_case| {
            println!("test_query_parser: \n\t{}", test_case.name);

            let res = query::QueryParser::parse(query::Rule::tagsearch, test_case.input);
            if res.is_err() {
                assert!(test_case.expected_error);
                return;
            }

            assert!(!test_case.expected_error);

            assert_eq!(test_case.input, res.unwrap().as_str())
        })
    }

    #[test]
    fn test_construct_query_ast() {
        struct TestCase<'a> {
            name: &'a str,
            input_query: &'a str,
            input_tags: Vec<String>,
            expected_ast: Expr,
        }

        let test_cases = [
            TestCase {
                name: "success_flat",
                input_query: "#a & #b",
                input_tags: vec![],
                expected_ast: Expr::Operation {
                    lhs: Box::new(Expr::Bool(false)),
                    op: Op::And,
                    rhs: Box::new(Expr::Bool(false)),
                },
            },
            TestCase {
                name: "success_nested",
                input_query: "#a & #b | (#c & #d)",
                input_tags: vec!["#c".to_string(), "#d".to_string()],
                expected_ast: Expr::Operation {
                    lhs: Box::new(Expr::Operation {
                        lhs: Box::new(Expr::Bool(false)),
                        op: Op::And,
                        rhs: Box::new(Expr::Bool(false)),
                    }),
                    op: Op::Or,
                    rhs: Box::new(Expr::Operation {
                        lhs: Box::new(Expr::Bool(true)),
                        op: Op::And,
                        rhs: Box::new(Expr::Bool(true)),
                    }),
                },
            },
        ];

        test_cases.iter().for_each(|test_case| {
            println!("test_construct_query_ast: \n\t{}", test_case.name);

            let ast = construct_query_ast(
                QueryParser::parse(query::Rule::tagsearch, test_case.input_query)
                    .unwrap()
                    .next()
                    .unwrap()
                    .into_inner(),
                test_case
                    .input_tags
                    .iter()
                    .map(|tag| tag.as_str())
                    .collect(),
            );

            assert_eq!(test_case.expected_ast, ast);
        })
    }
}
