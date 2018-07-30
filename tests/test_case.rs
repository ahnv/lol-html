use serde_json;
use super::unescape::Unescape;
use cool_thing::{LexResult, Token, Tokenizer};
use token::TokenDef;

#[derive(Clone, Copy, Deserialize, Debug)]
pub enum InitialState {
    #[serde(rename = "Data state")]
    Data,
    #[serde(rename = "PLAINTEXT state")]
    PlainText,
    #[serde(rename = "RCDATA state")]
    RCData,
    #[serde(rename = "RAWTEXT state")]
    RawText,
    #[serde(rename = "Script data state")]
    ScriptData,
    #[serde(rename = "CDATA section state")]
    CDataSection,
}

impl InitialState {
    fn to_tokenizer_state<'t, H: FnMut(LexResult)>(self) -> fn(&mut Tokenizer<'t, H>, Option<u8>) {
        match self {
            InitialState::Data => Tokenizer::data_state,
            InitialState::PlainText => Tokenizer::plaintext_state,
            InitialState::RCData => Tokenizer::rcdata_state,
            InitialState::RawText => Tokenizer::rawtext_state,
            InitialState::ScriptData => Tokenizer::script_data_state,
            InitialState::CDataSection => Tokenizer::cdata_section_state,
        }
    }
}

fn default_initial_states() -> Vec<InitialState> {
    vec![InitialState::Data]
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestCase {
    pub description: String,
    pub input: String,

    #[serde(rename = "output", with = "TokenDef")]
    pub expected_tokens: Vec<Token>,

    #[serde(skip)]
    pub ignored: bool,

    #[serde(default = "default_initial_states")]
    pub initial_states: Vec<InitialState>,

    #[serde(default)]
    pub double_escaped: bool,

    #[serde(default)]
    pub last_start_tag: String,
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

impl TestCase {
    pub fn init(&mut self) {
        self.ignored = self.unescape().is_err();

        // NOTE: tokenizer should always produce EOF token
        self.expected_tokens.push(Token::Eof);
    }

    pub fn run(&self) {
        for &cs in &self.initial_states {
            let mut actual_tokens = Vec::new();

            {
                let mut tokenizer = Tokenizer::new(2048, |lex_res: LexResult| {
                    let token: Token = lex_res.into();
                    let mut is_consequent_char = false;

                    if let (&Token::Character(ref cs), Some(&mut Token::Character(ref mut ps))) =
                        (&token, actual_tokens.last_mut())
                    {
                        *ps += cs;
                        is_consequent_char = true;
                    }

                    if !is_consequent_char {
                        actual_tokens.push(token);
                    }
                });

                tokenizer.set_state(cs.to_tokenizer_state());

                tokenizer
                    .write(self.input.bytes().collect())
                    .expect("Tokenizer buffer capacity exceeded");
            }

            assert!(
                self.expected_tokens == actual_tokens,
                "Token mismatch\n\
                 state: {:?}\n\
                 input: {:?}\n\
                 actual: {:#?}\n\
                 expected: {:#?}",
                cs,
                self.input,
                actual_tokens,
                self.expected_tokens
            );
        }
    }
}