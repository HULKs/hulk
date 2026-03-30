#[derive(PartialEq, Debug, Clone)]
pub enum Status {
    Success,
    Failure,
    Running,
}

type ConditionFunction<Context> = Box<dyn Fn(&mut Context) -> bool + Send + Sync>;
type ActionFunction<Context> = Box<dyn Fn(&mut Context) -> Status + Send + Sync>;

pub enum Node<Context> {
    Selection(Vec<Node<Context>>),
    Sequence(Vec<Node<Context>>),
    Condition(ConditionFunction<Context>),
    Action(ActionFunction<Context>),
}

impl<Context> Node<Context> {
    pub fn tick(&self, context: &mut Context) -> Status {
        match self {
            Node::Selection(children) => {
                for child in children {
                    let status = child.tick(context);
                    if matches!(status, Status::Success | Status::Running) {
                        return status;
                    }
                }
                Status::Failure
            }
            Node::Sequence(children) => {
                for child in children {
                    let status = child.tick(context);
                    if matches!(status, Status::Failure | Status::Running) {
                        return status;
                    }
                }
                Status::Success
            }
            Node::Condition(condition) => {
                if condition(context) {
                    Status::Success
                } else {
                    Status::Failure
                }
            }
            Node::Action(action) => action(context),
        }
    }
}

pub fn condition<Context, F>(f: F) -> Node<Context>
where
    F: Fn(&mut Context) -> bool + Send + Sync + 'static,
{
    Node::Condition(Box::new(f))
}

pub fn action<Context, F>(f: F) -> Node<Context>
where
    F: Fn(&mut Context) -> Status + Send + Sync + 'static,
{
    Node::Action(Box::new(f))
}

#[macro_export]
macro_rules! selection {
    ($($child:expr),* $(,)?) => {
        $crate::behavior::new_behavior::behavior_tree::Node::Selection(vec![$($child),*])
    };
}

#[macro_export]
macro_rules! sequence {
    ($($child:expr),* $(,)?) => {
        $crate::behavior::new_behavior::behavior_tree::Node::Sequence(vec![$($child),*])
    };
}
