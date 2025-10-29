
pub fn remote_controll(context: &Context) {
    let walk_parameters = &context.parameters.remote_controll_parameters.walk;
    let motion_command = MotionCommand::Walk{
        head: HeadMotion::Center,
        left_arm: ArmMotion::Relaxed,
        right_arm: ArmMotion::Relaxed,
        forward: walk_parameters.forward,
        left: walk_parameters.left,
        turn: walk_parameters.turn,
    };


}