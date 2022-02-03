#pragma once

#include "Data/CycleInfo.hpp"
#include "Data/HeadMatrixBuffer.hpp"
#include "Data/RobotKinematics.hpp"
#include "Framework/Module.hpp"
#include <array>
#include <boost/circular_buffer.hpp>


class Motion;

class HeadMatrixBufferProvider : public Module<HeadMatrixBufferProvider, Motion>
{
public:
  /// the name of this module
  ModuleName name__{"HeadMatrixBufferProvider"};

  explicit HeadMatrixBufferProvider(const ModuleManagerInterface& manager);
  void cycle() override;

private:
  const Dependency<CycleInfo> cycleInfo_;
  const Dependency<RobotKinematics> robotKinematics_;
  Production<HeadMatrixBuffer> headMatrixBuffer_;

  /// the fixed size of the buffer (measurements indicate that there is never a difference of more
  /// than 300ms between joint angles and camera image)
  static constexpr unsigned int bufferSize__ = 30;
  /// the actual buffer
  boost::circular_buffer<HeadMatrixWithTimestamp> buffer_{bufferSize__};
};
