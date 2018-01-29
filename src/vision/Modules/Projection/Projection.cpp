#include "Framework/Module.hpp"
#include "Tools/Kinematics/KinematicMatrix.h"

#include "Projection.hpp"


Projection::Projection(const ModuleManagerInterface& manager)
  : Module(manager, "Projection")
  , torso_calibration_(*this, "torso_calibration", [this] { updateTorsoCalibrationMatrix(); })
  , image_data_(*this)
  , head_matrix_buffer_(*this)
  , camera_matrix_(*this)
  , top_camera_(*this, Camera::TOP)
  , bottom_camera_(*this, Camera::BOTTOM)
{
  updateTorsoCalibrationMatrix();
}

void Projection::cycle()
{
  ProjectionCamera& camera = (image_data_->camera == Camera::TOP) ? top_camera_ : bottom_camera_;
  // TODO: continue only if the robot is approximately upright
  if (head_matrix_buffer_->buffer.empty())
  {
    return;
  }
  // Get the camera matrix for 17 milliseconds after image recording (17 is approximately 1000/30/2)
  const HeadMatrixWithTimestamp& buffer_entry = head_matrix_buffer_->getBestMatch(image_data_->timestamp
  // Except when in SimRobot because camera images are captured at one exact time point there.
#ifndef SIMROBOT
                                                                                  + std::chrono::milliseconds(17)
#endif
  );
  // This is a calibrated head to ground matrix (the camera to head part is applied a few lines below).
  camera_matrix_->camera2torso = torso_calibration_matrix_ * buffer_entry.head2torso;
  camera_matrix_->camera2ground = buffer_entry.torso2ground * camera_matrix_->camera2torso;
  {
    std::lock_guard<std::mutex> lg(camera.camera2head_lock);
    // This matrix transforms a vector in the camera coordinate system to a vector in the robot coordinate system.
    camera_matrix_->camera2ground *= camera.camera2head;
    camera_matrix_->camera2torso *= camera.camera2head;
  }
  // divide position by 1000 because we want it in meters but the head matrix buffer stores them in millimeters.
  camera_matrix_->camera2torso.posV /= 1000.f;
  camera_matrix_->camera2ground.posV /= 1000.f;
  // do some calculations here because they are needed in other functions that may be called often
  camera_matrix_->camera2torso_inv = camera_matrix_->camera2torso.invert();
  camera_matrix_->camera2ground_inv = camera_matrix_->camera2ground.invert();
  // fc and cc have to be scaled for the image resolution
  camera_matrix_->fc = camera.fc();
  camera_matrix_->fc.x() *= image_data_->image.size_.x();
  camera_matrix_->fc.y() *= image_data_->image.size_.y();
  camera_matrix_->cc = camera.cc();
  camera_matrix_->cc.x() *= image_data_->image.size_.x();
  camera_matrix_->cc.y() *= image_data_->image.size_.y();
  const auto rM = camera_matrix_->camera2ground.rotM.toRotationMatrix();
  if (rM(2, 2) == 0.f)
  {
    // Assume that the horizon is above the image.
    camera_matrix_->horizon_a = 0;
    camera_matrix_->horizon_b = 0;
  }
  else
  {
    // These formula can be derived from the condition that at the coordinates (x, y) the pixel ray is parallel to the ground.
    camera_matrix_->horizon_a = -camera_matrix_->fc.y() * rM(2, 1) / (camera_matrix_->fc.x() * rM(2, 2));
    camera_matrix_->horizon_b =
        camera_matrix_->cc.y() + camera_matrix_->fc.y() * (rM(2, 0) + camera_matrix_->cc.x() * rM(2, 1) / camera_matrix_->fc.x()) / rM(2, 2);
  }
  camera_matrix_->valid = true;
}

void Projection::updateTorsoCalibrationMatrix()
{
  torso_calibration_matrix_ = KinematicMatrix::rotY(torso_calibration_().y()) * KinematicMatrix::rotX(torso_calibration_().x());
}
