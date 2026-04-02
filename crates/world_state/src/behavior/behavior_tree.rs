use serde::{Serialize, ser::SerializeStruct};
use types::behavior_tree::{NodeTrace, Status};

type ConditionFunction<Context> = Box<dyn Fn(&mut Context) -> bool + Send + Sync>;
type ActionFunction<Context> = Box<dyn Fn(&mut Context) -> Status + Send + Sync>;

pub enum Node<Context> {
    Action {
        name: &'static str,
        action: ActionFunction<Context>,
    },
    Condition {
        name: &'static str,
        condition: ConditionFunction<Context>,
    },
    Failure,
    Selection {
        name: &'static str,
        children: Vec<Node<Context>>,
    },
    Sequence {
        name: &'static str,
        children: Vec<Node<Context>>,
    },
}

impl<Context> Node<Context> {
    pub fn tick_with_trace(&self, context: &mut Context) -> (Status, NodeTrace) {
        let name = match self {
            Node::Action { name, .. }
            | Node::Condition { name, .. }
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
            Node::Action { action, .. } => action(context),
            Node::Condition { condition, .. } => {
                if condition(context) {
                    Status::Success
                } else {
                    Status::Failure
                }
            }
            Node::Failure => Status::Failure,
            Node::Selection { children, .. } => {
                let mut selection_status = Status::Failure;
                for child in children {
                    let (child_status, child_trace) = child.tick_with_trace(context);
                    trace.children.push(child_trace);

                    if matches!(child_status, Status::Success | Status::Running) {
                        selection_status = child_status;
                        break;
                    }
                }
                selection_status
            }
            Node::Sequence { children, .. } => {
                let mut sequence_status = Status::Success;
                for child in children {
                    let (child_status, child_trace) = child.tick_with_trace(context);
                    trace.children.push(child_trace);

                    if matches!(child_status, Status::Failure | Status::Running) {
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
            | Node::Selection { name, .. }
            | Node::Sequence { name, .. } => name,
            Node::Failure => &"Failure",
        };

        let children = match self {
            Node::Selection { children, .. } | Node::Sequence { children, .. } => {
                children.iter().map(|c| c.static_layout_trace()).collect()
            }
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

impl<Context> Serialize for Node<Context> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let (node_type, name, children) = match self {
            Node::Action { name, .. } => ("Action", *name, None),
            Node::Condition { name, .. } => ("Condition", *name, None),
            Node::Failure => ("Failure", "Failure", None),
            Node::Selection { name, children } => ("Selection", *name, Some(children)),
            Node::Sequence { name, children } => ("Sequence", *name, Some(children)),
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
