#pragma once

#include <string>

#include <Data/CycleInfo.hpp>
#include <Data/JointSensorData.hpp>
#include <Data/MotionActivation.hpp>
#include <Data/MotionRequest.hpp>
#include <Data/SitUpOutput.hpp>
#include <Framework/Module.hpp>

#include "Utils/MotionFile/MotionFilePlayer.hpp"

class Motion;

class SitUp : public Module<SitUp, Motion>
{
public:
  ModuleName name = "SitUp";
  /**
   * @brief SitUp initializes members and loads motion files
   * @param manager a reference to motion
   */
  SitUp(const ModuleManagerInterface& manager);
  /**
   * @brief cycle checks for a new command and initiates a sit up motion if needed
   */
  void cycle();

private:
  /**
   * @brief Status is an enum to specify the status of the SitUp module
   */
  enum class Status
  {
    IDLE,
    SITTING_UP,
    DONE
  };
  /// name of motion file containing the needed motion for sitting up
  const Parameter<std::string> sitUpMotionFile_;
  /// a reference to the motion request
  const Dependency<MotionRequest> motionRequest_;
  /// a reference to the motion activation
  const Dependency<MotionActivation> motionActivation_;
  /// a reference to the cycle info
  const Dependency<CycleInfo> cycleInfo_;
  /// a reference to the joint sensor data
  const Dependency<JointSensorData> jointSensorData_;
  /// a reference to the sit up output
  Production<SitUpOutput> sitUpOutput_;
  /// state of the SitUp-module
  Status status_;
  /// motion-object for whole sit up motion
  MotionFilePlayer sitUpMotion_;
};
