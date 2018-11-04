#pragma once

#include <atomic>
#include <condition_variable>
#include <mutex>

#include <SimRobotCore2.h>

#include "Hardware/CameraInterface.hpp"

class SimRobotCamera : public CameraInterface
{
public:
  /**
   * @brief SimRobotCamera initializes members
   * @param camera the camera which is accessed through this object
   */
  SimRobotCamera(const Camera camera);
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
  TimePoint readImage(Image422& image);
  /**
   * @brief releaseImage is used to possible release the image of a camera
   */
  void releaseImage();
  /**
   * @brief startCapture does nothing
   */
  void startCapture();
  /**
   * @brief stopCapture does nothing
   */
  void stopCapture();
  /**
   * @brief getRequiresRenderedImage a method to to check whether the camera requires a renderd
   * image
   * @return true if a rendered image is requested
   */
  bool getRequiresRenderedImage();
  /**
   * @brief getNextCamera returns the camera that needs to be processed next
   * @param cameras an array of all existing SimRobotCameras
   * @return returns nullptr if no camera is available, the camera with the oldest image otherwise
   */
  static SimRobotCamera* getNextCamera(std::array<SimRobotCamera*, 2> cameras);
  /**
   * @brief renderCameras starts the image rendering process if all images where rendered
   * @param cameras an array of all existing SimobotCameras
   * @param simrobotCameras an array of all existing SimRobot camera objects
   * @return whether images where rendered
   */
  static bool renderCameras(std::array<SimRobotCamera*, 2> cameras,
                            SimRobot::Object** simrobotCameras);
  /**
   * @brief getCameraType returns the type of the camera
   * @return the type of the camera
   */
  Camera getCameraType();
  /**
   * @brief setSize sets the size of all images this camera will deliver
   * @param width the number of columns in the image
   * @param height the number of rows in the image
   */
  void setSize(const unsigned int width, const unsigned int height);
  /**
   * @brief setImage gets called by SimRobot thread to set a new image and wakes readImage up
   * @param bytes start of an RGB image
   * @param timestamp the timestamp of the image
   */
  void setImage(const unsigned char* bytes, TimePoint timestamp);
  /**
   * @brief setShutdownRequest sets the shutdown request
   */
  void setShutdownRequest();

private:
  /// the width of the image
  unsigned int width_ = 0;
  /// the height of the image
  unsigned int height_ = 0;
  /// the type of the camera
  Camera cameraType_;
  /// whether an image is avialable from this camera
  std::atomic<bool> imageAvailable_;
  /// local copy of the image
  Image422 image_;
  /// true if images should be rendered
  bool requiresRenderedImage_;
  /// the timestamp of the image
  TimePoint timestamp_;
};
