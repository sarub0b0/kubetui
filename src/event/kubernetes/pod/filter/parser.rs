use anyhow::{bail, Result};
use std::str::SplitWhitespace;

use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{alphanumeric1, char},
    combinator::{recognize, rest},
    error::{convert_error, ContextError, ParseError, VerboseError},
    multi::many1,
    sequence::separated_pair,
    Err, IResult,
};

use super::{FilterAttribute, SpecifiedResource};

pub struct FilterParser<'a>(SplitWhitespace<'a>);

impl<'a> FilterParser<'a> {
    pub fn new(query: &'a str) -> Self {
        Self(query.split_whitespace())
    }

    pub fn try_collect(&mut self) -> Result<Vec<FilterAttribute<'a>>> {
        let mut vec = Vec::new();

        while let Some(filter) = self.try_next()? {
            vec.push(filter);
        }

        Ok(vec)
    }

    fn try_next(&mut self) -> Result<Option<FilterAttribute<'a>>> {
        let Some(f) = self.0.next() else {
            return Ok(None);
        };

        match parse::<VerboseError<&str>>(f) {
            Ok((_, filter)) => Ok(Some(filter)),
            Err(Err::Error(err) | Err::Failure(err)) => bail!(convert_error(f, err)),
            Err(err) => bail!(err.to_string()),
        }
    }
}

fn resource_name<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&str, &str, E> {
    recognize(many1(alt((alphanumeric1, tag("-"), tag(".")))))(s)
}

fn pod<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute, E> {
    let (remaining, (_, value)) =
        separated_pair(alt((tag("pod"), tag("po"), tag("p"))), char(':'), rest)(s)?;
    Ok((remaining, FilterAttribute::Pod(value)))
}

fn exclude_pod<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute, E> {
    let (remaining, (_, value)) =
        separated_pair(alt((tag("!pod"), tag("!po"), tag("!p"))), char(':'), rest)(s)?;
    Ok((remaining, FilterAttribute::ExcludePod(value)))
}

fn container<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute, E> {
    let (remaining, (_, value)) = separated_pair(
        alt((tag("container"), tag("co"), tag("c"))),
        char(':'),
        rest,
    )(s)?;
    Ok((remaining, FilterAttribute::Container(value)))
}

fn exclude_container<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute, E> {
    let (remaining, (_, value)) = separated_pair(
        alt((tag("!container"), tag("!co"), tag("!c"))),
        char(':'),
        rest,
    )(s)?;
    Ok((remaining, FilterAttribute::ExcludeContainer(value)))
}

fn include_log<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute, E> {
    let (remaining, (_, value)) = separated_pair(alt((tag("log"), tag("lo"))), char(':'), rest)(s)?;
    Ok((remaining, FilterAttribute::IncludeLog(value)))
}

fn exclude_log<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute, E> {
    let (remaining, (_, value)) =
        separated_pair(alt((tag("!log"), tag("!lo"))), char(':'), rest)(s)?;
    Ok((remaining, FilterAttribute::ExcludeLog(value)))
}

fn label_selector<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute, E> {
    let (remaining, (_, value)) = separated_pair(
        alt((tag("labels"), tag("label"), tag("l"))),
        char(':'),
        rest,
    )(s)?;
    Ok((remaining, FilterAttribute::LabelSelector(value)))
}

