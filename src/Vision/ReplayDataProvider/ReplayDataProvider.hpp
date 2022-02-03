#pragma once

#include <array>
#include <set>

#include "Data/CycleInfo.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/JointSensorData.hpp"
#include "Data/ReplayData.hpp"
#include "Framework/Module.hpp"

class Brain;

/**
 * @brief ReplayDataProvider restores data from the replay json file
 *
 * This module restores the head matrix buffer as well as the game controller state from the replay
 * data as they are not so easy to handle in the modules that provided this data on the nao.
 *
 * All other data is provided by the ImageReceiver and SensorDataProvider via the ReplayInterface.
 */
class ReplayDataProvider : public Module<ReplayDataProvider, Brain>
{
public:
  /// the name of this module
  ModuleName name__{"ReplayDataProvider"};

  /**
   * @brief ReplayDataProvider initializes members
   * @param manager the module manager
   */
  explicit ReplayDataProvider(const ModuleManagerInterface& manager);
  void cycle() override;

private:
  /// When activated, the module tries to restore the head matrix buffer from replay
  /// NOTE: when enabling this behaviour, the HeadMatrixBufferProvider should be disabled in
  /// moduleSetup.
  const Parameter<bool> fakeHeadMatrixBuffer_;
  const Parameter<KinematicMatrix> head2torso_;
  const Parameter<KinematicMatrix> torso2ground_;

  std::array<HeadMatrixWithTimestamp, 1> buffer_;

  /// Cycle info is not needed. We require it to make sure that we run **after** the sensor data
  /// provider. Otherwise the fake data required for the head matrix buffer will not be valid at
  /// all.
  Dependency<CycleInfo> cycleInfo_;

  /// The restored HeadMatrixBuffer, if fakeHeadMatrixBuffer is enabled
  Production<HeadMatrixBuffer> headMatrixBuffer_;
  Production<GameControllerState> gameControllerState_;
  /**
   * @brief restoreHeadMatrixBuffer reads the hmb from the replay data and writes it in the DataType
   */
  void restoreHeadMatrixBuffer();
  /**
   * @brief updates a buffer i guess. Naming is on point here. (TODO: Rename this shit)
   */
  void updateBuffer();
};
