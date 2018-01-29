#include "Modules/NaoProvider.h"
#include "Tools/Chronometer.hpp"

#include "HeadMatrixBufferProvider.hpp"


HeadMatrixBufferProvider::HeadMatrixBufferProvider(const ModuleManagerInterface& manager)
  : Module(manager, "HeadMatrixBufferProvider")
  , cycleInfo_(*this)
  , robotKinematics_(*this)
  , headMatrixBuffer_(*this)
{
}

void HeadMatrixBufferProvider::cycle()
{
  Chronometer time(debug(), mount_ + ".cycleTime");

  // index should always point to the oldest entry in the buffer.
  buffer_[index_].head2torso = robotKinematics_->matrices[JOINTS::HEAD_PITCH];
  buffer_[index_].torso2ground = robotKinematics_->matrices[JOINTS::TORSO2GROUND_IMU];
  buffer_[index_].timestamp = cycleInfo_->startTime;
  index_++;
  index_ %= bufferSize_;

  headMatrixBuffer_->buffer.assign(buffer_.begin(), buffer_.end());
}
