#include "FakeBallProvider.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Pose.hpp"
#include "Tools/Math/Random.hpp"
#include "math.h"


FakeBallProvider::FakeBallProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , overlap_(*this, "overlap", [] {})
  , enableNoise_(*this, "enableNoise", [] {})
  , pixelNoiseSigma_(*this, "pixelNoiseSigma", [] {})
  , enableFieldOfView_(*this, "enableFieldOfView", [] {})
  , limitSight_(*this, "limitSight", [] {})
  , maxDetectionDistance_(*this, "maxDetectionDistance", [] {})
  , enableDetectionRate_(*this, "enableDetectionRate", [] {})
  , fakeImageData_(*this)
  , cycleInfo_(*this)
  , cameraMatrix_(*this)
  , fakeBallState_(*this)
{
}

void FakeBallProvider::cycle()
{
  Vector2f ball2Robot = Vector2f::Zero();
  Vector2i ball2Image = Vector2i::Zero();

  auto& fakeDataInterface = robotInterface().getFakeData();
  const bool fakeBallPositionAvailable = fakeDataInterface.readFakeBallPosition(ball2Robot);

  if (fakeBallPositionAvailable)
  {
    // by default we assume that we can see the ball. If parameters are
    // enabled, we invalidate or modify the ball position to simulate a more
    // realistic perception model
    bool isInFOV = true;
    bool inSight = true;
    bool isDetected = true;
    const bool ballTransformableToImage = cameraMatrix_->robotToPixel(ball2Robot, ball2Image);

    // invalidate ball if not in FOV and param enabled
    if (enableFieldOfView_())
    {
      // overlap_ is necessary for not losing the ball between top and bottom camera
      isInFOV = ballTransformableToImage && ball2Image.x() > 0 &&
                ball2Image.x() <= fakeImageData_->imageSize.x() && (ball2Image.y() > -overlap_()) &&
                ball2Image.y() <= fakeImageData_->imageSize.y();
    }
    // invalidate ball if too far away and param enabled
    if (limitSight_())
    {
      Vector3f ball2Camera(cameraMatrix_->camera2groundInv *
                           Vector3f(ball2Robot.x(), ball2Robot.y(), 0));
      inSight = ball2Camera.x() < maxDetectionDistance_();
    }
    // invalidate ball randomly to model limited detection rate if param enabled
    if (enableDetectionRate_())
    {
      // error rate gets larger with distance (log)
      const float randomFloat = Random::uniformFloat(0.0f, 1.0f);
      const float x = ball2Robot.x() / maxDetectionDistance_();
      isDetected = x > 0.f && x < 1.f ? randomFloat > -(1.f / 5.f) * log(1 - x) : false;
    }

    if (isInFOV && inSight && isDetected)
    {
      // Add noise to ball in image coordinates (to obtain the right
      // distribution in robot coodinates). Thus, noise can only be added if
      // the ball could be transformed to image coordinates!
      if (enableNoise_() && ballTransformableToImage)
      {
        auto noisyBall2Robot = ball2Robot;
        Vector2i noisyBall2Image = addGaussianNoise(ball2Image, pixelNoiseSigma_());
        if (cameraMatrix_->pixelToRobot(noisyBall2Image, ball2Robot))
        {
          Vector3f noisyBall2Camera(cameraMatrix_->camera2groundInv *
                                    Vector3f(ball2Robot.x(), ball2Robot.y(), 0));
          // prevents that the ball is behind the robot due to noise
          if (noisyBall2Camera.x() < 0)
          {
            noisyBall2Camera.x() = 0;
            // temporary vector for 3d to 2d conversion
            Vector3f noisyBall2RobotTemp = cameraMatrix_->camera2ground * noisyBall2Camera;
            noisyBall2Robot = Vector2f(noisyBall2RobotTemp.x(), noisyBall2RobotTemp.y());
          }
        }
        // if noise is enabled we add the potentially noisy ball to the list of
        // detected balls
        fakeBallState_->positions = {noisyBall2Robot};
        fakeBallState_->timestamp = cycleInfo_->startTime;
      }
      else
      {
        // the noise is not enabled or noisification not posstibe since no
        // transformation to the image plane can be computed.
        fakeBallState_->positions = {ball2Robot};
        fakeBallState_->timestamp = cycleInfo_->startTime;
      }
    }
  }
}

Vector2i FakeBallProvider::addGaussianNoise(const Vector2i& pixelPosition,
                                            const Vector2f sigma) const
{
  Vector2i ret;
  ret.x() = Random::gaussianFloat(pixelPosition.x(), sigma.x());
  ret.y() = Random::gaussianFloat(pixelPosition.y(), sigma.y());
  return ret;
}
