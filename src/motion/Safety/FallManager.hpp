#pragma once

#include "Modules/NaoProvider.h"

#include "Data/BodyPose.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/FallManagerOutput.hpp"
#include "Data/JointSensorData.hpp"
#include "Data/MotionActivation.hpp"
#include "Data/MotionRequest.hpp"
#include "Framework/Module.hpp"

#include "Utils/Interpolator/Interpolator.hpp"
#include "Utils/MotionFile/MotionFilePlayer.hpp"


class Motion;

class FallManager : public Module<FallManager, Motion>
{
public:
  /// the name of this module
  ModuleName name = "FallManager";
  /**
   * @brief FallManager initializes members and loads motion files
   * @param manager a reference to motion
   */
  FallManager(const ModuleManagerInterface& manager);
  /**
   * @brief cycle checks if the robot is falling and initializes a motion to prevent it
   */
  void cycle();

private:
  /**
   * @brief prepareFalling is executed when falling is detected
   * @param fallDirection the falling direction tendency
   */
  void prepareFalling(const FallDirection fallDirection);
  /// the name of the motion file for kneeing
  const Parameter<std::string> kneeDownMotionFile_;
  /// whether the FallManager is enabled to do something
  const Parameter<bool> enabled_;
  /// a reference to the motion activation
  const Dependency<MotionActivation> motionActivation_;
  /// a reference to the motion request
  const Dependency<MotionRequest> motionRequest_;
  /// a reference to the body pose
  const Dependency<BodyPose> bodyPose_;
  /// a reference to the cycle info
  const Dependency<CycleInfo> cycleInfo_;
  /// a reference to the joint sensor data
  const Dependency<JointSensorData> jointSensorData_;
  /// a reference to the fall manager output
  Production<FallManagerOutput> fallManagerOutput_;
  /// whether the fall manager should initiate a fall preventing motion
  bool hot_;
  /// interpolator for catch front
  Interpolator catchFrontInterpolator_;
  /// the catch front interpolation duration
  const Parameter<unsigned int> catchFrontDuration_;
  /// the catch front hip pitch
  Parameter<float> catchFrontHipPitch_;
  /// motion file for kneeing
  MotionFilePlayer kneeDown_;
  /// time that the fall prevention motion needs (milliseconds)
  int timerClock_;
  /// the last fall manager output
  std::vector<float> lastAngles_;
};
