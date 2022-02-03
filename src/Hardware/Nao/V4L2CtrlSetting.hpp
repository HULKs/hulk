#pragma once

#include <limits>
#include <string>

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
   * @param fd The file descriptor to use for communicating with the camera device.
   * @param name the name of this setting (must equal the name in the config)
   * @param command the V4L2 setting to represent
   * @param configuredValue the initial value of this setting
   * @param acceptFailure whether it is okay for us that this control setting is not always being
   * set correctly
   * @param retries how often to retry the ioctl before giving up
   */
  V4L2CtrlSetting(int fd, std::string name, int command, int configuredValue,
                  bool acceptFailure = false, unsigned int retries = 3);
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
  /**
   * @brief getName returns the name of this setting
   *
   * @return the config name
   */
  const std::string& getName() const;

private:
  /**
   * @brief setCameraBounds sets the camera bounds (allowed value range)
   *
   * @param min
   * @param max
   * @param step a value to apply. Value - min_ needs to be dividable by this value
   */
  void setCameraBounds(int min, int max, int step);

  /// The name of this setting (equals the name in the config)
  const std::string name_;
  /// The setting this object represents.
  const int command_ = 0;
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
