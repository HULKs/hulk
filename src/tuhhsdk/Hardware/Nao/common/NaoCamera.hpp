#pragma once

#include <array>
#include <linux/videodev2.h>

#include "Hardware/CameraInterface.hpp"
#include "Hardware/RobotInterface.hpp"


class Configuration;

class NaoCamera : public CameraInterface
{
public:
  /**
   * @brief NaoCamera gets a filehandle for the selected camera
   * @param camera one of the cameras that the NAO has
   */
  explicit NaoCamera(const Camera camera);
  /**
   * @brief ~NaoCamera frees memory and closes the filehandle
   */
  virtual ~NaoCamera();
  /**
   * @brief configure loads configuration parameters and applies settings for the camera
   * This is needed because during the runtime of the constructor the identity of the robot
   * is not known.
   * @param config a reference to the Configuration instance
   */
  virtual void configure(Configuration& config, NaoInfo& naoInfo) = 0;
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
   * @return the timestamp of the image
   */
  __u64 getTimeStamp()
  {
    return timestamp;
  }

protected:
  /**
   * @brief V4L2CtrlSetting represents a single V4L2 control setting used to configure a camera.
   *
   * This class represents the state of a given V4L2 control setting and implements all
   * functionality needed to read, write and validate the setting's state.
   */
  class V4L2CtrlSetting
  {
  public:
    /**
     * @brief V4L2CtrlSetting initializes members.
     *
     * Note that one must call initialize before first use
     *
     * @param name the name of this setting (must equal the name in the config)
     * @param command the V4L2 setting to represent
     * @param configuredValue the initial value of this setting
     * @param acceptFailure whether it is okay for us that this control setting is not always being
     * set correctly
     */
    V4L2CtrlSetting(const std::string name, const int command, int configuredValue = 0,
                    bool acceptFailure = false);
    /**
     * @brief initialize reads the camera setting and initializes this object
     *
     * Opens the camera device, does some sanity checks and sets the bounds for this setting given
     * by the camera device driver.
     *
     * @param fd The file descriptor to use for communicating with the camera device.
     * @param retries How often to retry the ioctl before giving up
     */
    void initialize(int fd, unsigned int retries = 3);
    /**
     * @brief isValid checks whether the given value is in the camera bounds.
     *
     * @param value the value to check
     * @return whether the value is in the bounds.
     */
    bool isValid(int value) const;
    /**
     * @brief clipToRange returns a value that ensures isValid(value) by clipping the given value
     *
     * Ensures that the value - min_ is dividable by step_ and is inside the camera bounds.
     *
     * @param value the value to clip
     * @return the clipped value
     */
    int clipToRangeAndStep(int value) const;
    /**
     * @brief isApplied checks whether the setting is applied to the camera device
     *
     * @return whether the setting is applied to the camera device
     */
    bool isApplied();
    /**
     * @brief isAppliedGracefully checks whether the setting is applied or if failures are accepted
     *
     * @return whether the setting is applied to the camera device or if failures are accepted
     */
    bool isAppliedGracefully();
    /**
     * @brief applyValue applies a given value to the camera device
     *
     * The given value will be clipped to the camera bounds.
     *
     * @param value the value to apply
     * @param retries how often to retry the ioctl before giving up
     * @return isApplied()
     */
    bool applyValue(int value, unsigned int retries = 3);
    /**
     * @brief applyValue applies the configured value to the camera
     *
     * The configured value must have been set via setConfiguredValue() or applyValue(value). In the
     * second case the already applied value will be re-applied
     *
     * @param retries how often to retry the ioctl before giving up
     * @return isApplied()
     */
    bool applyValue(unsigned int retries = 3);
    /**
     * @brief setConfiguredValue sets the value to apply (no sanity checks)
     *
     * @param value the value to set
     */
    void setConfiguredValue(int value);
    /**
     * @brief getAppliedValue returns the value applied by the device driver
     *
     * @param retries how often to retry the ioctl before giving up
     * @return the applied value
     */
    int getAppliedValue(unsigned int retries = 3);
    /**
     * @brief getConfiguredValue returns the value that was passed to applyValue()
     *
     * @return the configured value
     */
    int getConfiguredValue() const;

