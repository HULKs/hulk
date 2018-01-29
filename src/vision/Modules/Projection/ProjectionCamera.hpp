#pragma once

#include <mutex>

#include "Framework/Module.hpp"
#include "Hardware/CameraInterface.hpp"
#include "Tools/Kinematics/KinematicMatrix.h"
#include "Tools/Math/Eigen.hpp"

class ProjectionCamera
{
public:
  /**
   * @brief ProjectionCamera loads configuration values
   * @param module a reference to the Projection module
   * @param camera the camera which is managed by this object
   */
  ProjectionCamera(const ModuleBase& module, const Camera camera);
  /**
   * @brief updateCamera2Head recalculates the calibrated camera2head matrix
   */
  void updateCamera2Head();
  /// angles around x, y, z axes respectively for extrinsic camera calibration
  const Parameter<Vector3f> ext;
  /// the focal length with compensation for pixel size
  const Parameter<Vector2f> fc;
  /// the optical center in pixel coordinates
  const Parameter<Vector2f> cc;
  /// a transformation matrix that describes the camera to head pitch without calibration
  KinematicMatrix camera2head_uncalib;
  /// a transformation matrix that describes the camera to head pitch - updated on calibration change
  KinematicMatrix camera2head;
  /// mutex for camera2head
  std::mutex camera2head_lock;
};
