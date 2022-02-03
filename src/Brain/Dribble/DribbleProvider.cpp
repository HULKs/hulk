#include "Brain/Dribble/DribbleProvider.hpp"
#include "Framework/Log/Log.hpp"
#include "Tools/Math/Geometry.hpp"

DribbleProvider::DribbleProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , actionCommand_(*this)
  , teamBallModel_(*this)
  , pathPlannerData_(*this)
  , dribbleAngleTolerance_(*this, "dribbleAngleTolerance",
                           [this] { dribbleAngleTolerance_() *= TO_RAD; })
  , dribbleSpeed_(*this, "dribbleSpeed", [] {})
  , maxDistanceToDribbleLine_(*this, "maxDistanceToDribbleLine", [] {})
  , maxDistanceToDribblePosition_(*this, "maxDistanceToDribblePosition", [] {})
  , dribbleData_(*this)
{
  dribbleAngleTolerance_() *= TO_RAD;
}

void DribbleProvider::cycle()
{
  if (actionCommand_->body().walkMode != ActionCommand::Body::WalkMode::DRIBBLE)
  {
    return;
  }
  if (isDribbleTargetReached())
  {
    dribbleData_->isDribbling = true;
    dribbleData_->stepRequest = Pose{dribbleSpeed_(), 0.f, 0.f};
  }
  else
  {
    dribbleData_->stepRequest = pathPlannerData_->nextRelativePathPose;
  }
  dribbleData_->valid = true;
}

bool DribbleProvider::isDribbleTargetReached()
{
  const Pose& walkTarget = actionCommand_->body().walkTarget;
  const Vector2f relBallPos = teamBallModel_->relPosition;
  const Vector2f& relBallTarget = actionCommand_->body().ballTarget;
  // distance of robot to line between ball source and target
  const float distToBallTargetLine =
      Geometry::distPointToLine(relBallPos, relBallTarget, {0.f, 0.f});
  wasDribbleTargetReachedLastCycle_ =
      Hysteresis::smallerThan(walkTarget.position().norm(), maxDistanceToDribblePosition_(), 0.03f,
                              wasDribbleTargetReachedLastCycle_) &&
      Hysteresis::smallerThan(std::abs(walkTarget.angle()), dribbleAngleTolerance_(), 5.f * TO_RAD,
                              wasDribbleTargetReachedLastCycle_) &&
      Hysteresis::smallerThan(distToBallTargetLine, maxDistanceToDribbleLine_(), 0.03f,
                              wasDribbleTargetReachedLastCycle_);
  return wasDribbleTargetReachedLastCycle_;
}
