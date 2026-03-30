#[derive(PartialEq, Eq)]
pub enum Status {
    Success,
    Failure,
    Running,
}
pub enum Node<Context> {
    Selection(Vec<Node<Context>>),
    Sequence(Vec<Node<Context>>),
    Condition(Box<dyn Fn(&Context) -> Status + Send + Sync>),
    Action(Box<dyn Fn(&Context) -> Status + Send + Sync>),
}

impl<Context> Node<Context> {
    pub fn tick(&self, context: &Context) -> Status {
        match self {
            Node::Selection(children) => {
                for child in children {
                    if child.tick(context) == Status::Success {
                        return Status::Success;
                    }
                }
                Status::Failure
            }
            Node::Sequence(children) => {
                for child in children {
                    if child.tick(context) == Status::Failure {
                        return Status::Failure;
                    }
                }
                Status::Success
            }
            Node::Condition(condition) => condition(context),
            Node::Action(action) => action(context),
        }
    }
}

pub fn condition<Context, F>(f: F) -> Node<Context>
where
    F: Fn(&Context) -> Status + Send + Sync + 'static,
{
    Node::Condition(Box::new(f))
}

pub fn action<Context, F>(f: F) -> Node<Context>
where
    F: Fn(&Context) -> Status + Send + Sync + 'static,
{
    Node::Action(Box::new(f))
}

#[macro_export]
macro_rules! selection {
    ($($child:expr),* $(,)?) => {
        $crate::behavior::behavior_tree::nodes::Node::Selection(vec![$($child),*])
    };
}

#[macro_export]
macro_rules! sequence {
    ($($child:expr),* $(,)?) => {
        $crate::behavior::behavior_tree::nodes::Node::Sequence(vec![$($child),*])
    };
}
