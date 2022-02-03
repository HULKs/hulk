#pragma once

#include "Data/ActionCommand.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/JointSensorData.hpp"
#include "Data/PointOutput.hpp"
#include "Data/RobotKinematics.hpp"
#include "Framework/Module.hpp"
#include "Motion/Utils/Interpolator/Interpolator.hpp"

class Motion;

/**
 * @brief Point A module that provides the joint angles to point somewhere
 *
 * This module is used if one wants to point to a specific location on the field.
 * It was originally used for the 'no WIFI challenge' back in 2016.
 */
class Point : public Module<Point, Motion>
{
public:
  /// the name of this module
  ModuleName name__{"Point"};
  /**
   * @brief Point initializes members
   * @param manager a reference to motion
   */
  explicit Point(const ModuleManagerInterface& manager);
  /**
   * @brief cycle checks for a point command and points there
   */
  void cycle() override;

private:
  using MotionType = ActionCommand::Arm::MotionType;

  const Dependency<CycleInfo> cycleInfo_;
  /// a reference to the motion request
  const Dependency<ActionCommand> actionCommand_;
  /// a reference to the joint sensor data
  const Dependency<JointSensorData> jointSensorData_;
  /// a reference to the robot kinematics
  const Dependency<RobotKinematics> robotKinematics_;
  /// a reference to the point output
  Production<PointOutput> pointOutput_;
  /// an interpolator for the left arm
  Interpolator<Clock::duration, static_cast<std::size_t>(JointsArm::MAX)> leftInterpolator_;
  /// an interpolator for the right arm
  Interpolator<Clock::duration, static_cast<std::size_t>(JointsArm::MAX)> rightInterpolator_;
  /// the last arm motion type that was executed (left arm)
  MotionType lastLeftArmMotion_ = MotionType::BODY;
  /// the last arm motion type that was executed (right arm)
  MotionType lastRightArmMotion_ = MotionType::BODY;
};
