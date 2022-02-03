#include "Motion/HeadMatrixBufferProvider/HeadMatrixBufferProvider.hpp"
#include "Tools/Chronometer.hpp"


HeadMatrixBufferProvider::HeadMatrixBufferProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , cycleInfo_(*this)
  , robotKinematics_(*this)
  , headMatrixBuffer_(*this)
{
}

void HeadMatrixBufferProvider::cycle()
{
  Chronometer time(debug(), mount_ + ".cycleTime");

  buffer_.push_back({robotKinematics_->matrices[Joints::HEAD_PITCH], robotKinematics_->torso2ground,
                     cycleInfo_->startTime});

  headMatrixBuffer_->buffer.assign(buffer_.begin(), buffer_.end());
  headMatrixBuffer_->valid = true;
}
