#pragma once

#include "Data/CycleInfo.hpp"
#include "Data/JointSensorData.hpp"
#include "Data/KeeperOutput.hpp"
#include "Data/MotionActivation.hpp"
#include "Data/MotionRequest.hpp"
#include "Framework/Module.hpp"

#include "Utils/MotionFile/MotionFilePlayer.hpp"

class Motion;

/**
 * @brief The Keeper class controls the goalKeeper motion of the robot.
 * @author Olaf Lueders
 */
class Keeper : public Module<Keeper, Motion>
{
public:
  /// the name of this module
  ModuleName name = "Keeper";
  /**
   * @brief Keeper initializes members
   * @param manager a reference to motion
   */
  Keeper(const ModuleManagerInterface& manager);
  /**
   * @brief cycle checks for commands and may execute a keeper motion if requested
   */
  void cycle();

private:
  /// a reference to the motion activation
  const Dependency<MotionActivation> motionActivation_;
  /// a reference to the motion request
  const Dependency<MotionRequest> motionRequest_;
  /// a reference to the cycle info
  const Dependency<CycleInfo> cycleInfo_;
  /// a reference to the joint sensor data
  const Dependency<JointSensorData> jointSensorData_;
  /// a reference to the keeper output
  Production<KeeperOutput> keeperOutput_;
  /// motion file for genuflect catch front motion
  MotionFilePlayer genuflectCatchFront_;
  /// motion file for left stationary catch
  MotionFilePlayer stationaryCatchLeft_;
  /// motion file for right stationary catch
  MotionFilePlayer stationaryCatchRight_;
  /// motion file for left jumping catch
  MotionFilePlayer jumpingCatchLeft_;
  /// motion file for right jumping catch
  MotionFilePlayer jumpingCatchRight_;
  /// motion file for stand up after genuflect catch front motion
  MotionFilePlayer standUpFromGenuflect_;
  /// whether the keeper was active in the last cycle
  bool wasActive_;
  /// the last motion that was requested
  MotionKeeper previousMotion_;
  /// the last values
  MotionFilePlayer::JointValues previousValues_;
};
