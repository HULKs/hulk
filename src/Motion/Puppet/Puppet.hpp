#pragma once

#include "Data/CycleInfo.hpp"
#include "Data/JointSensorData.hpp"
#include "Data/PuppetMotionOutput.hpp"
#include "Framework/Module.hpp"
#include "Hardware/Definitions.hpp"
#include "Motion/Utils/Interpolator/Interpolator.hpp"

class Motion;

struct JointKeyFrame : public Uni::From, public Uni::To
{
  /**
   * @brief JointKeyFrame default constructor
   */
  JointKeyFrame() = default;

  /**
   * @brief Constructs JointKeyFrame from given joint angles and their interpolation time
   */
  JointKeyFrame(const JointsArray<float>& jointAngles, float interpolationTime)
    : jointAngles(jointAngles)
    , interpolationTime(interpolationTime)
  {
  }

  /// vector of all joint angles in this keyframe
  JointsArray<float> jointAngles{};
  /// time to interpolate until the jointAngles are reached
  Clock::duration interpolationTime{};

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["jointAngles"] << jointAngles;
    value["interpolationTime"] << interpolationTime;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["jointAngles"] >> jointAngles;
    value["interpolationTime"] >> interpolationTime;
  }
};

/**
 * @brief Puppet produces the PuppetMotionOutput to control the robot joints remotely
 */
class Puppet : public Module<Puppet, Motion>
{
public:
  /// the name of this module
  ModuleName name__{"Puppet"};
  /**
   *@brief The constructor of this class
   */
  explicit Puppet(const ModuleManagerInterface& manager);
  /// the cycle method of this module
  void cycle() override;

private:
  /// updates the actualRemotePuppetJointKeyFrame_ with the new frame from config
  void updateKeyFrame();
  /// updates the stiffness with the new stiffness data from config
  void updateStiffness();
  /// key frame that specifies the next joint angles and an interpolation time (set via config)
  const Parameter<JointKeyFrame> remotePuppetJointKeyFrame_;
  /// stiffness vector of joints (set via config)
  const Parameter<JointsArray<float>> remotePuppetStiffnesses_;
  /// a reference to the cycle info
  const Dependency<CycleInfo> cycleInfo_;
  /// a reference to the joint sensor data
  const Dependency<JointSensorData> jointSensorData_;
  /// a reference to the puppet motion output
  Production<PuppetMotionOutput> puppetMotionOutput_;
  /// a thread-safe copy of the remote joint keyframe
  JointKeyFrame actualRemotePuppetJointKeyFrame_;
  /// mutex that locks the actual remote puppet joint keyframe
  std::mutex actualRemotePuppetJointKeyFrameLock_;
  /// indicating a new frame was set
  std::atomic_bool newRemotePuppetKeyFrame_{false};
  /// interpolator used to approach joint angels of actualRemotePuppetJointKeyFrame
  Interpolator<Clock::duration, static_cast<std::size_t>(Joints::MAX)> keyFrameInterpolator_;
  /// vector of the stiffness all joint in this keyframe
  JointsArray<float> stiffnesses_{};
  /// mutex that locks the stiffness vector
  std::mutex stiffnessLock_;
};
