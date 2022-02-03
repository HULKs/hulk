#include "Hardware/SimRobot/SimRobotCamera.hpp"
#include <mutex>


SimRobotCamera::SimRobotCamera(const CameraPosition camera)
  : cameraPosition_{camera}
{
}

void SimRobotCamera::produce(CycleInfo& cycleInfo, ImageData& imageData)
{
  imageData.image422 = image_;
  imageData.cameraPosition = cameraPosition_;
  imageData.identification = imageData.cameraPosition == CameraPosition::TOP ? "top" : "bottom";
  imageData.captureTimePoint = timePoint_;
  cycleInfo.startTime = timePoint_;
  // cycleInfo.cycleTime will be set in SimRobotInterface::produceImageData()
  imageAvailable_.store(false);
}

void SimRobotCamera::setSize(const int width, const int height)
{
  assert(width_ == 0);
  assert(height_ == 0);
  width_ = width;
  height_ = height;
  assert(width_ > 0);
  assert(height_ > 0);
  image_.resize(Vector2i{width_, height_});
}

void SimRobotCamera::setImage(const unsigned char* bytes, const Clock::time_point& timePoint)
{
  if (enabled_)
  {
    assert(bytes);
    assert(width_ > 0);
    assert(height_ > 0);
    // both images are stored from bottom to top as they come from SimRobot, therefore the strange
    // counting
    YCbCr422* dest = image_.data + image_.calculateNeededSpace(
                                       Image422::get422From444Vector({width_, height_ - 1}));
    // TODO: SSE/AVX(2) optimization
    for (int y = 0; y < image_.size.y(); y++)
    {
      for (int x = 0; x < image_.size.x(); x++, dest++)
      {
        unsigned char r1{*(bytes++)};
        unsigned char g1{*(bytes++)};
        unsigned char b1{*(bytes++)};
        unsigned char r2{*(bytes++)};
        unsigned char g2{*(bytes++)};
        unsigned char b2{*(bytes++)};

        // YCbCr to RGB conversion
        // Conversion factors from
        // https://de.wikipedia.org/wiki/YCbCr-Farbmodell#Umrechnung_zwischen_RGB_und_YCbCr
        dest->y1 =
            static_cast<std::uint8_t>(std::clamp(0.299 * r1 + 0.587 * g1 + 0.114 * b1, 0.0, 255.0));
        dest->cb = static_cast<std::uint8_t>(
            std::clamp(128 - 0.168736 * r1 - 0.331264 * g1 + 0.5 * b1, 0.0, 255.0));
        dest->y2 =
            static_cast<std::uint8_t>(std::clamp(0.299 * r2 + 0.587 * g2 + 0.114 * b2, 0.0, 255.0));
        dest->cr = static_cast<std::uint8_t>(
            std::clamp(128 + 0.5 * r1 - 0.418688 * g1 - 0.081312 * b1, 0.0, 255.0));
      }
      dest -= 2 * image_.size.x();
    }
  }

  timePoint_ = timePoint;
  imageAvailable_.store(true);
}

void SimRobotCamera::setShutdownRequest()
{
  shutdownRequested_.store(true);
}

void SimRobotCamera::enable()
{
  enabled_ = true;
}

void SimRobotCamera::disable()
{
  enabled_ = false;
}

bool SimRobotCamera::isEnabled() const
{
  return enabled_;
}

SimRobotCamera* SimRobotCamera::getNextCamera(std::array<SimRobotCamera*, 2> cameras)
{
  SimRobotCamera* firstCamera{nullptr};
  // Search for the camera with the oldest image
  for (auto* const camera : cameras)
  {
    if (camera->imageAvailable_.load() || camera->shutdownRequested_.load())
    {
      if (firstCamera == nullptr)
      {
        firstCamera = camera;
      }
      if (firstCamera->timePoint_ > camera->timePoint_)
      {
        firstCamera = camera;
      }
    }
  }
  return firstCamera;
}

bool SimRobotCamera::renderCameras(std::array<SimRobotCamera*, 2> cameras,
                                   std::array<SimRobot::Object*, 2> simrobotCameras,
                                   const Clock::time_point& timePoint)
{
  bool allCamerasReadyForNewImage{true};
  for (auto* const camera : cameras)
  {
    if (camera->imageAvailable_.load())
    {
      allCamerasReadyForNewImage = false;
    }
  }

  if (allCamerasReadyForNewImage)
  {
    reinterpret_cast<SimRobotCore2::SensorPort*>(simrobotCameras[0])
        ->renderCameraImages(reinterpret_cast<SimRobotCore2::SensorPort**>(simrobotCameras.data()),
                             2);
    // top camera
    cameras[0]->setImage(
        reinterpret_cast<SimRobotCore2::SensorPort*>(simrobotCameras[0])->getValue().byteArray,
        timePoint);
    // bottom camera
    cameras[1]->setImage(
        reinterpret_cast<SimRobotCore2::SensorPort*>(simrobotCameras[1])->getValue().byteArray,
        timePoint + 1ms);

    return true;
  }

  return false;
}
