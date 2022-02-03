#pragma once

#include "Data/BodyPose.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/RobotPosition.hpp"
#include "Data/SetPlayStrikerAction.hpp"
#include "Data/StrikerAction.hpp"
#include "Data/TeamBallModel.hpp"
#include "Data/TimeToReachBall.hpp"
#include "Data/WalkGeneratorOutput.hpp"
#include "Framework/Module.hpp"


class Brain;

class TimeToReachBallProvider : public Module<TimeToReachBallProvider, Brain>
{
public:
  /// the name of this module
  ModuleName name__{"TimeToReachBallProvider"};
  explicit TimeToReachBallProvider(const ModuleManagerInterface& manager);
  void cycle() override;

private:
  /// the time added as a penalty if the robot is fallen
  const Parameter<Clock::duration> fallenPenalty_;
  /// the time substract if we are striker (hysteresis, for robust decision)
  const Parameter<Clock::duration> strikerBonus_;
  /// the time added if we didn't see the ball ourselfs
  const Parameter<Clock::duration> ballNotSeenPenalty_;
  /// the factor that will be multiplied with the time to walk around ball
  const Parameter<float> walkAroundBallVelocityFactor_;

  const Dependency<BodyPose> bodyPose_;
  const Dependency<SetPlayStrikerAction> setPlayStrikerAction_;
  const Dependency<GameControllerState> gameControllerState_;
  const Dependency<RobotPosition> robotPosition_;
  const Dependency<StrikerAction> strikerAction_;
  const Dependency<TeamBallModel> teamBallModel_;
  const Dependency<WalkGeneratorOutput> walkGeneratorOutput_;

  /*
   * @brief generates estimateTimeToReachBall for the timeToReachBall_ production.
   */
  void generateEstimateTimeToReachBallFunction();

  Production<TimeToReachBall> timeToReachBall_;
};
