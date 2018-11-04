#include <mutex>

#include "SimRobotCamera.hpp"


SimRobotCamera::SimRobotCamera(const Camera camera)
  : cameraType_(camera)
  , imageAvailable_(false)
  , requiresRenderedImage_(false)
{
}

float SimRobotCamera::waitForImage()
{
  return 0.033333;
}

TimePoint SimRobotCamera::readImage(Image422& image)
{
  image = image_;
  imageAvailable_.store(false);
  return timestamp_;
}

void SimRobotCamera::releaseImage() {}

void SimRobotCamera::startCapture()
{
  requiresRenderedImage_ = true;
}

void SimRobotCamera::stopCapture()
{
  requiresRenderedImage_ = false;
}

bool SimRobotCamera::getRequiresRenderedImage()
{
  return requiresRenderedImage_;
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
  if (requiresRenderedImage_)
  {
    assert(bytes);
    assert(width_ > 0);
    assert(height_ > 0);
    // both images are stored from bottom to top as they come from SimRobot, therefore the strange
    // counting
    YCbCr422* dest = image_.data +
                     image_.calculateNeededSpace(image_.get422From444Vector({width_, height_ - 1}));
    // TODO: SSE/AVX(2) optimization
    for (int y = 0; y < image_.size.y(); y++)
    {
      for (int x = 0; x < image_.size.x(); x++, dest++)
      {
        unsigned char r1 = *(bytes++), g1 = *(bytes++), b1 = *(bytes++);
        unsigned char r2 = *(bytes++), g2 = *(bytes++), b2 = *(bytes++);

        // YCbCr to RGB conversion
        // Conversion factors from
        // https://de.wikipedia.org/wiki/YCbCr-Farbmodell#Umrechnung_zwischen_RGB_und_YCbCr
        dest->y1_ = std::min(255.0, std::max(0.0, 0.299 * r1 + 0.587 * g1 + 0.114 * b1));
        dest->cb_ = std::min(255.0, std::max(0.0, 128 - 0.168736 * r1 - 0.331264 * g1 + 0.5 * b1));
        dest->y2_ = std::min(255.0, std::max(0.0, 0.299 * r2 + 0.587 * g2 + 0.114 * b2));
        dest->cr_ = std::min(255.0, std::max(0.0, 128 + 0.5 * r1 - 0.418688 * g1 - 0.081312 * b1));
      }
      dest -= 2 * image_.size.x();
    }
  }

  timestamp_ = timestamp;
  imageAvailable_.store(true);
}

void SimRobotCamera::setShutdownRequest()
{
  imageAvailable_.store(true);
}

SimRobotCamera* SimRobotCamera::getNextCamera(std::array<SimRobotCamera*, 2> cameras)
{
  SimRobotCamera* firstCamera = nullptr;

  // Search for the camera with the oldest image
  for (auto camera : cameras)
  {
    if (camera->imageAvailable_)
    {
      if (!firstCamera)
      {
        firstCamera = camera;
      }

      if (firstCamera->timestamp_ > camera->timestamp_)
      {
        firstCamera = camera;
      }
    }
  }

  return firstCamera;
}

bool SimRobotCamera::renderCameras(std::array<SimRobotCamera*, 2> cameras,
                                   SimRobot::Object* simrobotCameras[2])
{
  bool imageAvailable = false;
  for (auto camera : cameras)
  {
    imageAvailable |= camera->imageAvailable_.load();
  }

  if (!imageAvailable)
  {
    auto srCameras = reinterpret_cast<SimRobotCore2::SensorPort**>(simrobotCameras);
    // only render images if requested by the camera interface
    reinterpret_cast<SimRobotCore2::SensorPort*>(simrobotCameras[0])
        ->renderCameraImages(srCameras, 2);
    // TopCamera
    cameras[0]->setImage(srCameras[0]->getValue().byteArray, TimePoint::getCurrentTime());
    // Bottom Camera
    cameras[1]->setImage(srCameras[1]->getValue().byteArray,
                         TimePoint::getCurrentTime() + std::chrono::milliseconds(1));

    return true;
  }

  return false;
}
