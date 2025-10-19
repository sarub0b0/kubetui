use std::borrow::Cow;

use nom::{
    branch::alt,
    bytes::complete::{is_not, tag},
    character::complete::{alphanumeric1, anychar, char, multispace0, multispace1},
    combinator::{all_consuming, map, recognize, value, verify},
    error::{ContextError, ParseError},
    multi::{fold_many0, many1_count, separated_list1},
    sequence::{delimited, preceded, separated_pair},
    IResult, Parser,
};

use super::{FilterAttribute, SpecifiedResource};

/// 空白文字を含まない文字列をパースする
fn non_space<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, Cow<'a, str>, E> {
    let (remaining, value) =
        verify(is_not(" \t\r\n"), |s: &str| !s.starts_with(['"', '\''])).parse(s)?;
    Ok((remaining, Cow::Borrowed(value)))
}

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

fn unquoted<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, Cow<'a, str>, E> {
    non_space(s)
}

fn regex<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, Cow<'a, str>, E> {
    alt((quoted, unquoted)).parse(s)
}

fn selector<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, Cow<'a, str>, E> {
    alt((quoted, unquoted)).parse(s)
}

fn jq_expr<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, Cow<'a, str>, E> {
    alt((quoted, unquoted)).parse(s)
}

fn resource_name<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, &'a str, E> {
    recognize(many1_count(alt((alphanumeric1, tag("-"), tag("."))))).parse(s)
}

fn pod<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute<'a>, E> {
    let (remaining, (_, value)) = separated_pair(
        alt((tag("pods"), tag("pod"), tag("po"), tag("p"))),
        char(':'),
        regex,
    )
    .parse(s)?;
    Ok((remaining, FilterAttribute::Pod(value)))
}

fn exclude_pod<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute<'a>, E> {
    let (remaining, (_, value)) = separated_pair(
        alt((tag("!pods"), tag("!pod"), tag("!po"), tag("!p"))),
        char(':'),
        regex,
    )
    .parse(s)?;
    Ok((remaining, FilterAttribute::ExcludePod(value)))
}

fn container<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute<'a>, E> {
    let (remaining, (_, value)) = separated_pair(
        alt((tag("containers"), tag("container"), tag("co"), tag("c"))),
        char(':'),
        regex,
    )
    .parse(s)?;
    Ok((remaining, FilterAttribute::Container(value)))
}

fn exclude_container<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute<'a>, E> {
    let (remaining, (_, value)) = separated_pair(
        alt((tag("!containers"), tag("!container"), tag("!co"), tag("!c"))),
        char(':'),
        regex,
    )
    .parse(s)?;
    Ok((remaining, FilterAttribute::ExcludeContainer(value)))
}

fn include_log<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute<'a>, E> {
    let (remaining, (_, value)) = separated_pair(
        alt((tag("logs"), tag("log"), tag("lo"), tag("l"))),
        char(':'),
        regex,
    )
    .parse(s)?;
    Ok((remaining, FilterAttribute::IncludeLog(value)))
}

fn exclude_log<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute<'a>, E> {
    let (remaining, (_, value)) = separated_pair(
        alt((tag("!logs"), tag("!log"), tag("!lo"), tag("!l"))),
        char(':'),
        regex,
    )
    .parse(s)?;
    Ok((remaining, FilterAttribute::ExcludeLog(value)))
}

fn label_selector<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute<'a>, E> {
    let (remaining, (_, value)) =
        separated_pair(alt((tag("labels"), tag("label"))), char(':'), selector).parse(s)?;
    Ok((remaining, FilterAttribute::LabelSelector(value)))
}

fn field_selector<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute<'a>, E> {
    let (remaining, (_, value)) =
        separated_pair(alt((tag("fields"), tag("field"))), char(':'), selector).parse(s)?;
    Ok((remaining, FilterAttribute::FieldSelector(value)))
}

fn jq<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute<'a>, E> {
    let (remaining, (_, value)) = separated_pair(tag("jq"), char(':'), jq_expr).parse(s)?;
    Ok((remaining, FilterAttribute::Jq(value)))
}

