#pragma once

#include "Tools/Storage/Image422.hpp"
#include "Tools/Time.hpp"

enum class Camera
{
  TOP,   ///< value for top camera
  BOTTOM ///< value for bottom camera
};

class CameraInterface
{
public:
  /**
   * @brief ~CameraInterface a virtual destructor for polymorphism
   */
  virtual ~CameraInterface() = default;
  /**
   * @brief waitForImage waits until there is a new image available to be processed
   * @return the number of seconds that have been waited
   */
  virtual float waitForImage() = 0;
  /**
   * @brief readImage copies the next image. Make sure to call waitForImage before readImage.
   * @param image is filled with the new image
   * @return the time point at which the first pixel of the image was recorded
   */
  virtual TimePoint readImage(Image422& image) = 0;
  /**
   * @brief releaseImage is used to possible release the image of a camera
   */
  virtual void releaseImage() = 0;
  /**
   * @brief startCapture starts capturing images
   */
  virtual void startCapture() = 0;
  /**
   * @brief stopCapture stops capturing images
   */
  virtual void stopCapture() = 0;
  /**
   * @brief getCamera queries if it represents a TOP or BOTTOM camera
   * @return the camera type
   */
  virtual Camera getCameraType() = 0;
};
