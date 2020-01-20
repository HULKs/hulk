#pragma once

#include "Framework/Module.hpp"
#include "Modules/NaoProvider.h"
#include "Tools/Math/Pose.hpp"

#include "print.h"

struct BehaviorParameters
{
  BehaviorParameters(const ModuleBase& module)
    : isCameraCalibration(module, "isCameraCalibration", [] {})
    , calibrationHeadPitch(module, "calibrationHeadPitch",
                           [this] { calibrationHeadPitch() *= TO_RAD; })
    , calibrationHeadYaw(module, "calibrationHeadYaw", [this] { calibrationHeadYaw() *= TO_RAD; })
    , lookAroundInnerYaw(module, "lookAroundInnerYaw", [this] { lookAroundInnerYaw() *= TO_RAD; })
    , lookAroundOuterPosition(module, "lookAroundOuterPosition",
                              [this] { lookAroundOuterPosition() *= TO_RAD; })
    , lookAroundYawVelocity(module, "lookAroundYawVelocity",
                            [this] { lookAroundYawVelocity() *= TO_RAD; })
    , lookAroundBallYawVelocity(module, "lookAroundBallYawVelocity",
                                [this] { lookAroundBallYawVelocity() *= TO_RAD; })
    , debugTargetEnable(module, "debugTargetEnable", [] {})
    , debugTargetRelativePose(module, "debugTargetRelativePose",
                              [this] { debugTargetRelativePose().orientation *= TO_RAD; })
  {
    calibrationHeadYaw() *= TO_RAD;
    calibrationHeadPitch() *= TO_RAD;
    lookAroundInnerYaw() *= TO_RAD;
    lookAroundOuterPosition() *= TO_RAD;
    lookAroundYawVelocity() *= TO_RAD;
    lookAroundBallYawVelocity() *= TO_RAD;
    debugTargetRelativePose().orientation *= TO_RAD;

    if (debugTargetEnable())
    {
      print("DebugTarget IS ENABLED. This should be off for normal usage.", LogLevel::WARNING);
    }
  }
  /// is calibration running
  const Parameter<bool> isCameraCalibration;
  /// calibration head pitch
  Parameter<float> calibrationHeadPitch;
  /// calibration head yaw
  Parameter<float> calibrationHeadYaw;
  /// yaw position used in between balltrackerHeadPosition yaw
  Parameter<float> lookAroundInnerYaw;
  /// balltracker head position (yaw, pitch)
  Parameter<Vector2f> lookAroundOuterPosition;
  /// look around yaw velocity
  Parameter<float> lookAroundYawVelocity;
  /// look around ball yaw velocity
  Parameter<float> lookAroundBallYawVelocity;
  /// Use debug target pose
  Parameter<bool> debugTargetEnable;
  /// Debug fixed relative target pose
  Parameter<Pose> debugTargetRelativePose;
};
