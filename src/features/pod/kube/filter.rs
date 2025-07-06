mod parser;

use std::borrow::Cow;

use anyhow::{Result, bail};
use regex::Regex;

use self::parser::parse_attributes;

#[derive(Debug, thiserror::Error)]
pub enum FilterError {
    #[error(transparent)]
    Regex(#[from] regex::Error),
    #[error("{0}")]
    Syntax(String),
}

#[derive(Debug, Default, Clone)]
pub struct Filter {
    pub pod: Option<Regex>,
    pub exclude_pod: Option<Vec<Regex>>,
    pub container: Option<Regex>,
    pub exclude_container: Option<Vec<Regex>>,
    pub field_selector: Option<String>,
    pub label_selector: Option<LabelSelector>,
    pub include_log: Option<Vec<Regex>>,
    pub exclude_log: Option<Vec<Regex>>,
}

impl Filter {
    pub fn parse(query: &str) -> Result<Self> {
        let parsed_attrs = FilterAttributes::parse(query)?;

        let valid_attrs = Self::validate_attrs(parsed_attrs)?;

        let mut filter = Filter::default();

        for attr in valid_attrs {
            match attr {
                FilterAttribute::Pod(regex) => {
                    let regex = Regex::new(&regex)?;
                    filter.pod = Some(regex);
                }

                FilterAttribute::ExcludePod(regex) => {
                    let regex = Regex::new(&regex)?;

                    if let Some(vec) = &mut filter.exclude_pod {
                        vec.push(regex);
                    } else {
                        filter.exclude_pod = Some(vec![regex]);
                    }
                }

                FilterAttribute::Container(regex) => {
                    let regex = Regex::new(&regex)?;
                    filter.container = Some(regex);
                }

                FilterAttribute::ExcludeContainer(regex) => {
                    let regex = Regex::new(&regex)?;

                    if let Some(vec) = &mut filter.exclude_container {
                        vec.push(regex);
                    } else {
                        filter.exclude_container = Some(vec![regex]);
                    }
                }

                FilterAttribute::Resource(resource) => match resource {
                    SpecifiedResource::Pod(name) => {
                        let regex = Regex::new(&format!("^{name}$"))?;
                        filter.pod = Some(regex);
                    }

                    SpecifiedResource::DaemonSet(name) => {
                        filter.label_selector = Some(LabelSelector::Resource(
                            RetrievableResource::DaemonSet(name.to_string()),
                        ));
                    }

                    SpecifiedResource::Deployment(name) => {
                        filter.label_selector = Some(LabelSelector::Resource(
                            RetrievableResource::Deployment(name.to_string()),
                        ));
                    }

                    SpecifiedResource::Job(name) => {
                        filter.label_selector = Some(LabelSelector::Resource(
                            RetrievableResource::Job(name.to_string()),
                        ));
                    }

                    SpecifiedResource::ReplicaSet(name) => {
                        filter.label_selector = Some(LabelSelector::Resource(
                            RetrievableResource::ReplicaSet(name.to_string()),
                        ));
                    }

                    SpecifiedResource::Service(name) => {
                        filter.label_selector = Some(LabelSelector::Resource(
                            RetrievableResource::Service(name.to_string()),
                        ));
                    }

                    SpecifiedResource::StatefulSet(name) => {
                        filter.label_selector = Some(LabelSelector::Resource(
                            RetrievableResource::StatefulSet(name.to_string()),
                        ));
                    }
                },

                FilterAttribute::LabelSelector(selector) => {
                    filter.label_selector = Some(LabelSelector::String(selector.to_string()));
                }

                FilterAttribute::FieldSelector(selector) => {
                    filter.field_selector = Some(selector.to_string());
                }

                FilterAttribute::IncludeLog(regex) => {
                    let regex = Regex::new(&regex)?;

                    if let Some(include) = &mut filter.include_log {
                        include.push(regex);
                    } else {
                        filter.include_log = Some(vec![regex]);
                    }
                }

                FilterAttribute::ExcludeLog(regex) => {
                    let regex = Regex::new(&regex)?;

                    if let Some(exclude) = &mut filter.exclude_log {
                        exclude.push(regex);
                    } else {
                        filter.exclude_log = Some(vec![regex]);
                    }
                }
            }
        }

        Ok(filter)
    }

