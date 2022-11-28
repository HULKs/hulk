pub mod message_receiver;
pub mod spl_message_sender;

#[derive(Clone, Copy, Debug)]
pub enum CyclerInstance {
    SplNetwork,
}
