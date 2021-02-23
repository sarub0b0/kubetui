pub mod log;
pub mod pod;

use self::log::Logs;
use self::pod::Pods;

pub trait WidgetTrait {
    fn selectable(&self) -> bool;
}

pub enum Widgets {
    Pod(Pods),
    Log(Logs),
}

impl Widgets {
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

    pub fn mut_pod(&mut self) -> Option<&mut Pods> {
        match self {
            Widgets::Pod(pod) => Some(pod),
            _ => None,
        }
    }

    pub fn mut_log(&mut self) -> Option<&mut Logs> {
        match self {
            Widgets::Log(log) => Some(log),
            _ => None,
        }
    }

    fn selectable(&self) -> bool {
        match self {
            Widgets::Pod(pod) => pod.selectable(),
            Widgets::Log(log) => log.selectable(),
        }
    }

    pub fn next(&self) {
        match self {
            Widgets::Pod(pod) => pod.next(),
            Widgets::Log(log) => log.next(),
        }
    }

    pub fn prev(&self) {
        match self {
            Widgets::Pod(pod) => pod.prev(),
            Widgets::Log(log) => log.prev(),
        }
    }
}

impl WidgetTrait for Widgets {
    fn selectable(&self) -> bool {
        match self {
            Widgets::Pod(pod) => pod.selectable(),
            Widgets::Log(log) => log.selectable(),
        }
    }
}
