#include "Motion/StepPlanning/StepPlanner.hpp"
#include "Framework/Log/Log.hpp"
#include "Tools/Math/Geometry.hpp"
#include "Tools/Math/Hysteresis.hpp"

StepPlanner::StepPlanner(const ModuleManagerInterface& manager)
  : Module(manager)
  , actionCommand_(*this)
  , bodyPose_(*this)
  , robotPosition_(*this)
  , pathPlannerData_(*this)
  , cycleInfo_(*this)
  , teamObstacleData_(*this)
  , teamBallModel_(*this)
  , dribbleData_(*this)
  , walkGeneratorOutput_(*this)
  , insideTurnRatio_{*this, "insideTurnRatio", [] {}}
  , maxForwardAcceleration_(*this, "maxForwardAcceleration", [] {})
  , walkVolumeTranslationExponent_(*this, "walkVolumeTranslationExponent", [] {})
  , walkVolumeRotationExponent_(*this, "walkVolumeRotationExponent", [] {})
  , maxStepSize_{*this, "maxStepSize", [this] { maxStepSize_().angle() *= TO_RAD; }}
  , maxStepSizeBackwards_{*this, "maxStepSizeBackwards", [] {}}
  , stepPlan_(*this)
{
  maxStepSize_().angle() *= TO_RAD;
}

void StepPlanner::cycle()
{
  // export config values
  stepPlan_->maxStepSize = maxStepSize_();

  if (actionCommand_->body().type != ActionCommand::Body::MotionType::WALK ||
      !walkGeneratorOutput_->valid)
  {
    // Brain does not want to walk
    return;
  }
  // Brain requested to walk --> StepPlanner has to plan

  // compute the actual planned request
  Pose request = calculateNextPose();
  debug().update(mount_ + ".targetPosition", request.position());

  // compensate with the distance the robot is walking anyways when stopping in this step
  if (!dribbleData_->isDribbling)
  {
    request = compensateWithReturnOffset(request);
  }

  // calculate the step size to request
  request = clampStepToWalkVolume(maxStepSize_(), maxStepSizeBackwards_(), request);

  assert(calculateWalkVolume(request.x(), request.y(), request.angle(), maxStepSize_().x(),
                             maxStepSizeBackwards_(), maxStepSize_().y(),
                             maxStepSize_().angle()) <= 1.0001f);

  request = clampAcceleration(request);

  // publish request

  request = clampToAnatomicConstraints(request);

  stepPlan_->forward = request.x();
  stepPlan_->left = request.y();
  stepPlan_->turn = request.angle();
  stepPlan_->valid = true;
}


Pose StepPlanner::calculateNextPose() const
{
  const auto& walkMode = actionCommand_->body().walkMode;
  switch (walkMode)
  {
    case ActionCommand::Body::WalkMode::DRIBBLE:
      if (dribbleData_->valid)
      {
        return dribbleData_->stepRequest;
      }
      Log<M_MOTION>(LogLevel::ERROR)
          << "Requested DRIBBLE walk mode from StepPlanner, but DribbleData is not valid";
      return Pose{0.f, 0.f, 0.f};
    case ActionCommand::Body::WalkMode::DIRECT:
      [[fallthrough]];
    case ActionCommand::Body::WalkMode::DIRECT_WITH_ORIENTATION:
      [[fallthrough]];
    case ActionCommand::Body::WalkMode::PATH:
      [[fallthrough]];
    case ActionCommand::Body::WalkMode::PATH_WITH_ORIENTATION:
      [[fallthrough]];
    case ActionCommand::Body::WalkMode::WALK_BEHIND_BALL:
      return pathPlannerData_->nextRelativePathPose;
    case ActionCommand::Body::WalkMode::VELOCITY:
      assert(false && "WalkMode VELOCITY is not implemented");
      return Pose{};
  }
  assert(false && "Uncatched walk mode in calculateTranslation");
  return Pose{};
}

