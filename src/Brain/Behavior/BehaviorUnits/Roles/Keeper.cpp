#include "Brain/Behavior/Units.hpp"
#include "Tools/SelectWalkMode.hpp"

ActionCommand keeper(const DataSet& d)
{
  // only use keeper action if it is valid
  if (d.keeperAction.action.valid)
  {
    switch (d.keeperAction.action.type)
    {
      case KeeperAction::Type::BLOCK_GOAL: {
        const Pose relPlayingPose = d.robotPosition.fieldToRobot(d.keeperAction.action.pose);

        // select walk mode
        const float distanceThreshold = 1.5f;
        const float angleThreshold = 30 * TO_RAD;
        const ActionCommand::Body::WalkMode walkMode = SelectWalkMode::pathOrPathWithOrientation(
            relPlayingPose, distanceThreshold, angleThreshold);

        return walkToPose(d, relPlayingPose, false, walkMode)
            .combineHead(activeVision(d, VisionMode::LOOK_AROUND_BALL));
      }
      case KeeperAction::Type::SQUAT: {
        return ActionCommand::jump(JumpOutput::Type::SQUAT);
      }
      default: {
        return ActionCommand::stand().combineHead(activeVision(d, VisionMode::LOOK_AROUND));
      }
    }
  }
  else
  {
    Log<M_BRAIN>(LogLevel::WARNING) << "Invalid keeper action";
    return ActionCommand::stand().combineHead(activeVision(d, VisionMode::LOOK_AROUND));
  }
}
