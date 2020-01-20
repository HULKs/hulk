#pragma once
#include "Tools/Math/Eigen.hpp"

#include "Data/BallState.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/HeadMotionOutput.hpp"
#include "Data/HeadPositionData.hpp"
#include "Data/RobotPosition.hpp"
#include "Data/TeamBallModel.hpp"
#include "Framework/Module.hpp"
#include "Knowledge/Position/FieldInfo.hpp"

class Brain;

/**
 * @brief HeadPositionProvider
 */
class HeadPositionProvider : public Module<HeadPositionProvider, Brain>
{
public:
  /// the name of this module
  ModuleName name = "HeadPositionProvider";
  HeadPositionProvider(const ModuleManagerInterface& manager);
  void cycle() override;

private:
  enum LookAroundState
  {
    INITIAL,
    GOING_RIGHT,
    GOING_LEFT,
    GOING_MIDDLE,
  };

  const Dependency<BallState> ballState_;
  const Dependency<GameControllerState> gameControllerState_;
  const Dependency<TeamBallModel> teamBallModel_;
  const Dependency<RobotPosition> robotPosition_;
  const Dependency<HeadMotionOutput> headMotionOutput_;
  const Dependency<CycleInfo> cycleInfo_;

  Production<HeadPositionData> headPositionData_;

  /// Resting time for the look around state machine
  const Parameter<float> timeToRest_;
  /// head yaw max
  ConditionalParameter<float> yawMax_;
  /// max yaw angle to keep a target on the image
  Parameter<float> keepTargetOnImageMaxAngle_;
  // Tolerance to effectivly reach a requested position
  const Parameter<float> targetPositionTolerance_;
  /// pitch for lookaround
  Parameter<float> lookAroundPitch_;

  /// States for the look around state machine
  LookAroundState lastLookAroundState_{LookAroundState::INITIAL};
  LookAroundState nextLookAroundState_{LookAroundState::INITIAL};

  /// Resting positions for the look around state machine
  HeadPosition outerPositionLeft_;
  HeadPosition outerPositionRight_;
  HeadPosition innerPosition_;

  /**
   * Method to calculate head positions to look around the ball
   */
  HeadPosition calculateLookAroundBallHeadPositions();
  /**
   * Calculates the head position to look around
   * @param angle the angle the robot looks around from
   */
  HeadPosition calculateLookAroundHeadPositions(float yawMax, float angle);
};
