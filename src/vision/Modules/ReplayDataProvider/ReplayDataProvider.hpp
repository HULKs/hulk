#pragma once

#include <array>
#include <set>

#include "Data/GameControllerState.hpp"
#include "Data/JointSensorData.hpp"
#include "Data/ReplayData.hpp"
#include "Framework/Module.hpp"

class Brain;

class ReplayDataProvider : public Module<ReplayDataProvider, Brain>
{
public:
  /// the name of this module
  ModuleName name = "ReplayDataProvider";

  ReplayDataProvider(const ModuleManagerInterface& manager);
  void cycle();

private:
  /// When activated, the module tries to restore the head matrix buffer from replay
  /// NOTE: when enabling this behaviour, the HeadMatrixBufferProvider should be disabled in
  /// moduleSetup.
  const Parameter<bool> fakeHeadMatrixBuffer_;
  const Parameter<KinematicMatrix> head2torso_;
  const Parameter<KinematicMatrix> torso2ground_;


  std::array<HeadMatrixWithTimestamp, 1> buffer_;

  /// The restored HeadMatrixBuffer, if fakeHeadMatrixBuffer is enabled
  Production<HeadMatrixBuffer> headMatrixBuffer_;
  Production<JointSensorData> jointSensorData_;
  Production<GameControllerState> gameControllerState_;
  void restoreHeadMatrixBuffer();
  void restoreJointSensorData();
  void updateBuffer();
};
