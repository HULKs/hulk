#pragma once

#include "Data/BodyPose.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/FallManagerOutput.hpp"
#include "Data/JointSensorData.hpp"
#include "Data/MotionActivation.hpp"
#include "Framework/Module.hpp"

#include "Utils/MotionFile/MotionFilePlayer.hpp"


class Motion;

class FallManager : public Module<FallManager, Motion>
{
public:
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
  /**
   * @brief enableProtection activates the fall prevention
   */
  void enableProtection();
  /**
   * @brief disableProtection deactivates the fall prevention
   */
  void disableProtection();
  /// the name of the motion file for front caching
  const Parameter<std::string> catchFrontMotionFile_;
  /// the name of the motion file for kneeing
  const Parameter<std::string> kneeDownMotionFile_;
  /// whether the FallManager is enabled to do something
  const Parameter<bool> enabled_;
  /// a reference to the motion activation
  const Dependency<MotionActivation> motionActivation_;
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
  /// motion file for catching
  MotionFilePlayer catchFront_;
  /// motion file for kneeing
  MotionFilePlayer kneeDown_;
  /// time that the fall prevention motion needs (milliseconds)
  int timerClock_;
};