    /// The name of this setting (equals the name in the config)
    const std::string name;
    /// The setting this object represents.
    const int command = 0;

  private:
    /**
     * @brief setCameraBounds sets the camera bounds (allowed value range)
     *
     * @param min
     * @param max
     * @param step a value to apply. Value - min_ needs to be dividable by this value
     */
    void setCameraBounds(const int min, const int max, const int step);

    /// The file descriptor to use for communication with the camera device
    int fd_ = -1;
    /// The value we want to apply
    int configuredValue_ = 0;
    /// The value that was applied by the camera device driver
    int appliedValue_ = 0;
    /// The minimum for configuredValue_
    int min_ = std::numeric_limits<int>::min();
    /// The maximum for configuredValue_
    int max_ = std::numeric_limits<int>::max();
    /// a (value - min_) to apply needs to be dividable by step_
    int step_ = 0;
    /// Whether it is okay if ioctl fails during applyValue()
    bool acceptFailure_ = false;
  };

  /**
   * @brief setFormat sets image resolution and format
   */
  void setFormat();
  /**
   * @brief setFrameRate sets the framerate
   */
  void setFrameRate();
  /**
   * @brief setControlSettings sets all control settings to the camera device
   *
   * This does not include settings like FPS, bufferCount, orientation, ... as they are non standard
   * settings
   */
  virtual void setControlSettings() = 0;

  /**
   * @brief setSpecialControlSettings sets all control settings that are not represented by
   * V4L2CtrlSetting objects
   */
  virtual void setSpecialControlSettings() = 0;

  /**
   * @brief verifyControlSettings checks if configuredValue == appliedValue for all settings.
   *
   * This does include special settings like FPS
   */
  virtual void verifyControlSettings() = 0;
  /**
   * @brief createBuffers maps the image buffers to process memory
   */
  void createBuffers();
  /**
   * @brief clearBuffers clears the image buffers.
   */
  void clearBuffers();

  /**
   * @brief onOrientationChange
   */
  virtual void onOrientationChange() = 0;
  /**
   * @brief onExposureChange
   */
  virtual void onExposureChange() = 0;
  /**
   * @brief onWhiteBalanceTemperatureChange
   */
  void onWhiteBalanceTemperatureChange();
  /**
   * @brief onHueChange
   */
  void virtual onHueChange() = 0;


  /// the camera which this class manages
  const Camera camera_;
  /// pointer to the config protocol
  Configuration* config_;
  /// the mount point
  const std::string mount_;
  /// the file descriptor of the camera handle
  int fd_;
  /// a vector of pointers to the buffers
  std::vector<unsigned char*> bufferMem_;
  /// a vector of the lengths of the buffers
  std::vector<unsigned int> bufferLength_;
  /// the currently used buffer
  v4l2_buffer currentBuffer_;
  /// is current buffer valid
  bool imageValid;
  /// the timestamp of the current buffer
  __u64 timestamp;
  /// information about the nao version the executable is running on
  NaoInfo naoInfo_;

  // Config Parameter

  /// list of all V4L2 control settings
  std::vector<NaoCamera::V4L2CtrlSetting*> cameraControlSettings_;

  /// the desired framerate
  unsigned int fps_;
  /// the number of buffers
  unsigned int bufferCount_;
  /// whether the buffers are actually initialized
  bool buffersInitialized_;
  /// the desired image resolution
  Vector2i resolution_;

  /// whether to use auto exposure
  V4L2CtrlSetting autoExposure_;
  /// whether to use auto white balance
  V4L2CtrlSetting autoWhiteBalance_;
  /// brightness
  V4L2CtrlSetting brightness_;
  /// contrast
  V4L2CtrlSetting contrast_;
  /// gain - if auto exposure is enabled this is the brightness
  V4L2CtrlSetting gain_;
  /// hue
  V4L2CtrlSetting hue_;
  /// saturation
  V4L2CtrlSetting saturation_;
  /// sharpness
  V4L2CtrlSetting sharpness_;
  /// white balance temperature - 0 means auto white balance
  V4L2CtrlSetting whiteBalanceTemperature_;

  // end Config parameter
};
