#include "Modules/Configuration/Configuration.h"
#include "Tools/Chronometer.hpp"
#include "Tools/Storage/Image.hpp"
#include "print.hpp"

#include "CameraCalibration.hpp"


CameraCalibration::CameraCalibration(const ModuleManagerInterface& manager)
  : Module(manager)
  , image_data_(*this)
  , camera_matrix_(*this)
  , field_dimensions_(*this)
  , head_matrix_buffer_(*this)
{
}

void CameraCalibration::cycle()
{
  Chronometer time(debug(), mount_ + ".cycle_time");
  /**
   * Put all calibration code AFTER this if block
   */

  /// torso2ground and head2torso are needed to construct the transformation chain.
  if (!head_matrix_buffer_->buffer.empty())
  {
    const HeadMatrixWithTimestamp& bufferEntry =
        head_matrix_buffer_->getBestMatch(image_data_->timestamp);
    debug().update(mount_ + ".Torso2Ground", bufferEntry.torso2ground);
    debug().update(mount_ + ".Head2Torso", bufferEntry.head2torso);
  }
  // send cam2ground via debug. ex: mount_Camera2Ground
  debug().update(mount_ + ".Camera2Ground_" + image_data_->identification,
                 camera_matrix_->camera2ground);

  const std::string syncImageMount = mount_ + "." + image_data_->identification + "_image";
  if (debug().isSubscribed(syncImageMount))
  {
    /// send raw image. Explicit purpose is to ensure synchronization.
    debug().sendImage(syncImageMount, image_data_->image422.to444Image());
  }

  /**
   * If and only if calibration image is requested for penalty area based calibration
   */
  if (!debug().isSubscribed(mount_ + "." + image_data_->identification + "_penalty_project_image"))
  {
    return;
  }

  projectPenaltyAreaOnImages();
}

void CameraCalibration::projectPenaltyAreaOnImages()
{
  Vector2f penaltyTopLeft, penalty_top_right, penalty_bottom_left, penalty_bottom_right,
      corner_left, corner_right;
  // Retrieve the field dimensions in meters
  float fieldLength = field_dimensions_->fieldLength;
  float fieldWidth = field_dimensions_->fieldWidth;
  float penaltyLength = field_dimensions_->fieldPenaltyAreaLength;
  float penaltyWidth = field_dimensions_->fieldPenaltyAreaWidth;

  // Calculate positions of the penalty area corner points first
  // Top left penalty area point
  penaltyTopLeft.x() = fieldLength / 2;
  penaltyTopLeft.y() = penaltyWidth / 2;
  // Top right penalty area point
  penalty_top_right.x() = penaltyTopLeft.x();
  penalty_top_right.y() = -penaltyTopLeft.y();
  // bottom left penalty area point
  penalty_bottom_left.x() = penaltyTopLeft.x() - penaltyLength;
  penalty_bottom_left.y() = penaltyTopLeft.y();
  // bottom right penalty area point
  penalty_bottom_right.x() = penalty_bottom_left.x();
  penalty_bottom_right.y() = penalty_top_right.y();
  // Calculate positions of the field corners
  // Top left field corner
  corner_left.x() = penaltyTopLeft.x();
  corner_left.y() = fieldWidth / 2;
  // Top right field corner
  corner_right.x() = penaltyTopLeft.x();
  corner_right.y() = -corner_left.y();

  // Get the pixel positions of the points on the 2D camera image
  Vector2i ptl, ptr, pbl, pbr, cl, cr;
  // Check if all projection points lie outside of the image frame.
  if (!camera_matrix_->robotToPixel(penaltyTopLeft, ptl) ||
      !camera_matrix_->robotToPixel(penalty_top_right, ptr) ||
      !camera_matrix_->robotToPixel(penalty_bottom_left, pbl) ||
      !camera_matrix_->robotToPixel(penalty_bottom_right, pbr) ||
      !camera_matrix_->robotToPixel(corner_left, cl) ||
      !camera_matrix_->robotToPixel(corner_right, cr))
  {
    Log(LogLevel::WARNING) << "The penalty area projection is outside of the observable image!";
    // Send the unmodified camera image when the projection points are outside of the image.
    debug().sendImage(mount_ + "." + image_data_->identification + "_penalty_project_image",
                      image_data_->image422.to444Image());
    return;
  }

  Image calibImage(image_data_->image422.to444Image());

  ptl = image_data_->image422.get444From422Vector(ptl);
  ptr = image_data_->image422.get444From422Vector(ptr);
  pbl = image_data_->image422.get444From422Vector(pbl);
  pbr = image_data_->image422.get444From422Vector(pbr);
  cl = image_data_->image422.get444From422Vector(cl);
  cr = image_data_->image422.get444From422Vector(cr);

  // Draw lines for the penalty area on the camera image.
  calibImage.cross((ptl + ptr) / 2, 8, Color::RED); // middle of penalty line.
  calibImage.cross((pbl + pbr) / 2, 8, Color::RED); // middle of penalty_box? line.
  calibImage.cross(ptl, 8, Color::RED);
  calibImage.cross(ptr, 8, Color::RED);
  calibImage.cross(pbl, 8, Color::RED);
  calibImage.cross(pbr, 8, Color::RED);
  calibImage.line(ptl, ptr, Color::PINK);
  calibImage.line(pbl, pbr, Color::PINK);
  calibImage.line(pbl, ptl, Color::PINK);
  calibImage.line(pbr, ptr, Color::PINK);
  // Draw the line between the field corners and mark them with crosses
  calibImage.line(cl, cr, Color::PINK);
  calibImage.cross(cl, 8, Color::RED);
  calibImage.cross(cr, 8, Color::RED);

  debug().sendImage(mount_ + "." + image_data_->identification + "_penalty_project_image",
                    calibImage);
}
