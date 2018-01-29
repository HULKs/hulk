#pragma once

#include <mutex>
#include <condition_variable>

#include "Hardware/CameraInterface.hpp"

class ReplayCamera : public CameraInterface {
public:
  /**
   */
  ReplayCamera();
  /**
   * @brief setImage sets the data that will be returned in subsequent calls to readImage
   * @param image an image
   * @param camera the camera type of the loaded image
   * @param timestamp the timestamp of the image
   */
  void setImage(const Image& image, const Camera camera, const TimePoint timestamp);
  /**
   * @brief waitForImage waits until there is a new image available to be processed
   * @return the number of seconds that have been waited
   */
  float waitForImage();
  /**
   * @brief readImage copies the next image
   * @param image is filled with the new image
   * @return the time point at which the first pixel of the image was recorded
   */
  TimePoint readImage(Image& image);
  /**
   * @brief startCapture starts capturing images
   */
  void startCapture();
  /**
   * @brief stopCapture stops capturing images
   */
  void stopCapture();
  /**
   * @brief getCamera queries if it represents a TOP or BOTTOM camera
   * @return the camera type
   */
  virtual Camera getCameraType();
private:
  /// the current image that the camera would return
  Image image_;
  /// the timestamp of the current image
  TimePoint timestamp_;
  /// lock to prevent races between setImageData and readImage
  std::mutex new_lock_;
  /// condition variable to wake brain thread up
  std::condition_variable new_cv_;
  /// whether the image has not been processed yet
  volatile bool new_;
  /// camera type
  Camera camera_;
};
