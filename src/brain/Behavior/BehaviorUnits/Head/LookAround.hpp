#pragma once
#include "Behavior/Units.hpp"

ActionCommand::Head lookAround(const DataSet& d, const float yawMax = 0.f, const float timeToRest = 0.3)
{
  assert(yawMax >= 0.f);
  assert((yawMax == 0.f || yawMax >= d.parameters.lookAroundInnerYaw()) && "Outer yaw was chosen even smaller than inner yaw");
  const auto& configuredOuterPosition = d.parameters.lookAroundOuterPosition();
  const Vector2f limitedOuterPosition = {yawMax, configuredOuterPosition.y()};
  const Vector2f& headPosition = (yawMax == 0.f) ? configuredOuterPosition : limitedOuterPosition;
  const auto yawVelocity = d.parameters.lookAroundYawVelocity();
  const auto middleHeadYaw = d.parameters.lookAroundInnerYaw();
  const auto lastTargetWasMiddleOffset = d.lastActionCommand.head().yaw() == middleHeadYaw;
  const auto lastTargetWasMinusMiddleOffset = d.lastActionCommand.head().yaw() == -middleHeadYaw;
  const auto currentTargetIsMiddleOffset = d.headMotionOutput.target[0] == middleHeadYaw;
  const auto currentTargetIsMinusMiddleOffset = d.headMotionOutput.target[0] == -middleHeadYaw;
  const auto restedOnTargetPosition = d.cycleInfo.getTimeDiff(d.headMotionOutput.timeWhenReachedTarget) > timeToRest;
  const auto lastHeadActionPositive = d.lastActionCommand.head().yaw() > 0;
  const auto lastHeadActionNegative = d.lastActionCommand.head().yaw() < 0;
  const auto lastHeadActionWasLeftOuter = d.headMotionOutput.target[0] == headPosition[0];
  const auto lastHeadActionWasRightOuter = d.headMotionOutput.target[0] == -headPosition[0];
  const auto lastHeadActionActionDiffersFromSearchStates =
      std::abs(d.headMotionOutput.target[0]) != headPosition[0] && std::abs(d.headMotionOutput.target[0]) != middleHeadYaw;

  if (d.motionState.headMotion != MotionRequest::HeadMotion::ANGLES ||
      (lastHeadActionPositive && currentTargetIsMiddleOffset && lastTargetWasMiddleOffset && d.headMotionOutput.atTarget && restedOnTargetPosition))
  {
    return ActionCommand::Head::angles(headPosition[0], headPosition[1], yawVelocity);
  }
  else if ((lastHeadActionPositive && !lastTargetWasMinusMiddleOffset && lastHeadActionWasLeftOuter && d.headMotionOutput.atTarget && restedOnTargetPosition))
  {
    return ActionCommand::Head::angles(-middleHeadYaw, headPosition[1], yawVelocity);
  }
  else if (lastHeadActionNegative && lastTargetWasMinusMiddleOffset && d.headMotionOutput.atTarget && restedOnTargetPosition &&
           currentTargetIsMinusMiddleOffset)
  {
    return ActionCommand::Head::angles(-headPosition[0], headPosition[1], yawVelocity);
  }
  else if (lastHeadActionNegative && !lastTargetWasMiddleOffset && d.headMotionOutput.atTarget && restedOnTargetPosition && lastHeadActionWasRightOuter)
  {
    return ActionCommand::Head::angles(middleHeadYaw, headPosition[1], yawVelocity);
  }
  else if (lastHeadActionActionDiffersFromSearchStates)
  {
    return ActionCommand::Head::angles(headPosition[0], headPosition[1], yawVelocity);
  }
  return ActionCommand::Head::angles(d.lastActionCommand.head().yaw(), headPosition[1], yawVelocity);
}
