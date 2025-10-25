#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct Parameters {
    pub remote_controll_parameters: RemoteControllParameters,
}

pub struct RemoteControllParameters {
    pub walk: Step,
}
