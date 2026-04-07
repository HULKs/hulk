use std::slice::from_ref;

use serde::{Serialize, ser::SerializeStruct};
use types::behavior_tree::{NodeTrace, Status};

type ConditionFunction<Blackboard> = Box<dyn Fn(&mut Blackboard) -> bool + Send + Sync>;
type ActionFunction<Blackboard> = Box<dyn Fn(&mut Blackboard) -> Status + Send + Sync>;

pub enum Node<Blackboard> {
    Action {
        name: &'static str,
        action: ActionFunction<Blackboard>,
    },
    Condition {
        name: &'static str,
        condition: ConditionFunction<Blackboard>,
    },
    Failure,
    Negation {
        name: &'static str,
        child: Box<Node<Blackboard>>,
    },
    Selection {
        name: &'static str,
        children: Vec<Node<Blackboard>>,
    },
    Sequence {
        name: &'static str,
        children: Vec<Node<Blackboard>>,
    },
}

impl<Blackboard> Node<Blackboard> {
    pub fn tick_with_trace(&self, blackboard: &mut Blackboard) -> (Status, NodeTrace) {
        let name = match self {
            Node::Action { name, .. }
            | Node::Condition { name, .. }
            | Node::Negation { name, .. }
            | Node::Selection { name, .. }
            | Node::Sequence { name, .. } => name,
            Node::Failure => &"Failure",
        };
        let mut trace = NodeTrace {
            name: name.to_string(),
            status: Status::Failure,
            children: Vec::new(),
        };

        let status = match self {
            Node::Action { action, .. } => action(blackboard),
            Node::Condition { condition, .. } => {
                if condition(blackboard) {
                    Status::Success
                } else {
                    Status::Failure
                }
            }
            Node::Failure => Status::Failure,
            Node::Negation { child, .. } => {
                let (child_status, child_trace) = child.tick_with_trace(blackboard);
                trace.children.push(child_trace);
                match child_status {
                    Status::Success => Status::Failure,
                    Status::Failure => Status::Success,
                    _ => child_status,
                }
            }
            Node::Selection { children, .. } => {
                let mut selection_status = Status::Failure;
                for child in children {
                    let (child_status, child_trace) = child.tick_with_trace(blackboard);
                    trace.children.push(child_trace);

                    if matches!(child_status, Status::Success) {
                        selection_status = child_status;
                        break;
                    }
                }
                selection_status
            }
            Node::Sequence { children, .. } => {
                let mut sequence_status = Status::Success;
                for child in children {
                    let (child_status, child_trace) = child.tick_with_trace(blackboard);
                    trace.children.push(child_trace);

                    if matches!(child_status, Status::Failure) {
                        sequence_status = child_status;
                        break;
                    }
                }
                sequence_status
            }
        };

        trace.status = status.clone();
        (status, trace)
    }

    pub fn static_layout_trace(&self) -> NodeTrace {
        let name = match self {
            Node::Action { name, .. }
            | Node::Condition { name, .. }
            | Node::Negation { name, .. }
            | Node::Selection { name, .. }
            | Node::Sequence { name, .. } => name,
            Node::Failure => &"Failure",
        };

        let children = match self {
            Node::Selection { children, .. } | Node::Sequence { children, .. } => {
                children.iter().map(|c| c.static_layout_trace()).collect()
            }
            Node::Negation { child, .. } => vec![child.static_layout_trace()],
            _ => vec![],
        };

        NodeTrace {
            name: name.to_string(),
            status: Status::Idle,
            children,
        }
    }
}

#[macro_export]
macro_rules! action {
    ($func:expr) => {
        $crate::behavior::behavior_tree::Node::Action{
            name: stringify!($func),
            action: Box::new($func)
        }
    };
    ($func:expr, $($arg:expr),+ $(,)?) => {
        $crate::behavior::behavior_tree::Node::Action{
            name: stringify!($func:$($arg),+),
            action: Box::new(move |ctx| {
                $func(ctx, $($arg.clone()),+)
            })
        }
    };
}

#[macro_export]
macro_rules! condition {
    ($func:ident) => {
        $crate::behavior::behavior_tree::Node::Condition {
            name: stringify!($func),
            condition: Box::new($func),
        }
    };
    ($func:ident, $($arg:expr),+ $(,)?) => {
        $crate::behavior::behavior_tree::Node::Condition {
            name: stringify!($func:$($arg),+),
            condition: Box::new(move |ctx| {
                $func(ctx, $($arg.clone()),+)
            }),
        }
    };
}

#[macro_export]
macro_rules! negation {
    ($child:expr) => {
        $crate::behavior::behavior_tree::Node::Negation {
            name: "Negation",
            child: Box::new($child),
        }
    };
}

#[macro_export]
macro_rules! selection {
    ($($child:expr),* $(,)?) => {
        $crate::behavior::behavior_tree::Node::Selection{
            name: "Selection",
            children: vec![$($child),*]
        }
    };
}

#[macro_export]
macro_rules! sequence {
    ($($child:expr),* $(,)?) => {
        $crate::behavior::behavior_tree::Node::Sequence{
            name: "Sequence",
            children: vec![$($child),*]
        }
    };
}

impl<Blackboard> Serialize for Node<Blackboard> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let (node_type, name, children) = match self {
            Node::Action { name, .. } => ("Action", *name, None),
            Node::Condition { name, .. } => ("Condition", *name, None),
            Node::Failure => ("Failure", "Failure", None), // Note: updated string ref
            Node::Negation { name, child } => ("Negation", *name, Some(from_ref(child.as_ref()))),
            Node::Selection { name, children } => ("Selection", *name, Some(children.as_slice())),
            Node::Sequence { name, children } => ("Sequence", *name, Some(children.as_slice())),
        };

        let num_fields = if children.is_some() { 3 } else { 2 };
        let mut state = serializer.serialize_struct("Node", num_fields)?;

        state.serialize_field("type", node_type)?;
        state.serialize_field("name", name)?;

        if let Some(c) = children {
            state.serialize_field("children", c)?;
        }

        state.end()
    }
}
