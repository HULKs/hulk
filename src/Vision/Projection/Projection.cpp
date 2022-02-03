#include "Framework/Module.hpp"
#include "Tools/Math/KinematicMatrix.hpp"

#ifdef HULK_TARGET_Replay
#include "Hardware/Replay/ReplayInterface.hpp"
#endif

#include "Vision/Projection/Projection.hpp"


Projection::Projection(const ModuleManagerInterface& manager)
  : Module(manager)
  , torsoCalibration_(*this, "torsoCalibration", [this] { updateTorsoCalibrationMatrix(); })
  , cam2groundStand_(*this, "cam2groundStand", [] {})
  , fov_(*this, "fov", [] {})
  , imageData_(*this)
  , headMatrixBuffer_(*this)
  , cameraMatrix_(*this)
  , topCamera_(*this, CameraPosition::TOP)
  , bottomCamera_(*this, CameraPosition::BOTTOM)
{
  updateTorsoCalibrationMatrix();
}

void Projection::cycle()
{
  ProjectionCamera& camera =
      (imageData_->cameraPosition == CameraPosition::TOP) ? topCamera_ : bottomCamera_;
  // TODO: continue only if the robot is approximately upright
  if (headMatrixBuffer_->buffer.empty())
  {
    return;
  }
#ifndef HULK_TARGET_Replay
  const Clock::time_point timestamp = imageData_->captureTimePoint;
#else
  const Clock::time_point timestamp =
      reinterpret_cast<ReplayInterface&>(robotInterface()).getRealFrameTime();
#endif
  // Get the camera matrix for 17 milliseconds after image recording (17 is approximately 1000/30/2)
  const HeadMatrixWithTimestamp& bufferEntry =
      headMatrixBuffer_->getBestMatch(timestamp
  // Except when in SimRobot because camera images are captured at one exact time point there.
#ifndef HULK_TARGET_SimRobot
                                      + std::chrono::milliseconds(17)
#endif
      );
  // This is a calibrated head to ground matrix (the camera to head part is applied a few lines
  // below).
  cameraMatrix_->camera2torso = torsoCalibrationMatrix_ * bufferEntry.head2torso;
  cameraMatrix_->camera2ground = bufferEntry.torso2ground * cameraMatrix_->camera2torso;
  cameraMatrix_->cam2groundStand = cam2groundStand_()[static_cast<int>(imageData_->cameraPosition)];
  {
    std::lock_guard<std::mutex> lg(camera.camera2head_lock);
    // This matrix transforms a vector in the camera coordinate system to a vector in the robot
    // coordinate system.
    cameraMatrix_->camera2ground *= camera.camera2head;
    cameraMatrix_->camera2torso *= camera.camera2head;
  }
  // divide position by 1000 because we want it in meters but the head matrix buffer stores them in
  // millimeters.
  cameraMatrix_->camera2torso.posV /= 1000.f;
  cameraMatrix_->camera2ground.posV /= 1000.f;
  // do some calculations here because they are needed in other functions that may be called often
  cameraMatrix_->camera2torsoInv = cameraMatrix_->camera2torso.inverted();
  cameraMatrix_->camera2groundInv = cameraMatrix_->camera2ground.inverted();
  // fc and cc have to be scaled for the image resolution
  cameraMatrix_->fc = camera.fc();
  cameraMatrix_->fc.x() *= imageData_->image422.size.x();
  cameraMatrix_->fc.y() *= imageData_->image422.size.y();
  cameraMatrix_->cc = camera.cc();
  cameraMatrix_->cc.x() *= imageData_->image422.size.x();
  cameraMatrix_->cc.y() *= imageData_->image422.size.y();
  cameraMatrix_->fov = fov_();
  const auto rM = cameraMatrix_->camera2ground.rotM.toRotationMatrix();
  if (rM(2, 2) == 0.f)
  {
    // Assume that the horizon is above the image.
    cameraMatrix_->horizonA = 0;
    cameraMatrix_->horizonB = 0;
  }
  else
  {
    // These formula can be derived from the condition that at the coordinates (x, y) the pixel ray
    // is parallel to the ground.
    cameraMatrix_->horizonA =
        -cameraMatrix_->fc.y() * rM(2, 1) / (cameraMatrix_->fc.x() * rM(2, 2));
    cameraMatrix_->horizonB =
        cameraMatrix_->cc.y() +
        cameraMatrix_->fc.y() *
            (rM(2, 0) + cameraMatrix_->cc.x() * rM(2, 1) / cameraMatrix_->fc.x()) / rM(2, 2);
  }
  cameraMatrix_->valid = true;
}

void Projection::updateTorsoCalibrationMatrix()
{
  torsoCalibrationMatrix_ = KinematicMatrix::rotY(torsoCalibration_().y()) *
                            KinematicMatrix::rotX(torsoCalibration_().x());
}
