#include "Brain/Behavior/Units.hpp"

ActionCommand penaltyShootoutStriker(const DataSet& d)
{
  if (d.penaltyStrikerAction.valid)
  {
    return walkToBallAndKick(d, d.penaltyStrikerAction.kickPose, d.penaltyStrikerAction.kickable,
                             d.penaltyStrikerAction.target, false, Velocity(0.5f, 0.5f, true),
                             d.penaltyStrikerAction.kickType)
        .combineHead(activeVision(d, VisionMode::BALL_TRACKER))
        .combineLeftLED(ActionCommand::LED::red());
  }
  return ActionCommand::stand().combineHead(activeVision(d, VisionMode::BALL_TRACKER));
}

ActionCommand penaltyKeeper(const DataSet& d)
{
  switch (d.penaltyKeeperAction.type)
  {
    case PenaltyKeeperAction::Type::SQUAT:
      return ActionCommand::jump(JumpOutput::Type::SQUAT)
          .combineLeftLED(ActionCommand::LED::green());
    case PenaltyKeeperAction::Type::JUMP_LEFT:
      return ActionCommand::jump(JumpOutput::Type::JUMP_LEFT)
          .combineLeftLED(ActionCommand::LED::red());
    case PenaltyKeeperAction::Type::JUMP_RIGHT:
      return ActionCommand::jump(JumpOutput::Type::JUMP_RIGHT)
          .combineLeftLED(ActionCommand::LED::yellow());
    case PenaltyKeeperAction::Type::WAIT:
      break;
  }
  return ActionCommand::stand()
      .combineHead(activeVision(d, VisionMode::BALL_TRACKER))
      .combineLeftLED(ActionCommand::LED::lightblue());
}

ActionCommand penaltyShootoutPlaying(const DataSet& d)
{
  if (d.gameControllerState.kickingTeam)
  {
    return penaltyShootoutStriker(d).combineRightLED(ActionCommand::LED::red());
  }
  return penaltyKeeper(d).combineRightLED(ActionCommand::LED::blue());
}
