#pragma once

#include "Hardware/Nao/common/NaoCamera.hpp"

class Nao5Camera : public NaoCamera
{
public:
  Nao5Camera(const Camera camera);

  virtual ~Nao5Camera();

  void configure(Configuration& config, NaoInfo& naoInfo) override;

protected:
  void setControlSettings() override;
  void setSpecialControlSettings() override;
  void verifyControlSettings() override;
  void onOrientationChange() override;
  void onExposureChange() override;
  void onHueChange() override;

  /// exposure time in 0.1ms - 0 means auto exposure
  V4L2CtrlSetting exposure_;
  /// gamma
  V4L2CtrlSetting gamma_;
  /// fade to black
  V4L2CtrlSetting fadeToBlack_;

  /// ae max AGain
  V4L2CtrlSetting aeMaxAGain_;
  /// ae min AGain
  V4L2CtrlSetting aeMinAGain_;
  /// ae max DGain
  V4L2CtrlSetting aeMaxDGain_;
  /// ae min DGain
  V4L2CtrlSetting aeMinDGain_;
  /// ae target gain
  V4L2CtrlSetting aeTargetGain_;
  /// brightness dark
  V4L2CtrlSetting brightnessDark_;
  /// exposure algorithm
  V4L2CtrlSetting exposureAlgorithm_;

  /// horizontal flip
  V4L2CtrlSetting hFlip_;
  /// vertical flip
  V4L2CtrlSetting vFlip_;
};
