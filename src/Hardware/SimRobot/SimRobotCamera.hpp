#pragma once

#include "Data/CycleInfo.hpp"
#include "Data/ImageData.hpp"
#include "Hardware/Clock.hpp"
#include "Hardware/RobotInterface.hpp"
#include <SimRobotCore2.h>
#include <atomic>
#include <condition_variable>
#include <mutex>

class SimRobotCamera
{
public:
  /**
   * @brief SimRobotCamera initializes members
   * @param camera the camera which is accessed through this object
   */
  explicit SimRobotCamera(CameraPosition camera);
  /**
   * @brief readImage copies the next image
   * @param image is filled with the new image
   * @return the time point at which the first pixel of the image was recorded
   */
  void produce(CycleInfo& cycleInfo, ImageData& imageData);
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
   * @param timePoint the current timePoint
   * @return whether images where rendered
   */
  static bool renderCameras(std::array<SimRobotCamera*, 2> cameras,
                            std::array<SimRobot::Object*, 2> simrobotCameras,
                            const Clock::time_point& timePoint);
  void setSize(int width, int height);
  /**
   * @brief setImage gets called by SimRobot thread to set a new image and wakes readImage up
   * @param bytes start of an RGB image
   * @param timePoint the timePoint of the image
   */
  void setImage(const unsigned char* bytes, const Clock::time_point& timePoint);
  /**
   * @brief setShutdownRequest sets the shutdown request
   */
  void setShutdownRequest();

  void enable();
  void disable();
  bool isEnabled() const;

private:
  /// the width of the image
  unsigned int width_{0};
  /// the height of the image
  unsigned int height_{0};
  bool enabled_{false};
  /// the type of the camera
  CameraPosition cameraPosition_;
  /// whether an image is avialable from this camera
  std::atomic<bool> imageAvailable_{false};
  std::atomic<bool> shutdownRequested_{false};
  /// local copy of the image
  Image422 image_;
  Clock::time_point timePoint_;
};
