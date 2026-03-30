#[derive(PartialEq, Debug, Clone)]
pub enum Status<Output> {
    Success(Output),
    SuccessWithoutOutput,
    Failure,
    Running(Output),
}
pub enum Node<Context, Output> {
    Selection(Vec<Node<Context, Output>>),
    Sequence(Vec<Node<Context, Output>>),
    Condition(Box<dyn Fn(&Context) -> bool + Send + Sync>),
    Action(Box<dyn Fn(&Context) -> Status<Output> + Send + Sync>),
}

impl<Context, Output> Node<Context, Output> {
    pub fn tick(&self, context: &Context) -> Status<Output> {
        match self {
            Node::Selection(children) => {
                for child in children {
                    let status = child.tick(context);
                    if matches!(
                        status,
                        Status::Success(_) | Status::SuccessWithoutOutput | Status::Running(_)
                    ) {
                        return status;
                    }
                }
                Status::Failure
            }
            Node::Sequence(children) => {
                let mut status = Status::Failure;
                for child in children {
                    status = child.tick(context);
                    if matches!(status, Status::Failure) {
                        return Status::Failure;
                    }
                }
                status
            }
            Node::Condition(condition) => {
                if condition(context) {
                    Status::SuccessWithoutOutput
                } else {
                    Status::Failure
                }
            }
            Node::Action(action) => action(context),
        }
    }
}

pub fn condition<Context, Output, F>(f: F) -> Node<Context, Output>
where
    F: Fn(&Context) -> bool + Send + Sync + 'static,
{
    Node::Condition(Box::new(f))
}

pub fn action<Context, Output, F>(f: F) -> Node<Context, Output>
where
    // No Option! Just exactly what successful actions yield.
    F: Fn(&Context) -> Status<Output> + Send + Sync + 'static,
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
