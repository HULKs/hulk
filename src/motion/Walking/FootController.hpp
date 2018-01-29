#pragma once

#include <Data/IMUSensorData.hpp>
#include <Data/MotionRequest.hpp>
#include <Framework/Module.hpp>
#include <Tools/Math/Pose.hpp>

struct FootPose3D
{
  Vector3f position{0, 0, 0};
  float orientation;
};

struct Step2D
{
  Vector2f position{0, 0};
  float orientation;
};

class FootController
{
public:
  FootController(const ModuleBase& module, const IMUSensorData& imuSensorData);
  void getStep(const float progress, FootPose3D& currentFootPose, const Step2D& targetFootPose, const Step2D& lastFootPose, InWalkKickType kickType,
               float& maxImuError, float& maxLastImuError, float& dynamicStepAccumulator);

private:
  Vector3f getFootForcingTerm(const float progress, InWalkKickType kickType);
  float calculateCurrentStepHeight(const Step2D& lastFootPose, const Step2D& targetFootPose) const;
  const Parameter<bool> setDynamicSteps_;
  const Parameter<float> lowPassAlphaDynamicSteps_;
  const Parameter<float> stepHeight_;
  const Parameter<float> sideStepHeight_;
  const IMUSensorData& imuSensorData_;
};
