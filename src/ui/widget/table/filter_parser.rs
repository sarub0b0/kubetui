//! Shared column-aware filter parser core.
//!
//! Tokenizer + quoting + `Term` + `parse_table_filter`, used by every
//! column-aware tab filter (Node now; Pod/Config/Network later). The only
//! tab-specific part is the `validate_column` closure passed by the caller.

// The private helpers (quoted, unquoted, value_string, column_name, Term,
// parse_token) are only reachable via parse_table_filter, which is re-exported
// but not yet consumed outside of tests (Task 2 wires actual callers).
#![allow(dead_code)]

use std::borrow::Cow;
use std::collections::HashMap;

use nom::{
    branch::alt,
    bytes::complete::{is_not, tag},
    character::complete::{anychar, char, multispace0, multispace1},
    combinator::{map, value, verify},
    error::{ContextError, ParseError},
    multi::{fold_many0, separated_list0},
    sequence::{delimited, preceded},
    IResult,
    Parser,
};
use regex::Regex;

use crate::ui::widget::{normalize_column_name, TableFilterPredicate};

// ---------------------------------------------------------------------------
// Quoting helpers (copied from pod/kube/filter/parser.rs to avoid cross-feature dep)
// ---------------------------------------------------------------------------

/// Parse a quoted string (`"..."` or `'...'`), handling escape sequences.
///
/// Escape rules inside quotes:
///   `\"` → `"`   `\'` → `'`   `\\` → `\`   `\<other>` → `\<other>` (verbatim)
fn quoted<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, Cow<'a, str>, E> {
    #[inline]
    fn multispace<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    ) -> impl Parser<&'a str, Output = Cow<'a, str>, Error = E> {
        map(multispace1, Cow::Borrowed)
    }

    #[inline]
    fn escaped_char<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    ) -> impl Parser<&'a str, Output = Cow<'a, str>, Error = E> {
        preceded(
            char('\\'),
            alt((
                value(Cow::Borrowed("\""), char('"')),
                value(Cow::Borrowed("'"), char('\'')),
                value(Cow::Borrowed("\\"), char('\\')),
                map(anychar, |c| Cow::Owned(format!(r"\{}", c))),
            )),
        )
    }

    #[inline]
    fn not_quote_slash<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
        quote_slash: &'a str,
    ) -> impl Parser<&'a str, Output = Cow<'a, str>, Error = E> {
        map(
            verify(is_not(quote_slash), |s: &str| !s.is_empty()),
            Cow::Borrowed,
        )
    }

    #[inline]
    fn fold<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
        parser: impl Parser<&'a str, Output = Cow<'a, str>, Error = E>,
    ) -> impl Parser<&'a str, Output = String, Error = E> {
        fold_many0(parser, String::default, |mut s, parsed| {
            s.push_str(&parsed);
            s
        })
    }

    let double_quoted = delimited(
        char('"'),
        fold(alt((
            escaped_char(),
            not_quote_slash(r#""\"#),
            multispace(),
        ))),
        char('"'),
    );

    let single_quoted = delimited(
        char('\''),
        fold(alt((
            escaped_char(),
            not_quote_slash(r#"\'"#),
            multispace(),
        ))),
        char('\''),
    );

    let (remaining, value) = alt((double_quoted, single_quoted)).parse(s)?;

    Ok((remaining, Cow::Owned(value)))
}

/// Parse an unquoted value: any non-whitespace characters that do not start
/// with a quote character. Mirrors `non_space` in the pod filter parser.
fn unquoted<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, Cow<'a, str>, E> {
    let (remaining, value) =
        verify(is_not(" \t\r\n"), |s: &str| !s.starts_with(['"', '\''])).parse(s)?;
    Ok((remaining, Cow::Borrowed(value)))
}

/// Parse a value that may be quoted or unquoted.
fn value_string<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, String, E> {
    map(alt((quoted, unquoted)), |c| c.into_owned()).parse(s)
}