    fn validate_attrs(attrs: Vec<FilterAttribute<'_>>) -> Result<Vec<FilterAttribute<'_>>> {
        let (has_label_selector, has_retrieve_labels) =
            attrs
                .iter()
                .fold((false, false), |(ls, rl), filter| match filter {
                    FilterAttribute::Resource(_) => (ls, true),
                    FilterAttribute::LabelSelector(_) => (true, rl),
                    _ => (ls, rl),
                });

        if has_label_selector && has_retrieve_labels {
            bail!(FilterError::Syntax("Label selectors and resource/name queries cannot be used together. Please choose one filtering option.".into()));
        }

        Ok(attrs)
    }
}

impl std::fmt::Display for Filter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut buf = Vec::new();

        if let Some(regex) = &self.pod {
            buf.push(format!("pod={}", regex.as_str()));
        }

        if let Some(vec) = &self.exclude_pod {
            for re in vec {
                buf.push(format!("exclude_pod={}", re.as_str()));
            }
        }

        if let Some(regex) = &self.container {
            buf.push(format!("container={}", regex.as_str()));
        }

        if let Some(vec) = &self.exclude_container {
            for re in vec {
                buf.push(format!("exclude_container={}", re.as_str()));
            }
        }

        if let Some(label_selector) = &self.label_selector {
            buf.push(label_selector.to_string());
        }

        if let Some(field_selector) = &self.field_selector {
            buf.push(format!("field_selector={field_selector}"));
        }

        if let Some(include) = &self.include_log {
            for i in include {
                buf.push(format!("include={}", i.as_str()));
            }
        }

        if let Some(exclude) = &self.exclude_log {
            for e in exclude {
                buf.push(format!("exclude={}", e.as_str()));
            }
        }

        write!(f, "{}", buf.join(" "))
    }
}

#[derive(Debug, Clone)]
pub enum LabelSelector {
    Resource(RetrievableResource),
    String(String),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RetrievableResource {
    DaemonSet(String),
    Deployment(String),
    Job(String),
    ReplicaSet(String),
    Service(String),
    StatefulSet(String),
}

impl std::fmt::Display for LabelSelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LabelSelector::Resource(resource) => write!(f, "label_selector_from={resource}"),
            LabelSelector::String(value) => write!(f, "label_selector={value}"),
        }
    }
}

impl std::fmt::Display for RetrievableResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RetrievableResource::DaemonSet(name) => {
                write!(f, "daemonset/{name}")
            }
            RetrievableResource::Deployment(name) => {
                write!(f, "deployment/{name}")
            }
            RetrievableResource::Job(name) => {
                write!(f, "job/{name}")
            }
            RetrievableResource::ReplicaSet(name) => {
                write!(f, "replicaset/{name}")
            }
            RetrievableResource::Service(name) => {
                write!(f, "service/{name}")
            }
            RetrievableResource::StatefulSet(name) => {
                write!(f, "statefulset/{name}")
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SpecifiedResource<'a> {
    DaemonSet(&'a str),
    Deployment(&'a str),
    Job(&'a str),
    Pod(&'a str),
    ReplicaSet(&'a str),
    Service(&'a str),
    StatefulSet(&'a str),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum FilterAttribute<'a> {
    Pod(Cow<'a, str>),
    ExcludePod(Cow<'a, str>),
    Container(Cow<'a, str>),
    ExcludeContainer(Cow<'a, str>),
    Resource(SpecifiedResource<'a>),
    LabelSelector(Cow<'a, str>),
    FieldSelector(Cow<'a, str>),
    IncludeLog(Cow<'a, str>),
    ExcludeLog(Cow<'a, str>),
}

struct FilterAttributes;

impl FilterAttributes {
    fn parse(query: &str) -> Result<Vec<FilterAttribute<'_>>> {
        use nom::Err;
        use nom_language::error::{VerboseError, convert_error};

        match parse_attributes::<VerboseError<_>>(query) {
            Ok((_, filter)) => Ok(filter),
            Err(Err::Error(err) | Err::Failure(err)) => bail!(convert_error(query, err)),
            Err(err) => bail!(err.to_string()),
        }
    }
}

impl<'a> From<SpecifiedResource<'a>> for FilterAttribute<'a> {
    fn from(value: SpecifiedResource<'a>) -> Self {
        Self::Resource(value)
    }
}
