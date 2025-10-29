use types::step::Step;

#[derive(Clone, Debug, Default)]
pub struct Parameters {
    pub remote_controll_parameters: RemoteControllParameters,
}

pub struct RemoteControllParameters {
    pub walk: Step,
}
