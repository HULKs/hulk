#pragma once

#include "Framework/Module.hpp"

#include "Data/CameraMatrix.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/FakeImageData.hpp"
#include "Data/ImageData.hpp"
#include "Data/RobotKinematics.hpp"

class Brain;

class FakeImageReceiver : public Module<FakeImageReceiver, Brain>
{
public:
  /// the name of this module
  ModuleName name = "FakeImageReceiver";
  /// the constructor of this module
  FakeImageReceiver(const ModuleManagerInterface& manager);
  /// the desctructor of this module
  ~FakeImageReceiver(){};
  /// the cycle - called by the module manager
  void cycle();

private:
  /// the buffer of the last few head matrices
  const Dependency<RobotKinematics> robotKinematics_;
  /// a reference to the ImageData to check whether it is provided
  const Reference<ImageData> imageData_;
  /// some information about the cycle time
  Production<CycleInfo> cycleInfo_;
  /// a fake image to ensure that the faker chain is waiting for new simrobot data
  Production<FakeImageData> fakeImageData_;
  /// fake camera matrix
  Production<CameraMatrix> fakeCameraMatrix_;
  /// the focal length with compensation for pixel size
  Vector2f topFc_, bottomFc_;
  /// the optical center in pixel coordinates
  Vector2f topCc_, bottomCc_;
  /// image size for bottom and top camera  (needs to be defined because the image class returns 0)
  Vector2i bottomImageSize_;
  Vector2i topImageSize_;
  /// a transformation matrix that describes the camera to head pitch without calibration
  KinematicMatrix topCamera2head_uncalib_, bottomCamera2head_uncalib_;
};
