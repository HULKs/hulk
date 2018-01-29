#pragma once

#include "Data/CameraMatrix.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/HeadMatrixBuffer.hpp"
#include "Data/ImageData.hpp"
#include "Framework/Module.hpp"


class Brain;

/**
 * @brief The CameraCalibration class
 *
 * @author Erik Schröder
 */
class CameraCalibration : public Module<CameraCalibration, Brain>
{
public:
  /**
   * CameraCalibration constructor
   * @param manager a reference to the brain object
   *
   * @author Erik Schröder
   */
  CameraCalibration(const ModuleManagerInterface& manager);
  /**
   * @brief cycle draws an image of some defined points to see how to adjust the camera calibration parameters
   */
  void cycle();

private:
  /**
   * @brief draws the penalty area to an image
   *
   * The NAO has to be placed at the center point of the field, facing one of the two goals.
   * The feet should be perfectly parallel and and the middle point of the field should be exactly under his torso.
   *
   * @author Thomas Schattschneider
   */
  void projectPenaltyAreaOnImages();
  /// a reference to the image of the cycle
  const Dependency<ImageData> image_data_;
  /// a reference to the camera matrix
  const Dependency<CameraMatrix> camera_matrix_;
  /// a reference to the field dimensions
  const Dependency<FieldDimensions> field_dimensions_;
  /// a reference to the head matrix buffer
  const Dependency<HeadMatrixBuffer> head_matrix_buffer_;
};
