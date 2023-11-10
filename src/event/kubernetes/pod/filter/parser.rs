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

fn name<'a, E: ParseError<&'a str> + ContextError<&'a str>>(s: &'a str) -> IResult<&str, &str, E> {
    recognize(many1(alt((alphanumeric1, tag("-"), tag(".")))))(s)
}

fn regex<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute, E> {
    let (remaining, (_, value)) = separated_pair(alt((tag("name"), tag("n"))), char(':'), rest)(s)?;
    Ok((remaining, FilterAttribute::Regex(value)))
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

fn daemonset<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute, E> {
    let (remaining, (_, value)) =
        separated_pair(alt((tag("daemonset"), tag("ds"))), char('/'), name)(s)?;
    Ok((
        remaining,
        FilterAttribute::from(SpecifiedResource::DaemonSet(value)),
    ))
}

fn deployment<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute, E> {
    let (remaining, (_, value)) =
        separated_pair(alt((tag("deployment"), tag("deploy"))), char('/'), name)(s)?;
    Ok((
        remaining,
        FilterAttribute::from(SpecifiedResource::Deployment(value)),
    ))
}

fn job<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute, E> {
    let (remaining, (_, value)) = separated_pair(tag("job"), char('/'), name)(s)?;
    Ok((
        remaining,
        FilterAttribute::from(SpecifiedResource::Job(value)),
    ))
}

fn pod<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute, E> {
    let (remaining, (_, value)) = separated_pair(alt((tag("pod"), tag("po"))), char('/'), name)(s)?;
    Ok((
        remaining,
        FilterAttribute::from(SpecifiedResource::Pod(value)),
    ))
}

fn replicaset<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute, E> {
    let (remaining, (_, value)) =
        separated_pair(alt((tag("replicaset"), tag("rs"))), char('/'), name)(s)?;
    Ok((
        remaining,
        FilterAttribute::from(SpecifiedResource::ReplicaSet(value)),
    ))
}

fn service<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute, E> {
    let (remaining, (_, value)) =
        separated_pair(alt((tag("service"), tag("svc"))), char('/'), name)(s)?;
    Ok((
        remaining,
        FilterAttribute::from(SpecifiedResource::Service(value)),
    ))
}

fn statefulset<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute, E> {
    let (remaining, (_, value)) =
        separated_pair(alt((tag("statefulset"), tag("sts"))), char('/'), name)(s)?;
    Ok((
        remaining,
        FilterAttribute::from(SpecifiedResource::StatefulSet(value)),
    ))
}

fn parse<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute, E> {
    alt((
        pod,
        daemonset,
        deployment,
        job,
        replicaset,
        service,
        statefulset,
        field_selector,
        label_selector,
        regex,
    ))(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    /// Regex
    #[rstest]
    #[case("name:hoge", "hoge")]
    #[case("name:.*", ".*")]
    #[case("n:^app$", "^app$")]
    fn regex(#[case] query: &str, #[case] expected: &str) {
        assert_eq!(
            FilterParser::new(query).try_collect().unwrap(),
            vec![FilterAttribute::Regex(expected)]
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
    fn daemonset(#[case] query: &str, #[case] expected: &str) {
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
    fn deployment(#[case] query: &str, #[case] expected: &str) {
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
    fn job(#[case] query: &str, #[case] expected: &str) {
        assert_eq!(
            FilterParser::new(query).try_collect().unwrap(),
            vec![FilterAttribute::from(SpecifiedResource::Job(expected))]
        );
    }

    /// pod
    #[rstest]
    #[case("pod/app", "app")]
    #[case("po/app", "app")]
    fn pod(#[case] query: &str, #[case] expected: &str) {
        assert_eq!(
            FilterParser::new(query).try_collect().unwrap(),
            vec![FilterAttribute::from(SpecifiedResource::Pod(expected))]
        );
    }

    /// replicaset
    #[rstest]
    #[case("replicaset/app", "app")]
    #[case("rs/app", "app")]
    fn replicaset(#[case] query: &str, #[case] expected: &str) {
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
    fn service(#[case] query: &str, #[case] expected: &str) {
        assert_eq!(
            FilterParser::new(query).try_collect().unwrap(),
            vec![FilterAttribute::from(SpecifiedResource::Service(expected))]
        );
    }

    /// statefulset
    #[rstest]
    #[case("statefulset/app", "app")]
    #[case("sts/app", "app")]
    fn statefulset(#[case] query: &str, #[case] expected: &str) {
        assert_eq!(
            FilterParser::new(query).try_collect().unwrap(),
            vec![FilterAttribute::from(SpecifiedResource::StatefulSet(
                expected
            ))]
        );
    }
}
