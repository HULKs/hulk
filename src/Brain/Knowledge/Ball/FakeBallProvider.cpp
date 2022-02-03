#include "Brain/Knowledge/Ball/FakeBallProvider.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Pose.hpp"
#include "Tools/Math/Random.hpp"
#include <cmath>


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

// NOLINTNEXTLINE(readability-function-cognitive-complexity)
void FakeBallProvider::cycle()
{
  Vector2f ball2Robot = Vector2f::Zero();

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
    const std::optional<Vector2i> ball2Image = cameraMatrix_->robotToPixel(ball2Robot);

    // invalidate ball if not in FOV and param enabled
    if (enableFieldOfView_())
    {
      // overlap_ is necessary for not losing the ball between top and bottom camera
      isInFOV = ball2Image.has_value() && ball2Image->x() > 0 &&
                ball2Image->x() <= fakeImageData_->imageSize.x() && ball2Image->y() > -overlap_() &&
                ball2Image->y() <= fakeImageData_->imageSize.y();
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
      if (enableNoise_() && ball2Image.has_value())
      {
        const Vector2i noisyBall2Image = addGaussianNoise(ball2Image.value(), pixelNoiseSigma_());
        if (std::optional<Vector2f> noisyBall2Robot = cameraMatrix_->pixelToRobot(noisyBall2Image);
            noisyBall2Robot.has_value())
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
          fakeBallState_->positions = {noisyBall2Robot.value()};
          fakeBallState_->timestamp = cycleInfo_->startTime;
        }
        else
        {
          fakeBallState_->positions = {ball2Robot};
          fakeBallState_->timestamp = cycleInfo_->startTime;
        }
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
