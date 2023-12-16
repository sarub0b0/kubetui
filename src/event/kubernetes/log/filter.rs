mod parser;

use anyhow::{bail, Result};
use regex::Regex;

use self::parser::FilterParser;

#[derive(Debug, thiserror::Error)]
pub enum FilterError {
    #[error(transparent)]
    Regex(#[from] regex::Error),
    #[error("{0}")]
    Syntax(String),
}

#[derive(Debug, Default, Clone)]
pub struct Filter {
    pub pod_filter: Option<Regex>,
    pub field_selector: Option<String>,
    pub label_selector: Option<LabelSelector>,
}

impl Filter {
    pub fn parse(query: &str) -> Result<Self> {
        let parsed_attrs = FilterParser::new(query).try_collect()?;

        let valid_attrs = Self::validate_attrs(parsed_attrs)?;

        let mut filter = Filter::default();

        for attr in valid_attrs {
            match attr {
                FilterAttribute::Regex(regex) => {
                    let regex = Regex::new(regex)?;
                    filter.pod_filter = Some(regex);
                }
                FilterAttribute::Resource(resource) => match resource {
                    SpecifiedResource::Pod(name) => {
                        let regex = Regex::new(&format!("^{}$", name))?;
                        filter.pod_filter = Some(regex);
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

        if let Some(regex) = &self.pod_filter {
            buf.push(format!("pod_filter={}", regex.as_str()));
        }

        if let Some(label_selector) = &self.label_selector {
            buf.push(label_selector.to_string());
        }

        if let Some(field_selector) = &self.field_selector {
            buf.push(format!("field_selector={}", field_selector));
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
            LabelSelector::Resource(resource) => write!(f, "label_selector_from={}", resource),
            LabelSelector::String(value) => write!(f, "label_selector={}", value),
        }
    }
}

impl std::fmt::Display for RetrievableResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RetrievableResource::DaemonSet(name) => {
                write!(f, "daemonset/{}", name)
            }
            RetrievableResource::Deployment(name) => {
                write!(f, "deployment/{}", name)
            }
            RetrievableResource::Job(name) => {
                write!(f, "job/{}", name)
            }
            RetrievableResource::ReplicaSet(name) => {
                write!(f, "replicaset/{}", name)
            }
            RetrievableResource::Service(name) => {
                write!(f, "service/{}", name)
            }
            RetrievableResource::StatefulSet(name) => {
                write!(f, "statefulset/{}", name)
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
    Regex(&'a str),
    Resource(SpecifiedResource<'a>),
    LabelSelector(&'a str),
    FieldSelector(&'a str),
}

impl<'a> From<SpecifiedResource<'a>> for FilterAttribute<'a> {
    fn from(value: SpecifiedResource<'a>) -> Self {
        Self::Resource(value)
    }
}
