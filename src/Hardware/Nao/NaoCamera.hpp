#pragma once

#include "Hardware/Nao/V4L2CtrlSetting.hpp"
#include "Hardware/RobotInterface.hpp"
#include <array>
#include <linux/videodev2.h>

class Configuration;

class NaoCamera
{
public:
  /**
   * @brief NaoCamera gets a filehandle for the selected camera
   * @param cameraPosition one of the cameras that the NAO has
   */
  explicit NaoCamera(CameraPosition cameraPosition);
  NaoCamera(const NaoCamera&) = delete;
  NaoCamera(NaoCamera&&) = delete;
  NaoCamera& operator=(const NaoCamera&) = delete;
  NaoCamera& operator=(NaoCamera&&) = delete;
  /**
   * @brief ~NaoCamera frees memory and closes the filehandle
   */
  ~NaoCamera();
  /**
   * @brief configure loads configuration parameters and applies settings for the camera
   * This is needed because during the runtime of the constructor the identity of the robot
   * is not known.
   * @param config a reference to the Configuration instance
   * @param naoInfo a reference to the nao information
   */
  void configure(Configuration& config);
  /**
   * @brief waitForImage waits for two cameras to get the newest image of the cameras
   * @param cameras an array of the two cameras to be waited on
   * @param timeout the timeout of the poll in milliseconds
   * @return if there is a new image available
   */
  static bool waitForCameras(std::array<NaoCamera*, 2> cameras, int timeout);
  void produce(CycleInfo& cycleInfo, ImageData& imageData);
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
   * @return the camera position
   */
  CameraPosition getCameraPosition();
  /**
   * @brief isImageValid returns if the camera has an image ready for use
   * @return if there is a image waiting to be processed
   */
  bool isImageValid() const
  {
    return imageValid_;
  }
  /**
   * @brief getTimeStamp returns when the image was taken only valid if the image is valid
   * @return the timestamp of the image
   */
  Clock::time_point getTimeStamp()
  {
    return timePoint_;
  }

private:
  /// the position of the camera this class manages
  const CameraPosition cameraPosition_;
  /// pointer to the config protocol
  Configuration* config_{nullptr};
  /// the mount point
  const std::string mount_;
  /// the file descriptor of the camera handle
  int fd_{-1};
  /// a vector of pointers to the buffers
  std::vector<unsigned char*> bufferMem_;
  /// a vector of the lengths of the buffers
  std::vector<unsigned int> bufferLength_;
  /// whether the buffers are actually initialized
  bool buffersInitialized_{false};
  /// the number of buffers
  unsigned int bufferCount_{0};
  /// the currently used buffer
  v4l2_buffer currentBuffer_{};
  /// is current buffer valid
  bool imageValid_{false};
  /// the timestamp of the current buffer
  Clock::time_point timePoint_;

  // Config Parameter

  /// list of all V4L2 control settings
  std::vector<std::shared_ptr<V4L2CtrlSetting>> cameraControlSettings_;

  /// the desired image resolution
  Vector2i resolution_{0, 0};

  /// register address to access
  std::uint32_t registerAddr_{0};
  /// value to write in the register if registerWrite == true
  std::uint32_t registerValue_{0};
  /// whether to write or read the register in registerAddr_
  bool registerWrite_{false};


  /**
   * @brief adds an V4L2 control setting
   * @param name the name of the setting (also the name in configuration)
   * @param v4l2Command the V4L2 command to apply with this setting
   * @param setValue whether to set the value initially
   */
  void addV4L2CtrlSetting(const std::string& name, int v4l2Command);
  /**
   * @brief setFormat sets image resolution and format
   */
  void setFormat();
  /**
   * @brief setFrameRate sets the framerate
   */
  void setFrameRate() const;
  /**
   * @brief setOrientation rotates the camera image correctly
   */
  void setOrientation();

  /**
   * @brief verifyControlSettings checks if configuredValue == appliedValue for all settings.
   *
   * This does include special settings like FPS
   */
  void verifyControlSettings();
  /**
   * @brief createBuffers maps the image buffers to process memory
   */
  void createBuffers();
  /**
   * @brief clearBuffers clears the image buffers.
   */
  void clearBuffers();

  /**
   * @brief onRegisterAction
   */
  void onRegisterAction();

  /**
   * @brief gets or sets a value via the UVC Extension unit
   * @param set whether to set a value or only get it (write flag)
   * @param selector the unit to select
   * @param data a pointer to the data to write
   * @param size the size of the data
   * @return whether the ioctl was successful
   */
  bool queryExtensionUnit(bool set, __u8 selector, __u8* data, __u16 size) const;

  /**
   * @brief sets a value via the UVC Extension unit
   * @tparam T type of the data to set
   * @param selector the unit to select
   * @param data the data to write
   * @return whether the ioctl was successful
   */
  template <typename T>
  bool setExtensionUnit(__u8 selector, T& data) const
  {
    return queryExtensionUnit(true, selector, reinterpret_cast<__u8*>(&data),
                              static_cast<__u16>(sizeof(data)));
  }

  /**
   * @brief gets a value via the UVC Extension unit
   * @tparam T type of the data to set
   * @param selector the unit to select
   * @param data a reference to a variable to write the read data to
   * @return whether the ioctl was successful
   */
  template <typename T>
  bool getExtensionUnit(__u8 selector, T& data) const
  {
    return queryExtensionUnit(false, selector, reinterpret_cast<__u8*>(&data),
                              static_cast<__u16>(sizeof(data)));
  }
  /**
   * @brief reads the 8-bit register at address address from the camera
   * like it is done in
   * https://gitlab.com/clemolgat-SBR/leopard-imaging/blob/master/test-firmware/libCamera/src/CameraLIOV5640.cpp
   * @param address the 16-bit address of the register to be read
   * @param value [out] reference to write the value to
   * @return whether the read was successful
   */
  bool readRegister(std::uint16_t address, std::uint16_t& value) const;
  /**
   * @brief writes value to the 8-bit register at address addr from the camera
   * like it is done in
   * https://gitlab.com/clemolgat-SBR/leopard-imaging/blob/master/test-firmware/libCamera/src/CameraLIOV5640.cpp
   * @param address the 16-bit address of the target register
   * @param value the 16-bit value to be written
   * @return whether the read was successful
   */
  bool writeRegister(std::uint16_t address, std::uint16_t value) const;
  /**
   * @brief setSingleBit sets a single bit of the given value.
   * @param value the value to set the bit of
   * @param bit the bit to set
   * @param enable whether to set the bit to 1.
   */
  static void setSingleBit(std::uint16_t& value, std::uint8_t bit, bool enable);
  /*
   * @brief creates a new register setting with config key with type T
   * @param key the name of the config key
   * @param callback function to call when the register value should be updated
   * @tparam T type of the value to set
   */
  template <typename T>
  void addRegisterSetting(const std::string& key, std::function<void(T)> callback) const;
};