/// JMESPath expression parser - accepts quoted or unquoted strings
fn jmespath_expr<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, Cow<'a, str>, E> {
    alt((quoted, unquoted)).parse(s)
}

/// Parser for `jmespath:<expression>`, `jmes:<expression>`, or `jm:<expression>` syntax
fn jmespath<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute<'a>, E> {
    let (remaining, (_, value)) = separated_pair(
        alt((tag("jmespath"), tag("jmes"), tag("jm"))),
        char(':'),
        jmespath_expr,
    )
    .parse(s)?;
    Ok((remaining, FilterAttribute::JMESPath(value)))
}

fn specified_daemonset<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute<'a>, E> {
    let (remaining, (_, value)) = separated_pair(
        alt((tag("daemonsets"), tag("daemonset"), tag("ds"))),
        char('/'),
        resource_name,
    )
    .parse(s)?;
    Ok((
        remaining,
        FilterAttribute::from(SpecifiedResource::DaemonSet(value)),
    ))
}

fn specified_deployment<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute<'a>, E> {
    let (remaining, (_, value)) = separated_pair(
        alt((tag("deployments"), tag("deployment"), tag("deploy"))),
        char('/'),
        resource_name,
    )
    .parse(s)?;
    Ok((
        remaining,
        FilterAttribute::from(SpecifiedResource::Deployment(value)),
    ))
}

fn specified_job<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute<'a>, E> {
    let (remaining, (_, value)) =
        separated_pair(alt((tag("jobs"), tag("job"))), char('/'), resource_name).parse(s)?;
    Ok((
        remaining,
        FilterAttribute::from(SpecifiedResource::Job(value)),
    ))
}

fn specified_pod<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute<'a>, E> {
    let (remaining, (_, value)) = separated_pair(
        alt((tag("pods"), tag("pod"), tag("po"))),
        char('/'),
        resource_name,
    )
    .parse(s)?;
    Ok((
        remaining,
        FilterAttribute::from(SpecifiedResource::Pod(value)),
    ))
}

fn specified_replicaset<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute<'a>, E> {
    let (remaining, (_, value)) = separated_pair(
        alt((tag("replicasets"), tag("replicaset"), tag("rs"))),
        char('/'),
        resource_name,
    )
    .parse(s)?;
    Ok((
        remaining,
        FilterAttribute::from(SpecifiedResource::ReplicaSet(value)),
    ))
}

fn specified_service<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute<'a>, E> {
    let (remaining, (_, value)) = separated_pair(
        alt((tag("services"), tag("service"), tag("svc"))),
        char('/'),
        resource_name,
    )
    .parse(s)?;
    Ok((
        remaining,
        FilterAttribute::from(SpecifiedResource::Service(value)),
    ))
}

fn specified_statefulset<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute<'a>, E> {
    let (remaining, (_, value)) = separated_pair(
        alt((tag("statefulsets"), tag("statefulset"), tag("sts"))),
        char('/'),
        resource_name,
    )
    .parse(s)?;
    Ok((
        remaining,
        FilterAttribute::from(SpecifiedResource::StatefulSet(value)),
    ))
}

fn attribute<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute<'a>, E> {
    let (remaining, value) = alt((
        specified_pod,
        specified_daemonset,
        specified_deployment,
        specified_job,
        specified_replicaset,
        specified_service,
        specified_statefulset,
        field_selector,
        label_selector,
        pod,
        exclude_pod,
        container,
        exclude_container,
        include_log,
        exclude_log,
        jmespath,
        jq,
    ))
    .parse(s)?;

    Ok((remaining, value))
}

fn split_attributes<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, Vec<FilterAttribute<'a>>, E> {
    let (remaining, value) = delimited(
        multispace0,
        separated_list1(multispace1, attribute),
        multispace0,
    )
    .parse(s)?;

    Ok((remaining, value))
}

