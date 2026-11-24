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
    Subtree {
        name: &'static str,
        tree: Box<Node<Blackboard>>,
    },
}

impl<Blackboard> Node<Blackboard> {
    pub fn tick_with_trace(&self, blackboard: &mut Blackboard) -> (Status, NodeTrace) {
        let (status, trace) = self.tick(blackboard, true);
        (status, trace.expect("trace collection enabled"))
    }

    pub(crate) fn tick(
        &self,
        blackboard: &mut Blackboard,
        record_trace: bool,
    ) -> (Status, Option<NodeTrace>) {
        let name = match self {
            Node::Action { name, .. }
            | Node::Condition { name, .. }
            | Node::Negation { name, .. }
            | Node::Selection { name, .. }
            | Node::Sequence { name, .. }
            | Node::Subtree { name, .. } => name,
            Node::Failure => &"Failure",
        };
        let mut trace = record_trace.then(|| NodeTrace {
            name: name.to_string(),
            status: Status::Failure,
            children: Vec::new(),
        });
        let tick_child =
            |child: &Self, blackboard: &mut Blackboard, trace: &mut Option<NodeTrace>| {
                let (status, child_trace) = child.tick(blackboard, record_trace);
                if let (Some(trace), Some(child_trace)) = (trace, child_trace) {
                    trace.children.push(child_trace);
                }
                status
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
                let child_status = tick_child(child, blackboard, &mut trace);
                match child_status {
                    Status::Success => Status::Failure,
                    Status::Failure => Status::Success,
                    _ => child_status,
                }
            }
            Node::Selection { children, .. } => {
                let mut selection_status = Status::Failure;
                for child in children {
                    let child_status = tick_child(child, blackboard, &mut trace);

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
                    let child_status = tick_child(child, blackboard, &mut trace);

                    if matches!(child_status, Status::Failure) {
                        sequence_status = child_status;
                        break;
                    }
                }
                sequence_status
            }
            Node::Subtree { tree, .. } => tick_child(tree, blackboard, &mut trace),
        };

        if let Some(trace) = &mut trace {
            trace.status = status.clone();
        }
        (status, trace)
    }

    pub fn static_layout_trace(&self) -> NodeTrace {
        let name = match self {
            Node::Action { name, .. }
            | Node::Condition { name, .. }
            | Node::Negation { name, .. }
            | Node::Selection { name, .. }
            | Node::Sequence { name, .. }
            | Node::Subtree { name, .. } => name,
            Node::Failure => &"Failure",
        };

        let children = match self {
            Node::Selection { children, .. } | Node::Sequence { children, .. } => {
                children.iter().map(|c| c.static_layout_trace()).collect()
            }
            Node::Negation { child, .. } => vec![child.static_layout_trace()],
            Node::Subtree { tree, .. } => vec![tree.static_layout_trace()],
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
        $crate::behavior_tree::Node::Action{
            name: stringify!($func),
            action: Box::new($func)
        }
    };
    ($func:expr, $($arg:expr),+ $(,)?) => {
        $crate::behavior_tree::Node::Action{
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
        $crate::behavior_tree::Node::Condition {
            name: stringify!($func),
            condition: Box::new($func),
        }
    };
    ($func:ident, $($arg:expr),+ $(,)?) => {
        $crate::behavior_tree::Node::Condition {
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
        $crate::behavior_tree::Node::Negation {
            name: "Negation",
            child: Box::new($child),
        }
    };
}

#[macro_export]
macro_rules! selection {
    ($($child:expr),* $(,)?) => {
        $crate::behavior_tree::Node::Selection{
            name: "Selection",
            children: vec![$($child),*]
        }
    };
}

#[macro_export]
macro_rules! sequence {
    ($($child:expr),* $(,)?) => {
        $crate::behavior_tree::Node::Sequence{
            name: "Sequence",
            children: vec![$($child),*]
        }
    };
}

#[macro_export]
macro_rules! subtree {
    ($func:expr) => {
        $crate::behavior_tree::Node::Subtree {
            name: concat!("subtree_", stringify!($func)),
            tree: Box::new($func()),
        }
    };
    ($func:expr, $($arg:expr),+ $(,)?) => {
        $crate::behavior_tree::Node::Subtree {
            name: concat!("subtree_", stringify!($func:$($arg),+)),
            tree: Box::new($func($($arg.clone()),+)),
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
            Node::Failure => ("Failure", "Failure", None),
            Node::Negation { name, child } => ("Negation", *name, Some(from_ref(child.as_ref()))),
            Node::Selection { name, children } => ("Selection", *name, Some(children.as_slice())),
            Node::Sequence { name, children } => ("Sequence", *name, Some(children.as_slice())),
            Node::Subtree { name, tree } => ("Subtree", *name, Some(from_ref(tree.as_ref()))),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_tracing_preserves_execution_and_short_circuiting() {
        let tree = Node::Sequence {
            name: "sequence",
            children: vec![
                Node::Condition {
                    name: "condition",
                    condition: Box::new(|visits: &mut Vec<&str>| {
                        visits.push("condition");
                        true
                    }),
                },
                Node::Selection {
                    name: "selection",
                    children: vec![
                        Node::Failure,
                        Node::Subtree {
                            name: "subtree",
                            tree: Box::new(Node::Action {
                                name: "action",
                                action: Box::new(|visits| {
                                    visits.push("action");
                                    Status::Success
                                }),
                            }),
                        },
                        Node::Action {
                            name: "skipped",
                            action: Box::new(|_| panic!("selection did not short circuit")),
                        },
                    ],
                },
                Node::Negation {
                    name: "negation",
                    child: Box::new(Node::Failure),
                },
                Node::Failure,
                Node::Action {
                    name: "skipped",
                    action: Box::new(|_| panic!("sequence did not short circuit")),
                },
            ],
        };

        for record_trace in [false, true] {
            let mut visits = Vec::new();
            let (status, trace) = tree.tick(&mut visits, record_trace);
            assert_eq!(status, Status::Failure);
            assert_eq!(visits, ["condition", "action"]);
            assert_eq!(trace.is_some(), record_trace);
            if let Some(trace) = trace {
                assert_eq!(trace.name, "sequence");
                assert_eq!(trace.status, status);
                assert_eq!(trace.children.len(), 4);
                assert_eq!(trace.children[1].children.len(), 2);
                assert_eq!(trace.children[1].children[1].children[0].name, "action");
                assert_eq!(trace.children[2].status, Status::Success);
            }
        }
    }
}
