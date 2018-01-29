#pragma once

#include "Data/BodyPose.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/RobotPosition.hpp"
#include "Data/StrikerAction.hpp"
#include "Data/TeamBallModel.hpp"
#include "Data/TimeToReachBall.hpp"
#include "Framework/Module.hpp"


class Brain;

class TimeToReachBallProvider : public Module<TimeToReachBallProvider, Brain>
{
public:
  TimeToReachBallProvider(const ModuleManagerInterface& manager);
  void cycle();

private:
  const Parameter<float> translationVelocity_;
  Parameter<float> rotationVelocity_;
  Parameter<float> walkAroundBallVelocity_;
  const Parameter<float> fallenPenalty_;
  const Parameter<float> strikerBonus_;
  const Parameter<float> ballNotSeenPenalty_;
  const Dependency<BodyPose> bodyPose_;
  const Dependency<GameControllerState> gameControllerState_;
  const Dependency<RobotPosition> robotPosition_;
  const Dependency<StrikerAction> strikerAction_;
  const Dependency<TeamBallModel> teamBallModel_;
  Production<TimeToReachBall> timeToReachBall_;
};
