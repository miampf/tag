pub mod onfile {
    use pest_derive::Parser;

    #[derive(Parser)]
    #[grammar = "tagline.pest"]
    /// `TaglineParser` is responsible for parsing the taglines at the start of each searched file.
    /// The relevant rule is `tagline`.
    pub struct TaglineParser;
}

pub mod searchquery {
    use pest::{iterators::Pairs, pratt_parser::PrattParser};
    use pest_derive::Parser;

    /// Expr represents an AST for a search query.
    #[derive(Debug, PartialEq, Clone)]
    pub enum Expr {
        Bool(bool),
        UnaryNot(Box<Expr>),
        Operation {
            lhs: Box<Expr>,
            op: Op,
            rhs: Box<Expr>,
        },
    }

    /// Op is an Operation that can be used in a query.
    #[derive(Debug, PartialEq, Eq, Clone)]
    pub enum Op {
        And,
        Or,
    }

    lazy_static::lazy_static! {
        static ref PRATT_PARSER: PrattParser<Rule> = {
            use pest::pratt_parser::{Assoc::Left, Op};
            use Rule::{and, or, unary_not};

            PrattParser::new()
                // & and | are evaluated with the same precedence
                .op(Op::infix(and, Left) | Op::infix(or, Left))
                .op(Op::prefix(unary_not))
        };
    }

    #[derive(Parser)]
    #[grammar = "query.pest"]
    /// `QueryParser` is responsible for parsing the search query.
    /// The relevant rule is `tagsearch`.
    pub struct QueryParser;

    /// `construct_query_ast()` creates an AST from a string of symbols
    /// lexed by the `QueryParser` and a list of tags.
    #[must_use]
    pub fn construct_query_ast(pairs: Pairs<Rule>, tags: &Vec<&str>) -> Expr {
        PRATT_PARSER
            .map_primary(|primary| match primary.as_rule() {
                Rule::tag => Expr::Bool(tags.contains(&primary.as_str().trim())),
                Rule::expr => construct_query_ast(primary.into_inner(), tags),
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
            .map_prefix(|op, rhs| match op.as_rule() {
                Rule::unary_not => Expr::UnaryNot(Box::new(rhs)),
                rule => unreachable!("Expected unary not, found {:?}", rule),
            })
            .parse(pairs)
    }

    /// `evaluate_ast()` evaluates an AST created by `construct_query_ast()`
    /// and returns the result.
    #[must_use]
    pub fn evaluate_ast(ast: Expr) -> bool {
        match ast {
            Expr::Bool(value) => value,
            Expr::UnaryNot(expr) => !evaluate_ast(*expr),
            Expr::Operation { lhs, op, rhs } => {
                let left = evaluate_ast(*lhs);
                let right = evaluate_ast(*rhs);
                match op {
                    Op::Or => left | right,
                    Op::And => left & right,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parsers::searchquery::construct_query_ast;
    use crate::parsers::searchquery::evaluate_ast;
    use crate::parsers::searchquery::Expr;
    use crate::parsers::searchquery::Op;
    use crate::parsers::searchquery::QueryParser;

    use super::onfile;
    use super::searchquery;

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

        for test_case in test_cases {
            println!("test_tagline_parser: \n\t{}", test_case.name);

            let res = onfile::TaglineParser::parse(onfile::Rule::tagline, test_case.input);
            if res.is_err() {
                assert!(test_case.expected_error);
                return;
            }

            assert!(!test_case.expected_error);

            for (i, tag) in res.unwrap().enumerate() {
                if tag.as_rule() == onfile::Rule::tag {
                    assert_eq!(tag.as_str().trim(), test_case.expected_tags[i]);
                }
            }
        }
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
                input: "#a & !#b",
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

        for test_case in test_cases {
            println!("test_query_parser: \n\t{}", test_case.name);

            let res =
                searchquery::QueryParser::parse(searchquery::Rule::tagsearch, test_case.input);
            if res.is_err() {
                assert!(test_case.expected_error);
                return;
            }

            assert!(!test_case.expected_error);

            assert_eq!(test_case.input, res.unwrap().as_str());
        }
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
                input_query: "#a & #b | (!#c & #d)",
                input_tags: vec!["#c".to_string(), "#d".to_string()],
                expected_ast: Expr::Operation {
                    lhs: Box::new(Expr::Operation {
                        lhs: Box::new(Expr::Bool(false)),
                        op: Op::And,
                        rhs: Box::new(Expr::Bool(false)),
                    }),
                    op: Op::Or,
                    rhs: Box::new(Expr::Operation {
                        lhs: Box::new(Expr::UnaryNot(Box::new(Expr::Bool(true)))),
                        op: Op::And,
                        rhs: Box::new(Expr::Bool(true)),
                    }),
                },
            },
        ];

        for test_case in test_cases {
            println!("test_construct_query_ast: \n\t{}", test_case.name);

            let ast = construct_query_ast(
                QueryParser::parse(searchquery::Rule::tagsearch, test_case.input_query)
                    .unwrap()
                    .next()
                    .unwrap()
                    .into_inner(),
                &test_case
                    .input_tags
                    .iter()
                    .map(std::string::String::as_str)
                    .collect(),
            );

            assert_eq!(test_case.expected_ast, ast);
        }
    }

    #[test]
    fn test_evaluate_ast() {
        struct TestCase<'a> {
            name: &'a str,
            input_ast: Expr,
            expected_result: bool,
        }

        let test_cases = [
            TestCase {
                name: "success_flat",
                input_ast: Expr::Operation {
                    lhs: Box::new(Expr::Bool(true)),
                    op: Op::And,
                    rhs: Box::new(Expr::Bool(true)),
                },
                expected_result: true,
            },
            TestCase {
                name: "success_nested",
                input_ast: Expr::Operation {
                    lhs: Box::new(Expr::Operation {
                        lhs: Box::new(Expr::Bool(false)),
                        op: Op::And,
                        rhs: Box::new(Expr::Bool(false)),
                    }),
                    op: Op::Or,
                    rhs: Box::new(Expr::UnaryNot(Box::new(Expr::Operation {
                        lhs: Box::new(Expr::Bool(true)),
                        op: Op::And,
                        rhs: Box::new(Expr::Bool(true)),
                    }))),
                },
                expected_result: false,
            },
        ];

        for test_case in test_cases {
            println!("test_evaluate_ast: \n\t{}", test_case.name);

            assert_eq!(
                test_case.expected_result,
                evaluate_ast(test_case.input_ast.clone())
            );
        }
    }
}
