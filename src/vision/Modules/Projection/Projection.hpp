#pragma once

#include "Data/CameraMatrix.hpp"
#include "Data/HeadMatrixBuffer.hpp"
#include "Data/ImageData.hpp"
#include "Framework/Module.hpp"
#include "Tools/Math/Eigen.hpp"

#include "ProjectionCamera.hpp"


class Brain;

class Projection : public Module<Projection, Brain>
{
public:
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
  /// contains an angle around the x axis and an angle around the y axis for calibration of the torso matrix
  const Parameter<Vector2f> torso_calibration_;
  /// the current camera image
  const Dependency<ImageData> image_data_;
  /// the buffer of the last few head matrices
  const Dependency<HeadMatrixBuffer> head_matrix_buffer_;
  /// the result of the projection
  Production<CameraMatrix> camera_matrix_;
  /// the parameters and states of the top camera
  ProjectionCamera top_camera_;
  /// the parameters and states of the bottom camera
  ProjectionCamera bottom_camera_;
  /// a matrix that represents the transformations of the torso calibration
  KinematicMatrix torso_calibration_matrix_;
};