Pose StepPlanner::compensateWithReturnOffset(const Pose& request) const
{
  return Pose(request.x() - walkGeneratorOutput_->returnOffset.x(),
              request.y() - walkGeneratorOutput_->returnOffset.y(),
              request.angle() - walkGeneratorOutput_->returnOffset.angle());
}

Pose StepPlanner::clampAcceleration(const Pose& request) const
{
  // accelerate maximally with maxAcceleration
  const float lastForward = walkGeneratorOutput_->requestedStepOffsets.x();
  const float forwardDiff = request.x() - lastForward;
  const float forward = lastForward + std::min(forwardDiff, maxForwardAcceleration_());
  return Pose(forward, request.y(), request.angle());
}

float StepPlanner::calculateWalkVolume(const float forward, const float left, const float turn,
                                       const float maxForward, const float maxBackwards,
                                       const float maxSideways, const float maxTurn) const
{
  const bool walkingForward = forward >= 0.f;
  const float x = forward / (walkingForward ? maxForward : maxBackwards);
  const float y = left / maxSideways;
  const float angle = turn / maxTurn;
  assert(std::abs(angle) <= 1.f);
  return std::pow(std::pow(std::abs(x), walkVolumeTranslationExponent_()) +
                      std::pow(std::abs(y), walkVolumeTranslationExponent_()),
                  (walkVolumeRotationExponent_() / walkVolumeTranslationExponent_())) +
         std::pow(std::abs(angle), walkVolumeRotationExponent_());
}

Vector2f StepPlanner::calculateMaxStepSizeInWalkVolume(const float forward, const float left,
                                                       const float turn, const float maxForward,
                                                       const float maxBackwards,
                                                       const float maxSideways,
                                                       const float maxTurn) const
{
  const bool walkingForward = forward >= 0.f;
  const float x = forward / (walkingForward ? maxForward : maxBackwards);
  const float y = left / maxSideways;
  const float angle = turn / maxTurn;
  assert(std::abs(angle) <= 1.f);
  const float scale =
      std::pow(std::pow(1 - std::pow(std::abs(angle), walkVolumeRotationExponent_()),
                        walkVolumeTranslationExponent_() / walkVolumeRotationExponent_()) /
                   (std::pow(std::abs(x), walkVolumeTranslationExponent_()) +
                    std::pow(std::abs(y), walkVolumeTranslationExponent_())),
               1.f / walkVolumeTranslationExponent_());
  return {forward * scale, left * scale};
}

Pose StepPlanner::clampStepToWalkVolume(const Pose& maxStepSize, const float maxStepSizeBackwards,
                                        const Pose& targetPose) const
{
  // Values in range [-1..1]
  const float clampedTurn =
      std::clamp(targetPose.angle(), -maxStepSize.angle(), maxStepSize.angle());
  if (calculateWalkVolume(targetPose.x(), targetPose.y(), clampedTurn, maxStepSize.x(),
                          maxStepSizeBackwards, maxStepSize.y(), maxStepSize.angle()) <= 1.f)
  {
    return Pose(targetPose.position(), clampedTurn);
  }
  // the step has to be scaled to the ellipse
  const Vector2f scaledTranslation =
      calculateMaxStepSizeInWalkVolume(targetPose.x(), targetPose.y(), clampedTurn, maxStepSize.x(),
                                       maxStepSizeBackwards, maxStepSize.y(), maxStepSize.angle());
  return Pose(scaledTranslation, clampedTurn);
}


Pose StepPlanner::clampToAnatomicConstraints(const Pose& request)
{
  Pose clampedRequest{request};
  const bool isLeftSwingFoot{bodyPose_->supportSide < 0.f};
  clampedRequest.y() = std::signbit(request.y()) == isLeftSwingFoot ? 0.f : request.y();
  const float turnRatio{std::signbit(request.angle()) == isLeftSwingFoot
                            ? insideTurnRatio_()
                            : 1.f - insideTurnRatio_()};
  clampedRequest.angle() = turnRatio * request.angle();
  return clampedRequest;
}
