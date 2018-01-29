#include <mutex>

#include "SimRobotCamera.hpp"


SimRobotCamera::SimRobotCamera(const Camera camera)
  : cameraType_(camera)
  , shutdownRequest_(false)
{
}

float SimRobotCamera::waitForImage()
{
  std::unique_lock<std::mutex> lock(mutex_);
  cv_.wait(lock, [this]{return imageAvailable_ || shutdownRequest_;});
  return 0.033333;
}

TimePoint SimRobotCamera::readImage(Image& image)
{
  std::lock_guard<std::mutex> lg(mutex_);
  image = image_;
  imageAvailable_ = false;
  return timestamp_;
}

void SimRobotCamera::startCapture()
{
}

void SimRobotCamera::stopCapture()
{
}

Camera SimRobotCamera::getCameraType()
{
  return cameraType_;
}

void SimRobotCamera::setSize(const unsigned int width, const unsigned int height)
{
  assert(width_ == 0);
  assert(height_ == 0);
  width_ = width;
  height_ = height;
  assert(width_ > 0);
  assert(height_ > 0);
  image_.resize(Vector2i(width_, height_));
}

void SimRobotCamera::setImage(const unsigned char* bytes, TimePoint timestamp)
{
  assert(width_ > 0);
  assert(height_ > 0);
  {
    std::lock_guard<std::mutex> lg(mutex_);
    // both images are stored from bottom to top as they come from SimRobot, therefore the strange counting
    Color* dest = image_.data_ + width_ * (height_ - 1);
    // TODO: SSE/AVX(2) optimization
    for (unsigned int y = 0; y < height_; y++)
    {
      for (unsigned int x = 0; x < width_; x++, dest++)
      {
        unsigned char r = *(bytes++), g = *(bytes++), b = *(bytes++);
        // YCbCr to RGB conversion
        dest->y_ = std::min(255.0, std::max(0.0, 0.299 * r + 0.587 * g + 0.114 * b));
        dest->cb_ = std::min(255.0, std::max(0.0, 128 - 0.168736 * r - 0.331264 * g + 0.5 * b));
        dest->cr_ = std::min(255.0, std::max(0.0, 128 + 0.5 * r - 0.418688 * g - 0.081312 * b));
      }
      dest -= 2 * width_;
    }
    timestamp_ = timestamp;
    imageAvailable_ = true;
  }
  cv_.notify_one();
}

void SimRobotCamera::setShutdownRequest()
{
  {
    std::lock_guard<std::mutex> lg(mutex_);
    shutdownRequest_ = true;
  }
  cv_.notify_one();
}
