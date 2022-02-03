#pragma once

#include "Data/CycleInfo.hpp"
#include "Data/ImageData.hpp"
#include "Hardware/Definitions.hpp"
#include "Tools/Storage/Image422.hpp"
#include <condition_variable>
#include <mutex>

class ReplayCamera
{
public:
  /**
   * @brief setImage sets the data that will be returned in subsequent calls to readImage
   * @param image an image
   * @param camera the camera type of the loaded image
   * @param timestamp the timestamp of the image
   */
  void setImage(const Image422& image, CameraPosition camera, Clock::time_point timestamp);
  void produce(CycleInfo& cycleInfo, ImageData& imageData);

  Clock::time_point readImage(Image422& image);

private:
  /// the current image that the camera would return
  Image422 image_;
  /// the timestamp of the current image
  Clock::time_point timestamp_;
  /// lock to prevent races between setImageData and readImage
  std::mutex newLock_;
  /// condition variable to wake brain thread up
  std::condition_variable newCv_;
  /// whether the image has not been processed yet
  volatile bool new_{false};
  /// camera type
  CameraPosition camera_{CameraPosition::TOP};
};
