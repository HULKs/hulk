#include <array>
#include "FakeRobotPoseProvider.hpp"
#include "Tools/Math/Angle.hpp"


FakeRobotPoseProvider::FakeRobotPoseProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , fakeImageData_(*this)
  , cycleInfo_(*this)
  , fakeRobotPose_(*this)
  , lastPose_()
  , pose_()
  , lastTimeJumped_()
{
}

void FakeRobotPoseProvider::cycle()
{
  bool fakeDataAvailable = robotInterface().getFakeData().readFakeRobotPose(pose_);

  updateLastTimeJumped();

  fakeRobotPose_->pose = pose_;
  fakeRobotPose_->valid = fakeDataAvailable;
  fakeRobotPose_->lastTimeJumped = lastTimeJumped_;

  debug().update(mount_, *this);
}

// TODO: actually one could outsource this to a "Last time jumped provider"
void FakeRobotPoseProvider::updateLastTimeJumped()
{
  // Calculate the last time jumped
  const float jumpDistThreshSquared = 0.5f * 0.5f;
  const float jumpAngleThresh = 30 * TO_RAD;
  if ((pose_.position - lastPose_.position).squaredNorm() > jumpDistThreshSquared ||
      Angle::angleDiff(pose_.orientation, lastPose_.orientation) > jumpAngleThresh)
  {
    lastTimeJumped_ = cycleInfo_->startTime;
  }

  lastPose_ = pose_;
}

void FakeRobotPoseProvider::toValue(Uni::Value& value) const
{
  /**
   * TODO: Debug fake data here for fake data reference (e.g. ground truth)
   */
  value = Uni::Value(Uni::ValueType::OBJECT);
  value["pose"] << pose_;
}
