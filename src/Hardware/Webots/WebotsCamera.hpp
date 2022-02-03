#pragma once

#include "Data/CycleInfo.hpp"
#include "Data/ImageData.hpp"
#include "Hardware/Clock.hpp"
#include "Hardware/Definitions.hpp"
#include "Tools/Storage/Image422.hpp"
#include <condition_variable>
#include <mutex>
#include <webots/Camera.hpp>

class WebotsCamera
{
public:
  WebotsCamera(webots::Camera* camera, CameraPosition cameraPosition);
  void updateImage(const Clock::time_point& timePoint);
  void produce(CycleInfo& cycleInfo, ImageData& imageData);
  void enable();
  void disable();
  CameraPosition getCameraPosition();

private:
  webots::Camera* camera_{nullptr};
  CameraPosition cameraPosition_{CameraPosition::TOP};
  std::mutex imageMutex_;
  bool imageUpdated_{false};
  std::condition_variable imageUpdatedConditionVariable_;
  Image422 image_;
  Clock::time_point timePoint_;
};
