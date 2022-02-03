#include "Motion/Puppet/Puppet.hpp"
#include "Framework/Log/Log.hpp"
#include <type_traits>

Puppet::Puppet(const ModuleManagerInterface& manager)
  : Module(manager)
  , remotePuppetJointKeyFrame_(*this, "remotePuppetJointKeyFrame", [this] { updateKeyFrame(); })
  , remotePuppetStiffnesses_(*this, "remotePuppetStiffnesses", [this] { updateStiffness(); })
  , cycleInfo_(*this)
  , jointSensorData_(*this)
  , puppetMotionOutput_(*this)
{
  // 0.5 is assumed to be safe enough
  constexpr float safeStiffness = 0.5f;
  stiffnesses_.fill(safeStiffness);
}

void Puppet::updateKeyFrame()
{
  std::lock_guard<std::mutex> lg(actualRemotePuppetJointKeyFrameLock_);
  actualRemotePuppetJointKeyFrame_ = remotePuppetJointKeyFrame_();
  newRemotePuppetKeyFrame_ = true;
}

void Puppet::updateStiffness()
{
  std::lock_guard<std::mutex> lg(stiffnessLock_);
  stiffnesses_ = remotePuppetStiffnesses_();
}

void Puppet::cycle()
{
  std::lock_guard<std::mutex> lgKeyFrame(actualRemotePuppetJointKeyFrameLock_);
  std::lock_guard<std::mutex> lgStiffness(stiffnessLock_);
  // produce stiffness data
  puppetMotionOutput_->stiffnesses = stiffnesses_;

  if (newRemotePuppetKeyFrame_)
  {
    newRemotePuppetKeyFrame_ = false;
    keyFrameInterpolator_.reset(jointSensorData_->getBodyAngles(),
                                actualRemotePuppetJointKeyFrame_.jointAngles,
                                actualRemotePuppetJointKeyFrame_.interpolationTime);
  }
  if (!keyFrameInterpolator_.isFinished())
  {
    puppetMotionOutput_->angles = {keyFrameInterpolator_.step(cycleInfo_->cycleTime)};
  }
  else
  {
    puppetMotionOutput_->angles = actualRemotePuppetJointKeyFrame_.jointAngles;
  }
}