/// Parse a column name token: non-empty, stops at whitespace, `:`, `!`, or quote chars.
fn column_name<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, &'a str, E> {
    verify(is_not(" \t\r\n:!\"'"), |s: &str| !s.is_empty()).parse(s)
}

// ---------------------------------------------------------------------------
// Term types and token parser
// ---------------------------------------------------------------------------

/// One parsed term from the input.
#[derive(Debug)]
enum Term {
    /// Bare value (no prefix) → defaults to NAME include.
    Bare(String),
    /// `<col>:<value>` include.
    Include { column: String, value: String },
    /// `!<col>:<value>` exclude.
    Exclude { column: String, value: String },
    /// `label:<selector>` → passed verbatim to the k8s API as labelSelector.
    Label(String),
}

/// Parse one whitespace-delimited token into a `Term`.
///
/// Priority order:
///   1. `label:<value>` → Label (special-cased before generic Include)
///   2. `!<col>:<value>` → Exclude
///   3. `<col>:<value>` → Include
///   4. `<value>` → Bare
///
/// For (3)/(4) the value may be quoted (`"..."` / `'...'`) or unquoted.
fn parse_token<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, Term, E> {
    // 1. label: (must be tried before the generic include path)
    let label_result = preceded(tag("label:"), value_string::<E>).parse(s);
    if let Ok((rem, sel)) = label_result {
        return Ok((rem, Term::Label(sel)));
    }

    // 2. !col:value  (exclude)
    let exclude_result = preceded(char::<&'a str, E>('!'), |s: &'a str| {
        // We need col:value after the `!`
        let (s2, col) = column_name::<E>(s)?;
        let (s3, _) = char::<&'a str, E>(':').parse(s2)?;
        let (s4, val) = value_string::<E>(s3)?;
        Ok((s4, (col.to_lowercase(), val)))
    })
    .parse(s);
    if let Ok((rem, (col, val))) = exclude_result {
        return Ok((
            rem,
            Term::Exclude {
                column: col,
                value: val,
            },
        ));
    }

    // 3. col:value  (include) — but only if input is NOT starting with a quote
    //    (otherwise `"bare quoted"` would fail column_name and fall to Bare correctly)
    if !s.starts_with(['"', '\'']) {
        // Try to parse col:value.  If we successfully match `col:` followed by a
        // quote character, we MUST parse it as a quoted value — do not fall through
        // to Bare, which would silently read the whole token as unquoted.
        let col_colon_result = (|s: &'a str| -> IResult<&'a str, (&'a str, &'a str), E> {
            let (s2, col) = column_name::<E>(s)?;
            let (s3, _) = char::<&'a str, E>(':').parse(s2)?;
            Ok((s3, (s, col)))
        })(s);

        if let Ok((after_colon, (_orig, col))) = col_colon_result {
            // We have `col:` consumed.  Now parse the value — this will fail hard
            // if the value is an unclosed quote (nom Err::Error propagated).
            let (rem, val) = value_string::<E>(after_colon)?;
            return Ok((
                rem,
                Term::Include {
                    column: col.to_lowercase(),
                    value: val,
                },
            ));
        }
    }

    // 4. Bare value (quoted or unquoted)
    let (rem, val) = value_string::<E>(s)?;
    Ok((rem, Term::Bare(val)))
}

