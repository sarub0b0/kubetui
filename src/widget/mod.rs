pub mod log;
pub mod pod;

pub use self::log::*;
pub use self::pod::*;

pub trait WidgetTrait {
    fn selectable(&self) -> bool;
}

pub enum Widgets<'a> {
    Pod(Pods<'a>),
    Log(Logs<'a>),
}

impl<'a> Widgets<'a> {
    pub fn pod(&self) -> Option<&Pods> {
        match self {
            Widgets::Pod(pod) => Some(pod),
            _ => None,
        }
    }

    pub fn log(&self) -> Option<&Logs> {
        match self {
            Widgets::Log(log) => Some(log),
            _ => None,
        }
    }

    pub fn mut_pod(&mut self) -> Option<&mut Pods<'a>> {
        match self {
            Widgets::Pod(pod) => Some(pod),
            _ => None,
        }
    }

    pub fn mut_log(&mut self) -> Option<&mut Logs<'a>> {
        match self {
            Widgets::Log(log) => Some(log),
            _ => None,
        }
    }

    pub fn next(&mut self) {
        match self {
            Widgets::Pod(pod) => pod.next(),
            Widgets::Log(log) => log.next(),
        }
    }

    pub fn prev(&mut self) {
        match self {
            Widgets::Pod(pod) => pod.prev(),
            Widgets::Log(log) => log.prev(),
        }
    }

    pub fn first(&mut self) {
        match self {
            Widgets::Pod(pod) => pod.select_first(),
            Widgets::Log(log) => log.scroll_top(),
        }
    }

    pub fn last(&mut self) {
        match self {
            Widgets::Pod(pod) => pod.select_last(),
            Widgets::Log(log) => log.scroll_bottom(),
        }
    }
}

impl WidgetTrait for Widgets<'_> {
    fn selectable(&self) -> bool {
        match self {
            Widgets::Pod(pod) => pod.selectable(),
            Widgets::Log(log) => log.selectable(),
        }
    }
}
