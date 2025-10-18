mod parser;

use std::borrow::Cow;

use anyhow::{bail, Result};
use jaq_core::{
    load::{Arena, File, Loader},
    Native,
};
use jaq_json::Val;
use regex::Regex;

use self::parser::parse_attributes;

/// jqプログラムをコンパイル済みフィルターとソースコードとして保持する構造体
///
/// jqプログラムは一度だけコンパイルされ、各ログ行に対して再利用されます。
/// これにより、パフォーマンスを向上させることができます。
#[derive(Clone)]
pub struct JqProgram {
    /// コンパイル済みのjqプログラム
    pub program: jaq_core::Filter<Native<Val>>,
    /// ソースコード（デバッグおよび表示用）
    code: String,
}

impl std::fmt::Display for JqProgram {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.code)
    }
}

impl std::fmt::Debug for JqProgram {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Jq")
            .field("program", &"<compiled program>")
            .field("code", &self.code)
            .finish()
    }
}

/// JSONログに対するフィルター
///
/// 現在はjqのみサポートしていますが、将来的にJMESPathなど
/// 他のフィルター形式を追加する予定です。
#[derive(Debug, Clone)]
pub enum JsonFilter {
    /// jq式によるフィルター
    Jq(JqProgram),
}

#[derive(Debug, thiserror::Error)]
pub enum FilterError {
    #[error(transparent)]
    Regex(#[from] regex::Error),
    #[error("{0}")]
    Syntax(String),
    /// jqフィルターのロードエラー
    #[error("jq load error:\n{0}")]
    JqLoad(String),
    /// jqフィルターのコンパイルエラー
    #[error("jq compilation failed:\n{0}")]
    JqCompile(String),
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
    /// JSONログに適用するフィルター（jq、JMESPathなど）
    pub json_filter: Option<JsonFilter>,
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
                        let regex = Regex::new(&format!("^{}$", name))?;
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

                FilterAttribute::Jq(jq) => {
                    let program: File<&str, ()> = File {
                        code: &jq,
                        path: (),
                    };
                    let loader = Loader::new(jaq_std::defs().chain(jaq_json::defs()));
                    let arena = Arena::default();

                    let modules = loader
                        .load(&arena, program)
                        .map_err(|errors| FilterError::JqLoad(format_jaq_load_error(&errors)))?;

                    let compiled = jaq_core::Compiler::default()
                        .with_funs(jaq_std::funs().chain(jaq_json::funs()))
                        .compile(modules)
                        .map_err(|errors| {
                            FilterError::JqCompile(format_jaq_compile_error(&errors))
                        })?;

                    let json_filter = JsonFilter::Jq(JqProgram {
                        program: compiled,
                        code: jq.to_string(),
                    });

                    filter.json_filter = Some(json_filter);
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

fn format_jaq_load_error(errors: &jaq_core::load::Errors<&str, ()>) -> String {
    if errors.is_empty() {
        return "Unknown loading error".to_string();
    }

    let mut messages: Vec<String> = Vec::new();

    for (File { code, path: _ }, error) in errors {
        // Add code line with indent
        messages.push(format!("  Code:  {}", code));
        messages.push(String::new()); // blank line

        // Add error details with indent
        match error {
            jaq_core::load::Error::Io(items) => {
                for (path, msg) in items {
                    messages.push(format!("  Error: IO error - {} ({})", msg, path));
                }
            }
            jaq_core::load::Error::Lex(items) => {
                for (expect, s) in items {
                    messages.push(format!(
                        "  Error: Unexpected token - expected '{}', found '{}'",
                        expect.as_str(),
                        s
                    ));
                }
            }
            jaq_core::load::Error::Parse(items) => {
                for (expect, s) in items {
                    messages.push(format!(
                        "  Error: Parse error - expected '{}', found '{}'",
                        expect.as_str(),
                        s
                    ));
                }
            }
        }
    }

    if messages.is_empty() {
        "Loading failed".to_string()
    } else {
        messages.join("\n")
    }
}

fn format_jaq_compile_error(errors: &jaq_core::compile::Errors<&str, ()>) -> String {
    use jaq_core::compile::Undefined;

    if errors.is_empty() {
        return "Unknown compilation error".to_string();
    }

    let mut messages: Vec<String> = Vec::new();

    for (File { code, path: _ }, errors) in errors {
        // Add code line with indent
        messages.push(format!("  Code:  {}", code));
        messages.push(String::new()); // blank line

        // Add error details with indent
        for (name, undefined) in errors {
            let error_msg = match undefined {
                Undefined::Mod => format!("Undefined module '{}'", name),
                Undefined::Var => format!("Undefined variable '{}'", name),
                Undefined::Label => format!("Undefined label '{}'", name),
                Undefined::Filter(arity) => {
                    format!("Undefined filter '{}' (arity: {})", name, arity)
                }
                _ => format!("Undefined '{}'", name),
            };
            messages.push(format!("  Error: {}", error_msg));
        }
    }

    if messages.is_empty() {
        "Compilation failed".to_string()
    } else {
        messages.join("\n")
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
            buf.push(format!("field_selector={}", field_selector));
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

        if let Some(jq) = &self.json_filter {
            match jq {
                JsonFilter::Jq(jq) => buf.push(format!("jq={}", jq)),
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
    Pod(Cow<'a, str>),
    ExcludePod(Cow<'a, str>),
    Container(Cow<'a, str>),
    ExcludeContainer(Cow<'a, str>),
    Resource(SpecifiedResource<'a>),
    LabelSelector(Cow<'a, str>),
    FieldSelector(Cow<'a, str>),
    IncludeLog(Cow<'a, str>),
    ExcludeLog(Cow<'a, str>),
    Jq(Cow<'a, str>),
}

struct FilterAttributes;

impl FilterAttributes {
    fn parse(query: &str) -> Result<Vec<FilterAttribute<'_>>> {
        use nom::Err;
        use nom_language::error::{convert_error, VerboseError};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jq_filter_compilation_valid() {
        // 正常なjq式
        let result = Filter::parse("jq:.message");
        assert!(result.is_ok());
        let filter = result.unwrap();
        assert!(filter.json_filter.is_some());
    }

    #[test]
    fn test_jq_filter_compilation_complex() {
        // 複雑なjq式
        let result = Filter::parse("jq:{ts:.time,level:.level,msg:.msg}");
        assert!(result.is_ok());
        let filter = result.unwrap();
        assert!(filter.json_filter.is_some());
    }

    #[test]
    fn test_jq_filter_compilation_invalid_syntax() {
        // 無効なjq式（括弧の不一致）
        let result = Filter::parse("jq:invalid_syntax(((");
        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_msg = format!("{}", err);
        assert!(err_msg.contains("jq"));
    }

    #[test]
    fn test_jq_filter_compilation_invalid_filter() {
        // 無効なjq式（未定義の関数）
        let result = Filter::parse("jq:undefined_function()");
        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_msg = format!("{}", err);
        assert!(err_msg.contains("jq"));
    }

    #[test]
    fn test_jq_with_other_filters() {
        // jqと他のフィルターの組み合わせ
        let result = Filter::parse("pod:api log:error jq:.level");
        assert!(result.is_ok());
        let filter = result.unwrap();
        assert!(filter.pod.is_some());
        assert!(filter.include_log.is_some());
        assert!(filter.json_filter.is_some());
    }

    #[test]
    fn test_jq_with_container_and_exclude_filters() {
        // jqとcontainer、exclude_logの組み合わせ
        let result = Filter::parse("container:nginx !log:debug jq:.message");
        assert!(result.is_ok());
        let filter = result.unwrap();
        assert!(filter.container.is_some());
        assert!(filter.exclude_log.is_some());
        assert!(filter.json_filter.is_some());
    }

    #[test]
    fn test_multiple_jq_filters_last_wins() {
        // 複数のjqフィルターが指定された場合、最後のものが使用される
        let result = Filter::parse("jq:.message jq:.level");
        assert!(result.is_ok());
        let filter = result.unwrap();

        if let Some(JsonFilter::Jq(jq)) = &filter.json_filter {
            // 最後のjq式（.level）が使用されていることを確認
            assert_eq!(jq.code, ".level");
        } else {
            panic!("Expected jq filter to be present");
        }
    }

    #[test]
    fn test_jq_filter_display() {
        // Displayトレイトのテスト
        let filter = Filter::parse("pod:test jq:.message").unwrap();
        let display = format!("{}", filter);
        assert!(display.contains("jq=.message"));
    }
}
