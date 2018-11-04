#pragma once

#include <functional>

#include "Data/MotionOutput.hpp"
#include "Framework/DataType.hpp"
#include "Tools/Kinematics/KinematicMatrix.h"
#include "Tools/Math/Angle.hpp"
#include "Tools/Math/Pose.hpp"

class WalkGenerator : public DataType<WalkGenerator, MotionOutput>
{
public:
  enum class WalkMode
  {
    VELOCITY_MODE,
    STEP_SIZE_MODE,
    TARGET_MODE
  };

  enum class ArmState
  {
    NORMAL,
    MOVING_BACK,
    BACK,
    MOVING_FRONT
  };

  /// the name of this DataType
  DataTypeName name = "WalkGenerator";

  /// the reset function that is to be called before starting to walk (resetting feedback
  /// accumulators and step times etc.)
  std::function<void()> resetGenerator;
  /**
   * The main function to calculate the joints that should be used for walking
   * Calculates a new set of joint angles to let the robot walk or stand. Must be called every 10
   * ms.
   * @param speed The speed or step size to walk with. If everything is zero, the robot stands.
   * @param target The target to walk to if in target mode.
   * @param walkPathGradient the direction and requested speed in all directions
   * @param walkMode How are speed and target interpreted?
   * @param getKickFootOffset If set, provides an offset to add to the pose of the swing foot to
   *                          create a kick motion. It must be suited for the foot that actually is
   *                          the swing foot.
   */
  std::function<void(const Pose& speed, const Pose& target, const Pose& walkPathGradient,
                     const WalkMode walkMode,
                     const std::function<KinematicMatrix(const float phase)> getKickFootOffset)>
      calcJoints;

  /// the estimated duration of the current steps in seconds
  float stepDuration = 0.f;
  /// the time within this step with respect to the step start
  float t;
  /// true if the left foot is free (right is support)
  bool isLeftPhase = false;
  /// the pose offset of the torso with respect to the last cycle
  Pose odometryOffset;
  /// the speed at wich we are currently walking
  Pose speed;
  /// the max speed at which we can walk (due to configuration)
  Pose maxSpeed = Pose(0.1f, 0.1f, 45 * TO_RAD);
  /// the state of the arms (whether currently back or not)
  ArmState armState = ArmState::NORMAL;
  // this output implicitly has a angles and stiffnisses since it is a MotionOutput
};
