#pragma once
#include "Behavior/Units.hpp"

ActionCommand::Head lookAround(const DataSet& d, const float yawMax = 0.f,
                               const bool useEffectiveYawVelocity = true,
                               const float timeToRest = 0.3)
{
  /*
   * The desired head yaw is chosen from four distinct angles
   * Two are at outer positions and two are in the center (presumably to compensate joint backlash).
   */
  assert(yawMax >= 0.f);
  assert((yawMax == 0.f || yawMax >= d.parameters.lookAroundInnerYaw()) &&
         "Outer yaw was chosen even smaller than inner yaw");
  const auto& configuredOuterPosition = d.parameters.lookAroundOuterPosition();
  const Vector2f limitedOuterPosition = {yawMax, configuredOuterPosition.y()};
  /// the outer head yaw
  const Vector2f& headPosition = (yawMax == 0.f) ? configuredOuterPosition : limitedOuterPosition;
  const auto yawVelocity = d.parameters.lookAroundYawVelocity();
  /// the inner head yaw
  const auto middleHeadYaw = d.parameters.lookAroundInnerYaw();
  /// These cumbersome conditions are required because an ActionCommand has no state.
  const bool lastTargetWasMiddleOffset = d.lastActionCommand.head().yaw() == middleHeadYaw;
  const bool lastTargetWasMinusMiddleOffset = d.lastActionCommand.head().yaw() == -middleHeadYaw;
  const bool currentTargetIsMiddleOffset = d.headMotionOutput.target[0] == middleHeadYaw;
  const bool currentTargetIsMinusMiddleOffset = d.headMotionOutput.target[0] == -middleHeadYaw;
  const bool restedOnTargetPosition =
      d.cycleInfo.getTimeDiff(d.headMotionOutput.timeWhenReachedTarget) > timeToRest;
  const bool lastHeadActionPositive = d.lastActionCommand.head().yaw() > 0;
  const bool lastHeadActionNegative = d.lastActionCommand.head().yaw() < 0;
  const bool lastHeadActionWasLeftOuter = d.headMotionOutput.target[0] == headPosition[0];
  const bool lastHeadActionWasRightOuter = d.headMotionOutput.target[0] == -headPosition[0];
  /// True if the target yaw matches none of the 4 valid look around yaw.
  const auto lastHeadActionActionDiffersFromSearchStates =
      std::abs(d.headMotionOutput.target[0]) != headPosition[0] &&
      std::abs(d.headMotionOutput.target[0]) != middleHeadYaw;

  /// The head is at the middleHeadYaw orientation. The next action is outer left. headMotion !=
  /// ANGLES sets the initial head yaw.
  if (d.motionState.headMotion != MotionRequest::HeadMotion::ANGLES ||
      (lastHeadActionPositive && currentTargetIsMiddleOffset && lastTargetWasMiddleOffset &&
       d.headMotionOutput.atTarget && restedOnTargetPosition))
  {
    return ActionCommand::Head::angles(headPosition[0], headPosition[1], yawVelocity,
                                       useEffectiveYawVelocity);
  }
  /// The head is at the outer left orientation. The next action is -middleHeadYaw.
  else if ((lastHeadActionPositive && !lastTargetWasMinusMiddleOffset &&
            lastHeadActionWasLeftOuter && d.headMotionOutput.atTarget && restedOnTargetPosition))
  {
    return ActionCommand::Head::angles(-middleHeadYaw, headPosition[1], yawVelocity,
                                       useEffectiveYawVelocity);
  }
  /// The head is at the -middleHeadYaw orientation. The next action is outer right.
  else if (lastHeadActionNegative && lastTargetWasMinusMiddleOffset &&
           d.headMotionOutput.atTarget && restedOnTargetPosition &&
           currentTargetIsMinusMiddleOffset)
  {
    return ActionCommand::Head::angles(-headPosition[0], headPosition[1], yawVelocity,
                                       useEffectiveYawVelocity);
  }
  /// The head is at the outer right orientation. The next action is middleHeadYaw.
  else if (lastHeadActionNegative && !lastTargetWasMiddleOffset && d.headMotionOutput.atTarget &&
           restedOnTargetPosition && lastHeadActionWasRightOuter)
  {
    return ActionCommand::Head::angles(middleHeadYaw, headPosition[1], yawVelocity,
                                       useEffectiveYawVelocity);
  }
  /// This should only happen if we entered lookAround after another head motion command.
  else if (lastHeadActionActionDiffersFromSearchStates)
  {
    return ActionCommand::Head::angles(headPosition[0], headPosition[1], yawVelocity,
                                       useEffectiveYawVelocity);
  }
  /// If none of the 4 valid look around yaws is reached the last action command is repeated.
  return ActionCommand::Head::angles(d.lastActionCommand.head().yaw(), headPosition[1], yawVelocity,
                                     useEffectiveYawVelocity);
}
