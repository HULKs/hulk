#pragma once

#include <array>
#include <linux/videodev2.h>

#include "Hardware/CameraInterface.hpp"


class Configuration;

class NaoCamera : public CameraInterface
{
public:
  /**
   * @brief NaoCamera gets a filehandle for the selected camera
   * @param camera one of the cameras that the NAO has
   */
  NaoCamera(const Camera camera);
  /**
   * @brief ~NaoCamera frees memory and closes the filehandle
   */
  ~NaoCamera();
  /**
   * @brief configure loads configuration parameters and applies settings for the camera
   * This is needed because during the runtime of the constructor the identity of the robot
   * is not known.
   * @param config a reference to the Configuration instance
   */
  void configure(Configuration& config);
  /**
   * @brief waitForImage waits until there is a new image available to be processed
   * @return the number of seconds that have been waited
   */
  float waitForImage();
  /**
   * @brief waitForImage waits for two cameras to get the newest image of the cameras
   * @param cameras an array of the two cameras to be waited on
   * @param timeout the timeout of the poll in milliseconds
   * @return if there is a new image available
   */
  static bool waitForCameras(std::array<NaoCamera*, 2> cameras, int timeout);
  /**
   * @brief readImage copies the next image
   * @param image is filled with the new image
   * @return the time point at which the first pixel of the image was recorded
   */
  TimePoint readImage(Image422& image);
  /**
   * @brief releaseImage is used to release the current image of the camera if available
   */
  void releaseImage();
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
  Camera getCameraType();
  /**
   * @brief isImageValid returns if the camera has an image ready for use
   * @return if there is a image waiting to be processed
   */
  bool isImageValid()
  {
    return imageValid;
  }
  /**
   * @brief getTimeStamp returns when the image was taken only valid if the image is valid
   * @return the timespamp of the image
   */
  __u64 getTimeStamp()
  {
    return timestamp;
  }

private:
  /**
   * @brief setFormat sets image resolution and format
   */
  void setFormat();
  /**
   * @brief setFrameRate sets the framerate
   */
  void setFrameRate();
  /**
   * @brief setControlSettings sets the control settings to their initial values
   */
  void setControlSettings();
  /**
   * @brief createBuffers maps the image buffers to process memory
   */
  void createBuffers();
  /**
   * @brief setControlSetting sets a driver setting
   * @param id an ID for identifying the setting
   * @param value the value for the setting
   */
  void setControlSetting(__u32 id, __s32 value);
  /**
   * @brief onExposureChange
   * @param value the parameter as Uni::Value
   */
  void onExposureChange();
  /**
   * @brief onGainChange
   * @param value the parameter as Uni::Value
   */
  void onGainChange();
  /**
   * @brief onWhiteBalanceTemperatureChange
   * @param value the parameter as Uni::Value
   */
  void onWhiteBalanceTemperatureChange(const Uni::Value& value);
  /**
   * @brief onContrastChange
   * @param value the parameter as Uni::Value
   */
  void onContrastChange(const Uni::Value& value);
  /**
   * @brief onHueChange
   * @param value the parameter as Uni::Value
   */
  void onHueChange(const Uni::Value& value);
  /**
   * @brief onGammaChange
   * @param value the parameter as Uni::Value
   */
  void onGammaChange(const Uni::Value& value);
  /**
   * @brief onSaturationChange
   * @param value the parameter as Uni::Value
   */
  void onSaturationChange(const Uni::Value& value);
  /**
   * @brief onSharpnessChange
   * @param value the parameter as Uni::Value
   */
  void onSharpnessChange(const Uni::Value& value);
  /**
   * @brief onFadeToBlackChange
   * @param value the parameter as Uni::Value
   */
  void onFadeToBlackChange(const Uni::Value& value);
  /// the camera which this class manages
  const Camera camera_;
  /// the mount point
  const std::string mount_;
  /// the file descriptor of the camera handle
  int fd_;
  /// the desired image resolution
  Vector2i resolution_;
  /// the desired framerate
  unsigned int fps_;
  /// the number of buffers
  unsigned int bufferCount_;
  /// an array of pointers to the buffers
  unsigned char** bufferMem_;
  /// an array of the lengths of the buffers
  unsigned int* bufferLength_;
  /// exposure time in 0.1ms - 0 means auto exposure
  __s32 exposure_;
  /// gain - if auto exposure is enabled this is the brightness
  __s32 gain_;
  /// white balance temperature - 0 means auto white balance
  __s32 whiteBalanceTemperature_;
  /// contrast
  __s32 contrast_;
  /// gamma
  __s32 gamma_;
  /// hue
  __s32 hue_;
  /// saturation
  __s32 saturation_;
  /// sharpness
  __s32 sharpness_;
  /// fade to black
  __s32 fadeToBlack_;
  /// brightness
  __s32 brightness_;
  /// brightness dark
  __s32 brightnessDark_;
  /// exposure algorithm
  __s32 exposureAlgorithm_;
  /// ae target gain
  __s32 aeTargetGain_;
  /// ae min AGain
  __s32 aeMinAGain_;
  /// ae max AGain
  __s32 aeMaxAGain_;
  /// ae min DGain
  __s32 aeMinDGain_;
  /// ae max DGain
  __s32 aeMaxDGain_;
  /// the currently used buffer
  v4l2_buffer currentBuffer_;
  /// is current buffer valid
  bool imageValid;
  /// the timestamp of the current buffer
  __u64 timestamp;
  /// number of iterations for setControlSetting
  static const unsigned int CONTROL_SETTING_TRIES = 5;
};
