#include "Brain/Knowledge/Position/FakeRobotPoseProvider.hpp"
#include "Tools/Math/Angle.hpp"
#include <array>


FakeRobotPoseProvider::FakeRobotPoseProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , mirrorFakePose_(*this, "mirrorFakePose", [] {})
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
  if (mirrorFakePose_())
  {
    fakeRobotPose_->pose.angle() += 180 * TO_RAD;
    fakeRobotPose_->pose.x() = -fakeRobotPose_->pose.x();
    fakeRobotPose_->pose.y() = -fakeRobotPose_->pose.y();
  }
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
  if ((pose_.position() - lastPose_.position()).squaredNorm() > jumpDistThreshSquared ||
      Angle::angleDiff(pose_.angle(), lastPose_.angle()) > jumpAngleThresh)
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
