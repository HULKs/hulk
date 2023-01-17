use super::client::Client;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClientRequest<Request> {
    pub request: Request,
    pub client: Client,
}