pub fn parse_attributes<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, Vec<FilterAttribute<'a>>, E> {
    all_consuming(split_attributes).parse(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom::error::Error;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    /// Regex
    #[rstest]
    #[case("pods:hoge", "hoge")]
    #[case("pod:hoge", "hoge")]
    #[case("po:.*", ".*")]
    #[case("p:^app$", "^app$")]
    #[case("p:'^app$'", "^app$")]
    #[case("p:\"^app$\"", "^app$")]
    #[case("p:\"a b\"", "a b")]
    #[case("p:'a b'", "a b")]
    fn pod(#[case] query: &str, #[case] expected: &str) {
        let (remaining, actual) = super::pod::<Error<_>>(query).unwrap();

        assert_eq!(actual, FilterAttribute::Pod(expected.into()));
        assert_eq!(remaining, "");
    }

    #[rstest]
    #[case("!pods:hoge", "hoge")]
    #[case("!pod:hoge", "hoge")]
    #[case("!po:.*", ".*")]
    #[case("!p:^app$", "^app$")]
    fn exclude_pod(#[case] query: &str, #[case] expected: &str) {
        let (remaining, actual) = super::exclude_pod::<Error<_>>(query).unwrap();

        assert_eq!(actual, FilterAttribute::ExcludePod(expected.into()));
        assert_eq!(remaining, "");
    }

    #[rstest]
    #[case("containers:hoge", "hoge")]
    #[case("container:hoge", "hoge")]
    #[case("co:.*", ".*")]
    #[case("c:^app$", "^app$")]
    fn container(#[case] query: &str, #[case] expected: &str) {
        let (remaining, actual) = super::container::<Error<_>>(query).unwrap();

        assert_eq!(actual, FilterAttribute::Container(expected.into()));
        assert_eq!(remaining, "");
    }

    #[rstest]
    #[case("!containers:hoge", "hoge")]
    #[case("!container:hoge", "hoge")]
    #[case("!co:.*", ".*")]
    #[case("!c:^app$", "^app$")]
    fn exclude_container(#[case] query: &str, #[case] expected: &str) {
        let (remaining, actual) = super::exclude_container::<Error<_>>(query).unwrap();

        assert_eq!(actual, FilterAttribute::ExcludeContainer(expected.into()));
        assert_eq!(remaining, "");
    }

    /// Log
    #[rstest]
    #[case("logs:hoge", "hoge")]
    #[case("log:hoge", "hoge")]
    #[case("lo:hoge", "hoge")]
    #[case("l:hoge", "hoge")]
    fn include_log(#[case] query: &str, #[case] expected: &str) {
        let (remaining, actual) = super::include_log::<Error<_>>(query).unwrap();

        assert_eq!(actual, FilterAttribute::IncludeLog(expected.into()));
        assert_eq!(remaining, "");
    }

    #[rstest]
    #[case("!logs:hoge", "hoge")]
    #[case("!log:hoge", "hoge")]
    #[case("!lo:hoge", "hoge")]
    #[case("!l:hoge", "hoge")]
    fn exclude_log(#[case] query: &str, #[case] expected: &str) {
        let (remaining, actual) = super::exclude_log::<Error<_>>(query).unwrap();

        assert_eq!(actual, FilterAttribute::ExcludeLog(expected.into()));
        assert_eq!(remaining, "");
    }

    /// Label selector
    #[rstest]
    #[case("labels:foo=bar,baz=qux", "foo=bar,baz=qux")]
    #[case("label:foo=bar,baz=qux", "foo=bar,baz=qux")]
    #[case("label:\"foo in (bar),baz in (qux)\"", "foo in (bar),baz in (qux)")]
    #[case("label:\'foo in (bar),baz in (qux)\'", "foo in (bar),baz in (qux)")]
    fn label_selector(#[case] query: &str, #[case] expected: &str) {
        let (remaining, actual) = super::label_selector::<Error<_>>(query).unwrap();

        assert_eq!(actual, FilterAttribute::LabelSelector(expected.into()));
        assert_eq!(remaining, "");
    }

    /// Field selector
    #[rstest]
    #[case("fields:foo=bar,baz=qux", "foo=bar,baz=qux")]
    #[case("field:foo=bar,baz=qux", "foo=bar,baz=qux")]
    #[case("field:\"foo in (bar),baz in (qux)\"", "foo in (bar),baz in (qux)")]
    #[case("field:\'foo in (bar),baz in (qux)\'", "foo in (bar),baz in (qux)")]
    fn field_selector(#[case] query: &str, #[case] expected: &str) {
        let (remaining, actual) = super::field_selector::<Error<_>>(query).unwrap();

        assert_eq!(actual, FilterAttribute::FieldSelector(expected.into()));
        assert_eq!(remaining, "");
    }

    // Specified resoruces

    /// DaemonSet
    #[rstest]
    #[case("daemonsets/app", "app")]
    #[case("daemonset/app", "app")]
    #[case("ds/app", "app")]
    fn specified_daemonset(#[case] query: &str, #[case] expected: &str) {
        let (remaining, actual) = super::specified_daemonset::<Error<_>>(query).unwrap();

        assert_eq!(
            actual,
            FilterAttribute::from(SpecifiedResource::DaemonSet(expected))
        );
        assert_eq!(remaining, "");
    }

    /// Deployment
    #[rstest]
    #[case("deployments/app", "app")]
    #[case("deployment/app", "app")]
    #[case("deploy/app", "app")]
    fn specified_deployment(#[case] query: &str, #[case] expected: &str) {
        let (remaining, actual) = super::specified_deployment::<Error<_>>(query).unwrap();

        assert_eq!(
            actual,
            FilterAttribute::from(SpecifiedResource::Deployment(expected))
        );
        assert_eq!(remaining, "");
    }

    /// Job
    #[rstest]
    #[case("jobs/app", "app")]
    #[case("job/app", "app")]
    fn specified_job(#[case] query: &str, #[case] expected: &str) {
        let (remaining, actual) = super::specified_job::<Error<_>>(query).unwrap();

        assert_eq!(
            actual,
            FilterAttribute::from(SpecifiedResource::Job(expected))
        );
        assert_eq!(remaining, "");
    }

    /// pod
    #[rstest]
    #[case("pods/app", "app")]
    #[case("pod/app", "app")]
    #[case("po/app", "app")]
    fn specified_pod(#[case] query: &str, #[case] expected: &str) {
        let (remaining, actual) = super::specified_pod::<Error<_>>(query).unwrap();

        assert_eq!(
            actual,
            FilterAttribute::from(SpecifiedResource::Pod(expected))
        );
        assert_eq!(remaining, "");
    }

    /// replicaset
    #[rstest]
    #[case("replicasets/app", "app")]
    #[case("replicaset/app", "app")]
    #[case("rs/app", "app")]
    fn specified_replicaset(#[case] query: &str, #[case] expected: &str) {
        let (remaining, actual) = super::specified_replicaset::<Error<_>>(query).unwrap();

        assert_eq!(
            actual,
            FilterAttribute::from(SpecifiedResource::ReplicaSet(expected))
        );
        assert_eq!(remaining, "");
    }

    /// service
    #[rstest]
    #[case("services/app", "app")]
    #[case("service/app", "app")]
    #[case("svc/app", "app")]
    fn specified_service(#[case] query: &str, #[case] expected: &str) {
        let (remaining, actual) = super::specified_service::<Error<_>>(query).unwrap();
        assert_eq!(
            actual,
            FilterAttribute::from(SpecifiedResource::Service(expected))
        );
        assert_eq!(remaining, "");
    }

    /// statefulset
    #[rstest]
    #[case("statefulsets/app", "app")]
    #[case("statefulset/app", "app")]
    #[case("sts/app", "app")]
    fn specified_statefulset(#[case] query: &str, #[case] expected: &str) {
        let (remaining, actual) = super::specified_statefulset::<Error<_>>(query).unwrap();

        assert_eq!(
            actual,
            FilterAttribute::from(SpecifiedResource::StatefulSet(expected))
        );
        assert_eq!(remaining, "");
    }

    #[rstest]
    #[case(r#""foo bar""#, "foo bar")]
    #[case(r#""\"""#, r#"""#)]
    #[case(r#""'""#, "'")]
    #[case(r#""\\\"""#, r#"\""#)]
    #[case(r#"'"'"#, r#"""#)]
    #[case(r"'\''", "'")]
    #[case(r"'\\\''", r"\'")]
    #[case(r"'\a'", r"\a")]
    #[case(r#""\a""#, r"\a")]
    #[case(r#""\\n""#, r"\n")]
    #[case(
        r#""\" ' \\ \. \+ \* \? \( \) \| \[ \] \{ \} \^ \$ \# \& \- \~""#,
        r#"" ' \ \. \+ \* \? \( \) \| \[ \] \{ \} \^ \$ \# \& \- \~"#
    )]
    #[case(
        r#""\a \f \t \n \r \v \A \z \b \B \< \> \123 \x7F \x{10FFFF} \u007F \u{7F} \U0000007F \U{7F} \p{Letter} \P{Letter} \d \s \w \D \S \W""#,
        r"\a \f \t \n \r \v \A \z \b \B \< \> \123 \x7F \x{10FFFF} \u007F \u{7F} \U0000007F \U{7F} \p{Letter} \P{Letter} \d \s \w \D \S \W"
    )]
    #[case("'foo bar'", "foo bar")]
    #[case(
        r#"'" \' \\ \. \+ \* \? \( \) \| \[ \] \{ \} \^ \$ \# \& \- \~'"#,
        r#"" ' \ \. \+ \* \? \( \) \| \[ \] \{ \} \^ \$ \# \& \- \~"#
    )]
    #[case(
        r"'\a \f \t \n \r \v \A \z \b \B \< \> \123 \x7F \x{10FFFF} \u007F \u{7F} \U0000007F \U{7F} \p{Letter} \P{Letter} \d \s \w \D \S \W'",
        r"\a \f \t \n \r \v \A \z \b \B \< \> \123 \x7F \x{10FFFF} \u007F \u{7F} \U0000007F \U{7F} \p{Letter} \P{Letter} \d \s \w \D \S \W"
    )]
    fn quoted(#[case] query: &str, #[case] expected: &str) {
        let (remaining, actual) = super::quoted::<Error<_>>(query).unwrap();

        assert_eq!(actual, expected);
        assert_eq!(remaining, "");
    }

    #[rstest]
    #[case("foo", "foo")]
    #[case("foo\"bar", "foo\"bar")]
    #[case("foo'bar", "foo'bar")]
    #[case(
        r#"\"\'\\\.\+\*\?\(\)\|\[\]\{\}\^\$\#\&\-\~"#,
        r#"\"\'\\\.\+\*\?\(\)\|\[\]\{\}\^\$\#\&\-\~"#
    )]
    #[case(
        r"\a\f\t\n\r\v\A\z\b\B\<\>\123\x7F\x{10FFFF}\u007F\u{7F}\U0000007F\U{7F}\p{Letter}\P{Letter}\d\s\w\D\S\W",
        r"\a\f\t\n\r\v\A\z\b\B\<\>\123\x7F\x{10FFFF}\u007F\u{7F}\U0000007F\U{7F}\p{Letter}\P{Letter}\d\s\w\D\S\W"
    )]
    fn unquoted(#[case] query: &str, #[case] expected: &str) {
        let (remaining, actual) = super::unquoted::<Error<_>>(query).unwrap();

        assert_eq!(actual, expected);
        assert_eq!(remaining, "");
    }

    #[rstest]
    #[case("foo", "foo")]
    #[case("foo\"bar", "foo\"bar")]
    #[case("foo'bar", "foo'bar")]
    #[case(
        r#"\"\'\\\.\+\*\?\(\)\|\[\]\{\}\^\$\#\&\-\~"#,
        r#"\"\'\\\.\+\*\?\(\)\|\[\]\{\}\^\$\#\&\-\~"#
    )]
    #[case(
        r"\a\f\t\n\r\v\A\z\b\B\<\>\123\x7F\x{10FFFF}\u007F\u{7F}\U0000007F\U{7F}\p{Letter}\P{Letter}\d\s\w\D\S\W",
        r"\a\f\t\n\r\v\A\z\b\B\<\>\123\x7F\x{10FFFF}\u007F\u{7F}\U0000007F\U{7F}\p{Letter}\P{Letter}\d\s\w\D\S\W"
    )]
    #[case(r#""foo bar""#, "foo bar")]
    #[case(r#""\" \' \\ \( \) \[ \]""#, r#"" ' \ \( \) \[ \]"#)]
    #[case(
        r#""\" \' \\ \. \+ \* \? \( \) \| \[ \] \{ \} \^ \$ \# \& \- \~""#,
        r#"" ' \ \. \+ \* \? \( \) \| \[ \] \{ \} \^ \$ \# \& \- \~"#
    )]
    #[case(
        r#""\a \f \t \n \r \v \A \z \b \B \< \> \123 \x7F \x{10FFFF} \u007F \u{7F} \U0000007F \U{7F} \p{Letter} \P{Letter} \d \s \w \D \S \W""#,
        r"\a \f \t \n \r \v \A \z \b \B \< \> \123 \x7F \x{10FFFF} \u007F \u{7F} \U0000007F \U{7F} \p{Letter} \P{Letter} \d \s \w \D \S \W"
    )]
    #[case("'foo bar'", "foo bar")]
    #[case(r#"'\" \' \\ \( \) \[ \]'"#, r#"" ' \ \( \) \[ \]"#)]
    #[case(
        r#"'\" \' \\ \. \+ \* \? \( \) \| \[ \] \{ \} \^ \$ \# \& \- \~'"#,
        r#"" ' \ \. \+ \* \? \( \) \| \[ \] \{ \} \^ \$ \# \& \- \~"#
    )]
    #[case(
        r"'\a \f \t \n \r \v \A \z \b \B \< \> \123 \x7F \x{10FFFF} \u007F \u{7F} \U0000007F \U{7F} \p{Letter} \P{Letter} \d \s \w \D \S \W'",
        r"\a \f \t \n \r \v \A \z \b \B \< \> \123 \x7F \x{10FFFF} \u007F \u{7F} \U0000007F \U{7F} \p{Letter} \P{Letter} \d \s \w \D \S \W"
    )]
    fn regex(#[case] query: &str, #[case] expected: &str) {
        let (remaining, actual) = super::regex::<Error<_>>(query).unwrap();

        assert_eq!(actual, expected);
        assert_eq!(remaining, "");
    }

    #[rstest]
    #[case("jq:.message", ".message")]
    #[case("jq:map('.level')", "map('.level')")]
    fn jq(#[case] query: &str, #[case] expected: &str) {
        let (remaining, actual) = super::jq::<Error<_>>(query).unwrap();

        assert_eq!(actual, FilterAttribute::Jq(expected.into()));
        assert_eq!(remaining, "");
    }

    #[rstest]
    #[case("jmespath:message", "message")]
    #[case("jmes:level", "level")]
    #[case("jm:data.userId", "data.userId")]
    #[case("jmespath:[0]", "[0]")]
    #[case("jmes:items[*].name", "items[*].name")]
    fn jmespath(#[case] query: &str, #[case] expected: &str) {
        let (remaining, actual) = super::jmespath::<Error<_>>(query).unwrap();

        assert_eq!(actual, FilterAttribute::JMESPath(expected.into()));
        assert_eq!(remaining, "");
    }

    #[rustfmt::skip]
    #[rstest]
    #[case("pod:hoge", FilterAttribute::Pod("hoge".into()))]
    #[case("!pod:hoge", FilterAttribute::ExcludePod("hoge".into()))]
    #[case("container:hoge", FilterAttribute::Container("hoge".into()))]
    #[case("!container:hoge", FilterAttribute::ExcludeContainer("hoge".into()))]
    #[case("log:hoge", FilterAttribute::IncludeLog("hoge".into()))]
    #[case("!log:hoge", FilterAttribute::ExcludeLog("hoge".into()))]
    #[case("labels:foo=bar", FilterAttribute::LabelSelector("foo=bar".into()))]
    #[case("fields:foo=bar", FilterAttribute::FieldSelector("foo=bar".into()))]
    #[case("daemonset/app", FilterAttribute::Resource(SpecifiedResource::DaemonSet("app")))]
    #[case("deployment/app", FilterAttribute::Resource(SpecifiedResource::Deployment("app")))]
    #[case("job/app", FilterAttribute::Resource(SpecifiedResource::Job("app")))]
    #[case("pod/app", FilterAttribute::Resource(SpecifiedResource::Pod("app")))]
    #[case("replicaset/app", FilterAttribute::Resource(SpecifiedResource::ReplicaSet("app")))]
    #[case("service/app", FilterAttribute::Resource(SpecifiedResource::Service("app")))]
    #[case("statefulset/app", FilterAttribute::Resource(SpecifiedResource::StatefulSet("app")))]
    #[case("jq:.message", FilterAttribute::Jq(".message".into()))]
    #[case("jmespath:message", FilterAttribute::JMESPath("message".into()))]
    #[case("jmes:level", FilterAttribute::JMESPath("level".into()))]
    #[case("jm:data.id", FilterAttribute::JMESPath("data.id".into()))]
    fn attribute(#[case] query: &str, #[case] expected: FilterAttribute) {
        let (remaining, actual) = super::attribute::<Error<_>>(query).unwrap();

        assert_eq!(actual, expected);
        assert_eq!(remaining, "");
    }

    #[test]
    fn parse_attributes() {
        let query = [
            "     ",
            "pod:hoge",
            "!pod:hoge",
            "container:hoge",
            "!container:hoge",
            "log:hoge",
            "!log:hoge",
            "labels:foo=bar",
            "fields:foo=bar",
            "daemonset/app",
            "deployment/app",
            "job/app",
            "pod/app",
            "replicaset/app",
            "service/app",
            "statefulset/app",
            "jq:.message",
            "jmespath:data.id",
            "     ",
        ]
        .join("  ");

        let (remaining, actual) = super::parse_attributes::<Error<_>>(&query).unwrap();

        let expected = vec![
            FilterAttribute::Pod("hoge".into()),
            FilterAttribute::ExcludePod("hoge".into()),
            FilterAttribute::Container("hoge".into()),
            FilterAttribute::ExcludeContainer("hoge".into()),
            FilterAttribute::IncludeLog("hoge".into()),
            FilterAttribute::ExcludeLog("hoge".into()),
            FilterAttribute::LabelSelector("foo=bar".into()),
            FilterAttribute::FieldSelector("foo=bar".into()),
            FilterAttribute::Resource(SpecifiedResource::DaemonSet("app")),
            FilterAttribute::Resource(SpecifiedResource::Deployment("app")),
            FilterAttribute::Resource(SpecifiedResource::Job("app")),
            FilterAttribute::Resource(SpecifiedResource::Pod("app")),
            FilterAttribute::Resource(SpecifiedResource::ReplicaSet("app")),
            FilterAttribute::Resource(SpecifiedResource::Service("app")),
            FilterAttribute::Resource(SpecifiedResource::StatefulSet("app")),
            FilterAttribute::Jq(".message".into()),
            FilterAttribute::JMESPath("data.id".into()),
        ];

        assert_eq!(actual, expected);
        assert_eq!(remaining, "");
    }

    #[test]
    fn parse_attributes_with_quote() {
        let query = [
            "     ",
            "pod:hoge",
            r"log:'\'foo\' bar'",
            r#"log:"\"foo\" bar""#,
            "     ",
        ]
        .join("  ");

        let (remaining, actual) = super::parse_attributes::<Error<_>>(&query).unwrap();

        let expected = vec![
            FilterAttribute::Pod("hoge".into()),
            FilterAttribute::IncludeLog(r"'foo' bar".into()),
            FilterAttribute::IncludeLog(r#""foo" bar"#.into()),
        ];

        assert_eq!(actual, expected);
        assert_eq!(remaining, "");
    }

    #[rstest]
    #[case("     ")]
    #[case("")]
    #[case("hoge:hoge")]
    fn parse_error(#[case] query: &str) {
        let actual = super::parse_attributes::<Error<_>>(query);

        assert!(actual.is_err());
    }
}
