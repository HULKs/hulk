#pragma once

#include "Hardware/Nao/common/NaoCamera.hpp"

class Nao6Camera : public NaoCamera
{
public:
  /**
   * @brief Nao6Camera initializes base class
   * @param camera one of the cameras that the NAO has
   */
  explicit Nao6Camera(const Camera camera);
  /**
   * @brief ~NaoCamera frees memory and closes the filehandle
   */
  virtual ~Nao6Camera();

  void configure(Configuration& config, NaoInfo& naoInfo) override;

protected:
  void setControlSettings() override;
  void setSpecialControlSettings() override;
  void verifyControlSettings() override;
  void onOrientationChange() override;
  void onExposureChange() override;
  void onHueChange() override;

  /**
   * @brief onAutoFocusChange
   */
  void onFocusChange();
  /**
   * @brief onRegisterAction
   */
  void onRegisterAction();
  /**
   * @brief onDigitalEffectsChange applies the received config values to the camera.
   */
  void onDigitalEffectsChange();
  /**
   * @brief onAWBBiasChange applies the received config values to the camera.
   */
  void onAWBBiasChange();
  /**
   * @brief reads the 8-bit register at address addr from the camera
   * like it is done in
   * https://gitlab.com/clemolgat-SBR/leopard-imaging/blob/master/test-firmware/libCamera/src/CameraLIOV5640.cpp
   * @param addr the 16-bit address of the register to be read
   * @return the value of the read register
   */
  std::uint16_t readRegister(std::uint16_t addr) const;
  /**
   * @brief writes value to the 8-bit register at address addr from the camera
   * like it is done in
   * https://gitlab.com/clemolgat-SBR/leopard-imaging/blob/master/test-firmware/libCamera/src/CameraLIOV5640.cpp
   * @param addr the 16-bit address of the target register
   * @param value the 16-bit value to be written
   */
  void writeRegister(std::uint16_t addr, std::uint16_t value) const;

  /**
   * @brief setSingleBit sets a single bit of the given value.
   * @param value the value to set the bit of
   * @param bit the bit to set
   * @param enable whether to set the bit to 1.
   */
  void setSingleBit(std::uint16_t& value, const std::uint8_t bit, const bool enable);

  /// exposure time in 0.1ms - 0 means auto exposure
  V4L2CtrlSetting exposure_;
  /// whether to use auto hue or not
  V4L2CtrlSetting autoHue_;
  /// whether to use auto focus or not
  V4L2CtrlSetting autoFocus_;
  /// whether the "special digital effects" should be enabled (see doc for register 0x5001)
  bool enableDigitalEffects_;
  /// whether the "auto white balance bias" should be enabled (see doc for register 0x5005)
  bool enableAWBBias_;
  /// set focus value (in increments of 25)
  V4L2CtrlSetting focus_;
  /// register address to access
  std::uint32_t registerAddr_;
  /// value to write in the register if registerWrite == true
  std::uint32_t registerValue_;
  /// whether to write or read the register in registerAddr_
  bool registerWrite_;
};
