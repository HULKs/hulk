#pragma once

#include "Data/CameraMatrix.hpp"
#include "Data/HeadMatrixBuffer.hpp"
#include "Data/ImageData.hpp"
#include "Framework/Module.hpp"
#include "Tools/Math/Eigen.hpp"

#include "Vision/Projection/ProjectionCamera.hpp"


class Brain;

class Projection : public Module<Projection, Brain>
{
public:
  /// the name of this module
  ModuleName name__{"Projection"};
  /**
   * @brief Projection loads configuration values and initializes members
   * @param manager a reference to the module manager
   * @author Arne Hasselbring
   */
  Projection(const ModuleManagerInterface& manager);
  /**
   * @brief cycle calculates the camera matrix for the current frame and saves it
   * @author Arne Hasselbring
   */
  void cycle();

private:
  /**
   * @brief updateTorsoCalibrationMatrix recalculates the torso calibration matrix
   */
  void updateTorsoCalibrationMatrix();
  /// contains an angle around the x axis and an angle around the y axis for calibration of the
  /// torso matrix
  const Parameter<Vector2f> torsoCalibration_;
  /// fix cam2ground for both cameras for stand pose
  const Parameter<std::array<KinematicMatrix, 2>> cam2groundStand_;
  /// the field of view of the nao: x: horizontal, y: vertical
  const Parameter<Vector2f> fov_;
  /// the current camera image
  const Dependency<ImageData> imageData_;
  /// the buffer of the last few head matrices
  const Dependency<HeadMatrixBuffer> headMatrixBuffer_;
  /// the result of the projection
  Production<CameraMatrix> cameraMatrix_;
  /// the parameters and states of the top camera
  ProjectionCamera topCamera_;
  /// the parameters and states of the bottom camera
  ProjectionCamera bottomCamera_;
  /// a matrix that represents the transformations of the torso calibration
  KinematicMatrix torsoCalibrationMatrix_;
};
