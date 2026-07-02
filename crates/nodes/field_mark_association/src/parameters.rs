use ros_z::Message;
use serde::{Deserialize, Serialize};

use crate::global_association::{
    GlobalAssociationConfig as GlobalLocalizerParameters,
    PoseHintAssociationConfig as PoseHintAssociationParameters,
};

#[derive(Clone, Default, Debug, Deserialize, Serialize, Message)]
#[serde(deny_unknown_fields)]
pub struct FieldMarkAssociationParameters {
    pub global_localizer: GlobalLocalizerParameters,
    pub pose_hint: PoseHintAssociationParameters,
}

impl FieldMarkAssociationParameters {
    pub(crate) fn validate(&self) -> std::result::Result<(), String> {
        self.global_localizer.validate()?;
        self.pose_hint.validate()
    }
}
