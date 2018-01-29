#include "ProjectionCamera.hpp"

ProjectionCamera::ProjectionCamera(const ModuleBase& module, const Camera camera)
  : ext(module, (camera == Camera::TOP) ? "top_ext" : "bottom_ext", [this] { updateCamera2Head(); })
  , fc(module, (camera == Camera::TOP) ? "top_fc" : "bottom_fc")
  , cc(module, (camera == Camera::TOP) ? "top_cc" : "bottom_cc")
{
  // These values are from http://doc.aldebaran.com/2-1/family/robots/video_robot.html
  // They specify the translation and rotation of the cameras to the HEAD_PITCH joint.
  switch (camera)
  {
    case Camera::TOP:
      camera2head_uncalib = KinematicMatrix::transZ(63.64) * KinematicMatrix::transX(58.71) * KinematicMatrix::rotY(0.0209);
      break;
    case Camera::BOTTOM:
      camera2head_uncalib = KinematicMatrix::transZ(17.74) * KinematicMatrix::transX(50.71) * KinematicMatrix::rotY(0.6929);
      break;
  }
  updateCamera2Head();
}

void ProjectionCamera::updateCamera2Head()
{
  // Compute external calibration matrix. It specifies the offset between the real camera position/rotation and the HEAD_PITCH joint.
  // The order of these multiplications is important.
  std::lock_guard<std::mutex> lg(camera2head_lock);
  camera2head = camera2head_uncalib * KinematicMatrix::rotX(ext().x()) * KinematicMatrix::rotY(ext().y()) * KinematicMatrix::rotZ(ext().z());
}
