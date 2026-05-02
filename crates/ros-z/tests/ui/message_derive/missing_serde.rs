use ros_z::Message;

#[derive(Debug, Clone, Message)]
struct MissingSerde {
    value: u32,
}

fn main() {}
