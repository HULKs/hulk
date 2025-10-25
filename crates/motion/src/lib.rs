use parameters::Parameters;

pub mod parameters;

pub struct Context<'a> {
    pub parameters: &'a Parameters,
}
