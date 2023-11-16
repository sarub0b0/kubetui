use nom::{
    branch::alt,
    bytes::complete::{escaped, is_not, tag},
    character::complete::{alphanumeric1, char, multispace0, one_of, space1},
    combinator::{all_consuming, recognize},
    error::{ContextError, ParseError},
    multi::{many1_count, separated_list0},
    sequence::{delimited, separated_pair},
    IResult,
};

use super::{FilterAttribute, SpecifiedResource};

fn quoted_literal<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, &str, E> {
    let (remaining, value) = alt((
        delimited(
            tag("\""),
            escaped(is_not("\\\""), '\\', one_of(r#""\"#)),
            tag("\""),
        ),
        delimited(
            tag("'"),
            escaped(is_not("\\'"), '\\', one_of(r"'\")),
            tag("'"),
        ),
    ))(s)?;

    Ok((remaining, value))
}

fn unquoted_literal<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, &str, E> {
    let (remaining, value) = is_not(" ")(s)?;

    Ok((remaining, value))
}

fn regex<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, &str, E> {
    let (remaining, value) = alt((quoted_literal, unquoted_literal))(s)?;

    Ok((remaining, value))
}

fn resource_name<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&str, &str, E> {
    recognize(many1_count(alt((alphanumeric1, tag("-"), tag(".")))))(s)
}

fn pod<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute, E> {
    let (remaining, (_, value)) =
        separated_pair(alt((tag("pod"), tag("po"), tag("p"))), char(':'), regex)(s)?;
    Ok((remaining, FilterAttribute::Pod(value)))
}

fn exclude_pod<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute, E> {
    let (remaining, (_, value)) =
        separated_pair(alt((tag("!pod"), tag("!po"), tag("!p"))), char(':'), regex)(s)?;
    Ok((remaining, FilterAttribute::ExcludePod(value)))
}

fn container<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute, E> {
    let (remaining, (_, value)) = separated_pair(
        alt((tag("container"), tag("co"), tag("c"))),
        char(':'),
        regex,
    )(s)?;
    Ok((remaining, FilterAttribute::Container(value)))
}

fn exclude_container<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute, E> {
    let (remaining, (_, value)) = separated_pair(
        alt((tag("!container"), tag("!co"), tag("!c"))),
        char(':'),
        regex,
    )(s)?;
    Ok((remaining, FilterAttribute::ExcludeContainer(value)))
}

fn include_log<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute, E> {
    let (remaining, (_, value)) =
        separated_pair(alt((tag("log"), tag("lo"))), char(':'), regex)(s)?;
    Ok((remaining, FilterAttribute::IncludeLog(value)))
}

fn exclude_log<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute, E> {
    let (remaining, (_, value)) =
        separated_pair(alt((tag("!log"), tag("!lo"))), char(':'), regex)(s)?;
    Ok((remaining, FilterAttribute::ExcludeLog(value)))
}

fn label_selector<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute, E> {
    let (remaining, (_, value)) = separated_pair(
        alt((tag("labels"), tag("label"), tag("l"))),
        char(':'),
        unquoted_literal,
    )(s)?;
    Ok((remaining, FilterAttribute::LabelSelector(value)))
}

fn field_selector<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute, E> {
    let (remaining, (_, value)) = separated_pair(
        alt((tag("fields"), tag("field"), tag("f"))),
        char(':'),
        unquoted_literal,
    )(s)?;
    Ok((remaining, FilterAttribute::FieldSelector(value)))
}

fn specified_daemonset<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute, E> {
    let (remaining, (_, value)) =
        separated_pair(alt((tag("daemonset"), tag("ds"))), char('/'), resource_name)(s)?;
    Ok((
        remaining,
        FilterAttribute::from(SpecifiedResource::DaemonSet(value)),
    ))
}

fn specified_deployment<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute, E> {
    let (remaining, (_, value)) = separated_pair(
        alt((tag("deployment"), tag("deploy"))),
        char('/'),
        resource_name,
    )(s)?;
    Ok((
        remaining,
        FilterAttribute::from(SpecifiedResource::Deployment(value)),
    ))
}

fn specified_job<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute, E> {
    let (remaining, (_, value)) = separated_pair(tag("job"), char('/'), resource_name)(s)?;
    Ok((
        remaining,
        FilterAttribute::from(SpecifiedResource::Job(value)),
    ))
}

fn specified_pod<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute, E> {
    let (remaining, (_, value)) =
        separated_pair(alt((tag("pod"), tag("po"))), char('/'), resource_name)(s)?;
    Ok((
        remaining,
        FilterAttribute::from(SpecifiedResource::Pod(value)),
    ))
}

fn specified_replicaset<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute, E> {
    let (remaining, (_, value)) = separated_pair(
        alt((tag("replicaset"), tag("rs"))),
        char('/'),
        resource_name,
    )(s)?;
    Ok((
        remaining,
        FilterAttribute::from(SpecifiedResource::ReplicaSet(value)),
    ))
}

fn specified_service<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute, E> {
    let (remaining, (_, value)) =
        separated_pair(alt((tag("service"), tag("svc"))), char('/'), resource_name)(s)?;
    Ok((
        remaining,
        FilterAttribute::from(SpecifiedResource::Service(value)),
    ))
}

fn specified_statefulset<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute, E> {
    let (remaining, (_, value)) = separated_pair(
        alt((tag("statefulset"), tag("sts"))),
        char('/'),
        resource_name,
    )(s)?;
    Ok((
        remaining,
        FilterAttribute::from(SpecifiedResource::StatefulSet(value)),
    ))
}

fn attribute<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute, E> {
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
    ))(s)?;

    Ok((remaining, value))
}

fn split_attributes<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, Vec<FilterAttribute>, E> {
    let (remaining, value) =
        delimited(multispace0, separated_list0(space1, attribute), multispace0)(s)?;

    Ok((remaining, value))
}

pub fn parse_attributes<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, Vec<FilterAttribute>, E> {
    all_consuming(split_attributes)(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom::error::Error;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    /// Regex
    #[rstest]
    #[case("pod:hoge", "hoge")]
    #[case("po:.*", ".*")]
    #[case("p:^app$", "^app$")]
    #[case("p:'^app$'", "^app$")]
    #[case("p:\"^app$\"", "^app$")]
    #[case("p:\"a b\"", "a b")]
    #[case("p:'a b'", "a b")]
    fn pod(#[case] query: &str, #[case] expected: &str) {
        let (remaining, actual) = super::pod::<Error<_>>(query).unwrap();

        assert_eq!(actual, FilterAttribute::Pod(expected));
        assert_eq!(remaining, "");
    }

    #[rstest]
    #[case("!pod:hoge", "hoge")]
    #[case("!po:.*", ".*")]
    #[case("!p:^app$", "^app$")]
    fn exclude_pod(#[case] query: &str, #[case] expected: &str) {
        let (remaining, actual) = super::exclude_pod::<Error<_>>(query).unwrap();

        assert_eq!(actual, FilterAttribute::ExcludePod(expected));
        assert_eq!(remaining, "");
    }

    #[rstest]
    #[case("container:hoge", "hoge")]
    #[case("co:.*", ".*")]
    #[case("c:^app$", "^app$")]
    fn container(#[case] query: &str, #[case] expected: &str) {
        let (remaining, actual) = super::container::<Error<_>>(query).unwrap();

        assert_eq!(actual, FilterAttribute::Container(expected));
        assert_eq!(remaining, "");
    }

    #[rstest]
    #[case("!container:hoge", "hoge")]
    #[case("!co:.*", ".*")]
    #[case("!c:^app$", "^app$")]
    fn exclude_container(#[case] query: &str, #[case] expected: &str) {
        let (remaining, actual) = super::exclude_container::<Error<_>>(query).unwrap();

        assert_eq!(actual, FilterAttribute::ExcludeContainer(expected));
        assert_eq!(remaining, "");
    }

    /// Log
    #[rstest]
    #[case("log:hoge", "hoge")]
    #[case("lo:hoge", "hoge")]
    fn include_log(#[case] query: &str, #[case] expected: &str) {
        let (remaining, actual) = super::include_log::<Error<_>>(query).unwrap();

        assert_eq!(actual, FilterAttribute::IncludeLog(expected));
        assert_eq!(remaining, "");
    }

    #[rstest]
    #[case("!log:hoge", "hoge")]
    #[case("!lo:hoge", "hoge")]
    fn exclude_log(#[case] query: &str, #[case] expected: &str) {
        let (remaining, actual) = super::exclude_log::<Error<_>>(query).unwrap();

        assert_eq!(actual, FilterAttribute::ExcludeLog(expected));
        assert_eq!(remaining, "");
    }

    /// Label selector
    #[rstest]
    #[case("labels:foo=bar,baz=qux", "foo=bar,baz=qux")]
    #[case("label:foo=bar,baz=qux", "foo=bar,baz=qux")]
    #[case("l:foo=bar,baz=qux", "foo=bar,baz=qux")]
    fn label_selector(#[case] query: &str, #[case] expected: &str) {
        let (remaining, actual) = super::label_selector::<Error<_>>(query).unwrap();

        assert_eq!(actual, FilterAttribute::LabelSelector(expected));
        assert_eq!(remaining, "");
    }

    /// Field selector
    #[rstest]
    #[case("fields:foo=bar,baz=qux", "foo=bar,baz=qux")]
    #[case("field:foo=bar,baz=qux", "foo=bar,baz=qux")]
    #[case("f:foo=bar,baz=qux", "foo=bar,baz=qux")]
    fn field_selector(#[case] query: &str, #[case] expected: &str) {
        let (remaining, actual) = super::field_selector::<Error<_>>(query).unwrap();

        assert_eq!(actual, FilterAttribute::FieldSelector(expected));
        assert_eq!(remaining, "");
    }

    /// Specified resoruces

    /// DaemonSet
    #[rstest]
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
    #[case(r#""\"foo\" bar""#, r#"\"foo\" bar"#)]
    #[case(r#""\\foo\\ bar""#, r"\\foo\\ bar")]
    #[case("'foo bar'", "foo bar")]
    #[case(r"'\'foo\' bar'", r"\'foo\' bar")]
    #[case(r"'\\foo\\ bar'", r"\\foo\\ bar")]
    fn quoted_literal(#[case] query: &str, #[case] expected: &str) {
        let (remaining, actual) = super::quoted_literal::<Error<_>>(query).unwrap();

        assert_eq!(actual, expected);
        assert_eq!(remaining, "");
    }

    #[rstest]
    #[case("foo")]
    #[case("foo\"bar")]
    #[case("foo'bar")]
    #[case(r"\\foo\\bar")]
    fn unquoted_literal(#[case] query: &str) {
        let (remaining, actual) = super::unquoted_literal::<Error<_>>(query).unwrap();

        assert_eq!(actual, query);
        assert_eq!(remaining, "");
    }

    #[rstest]
    #[case(r#""foo bar""#, "foo bar")]
    #[case(r#""\"foo\" bar""#, r#"\"foo\" bar"#)]
    #[case(r#""\\foo\\ bar""#, r"\\foo\\ bar")]
    #[case("'foo bar'", "foo bar")]
    #[case(r"'\'foo\' bar'", r"\'foo\' bar")]
    #[case(r"'\\foo\\ bar'", r"\\foo\\ bar")]
    #[case("foo", "foo")]
    #[case("foo\"bar", "foo\"bar")]
    #[case("foo'bar", "foo'bar")]
    #[case(r"\\foo\\bar", r"\\foo\\bar")]
    fn regex(#[case] query: &str, #[case] expected: &str) {
        let (remaining, actual) = super::regex::<Error<_>>(query).unwrap();

        assert_eq!(actual, expected);
        assert_eq!(remaining, "");
    }

    #[rustfmt::skip]
    #[rstest]
    #[case("pod:hoge", FilterAttribute::Pod("hoge"))]
    #[case("!pod:hoge", FilterAttribute::ExcludePod("hoge"))]
    #[case("container:hoge", FilterAttribute::Container("hoge"))]
    #[case("!container:hoge", FilterAttribute::ExcludeContainer("hoge"))]
    #[case("log:hoge", FilterAttribute::IncludeLog("hoge"))]
    #[case("!log:hoge", FilterAttribute::ExcludeLog("hoge"))]
    #[case("labels:foo=bar", FilterAttribute::LabelSelector("foo=bar"))]
    #[case("fields:foo=bar", FilterAttribute::FieldSelector("foo=bar"))]
    #[case("daemonset/app", FilterAttribute::Resource(SpecifiedResource::DaemonSet("app")))]
    #[case("deployment/app", FilterAttribute::Resource(SpecifiedResource::Deployment("app")))]
    #[case("job/app", FilterAttribute::Resource(SpecifiedResource::Job("app")))]
    #[case("pod/app", FilterAttribute::Resource(SpecifiedResource::Pod("app")))]
    #[case("replicaset/app", FilterAttribute::Resource(SpecifiedResource::ReplicaSet("app")))]
    #[case("service/app", FilterAttribute::Resource(SpecifiedResource::Service("app")))]
    #[case("statefulset/app", FilterAttribute::Resource(SpecifiedResource::StatefulSet("app")))]
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
            "pod:\"hoge fuga\"",
            "pod:\"hoge\\\" fuga\"",
            "pod:'a b'",
            "pod:'a\\' b'",
            "!pod:hoge",
            "!pod:\"hoge fuga\"",
            "container:hoge",
            "container:\"hoge fuga\"",
            "!container:hoge",
            "!container:\"hoge fuga\"",
            "log:hoge",
            "log:\"hoge fuga\"",
            "!log:hoge",
            "!log:\"hoge fuga\"",
            "labels:foo=bar",
            "fields:foo=bar",
            "daemonset/app",
            "deployment/app",
            "job/app",
            "pod/app",
            "replicaset/app",
            "service/app",
            "statefulset/app",
            "     ",
        ]
        .join("  ");

        let (remaining, actual) = super::parse_attributes::<Error<_>>(&query).unwrap();

        let expected = vec![
            FilterAttribute::Pod("hoge"),
            FilterAttribute::Pod("hoge fuga"),
            FilterAttribute::Pod("hoge\\\" fuga"),
            FilterAttribute::Pod("a b"),
            FilterAttribute::Pod("a\\' b"),
            FilterAttribute::ExcludePod("hoge"),
            FilterAttribute::ExcludePod("hoge fuga"),
            FilterAttribute::Container("hoge"),
            FilterAttribute::Container("hoge fuga"),
            FilterAttribute::ExcludeContainer("hoge"),
            FilterAttribute::ExcludeContainer("hoge fuga"),
            FilterAttribute::IncludeLog("hoge"),
            FilterAttribute::IncludeLog("hoge fuga"),
            FilterAttribute::ExcludeLog("hoge"),
            FilterAttribute::ExcludeLog("hoge fuga"),
            FilterAttribute::LabelSelector("foo=bar"),
            FilterAttribute::FieldSelector("foo=bar"),
            FilterAttribute::Resource(SpecifiedResource::DaemonSet("app")),
            FilterAttribute::Resource(SpecifiedResource::Deployment("app")),
            FilterAttribute::Resource(SpecifiedResource::Job("app")),
            FilterAttribute::Resource(SpecifiedResource::Pod("app")),
            FilterAttribute::Resource(SpecifiedResource::ReplicaSet("app")),
            FilterAttribute::Resource(SpecifiedResource::Service("app")),
            FilterAttribute::Resource(SpecifiedResource::StatefulSet("app")),
        ];

        assert_eq!(actual, expected);
        assert_eq!(remaining, "");
    }

    #[test]
    fn parse_error() {
        let query = "hoge:hoge";

        let actual = super::parse_attributes::<Error<_>>(query);

        assert!(actual.is_err());
    }
}
