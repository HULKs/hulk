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
  bool fakeBallPositionAvailable = fakeDataInterface.readFakeBallPosition(ball2Robot);

  if (fakeBallPositionAvailable)
  {
    bool insideImage = true;
    bool inSight = true;
    bool isDetected = true;

    if (cameraMatrix_->robotToPixel(ball2Robot, ball2Image))
    {
      // sets the enabled features
      if (enableFieldOfView_())
      {
        // overlap_ is necessary for not losing the ball between top and bottom camera
        insideImage = ball2Image.x() > 0 && ball2Image.x() <= fakeImageData_->imageSize.x() &&
                      (ball2Image.y() > -overlap_()) &&
                      ball2Image.y() <= fakeImageData_->imageSize.y();
      }
      if (limitSight_())
      {
        Vector3f ball2Camera(cameraMatrix_->camera2groundInv *
                             Vector3f(ball2Robot.x(), ball2Robot.y(), 0));
        inSight = ball2Camera.x() < maxDetectionDistance_();
      }
      if (enableDetectionRate_())
      {
        // error rate gets larger with distance (log)
        const float randomFloat = Random::uniformFloat(0.0f, 1.0f);
        const float x = ball2Robot.x() / maxDetectionDistance_();
        isDetected = x > 0.f && x < 1.f ? randomFloat > -(1.f / 5.f) * log(1 - x) : false;
      }

      if (insideImage && inSight && isDetected)
      {
        // Add noise to output
        if (enableNoise_())
        {
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
              Vector3f ball2RobotTemp = cameraMatrix_->camera2ground * noisyBall2Camera;
              ball2Robot = Vector2f(ball2RobotTemp.x(), ball2RobotTemp.y());
            }
          }
        }
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
