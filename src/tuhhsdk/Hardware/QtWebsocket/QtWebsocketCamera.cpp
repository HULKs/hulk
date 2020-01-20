#include <unistd.h>

#include "QtWebsocketCamera.hpp"

QtWebsocketCamera::QtWebsocketCamera() : image_(Vector2<int>(640, 480))
{
}

void QtWebsocketCamera::setImage(const Image422& image, const TimePoint timestamp)
{
  std::lock_guard<std::mutex> lg(lock_);
  image_ = image;
  timestamp_ = timestamp;
}

float QtWebsocketCamera::waitForImage()
{
  usleep(33333);
  timestamp_ = TimePoint::getCurrentTime();
  return 0.033333;
}

TimePoint QtWebsocketCamera::readImage(Image422& image)
{
  std::lock_guard<std::mutex> lg(lock_);
  image = image_;
  return timestamp_;
}

void QtWebsocketCamera::releaseImage()
{
}

void QtWebsocketCamera::startCapture()
{
}

void QtWebsocketCamera::stopCapture()
{
}

Camera QtWebsocketCamera::getCameraType()
{
  return Camera::TOP;
}
