mod decoder;
mod feedback_tests;
mod test_token;
mod unescape;

pub use self::unescape::Unescape;
use super::for_each_test_file;
use crate::harness::Input;
use crate::harness::ASCII_COMPATIBLE_ENCODINGS;
use serde_json::{self, from_reader};
use std::fmt::Write;

pub use self::test_token::{TestToken, TestTokenList};

pub fn default_initial_states() -> Vec<String> {
    vec![String::from("Data state")]
}

#[derive(Deserialize, Default, Clone, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Bailout {
    pub reason: String,
    pub parsed_chunk: String,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TestCase {
    pub description: String,
    pub input: Input,

    #[serde(rename = "output")]
    pub expected_tokens: Vec<TestToken>,

    #[serde(default = "default_initial_states")]
    pub initial_states: Vec<String>,

    #[serde(default)]
    pub double_escaped: bool,

    #[serde(default)]
    pub last_start_tag: String,

    #[serde(skip)]
    pub expected_bailout: Option<Bailout>,
}

impl Unescape for TestCase {
    fn unescape(&mut self) -> Result<(), serde_json::error::Error> {
        if self.double_escaped {
            self.double_escaped = false;
            self.input.unescape()?;

            for token in &mut self.expected_tokens {
                token.unescape()?;
            }
        }

        Ok(())
    }
}

pub fn get_test_cases() -> Vec<TestCase> {
    let mut tests = Vec::default();

    #[derive(Deserialize)]
    struct Suite {
        #[serde(default)]
        pub tests: Vec<TestCase>,
    }

    for_each_test_file("html5lib-tests/tokenizer/*.test", &mut |file| {
        tests.extend(from_reader::<_, Suite>(file).unwrap().tests);
    });

    tests.append(&mut self::feedback_tests::get_test_cases());

    tests
        .iter_mut()
        .filter_map(|t| {
            if t.unescape().is_err() {
                println!(
                    "Ignoring test due to input unescape failure: `{}`",
                    t.description
                );
                None
            } else {
                Some(t)
            }
        })
        .fold(Vec::default(), |mut cases, t| {
            let mut encoding_variations = ASCII_COMPATIBLE_ENCODINGS
                .iter()
                .filter_map(|encoding| {
                    let mut t = t.to_owned();

                    match t.input.init(encoding, false) {
                        Ok(chunk_size) => {
                            let mut new_descr = String::new();

                            write!(
                                &mut new_descr,
                                "`{}` (Encoding: {}, Chunk size: {})",
                                t.description,
                                encoding.name(),
                                chunk_size,
                            )
                            .unwrap();

                            t.description = new_descr;

                            Some(t)
                        }
                        Err(_) => {
                            println!(
                                "Ignoring test for {} encoding due to unmappable characters: `{}`",
                                encoding.name(),
                                t.description,
                            );
                            None
                        }
                    }
                })
                .collect();

            cases.append(&mut encoding_variations);

            cases
        })
}