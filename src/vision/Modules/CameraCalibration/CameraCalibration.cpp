#include "Modules/Configuration/Configuration.h"
#include "Tools/Chronometer.hpp"
#include "Tools/Storage/Image.hpp"
#include "print.hpp"

#include "CameraCalibration.hpp"


CameraCalibration::CameraCalibration(const ModuleManagerInterface& manager)
  : Module(manager, "CameraCalibration")
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

  // torso2ground and head2torso are needed to construct the transformation chain.
  if (!head_matrix_buffer_->buffer.empty())
  {
    const HeadMatrixWithTimestamp& buffer_entry = head_matrix_buffer_->getBestMatch(image_data_->timestamp);
    debug().update(mount_ + ".Torso2Ground.rotM", buffer_entry.torso2ground.rotM);
    debug().update(mount_ + ".Torso2Ground.posV", buffer_entry.torso2ground.posV);
    debug().update(mount_ + ".Head2Torso.rotM", buffer_entry.head2torso.rotM);
    debug().update(mount_ + ".Head2Torso.posV", buffer_entry.head2torso.posV);
  }
  // send cam2ground via debug. ex: mount_Camera2Ground_top.rotM
  debug().update(mount_ + ".Camera2Ground_" + image_data_->identification + ".rotM", camera_matrix_->camera2ground.rotM);
  debug().update(mount_ + ".Camera2Ground_" + image_data_->identification + ".posV", camera_matrix_->camera2ground.posV);
  debug().update(mount_ + ".Camera2Ground_inv_" + image_data_->identification + ".rotM", camera_matrix_->camera2ground_inv.rotM);
  debug().update(mount_ + ".Camera2Ground_inv_" + image_data_->identification + ".posV", camera_matrix_->camera2ground_inv.posV);

  /**
   * If and only if calibration image is requested for penalty area based calibration
   */

  if (!debug().isSubscribed(mount_ + "." + image_data_->identification + "_image"))
  {
    return;
  }

  projectPenaltyAreaOnImages();
}

void CameraCalibration::projectPenaltyAreaOnImages()
{
  Vector2f penalty_top_left, penalty_top_right, penalty_bottom_left, penalty_bottom_right, corner_left, corner_right;
  // Retrieve the field dimensions in meters
  float field_length = field_dimensions_->fieldLength;
  float field_width = field_dimensions_->fieldWidth;
  float penalty_length = field_dimensions_->fieldPenaltyAreaLength;
  float penalty_width = field_dimensions_->fieldPenaltyAreaWidth;

  // Calculate positions of the penalty area corner points first
  // Top left penalty area point
  penalty_top_left.x() = field_length / 2;
  penalty_top_left.y() = penalty_width / 2;
  // Top right penalty area point
  penalty_top_right.x() = penalty_top_left.x();
  penalty_top_right.y() = -penalty_top_left.y();
  // bottom left penalty area point
  penalty_bottom_left.x() = penalty_top_left.x() - penalty_length;
  penalty_bottom_left.y() = penalty_top_left.y();
  // bottom right penalty area point
  penalty_bottom_right.x() = penalty_bottom_left.x();
  penalty_bottom_right.y() = penalty_top_right.y();
  // Calculate positions of the field corners
  // Top left field corner
  corner_left.x() = penalty_top_left.x();
  corner_left.y() = field_width / 2;
  // Top right field corner
  corner_right.x() = penalty_top_left.x();
  corner_right.y() = -corner_left.y();

  // Get the pixel positions of the points on the 2D camera image
  Vector2i ptl, ptr, pbl, pbr, cl, cr;
  // Check if all projection points lie outside of the image frame.
  if (!camera_matrix_->robotToPixel(penalty_top_left, ptl) || !camera_matrix_->robotToPixel(penalty_top_right, ptr) ||
      !camera_matrix_->robotToPixel(penalty_bottom_left, pbl) || !camera_matrix_->robotToPixel(penalty_bottom_right, pbr) ||
      !camera_matrix_->robotToPixel(corner_left, cl) || !camera_matrix_->robotToPixel(corner_right, cr))
  {
    Log(LogLevel::WARNING) << "The penalty area projection is outside of the observable image!";
    // Send the unmodified camera image when the projection points are outside of the image.
    debug().sendImage(mount_ + "." + image_data_->identification + "_image", image_data_->image);
    return;
  }

  Image calibImage(image_data_->image);
  // Draw lines for the penalty area on the camera image.
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

  debug().sendImage(mount_ + "." + image_data_->identification + "_image", calibImage);
}
