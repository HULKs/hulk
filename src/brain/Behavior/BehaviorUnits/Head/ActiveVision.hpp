#pragma once
#include "Behavior/Units.hpp"

enum class VisionMode
{
  LookAround,
  LookAroundBall,
  BallTracker,
  Localization,
  BallAndLocalization,
  LookForward,
};

ActionCommand::Head activeVision(const DataSet& d, VisionMode mode)
{
  switch (mode)
  {
    case VisionMode::LookAround:
      return ActionCommand::Head::angles(d.headPositionData.lookAroundHeadPosition,
                                         d.parameters.lookAroundYawVelocity());
      break;
    case VisionMode::LookAroundBall:
      return ActionCommand::Head::angles(d.headPositionData.lookAroundBallHeadPosition, 0.5f);
      break;
    case VisionMode::BallTracker:
      return ActionCommand::Head::angles(d.headPositionData.trackBallHeadPosition, 1.f);
      break;
    case VisionMode::Localization:
      return ActionCommand::Head::angles(d.headPositionData.localizationHeadPosition, 1.f);
      break;
    case VisionMode::BallAndLocalization:
      return ActionCommand::Head::angles(d.headPositionData.ballAndLocalizationHeadPosition, 1.f);
      break;
    case VisionMode::LookForward:
      return ActionCommand::Head::angles(0, d.parameters.lookAroundOuterPosition()[1]);
      break;
    default:
      return ActionCommand::Head::angles(0, d.parameters.lookAroundOuterPosition()[1]);
  }
}
