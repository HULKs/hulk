#include "FakeImageReceiver.hpp"

FakeImageReceiver::FakeImageReceiver(const ModuleManagerInterface& manager)
  : Module(manager)
  , robotKinematics_(*this)
  , imageData_(*this)
  , cycleInfo_(*this)
  , fakeImageData_(*this)
  , fakeCameraMatrix_(*this)
{
  topCamera2head_uncalib_ = KinematicMatrix::transZ(63.64) * KinematicMatrix::transX(58.71) *
                            KinematicMatrix::rotY(0.0209);
  bottomCamera2head_uncalib_ = KinematicMatrix::transZ(17.74) * KinematicMatrix::transX(50.71) *
                               KinematicMatrix::rotY(0.6929);

  configuration().mount("topCamera", "topCamera_v_6.json", ConfigurationType::HEAD);
  configuration().get("topCamera", "resolution") >> topImageSize_;

  configuration().mount("bottomCamera", "bottomCamera_v_6.json", ConfigurationType::HEAD);
  configuration().get("bottomCamera", "resolution") >> bottomImageSize_;

  configuration().mount("Projection", "Projection.json", ConfigurationType::HEAD);
  configuration().get("Projection", "top_fc") >> topFc_;
  configuration().get("Projection", "bottom_fc") >> bottomFc_;
  configuration().get("Projection", "top_cc") >> topCc_;
  configuration().get("Projection", "bottom_cc") >> bottomCc_;
}


void FakeImageReceiver::cycle()
{
  // ToDo change is_provided to isProvided
  if (!imageData_->is_provided)
  {
    Image422 dummy_image;
    CameraInterface& camera = robotInterface().getNextCamera();
    fakeImageData_->imageSize =
        camera.getCameraType() == Camera::TOP ? topImageSize_ : bottomImageSize_;

    camera.waitForImage();
    // use the readImage method of the camera interface to trigger notify
    // the camera, that the image was received (for the purpose thread of synchronization)
    auto cycleTime = camera.readImage(dummy_image);

    // This needs to be the first call to debug in the ModuleManager per cycle
    debug().setUpdateTime(cycleTime);

    cycleInfo_->cycleTime = 0.01666f;
    cycleInfo_->startTime = cycleTime;

    // head to ground matrix
    const auto& head2torso = robotKinematics_->matrices[JOINTS::HEAD_PITCH];
    const auto& torso2ground = robotKinematics_->matrices[JOINTS::TORSO2GROUND_IMU];
    if (camera.getCameraType() == Camera::TOP)
    {
      fakeCameraMatrix_->camera2ground = torso2ground * head2torso * topCamera2head_uncalib_;
      fakeCameraMatrix_->camera2torso = head2torso * topCamera2head_uncalib_;
      fakeCameraMatrix_->fc = topFc_;
      fakeCameraMatrix_->cc = topCc_;
    }
    else
    {
      fakeCameraMatrix_->camera2ground = torso2ground * head2torso * bottomCamera2head_uncalib_;
      fakeCameraMatrix_->camera2torso = head2torso * bottomCamera2head_uncalib_;
      fakeCameraMatrix_->fc = bottomFc_;
      fakeCameraMatrix_->cc = bottomCc_;
    }
    // fc and cc have to be scaled for the image resolution
    fakeCameraMatrix_->fc.x() *= fakeImageData_->imageSize.x();
    fakeCameraMatrix_->fc.y() *= fakeImageData_->imageSize.y();
    fakeCameraMatrix_->cc.x() *= fakeImageData_->imageSize.x();
    fakeCameraMatrix_->cc.y() *= fakeImageData_->imageSize.y();
    // divide position by 1000 because we want it in meters but the head matrix buffer stores them
    // in millimeters.
    fakeCameraMatrix_->camera2torso.posV /= 1000.f;
    fakeCameraMatrix_->camera2ground.posV /= 1000.f;
    // do some calculations here because they are needed in other functions that may be called often
    fakeCameraMatrix_->camera2torsoInv = fakeCameraMatrix_->camera2torso.invert();
    fakeCameraMatrix_->camera2groundInv = fakeCameraMatrix_->camera2ground.invert();
    const auto rM = fakeCameraMatrix_->camera2ground.rotM.toRotationMatrix();
    if (rM(2, 2) == 0.f)
    {
      // Assume that the horizon is above the image.
      fakeCameraMatrix_->horizonA = 0;
      fakeCameraMatrix_->horizonB = 0;
    }
    else
    {
      // These formula can be derived from the condition that at the coordinates (x, y) the pixel
      // ray is parallel to the ground.
      fakeCameraMatrix_->horizonA =
          -fakeCameraMatrix_->fc.y() * rM(2, 1) / (fakeCameraMatrix_->fc.x() * rM(2, 2));
      fakeCameraMatrix_->horizonB =
          fakeCameraMatrix_->cc.y() +
          fakeCameraMatrix_->fc.y() *
              (rM(2, 0) + fakeCameraMatrix_->cc.x() * rM(2, 1) / fakeCameraMatrix_->fc.x()) /
              rM(2, 2);
    }
    fakeCameraMatrix_->valid = true;
  }
}
