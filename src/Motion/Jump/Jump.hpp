#pragma once

#include "Data/ActionCommand.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/JointSensorData.hpp"
#include "Data/JumpOutput.hpp"
#include "Data/MotionActivation.hpp"
#include "Data/Poses.hpp"
#include "Framework/Module.hpp"
#include "Motion/Utils/MotionFile/MotionFilePlayer.hpp"

class Motion;

/**
 * @brief The Jump class controls the jump motion of the robot.
 */
class Jump : public Module<Jump, Motion>
{
public:
  /// the name of this module
  ModuleName name__{"Jump"};
  /**
   * @brief Jump initializes members
   * @param manager a reference to motion
   */
  explicit Jump(const ModuleManagerInterface& manager);
  /**
   * @brief cycle checks for commands and may execute a jump motion if requested
   */
  void cycle() override;

private:
  const Dependency<ActionCommand> actionCommand_;
  const Dependency<CycleInfo> cycleInfo_;
  const Dependency<JointSensorData> jointSensorData_;
  /// a reference to the motion activation of last cycle
  const Reference<MotionActivation> motionActivation_;
  const Dependency<Poses> poses_;

  Production<JumpOutput> jumpOutput_;

  /// motion file for squat catch front motion
  MotionFilePlayer squatCatchFront_;
  /// motion file for left stationary catch
  MotionFilePlayer stationaryCatchLeft_;
  /// motion file for right stationary catch
  MotionFilePlayer stationaryCatchRight_;
  /// motion file for left jumping catch
  MotionFilePlayer jumpingCatchLeft_;
  /// motion file for right jumping catch
  MotionFilePlayer jumpingCatchRight_;
  /// motion file for stand up after squat catch front motion
  MotionFilePlayer standUpFromGenuflect_;
  /// whether the jump is currently active
  bool isActive_ = false;
  /// the last motion that was requested
  JumpOutput::Type previousMotion_ = JumpOutput::Type::NONE;
  /// the last values
  MotionFilePlayer::JointValues previousValues_;
};
