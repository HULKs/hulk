#pragma once

#include <array>

#include "Data/CycleInfo.hpp"
#include "Data/HeadMatrixBuffer.hpp"
#include "Data/RobotKinematics.hpp"
#include "Framework/Module.hpp"


class Motion;

class HeadMatrixBufferProvider : public Module<HeadMatrixBufferProvider, Motion>
{
public:
  /// the name of this module
  ModuleName name = "HeadMatrixBufferProvider";

  HeadMatrixBufferProvider(const ModuleManagerInterface& manager);
  void cycle();

private:
  const Dependency<CycleInfo> cycleInfo_;
  const Dependency<RobotKinematics> robotKinematics_;
  Production<HeadMatrixBuffer> headMatrixBuffer_;

  /// the fixed size of the buffer (measurements indicate that there is never a difference of more than 300ms between joint angles and camera image)
  static constexpr unsigned int bufferSize_ = 30;
  /// the actual buffer
  std::array<HeadMatrixWithTimestamp, bufferSize_> buffer_;
  /// the index in the buffer that should be written next (i.e. the oldest entry)
  unsigned int index_ = 0;
};
