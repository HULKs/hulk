

interpolator: MotionFile::from_path(paths.motions.join("sit_down.json"))?.try_into()?,
        

let last_cycle_duration = context.cycle_time.last_cycle_duration;

if context.motion_selection.current_motion == MotionType::SitDown {
    self.interpolator
        .advance_by(last_cycle_duration, context.condition_input);
} else {
    self.interpolator.reset();
}

context.motion_safe_exits[MotionType::SitDown] = self.interpolator.is_finished();

Ok(MainOutputs {
    sit_down_joints_command: MotorCommands {
        positions: self.interpolator.value(),
        stiffnesses: Joints::fill(0.8),
    }
    .into(),
})