fn field_selector<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute, E> {
    let (remaining, (_, value)) = separated_pair(
        alt((tag("fields"), tag("field"), tag("f"))),
        char(':'),
        rest,
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

fn parse<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute, E> {
    alt((
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
    ))(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    /// Regex
    #[rstest]
    #[case("pod:hoge", "hoge")]
    #[case("po:.*", ".*")]
    #[case("p:^app$", "^app$")]
    fn pod(#[case] query: &str, #[case] expected: &str) {
        assert_eq!(
            FilterParser::new(query).try_collect().unwrap(),
            vec![FilterAttribute::Pod(expected)]
        );
    }

    #[rstest]
    #[case("!pod:hoge", "hoge")]
    #[case("!po:.*", ".*")]
    #[case("!p:^app$", "^app$")]
    fn exclude_pod(#[case] query: &str, #[case] expected: &str) {
        assert_eq!(
            FilterParser::new(query).try_collect().unwrap(),
            vec![FilterAttribute::ExcludePod(expected)]
        );
    }

    #[rstest]
    #[case("container:hoge", "hoge")]
    #[case("co:.*", ".*")]
    #[case("c:^app$", "^app$")]
    fn container(#[case] query: &str, #[case] expected: &str) {
        assert_eq!(
            FilterParser::new(query).try_collect().unwrap(),
            vec![FilterAttribute::Container(expected)]
        );
    }

    #[rstest]
    #[case("!container:hoge", "hoge")]
    #[case("!co:.*", ".*")]
    #[case("!c:^app$", "^app$")]
    fn exclude_container(#[case] query: &str, #[case] expected: &str) {
        assert_eq!(
            FilterParser::new(query).try_collect().unwrap(),
            vec![FilterAttribute::ExcludeContainer(expected)]
        );
    }

    /// Log
    #[rstest]
    #[case("log:hoge", "hoge")]
    #[case("lo:hoge", "hoge")]
    fn include_log(#[case] query: &str, #[case] expected: &str) {
        assert_eq!(
            FilterParser::new(query).try_collect().unwrap(),
            vec![FilterAttribute::IncludeLog(expected)]
        );
    }

    #[rstest]
    #[case("!log:hoge", "hoge")]
    #[case("!lo:hoge", "hoge")]
    fn exclude_log(#[case] query: &str, #[case] expected: &str) {
        assert_eq!(
            FilterParser::new(query).try_collect().unwrap(),
            vec![FilterAttribute::ExcludeLog(expected)]
        );
    }

    /// Label selector
    #[rstest]
    #[case("labels:foo=bar,baz=qux", "foo=bar,baz=qux")]
    #[case("label:foo=bar,baz=qux", "foo=bar,baz=qux")]
    #[case("l:foo=bar,baz=qux", "foo=bar,baz=qux")]
    fn label_selector(#[case] query: &str, #[case] expected: &str) {
        assert_eq!(
            FilterParser::new(query).try_collect().unwrap(),
            vec![FilterAttribute::LabelSelector(expected)]
        );
    }

    /// Field selector
    #[rstest]
    #[case("fields:foo=bar,baz=qux", "foo=bar,baz=qux")]
    #[case("field:foo=bar,baz=qux", "foo=bar,baz=qux")]
    #[case("f:foo=bar,baz=qux", "foo=bar,baz=qux")]
    fn field_selector(#[case] query: &str, #[case] expected: &str) {
        assert_eq!(
            FilterParser::new(query).try_collect().unwrap(),
            vec![FilterAttribute::FieldSelector(expected)]
        );
    }

    /// Specified resoruces

    /// DaemonSet
    #[rstest]
    #[case("daemonset/app", "app")]
    #[case("ds/app", "app")]
    fn specified_daemonset(#[case] query: &str, #[case] expected: &str) {
        assert_eq!(
            FilterParser::new(query).try_collect().unwrap(),
            vec![FilterAttribute::from(SpecifiedResource::DaemonSet(
                expected
            ))]
        );
    }

    /// Deployment
    #[rstest]
    #[case("deployment/app", "app")]
    #[case("deploy/app", "app")]
    fn specified_deployment(#[case] query: &str, #[case] expected: &str) {
        assert_eq!(
            FilterParser::new(query).try_collect().unwrap(),
            vec![FilterAttribute::from(SpecifiedResource::Deployment(
                expected
            ))]
        );
    }

    /// Job
    #[rstest]
    #[case("job/app", "app")]
    fn specified_job(#[case] query: &str, #[case] expected: &str) {
        assert_eq!(
            FilterParser::new(query).try_collect().unwrap(),
            vec![FilterAttribute::from(SpecifiedResource::Job(expected))]
        );
    }

    /// pod
    #[rstest]
    #[case("pod/app", "app")]
    #[case("po/app", "app")]
    fn specified_pod(#[case] query: &str, #[case] expected: &str) {
        assert_eq!(
            FilterParser::new(query).try_collect().unwrap(),
            vec![FilterAttribute::from(SpecifiedResource::Pod(expected))]
        );
    }

    /// replicaset
    #[rstest]
    #[case("replicaset/app", "app")]
    #[case("rs/app", "app")]
    fn specified_replicaset(#[case] query: &str, #[case] expected: &str) {
        assert_eq!(
            FilterParser::new(query).try_collect().unwrap(),
            vec![FilterAttribute::from(SpecifiedResource::ReplicaSet(
                expected
            ))]
        );
    }

    /// service
    #[rstest]
    #[case("service/app", "app")]
    #[case("svc/app", "app")]
    fn specified_service(#[case] query: &str, #[case] expected: &str) {
        assert_eq!(
            FilterParser::new(query).try_collect().unwrap(),
            vec![FilterAttribute::from(SpecifiedResource::Service(expected))]
        );
    }

    /// statefulset
    #[rstest]
    #[case("statefulset/app", "app")]
    #[case("sts/app", "app")]
    fn specified_statefulset(#[case] query: &str, #[case] expected: &str) {
        assert_eq!(
            FilterParser::new(query).try_collect().unwrap(),
            vec![FilterAttribute::from(SpecifiedResource::StatefulSet(
                expected
            ))]
        );
    }

    #[test]
    fn all_attributes() {
        let query = "pod:hoge !pod:hoge container:hoge !container:hoge log:hoge !log:hoge labels:foo=bar fields:foo=bar daemonset/app deployment/app job/app pod/app replicaset/app service/app statefulset/app";

        let actual = FilterParser::new(query).try_collect().unwrap();

        let expected = vec![
            FilterAttribute::Pod("hoge"),
            FilterAttribute::ExcludePod("hoge"),
            FilterAttribute::Container("hoge"),
            FilterAttribute::ExcludeContainer("hoge"),
            FilterAttribute::IncludeLog("hoge"),
            FilterAttribute::ExcludeLog("hoge"),
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
    }

    #[test]
    fn return_error() {
        let query = "hoge:hoge";

        assert!(FilterParser::new(query).try_collect().is_err());
    }
}
