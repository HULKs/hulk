#include "Tools/Chronometer.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Geometry.hpp"

#include "JumpActionProvider.hpp"


JumpActionProvider::JumpActionProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , ballState_(*this)
  , robotPosition_(*this)
  , fieldDimensions_(*this)
  , standingRobotRadius_(*this, "standingRobotRadius", [] {})
  , squattedRobotRadius_(*this, "squattedRobotRadius", [] {})
  , jumpedRobotRadius_(*this, "jumpedRobotRadius", [] {})
  , jumpAction_(*this)
{
}


void JumpActionProvider::cycle()
{
  Chronometer time(debug(), mount_ + ".cycleTime");

  const Vector2f relBallPosition = ballState_->position;
  const Vector2f relBallDestination = ballState_->destination;

  // ball is found and moving in a way a destination was calculated
  if ((ballState_->found && relBallPosition != relBallDestination))
  {
    // ball is in front of the robot and stop behind me
    if (relBallPosition.x() > 0 && relBallDestination.x() < 0)
    {
      Line<float> ballToDestinationLine(relBallPosition, relBallDestination);
      // distance the ball would pass the robot on its y axis
      float passDistance = ballToDestinationLine.getY(0);

      if (std::abs(passDistance) < standingRobotRadius_())
      {
        jumpAction_->suggestedType = JumpAction::Type::NONE;
        jumpAction_->valid = true;
      }
      else if (std::abs(passDistance) < squattedRobotRadius_())
      {
        jumpAction_->canCatchWithSquat = true;
        jumpAction_->suggestedType = JumpAction::Type::SQUAT;
        jumpAction_->valid = true;
      }
      else if (std::abs(passDistance) < jumpedRobotRadius_())
      {
        jumpAction_->canCatchWithJump = true;
        jumpAction_->suggestedType =
            passDistance < 0 ? JumpAction::Type::JUMP_RIGHT : JumpAction::Type::JUMP_LEFT;
        jumpAction_->valid = true;
      }
      else
      {
        jumpAction_->suggestedType = JumpAction::Type::NONE;
        jumpAction_->valid = true;
      }
    }
  }
}