/// Parse a column-aware filter string into a `TableFilterPredicate`.
///
/// `validate_column` is called with each `COL:`/`!COL:` column token (already
/// lowercased by the tokenizer) and returns `Ok(())` if acceptable, or
/// `Err(message)` to abort parsing with that message — the only tab-specific
/// part. Stored predicate keys are normalized via `normalize_column_name`.
/// Bare values map to the `name` include; `label:` is captured verbatim (last
/// wins); values may be quoted with the log-query escape rules.
pub fn parse_table_filter(
    input: &str,
    validate_column: impl Fn(&str) -> Result<(), String>,
) -> Result<TableFilterPredicate, String> {
    let trimmed = input.trim();
    let mut column_includes: HashMap<String, Vec<Regex>> = HashMap::new();
    let mut column_excludes: HashMap<String, Vec<Regex>> = HashMap::new();
    let mut label_selector: Option<String> = None;

    if trimmed.is_empty() {
        return Ok(TableFilterPredicate {
            column_includes,
            column_excludes,
            label_selector,
            raw: trimmed.to_string(),
        });
    }

    // Parse the whole trimmed input as whitespace-separated tokens.
    type E<'a> = nom::error::Error<&'a str>;
    let parse_result = delimited(
        multispace0,
        separated_list0(multispace1, parse_token::<E>),
        multispace0,
    )
    .parse(trimmed);

    let (remaining, terms) = parse_result.map_err(|e| format!("parse error: {}", e))?;

    if !remaining.is_empty() {
        return Err(format!("unexpected input near: {:?}", remaining));
    }

    for term in terms {
        match term {
            Term::Bare(v) => {
                let rx = Regex::new(&v).map_err(|e| format!("invalid regex '{}': {}", v, e))?;
                column_includes
                    .entry("name".to_string())
                    .or_default()
                    .push(rx);
            }
            Term::Include { column, value } => {
                validate_column(&column)?;
                let rx =
                    Regex::new(&value).map_err(|e| format!("invalid regex '{}': {}", value, e))?;
                column_includes
                    .entry(normalize_column_name(&column))
                    .or_default()
                    .push(rx);
            }
            Term::Exclude { column, value } => {
                validate_column(&column)?;
                let rx =
                    Regex::new(&value).map_err(|e| format!("invalid regex '{}': {}", value, e))?;
                column_excludes
                    .entry(normalize_column_name(&column))
                    .or_default()
                    .push(rx);
            }
            Term::Label(sel) => {
                // Last label: term wins (k8s API accepts only one labelSelector value).
                label_selector = Some(sel);
            }
        }
    }

    Ok(TableFilterPredicate {
        column_includes,
        column_excludes,
        label_selector,
        raw: trimmed.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn allow_all(_: &str) -> Result<(), String> {
        Ok(())
    }

    #[test]
    fn empty_input_yields_empty_predicate() {
        let p = parse_table_filter("", allow_all).unwrap();
        assert!(p.column_includes.is_empty());
        assert!(p.column_excludes.is_empty());
        assert_eq!(p.label_selector, None);
        assert_eq!(p.raw, "");
    }

    #[test]
    fn bare_value_becomes_name_include() {
        let p = parse_table_filter("worker", allow_all).unwrap();
        assert!(p.column_includes.contains_key("name"));
    }

    #[test]
    fn include_and_exclude_store_normalized_keys() {
        let p = parse_table_filter("Status:Ready !INTERNAL-IP:10.", allow_all).unwrap();
        assert!(p.column_includes.contains_key("status"));
        assert!(p.column_excludes.contains_key("internalip"));
    }

    #[test]
    fn label_is_captured_last_wins() {
        let p = parse_table_filter("label:a=1 label:b=2", allow_all).unwrap();
        assert_eq!(p.label_selector.as_deref(), Some("b=2"));
    }

    #[test]
    fn quoted_value_with_whitespace() {
        let p = parse_table_filter(r#"name:"foo bar""#, allow_all).unwrap();
        let patterns = p.column_includes.get("name").unwrap();
        assert!(patterns[0].is_match("foo bar"));
    }

    #[test]
    fn validate_column_error_aborts_parse() {
        let reject_status = |c: &str| {
            if c == "status" {
                Err("nope".to_string())
            } else {
                Ok(())
            }
        };
        let err = parse_table_filter("status:Ready", reject_status).unwrap_err();
        assert_eq!(err, "nope");
    }

    #[test]
    fn invalid_regex_errors() {
        let err = parse_table_filter("name:[", allow_all).unwrap_err();
        assert!(err.contains("invalid regex"), "got: {}", err);
    }
}
