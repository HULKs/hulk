#include "Framework/Log/Log.hpp"
#include "Tools/Chronometer.hpp"
#include "Tools/Storage/Image.hpp"

#include "Vision/CameraCalibration/CameraCalibration.hpp"


CameraCalibration::CameraCalibration(const ModuleManagerInterface& manager)
  : Module(manager)
  , rotate90Degrees_{*this, "rotate90Degrees", [] {}}
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
        head_matrix_buffer_->getBestMatch(image_data_->captureTimePoint
// Except when in SimRobot because camera images are captured at one exact time point there.
#ifndef HULK_TARGET_SimRobot
                                          + std::chrono::milliseconds(17)
#endif
        );
    Uni::Value matrixAndImageInfos;
    matrixAndImageInfos["torso2Head"] = bufferEntry.head2torso.inverted();
    matrixAndImageInfos["ground2Torso"] = bufferEntry.torso2ground.inverted();
    matrixAndImageInfos["imageInfos"] = Uni::Value();
    matrixAndImageInfos["imageInfos"]["ground2Camera"] << camera_matrix_->camera2ground.inverted();
    matrixAndImageInfos["imageInfos"]["identification"] << image_data_->identification;

    debug().update(mount_ + ".MatrixAndImageInfos", matrixAndImageInfos);
  }
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

  if (rotate90Degrees_())
  {
    std::swap(penaltyTopLeft.x(), penaltyTopLeft.y());
    std::swap(penalty_top_right.x(), penalty_top_right.y());
    std::swap(penalty_bottom_left.x(), penalty_bottom_left.y());
    std::swap(penalty_bottom_right.x(), penalty_bottom_right.y());
    std::swap(corner_left.x(), corner_left.y());
    std::swap(corner_right.x(), corner_right.y());
    penaltyTopLeft.x() *= -1.f;
    penalty_top_right.x() *= -1.f;
    penalty_bottom_left.x() *= -1.f;
    penalty_bottom_right.x() *= -1.f;
    corner_left.x() *= -1.f;
    corner_right.x() *= -1.f;
  }

  // Get the pixel positions of the points on the 2D camera image
  const std::optional<Vector2i> pixelPtl = camera_matrix_->robotToPixel(penaltyTopLeft);
  const std::optional<Vector2i> pixelPtr = camera_matrix_->robotToPixel(penalty_top_right);
  const std::optional<Vector2i> pixelPbl = camera_matrix_->robotToPixel(penalty_bottom_left);
  const std::optional<Vector2i> pixelPbr = camera_matrix_->robotToPixel(penalty_bottom_right);
  const std::optional<Vector2i> pixelCl = camera_matrix_->robotToPixel(corner_left);
  const std::optional<Vector2i> pixelCr = camera_matrix_->robotToPixel(corner_right);
  // Check if all projection points lie outside of the image frame.
  if (!pixelPtl.has_value() || !pixelPtr.has_value() || !pixelPbl.has_value() ||
      !pixelPbr.has_value() || !pixelCl.has_value() || !pixelCr.has_value())
  {
    Log<M_VISION>(LogLevel::WARNING)
        << "The penalty area projection is outside of the observable image";
    // Send the unmodified camera image when the projection points are outside of the image.
    debug().sendImage(mount_ + "." + image_data_->identification + "_penalty_project_image",
                      image_data_->image422.to444Image());
    return;
  }

  Image calibImage(image_data_->image422.to444Image());

  const Vector2i pixelPtl444 = Image422::get444From422Vector(pixelPtl.value());
  const Vector2i pixelPtr444 = Image422::get444From422Vector(pixelPtr.value());
  const Vector2i pixelPbl444 = Image422::get444From422Vector(pixelPbl.value());
  const Vector2i pixelPbr444 = Image422::get444From422Vector(pixelPbr.value());
  const Vector2i pixelCl444 = Image422::get444From422Vector(pixelCl.value());
  const Vector2i pixelCr444 = Image422::get444From422Vector(pixelCr.value());

  // Draw lines for the penalty area on the camera image.
  calibImage.drawCross((pixelPtl444 + pixelPtr444) / 2, 8, Color::RED); // middle of penalty line.
  calibImage.drawCross((pixelPbl444 + pixelPbr444) / 2, 8,
                       Color::RED); // middle of penalty_box? line.
  calibImage.drawCross(pixelPtl444, 8, Color::RED);
  calibImage.drawCross(pixelPtr444, 8, Color::RED);
  calibImage.drawCross(pixelPbl444, 8, Color::RED);
  calibImage.drawCross(pixelPbr444, 8, Color::RED);
  calibImage.drawLine(pixelPtl444, pixelPtr444, Color::PINK);
  calibImage.drawLine(pixelPbl444, pixelPbr444, Color::PINK);
  calibImage.drawLine(pixelPbl444, pixelPtl444, Color::PINK);
  calibImage.drawLine(pixelPbr444, pixelPtr444, Color::PINK);
  // Draw the line between the field corners and mark them with crosses
  calibImage.drawLine(pixelCl444, pixelCr444, Color::PINK);
  calibImage.drawCross(pixelCl444, 8, Color::RED);
  calibImage.drawCross(pixelCr444, 8, Color::RED);

  debug().sendImage(mount_ + "." + image_data_->identification + "_penalty_project_image",
                    calibImage);
}
