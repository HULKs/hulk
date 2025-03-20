#[derive(Deserialize, Serialize)]
pub struct SitDown {}

pub fn sit_down() -> bool {
    interpolator: MotionFile::from_path(paths.motions.join("sit_down.json"))?.try_into()?,


    let last_cycle_duration = context.cycle_time.last_cycle_duration;

        self.interpolator
            .advance_by(last_cycle_duration, context.condition_input);

    context.motion_safe_exits[MotionType::SitDown] = self.interpolator.is_finished();

    Ok(MainOutputs {
        sit_down_joints_command: MotorCommands {
            positions: self.interpolator.value(),
            stiffnesses: Joints::fill(0.8),
        }
        .into(),
    })
}
