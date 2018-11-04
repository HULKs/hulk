#pragma once

#include "Data/BallData.hpp"
#include "Data/CameraMatrix.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/FakeImageData.hpp"
#include "Framework/Module.hpp"

class Brain;

class FakeBallProvider : public Module<FakeBallProvider, Brain>
{
public:
  /// the name of this module
  ModuleName name = "FakeBallProvider";
  /// the constructor of this module
  FakeBallProvider(const ModuleManagerInterface& manager);
  /// the desctructor of this module
  virtual ~FakeBallProvider(){};
  /**
   * @brief cycle writes the fake data from the robot interface
   *        to the BallData production
   */
  void cycle();

private:
  /// The overlap of the bottom and top camera
  const Parameter<int> overlap_;
  /// noise can be added to the ball position
  const Parameter<bool> enableNoise_;
  /// The sigma for adding white gaussian noise to the ball position
  const Parameter<Vector2f> pixelNoiseSigma_;
  /// the projection of the ball must be inside the image plane
  const Parameter<bool> enableFieldOfView_;
  /// the ball must be inside the maxDetectionDistance
  const Parameter<bool> limitSight_;
  /// The maximum distance in which the nao is able to detect a ball
  const Parameter<float> maxDetectionDistance_;
  /// some measurements are assumed as lost. Depends on distance to ball
  const Parameter<bool> enableDetectionRate_;
  /// a dependency to ensure that there is fake data availabe before this module runs
  const Dependency<FakeImageData> fakeImageData_;
  /// the cycle info
  const Dependency<CycleInfo> cycleInfo_;
  /// camera matrix
  const Dependency<CameraMatrix> cameraMatrix_;
  /// the faked ball position
  Production<BallData> fakeBallState_;
  /// The function for adding white gaussian noise to the ball position
  Vector2i addGaussianNoise(const Vector2i& pixelPosition, const Vector2f sigma) const;
};
