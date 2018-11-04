#include "ReplayCamera.hpp"

ReplayCamera::ReplayCamera()
  : image_()
  , new_(false)
  , camera_(Camera::TOP)
{
}

void ReplayCamera::setImage(const Image422& image, const Camera camera, const TimePoint timestamp)
{
  camera_ = camera;
  {
    std::lock_guard<std::mutex> lg(new_lock_);
    image_ = image;
    timestamp_ = timestamp;
    new_ = true;
  }
  new_cv_.notify_all();
}

float ReplayCamera::waitForImage()
{
  std::unique_lock<std::mutex> lk(new_lock_);
  new_cv_.wait(lk, [this] { return new_; });
  return 0.033333;
}

TimePoint ReplayCamera::readImage(Image422& image)
{
  std::lock_guard<std::mutex> lg(new_lock_);
  image = image_;
  new_ = false;
  return timestamp_;
}

void ReplayCamera::releaseImage() {}

void ReplayCamera::startCapture() {}

void ReplayCamera::stopCapture() {}

Camera ReplayCamera::getCameraType()
{
  return camera_;
}
