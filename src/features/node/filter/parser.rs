use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

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

// ---------------------------------------------------------------------------
// Column validation helpers
// ---------------------------------------------------------------------------

/// Build the set of valid column names from the current table header,
/// normalized so matching is case/format-insensitive.
fn valid_columns(header: &[String]) -> HashSet<String> {
    header.iter().map(|h| normalize_column_name(h)).collect()
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Parse a Node-filter input string into a `TableFilterPredicate`.
///
/// `header` is the table's current display header. Column references are
/// validated (case/format-insensitive) against it; a column not in the current
/// view produces a parse error. When `header` is empty (e.g. before the first
/// poll populates the table) column validation is skipped.
///
/// Values may be quoted (`"..."` / `'...'`) with the same escape rules as the
/// Pod log query parser: `\"` → `"`  `\'` → `'`  `\\` → `\`  `\<other>` verbatim.
pub fn parse_node_filter(
    input: &str,
    header: &[String],
) -> Result<TableFilterPredicate, String> {
    let valid = valid_columns(header);
    let validate = !header.is_empty();

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
                let col = normalize_column_name(&column);
                if validate && !valid.contains(&col) {
                    return Err(format!("column '{}' is not in the current view", column));
                }
                let rx =
                    Regex::new(&value).map_err(|e| format!("invalid regex '{}': {}", value, e))?;
                column_includes.entry(col).or_default().push(rx);
            }
            Term::Exclude { column, value } => {
                let col = normalize_column_name(&column);
                if validate && !valid.contains(&col) {
                    return Err(format!("column '{}' is not in the current view", column));
                }
                let rx =
                    Regex::new(&value).map_err(|e| format!("invalid regex '{}': {}", value, e))?;
                column_excludes.entry(col).or_default().push(rx);
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

    fn header() -> Vec<String> {
        [
            "NAME",
            "STATUS",
            "ROLES",
            "AGE",
            "VERSION",
            "INTERNAL-IP",
            "EXTERNAL-IP",
            "OS-IMAGE",
            "KERNEL-VERSION",
            "CONTAINER-RUNTIME",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    #[test]
    fn empty_input_yields_empty_predicate() {
        let p = parse_node_filter("", &header()).unwrap();
        assert!(p.column_includes.is_empty());
        assert!(p.column_excludes.is_empty());
        assert_eq!(p.label_selector, None);
        assert_eq!(p.raw, "");
    }

    #[test]
    fn whitespace_only_input_yields_empty_predicate() {
        let p = parse_node_filter("   \t  ", &header()).unwrap();
        assert!(p.column_includes.is_empty());
        assert_eq!(p.raw, "");
    }

    #[test]
    fn single_bare_value_becomes_name_include() {
        let p = parse_node_filter("worker", &header()).unwrap();
        assert_eq!(p.column_includes.len(), 1);
        let patterns = p.column_includes.get("name").expect("name column");
        assert_eq!(patterns.len(), 1);
        assert!(patterns[0].is_match("gke-worker-1"));
        assert!(!patterns[0].is_match("gke-control-1"));
        assert_eq!(p.raw, "worker");
    }

    #[test]
    fn multiple_bare_values_become_name_or() {
        let p = parse_node_filter("foo bar", &header()).unwrap();
        let patterns = p.column_includes.get("name").expect("name column");
        assert_eq!(patterns.len(), 2);
        assert_eq!(p.raw, "foo bar");
    }

    #[test]
    fn explicit_column_include_creates_column_entry() {
        let p = parse_node_filter("status:Ready", &header()).unwrap();
        assert_eq!(p.column_includes.len(), 1);
        let patterns = p.column_includes.get("status").expect("status column");
        assert_eq!(patterns.len(), 1);
        assert!(patterns[0].is_match("Ready"));
        assert_eq!(p.raw, "status:Ready");
    }

    #[test]
    fn column_names_are_case_insensitive_canonicalized_lowercase() {
        let p = parse_node_filter("STATUS:Ready Name:worker", &header()).unwrap();
        assert!(p.column_includes.contains_key("status"));
        assert!(p.column_includes.contains_key("name"));
    }

    #[test]
    fn same_column_includes_accumulate_in_order() {
        let p = parse_node_filter("status:Ready status:Pending", &header()).unwrap();
        let patterns = p.column_includes.get("status").expect("status column");
        assert_eq!(patterns.len(), 2);
        assert!(patterns[0].is_match("Ready"));
        assert!(patterns[1].is_match("Pending"));
    }

    #[test]
    fn different_columns_coexist_in_predicate() {
        let p = parse_node_filter("status:Ready name:worker", &header()).unwrap();
        assert_eq!(p.column_includes.len(), 2);
    }

    #[test]
    fn bare_and_column_includes_mix() {
        // `foo status:Ready` → NAME has `foo`, STATUS has `Ready`
        let p = parse_node_filter("foo status:Ready", &header()).unwrap();
        assert_eq!(p.column_includes.len(), 2);
        assert_eq!(p.column_includes.get("name").unwrap().len(), 1);
        assert_eq!(p.column_includes.get("status").unwrap().len(), 1);
    }

    #[test]
    fn excludes_prefixed_with_bang_populate_column_excludes() {
        let p = parse_node_filter("!name:kube-system", &header()).unwrap();
        assert!(p.column_includes.is_empty());
        let patterns = p.column_excludes.get("name").expect("name column");
        assert_eq!(patterns.len(), 1);
        assert!(patterns[0].is_match("kube-system"));
    }

    #[test]
    fn includes_and_excludes_coexist() {
        let p = parse_node_filter("status:Ready !name:kube-system", &header()).unwrap();
        assert_eq!(p.column_includes.len(), 1);
        assert_eq!(p.column_excludes.len(), 1);
    }

    #[test]
    fn bang_without_colon_is_treated_as_bare_value() {
        // `!worker` は `!name:worker` の省略形ではない。bang は明示的な column と組でのみ意味を持つ。
        let p = parse_node_filter("!worker", &header()).unwrap();
        // 文字列 `!worker` がそのまま NAME 列の regex になる。regex crate は `!worker` をリテラル `!worker` のマッチとして受け入れる。
        let patterns = p.column_includes.get("name").expect("name column");
        assert_eq!(patterns.len(), 1);
        assert!(patterns[0].is_match("!worker"));
        assert!(p.column_excludes.is_empty());
    }

    #[test]
    fn label_selector_is_captured_verbatim() {
        let p = parse_node_filter("label:role=worker", &header()).unwrap();
        assert_eq!(p.label_selector.as_deref(), Some("role=worker"));
        assert!(p.column_includes.is_empty());
        assert!(p.column_excludes.is_empty());
    }

    #[test]
    fn label_selector_supports_kubectl_comma_and() {
        let p = parse_node_filter("label:role=worker,zone=us-west", &header()).unwrap();
        assert_eq!(
            p.label_selector.as_deref(),
            Some("role=worker,zone=us-west")
        );
    }

    #[test]
    fn multiple_label_terms_keep_the_last() {
        // The k8s API accepts only one labelSelector value; spec requires
        // last-wins to match the Pod log query convention.
        let p = parse_node_filter("label:a=1 label:b=2", &header()).unwrap();
        assert_eq!(p.label_selector.as_deref(), Some("b=2"));
    }

    #[test]
    fn label_and_column_terms_coexist() {
        let p = parse_node_filter(
            "status:Ready label:role=worker !name:kube-system",
            &header(),
        )
        .unwrap();
        assert_eq!(p.column_includes.len(), 1);
        assert_eq!(p.column_excludes.len(), 1);
        assert_eq!(p.label_selector.as_deref(), Some("role=worker"));
    }

    #[test]
    fn unknown_column_produces_parse_error() {
        let err = parse_node_filter("statusu:Ready", &header()).unwrap_err();
        assert!(
            err.contains("not in the current view") && err.contains("statusu"),
            "error should explain the column is not shown: {}",
            err
        );
    }

    #[test]
    fn unknown_column_in_exclude_also_errors() {
        let err = parse_node_filter("!agee:1h", &header()).unwrap_err();
        assert!(
            err.contains("not in the current view") && err.contains("agee"),
            "error should explain the column is not shown: {}",
            err
        );
    }

    #[test]
    fn builtin_columns_are_accepted() {
        // `name` and `status` are builtin headers — must not error.
        assert!(parse_node_filter("name:n status:s", &header()).is_ok());
    }

    #[test]
    fn header_column_is_accepted() {
        let mut h = header();
        h.push("ZONE".to_string());
        let p = parse_node_filter("zone:us-west", &h).unwrap();
        assert!(p.column_includes.contains_key("zone"));
    }

    #[test]
    fn label_keyword_is_not_treated_as_a_column_lookup() {
        // 'label:role=worker' must NOT trigger unknown-column validation
        // (it's the special-cased k8s labelSelector path).
        assert!(parse_node_filter("label:role=worker", &header()).is_ok());
    }

    #[test]
    fn multiword_column_with_space_is_filterable_via_compact_token() {
        // 課題 I: a header column with a space (e.g. NOMINATED NODE) is
        // addressable via a compact token, stored under its normalized key.
        let h = vec!["NAME".to_string(), "NOMINATED NODE".to_string()];
        let p = parse_node_filter("nominatednode:foo", &h).unwrap();
        assert!(p.column_includes.contains_key("nominatednode"));
    }

    #[test]
    fn column_not_in_header_produces_not_in_view_error() {
        // 課題 II: VERSION is a real builtin name, but with a header that omits
        // it, filtering on it must error rather than hide all rows.
        let h = vec!["NAME".to_string(), "STATUS".to_string()];
        let err = parse_node_filter("version:1.2", &h).unwrap_err();
        assert!(
            err.contains("not in the current view") && err.contains("version"),
            "error should explain the column is not shown: {}",
            err
        );
    }

    #[test]
    fn empty_header_skips_column_validation() {
        // Before the first poll the header may be empty; don't reject valid input.
        let p = parse_node_filter("status:Ready", &[]).unwrap();
        assert!(p.column_includes.contains_key("status"));
    }

    // -----------------------------------------------------------------------
    // New quoting / escape tests (Task 12)
    // -----------------------------------------------------------------------

    #[test]
    fn double_quoted_value_with_spaces_is_kept_intact() {
        let p = parse_node_filter(r#"os-image:"Ubuntu 22.04.3 LTS""#, &header()).unwrap();
        let patterns = p.column_includes.get("osimage").expect("osimage col");
        assert_eq!(patterns.len(), 1);
        assert!(patterns[0].is_match("Ubuntu 22.04.3 LTS"));
    }

    #[test]
    fn single_quoted_value_with_spaces_is_kept_intact() {
        let p = parse_node_filter(r#"os-image:'Ubuntu 22.04 LTS'"#, &header()).unwrap();
        let patterns = p.column_includes.get("osimage").unwrap();
        assert!(patterns[0].is_match("Ubuntu 22.04 LTS"));
    }

    #[test]
    fn quoted_value_with_escaped_quote() {
        let p = parse_node_filter(r#"name:"foo\"bar""#, &header()).unwrap();
        let patterns = p.column_includes.get("name").unwrap();
        // 値は `foo"bar` という regex
        assert!(patterns[0].is_match(r#"foo"bar"#));
    }

    #[test]
    fn quoted_value_preserves_regex_backslash_classes() {
        // `\s` をリテラルに残して regex `\s`（空白）になる
        let p = parse_node_filter(r#"name:"foo\sbar""#, &header()).unwrap();
        let patterns = p.column_includes.get("name").unwrap();
        assert!(patterns[0].is_match("foo bar")); // regex \s が空白マッチ
        assert!(!patterns[0].is_match("foobar"));
    }

    #[test]
    fn bare_value_with_quoted_spaces() {
        // bare の場合も quoted value をサポート: "node a" → NAME に regex "node a"
        let p = parse_node_filter(r#""node a""#, &header()).unwrap();
        let patterns = p.column_includes.get("name").unwrap();
        assert!(patterns[0].is_match("node a"));
    }

    #[test]
    fn mixed_quoted_and_unquoted_tokens() {
        let p =
            parse_node_filter(r#"status:Ready os-image:"Ubuntu 22.04""#, &header()).unwrap();
        assert_eq!(p.column_includes.len(), 2);
        assert!(p.column_includes.get("osimage").unwrap()[0].is_match("Ubuntu 22.04"));
    }

    #[test]
    fn unclosed_quote_is_a_parse_error() {
        assert!(parse_node_filter(r#"name:"unterminated"#, &header()).is_err());
    }
}
