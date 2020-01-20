#pragma once

#include <string>

#include <Data/CycleInfo.hpp>
#include <Data/JointSensorData.hpp>
#include <Data/MotionActivation.hpp>
#include <Data/MotionRequest.hpp>
#include <Data/SitDownOutput.hpp>
#include <Framework/Module.hpp>

#include "Utils/MotionFile/MotionFilePlayer.hpp"

class Motion;

class SitDown : public Module<SitDown, Motion>
{
public:
  ModuleName name = "SitDown";
  /**
   * @brief SitDown initializes members and loads motion files
   * @param manager a reference to motion
   */
  SitDown(const ModuleManagerInterface& manager);
  /**
   * @brief cycle checks for a new command and initiates a sit down motion if needed
   */
  void cycle();

private:
  /**
   * @brief Status is an enum to specify the status of the SitDown module
   */
  enum class Status
  {
    IDLE,
    SITTING_DOWN,
    DONE
  };
  /// name of motion file containing the needed motion for sitting down
  const Parameter<std::string> sitDownMotionFile_;
  /// a reference to the motion request
  const Dependency<MotionRequest> motionRequest_;
  /// a reference to the motion activation
  const Dependency<MotionActivation> motionActivation_;
  /// a reference to the cycle info
  const Dependency<CycleInfo> cycleInfo_;
  /// a reference to the joint sensor data
  const Dependency<JointSensorData> jointSensorData_;
  /// a reference to the sit down output
  Production<SitDownOutput> sitDownOutput_;
  /// state of the SitDown-module
  Status status_;
  /// motion-object for whole sit down motion
  MotionFilePlayer sitDownMotion_;
};
