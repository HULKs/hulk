#include "Hardware/Replay/ReplayCamera.hpp"
#include "Hardware/Definitions.hpp"

void ReplayCamera::setImage(const Image422& image, const CameraPosition camera,
                            const Clock::time_point timestamp)
{
  camera_ = camera;
  {
    std::lock_guard<std::mutex> lg(newLock_);
    image_ = image;
    timestamp_ = timestamp;
    new_ = true;
  }
  newCv_.notify_all();
}

void ReplayCamera::produce(CycleInfo& cycleInfo, ImageData& imageData)
{
  std::unique_lock<std::mutex> lk(newLock_);
  newCv_.wait(lk, [this] { return new_; });
  imageData.image422 = image_;
  new_ = false;
  imageData.cameraPosition = camera_;
  imageData.identification = imageData.cameraPosition == CameraPosition::TOP ? "top" : "bottom";
  imageData.captureTimePoint = timestamp_;
  cycleInfo.startTime = timestamp_;
}

Clock::time_point ReplayCamera::readImage(Image422& image)
{
  std::lock_guard<std::mutex> lg(newLock_);
  image = image_;
  new_ = false;
  return timestamp_;
}
