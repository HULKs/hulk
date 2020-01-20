#include "Tools/Chronometer.hpp"
#include "Tools/Kinematics/ForwardKinematics.h"
#include "Tools/Math/Angle.hpp"

#include "HeadPositionProvider.hpp"


HeadPositionProvider::HeadPositionProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , ballState_(*this)
  , gameControllerState_(*this)
  , teamBallModel_(*this)
  , robotPosition_(*this)
  , headMotionOutput_(*this)
  , cycleInfo_(*this)
  , headPositionData_(*this)
  , timeToRest_(*this, "timeToRest", [] {})
  , yawMax_(
        *this, "yawMax", [] {},
        [this]() { return gameControllerState_->gamePhase != GamePhase::PENALTYSHOOT; })
  , keepTargetOnImageMaxAngle_(*this, "keepTargetOnImageMaxAngle",
                               [this] { keepTargetOnImageMaxAngle_() *= TO_RAD; })
  , targetPositionTolerance_(*this, "targetPositionTolerance", [] {})
  , lookAroundPitch_(*this, "lookAroundPitch", [this] { lookAroundPitch_() *= TO_RAD; })
{
  keepTargetOnImageMaxAngle_() *= TO_RAD;
  lookAroundPitch_() *= TO_RAD;
}

void HeadPositionProvider::cycle()
{
  Chronometer time(debug(), mount_ + ".cycleTime");
  headPositionData_->lookAroundHeadPosition = calculateLookAroundHeadPositions(yawMax_(), 0.f);
  headPositionData_->lookAroundBallHeadPosition = calculateLookAroundBallHeadPositions();
}


HeadPosition HeadPositionProvider::calculateLookAroundBallHeadPositions()
{
  // Keep head aligned with body. Don't look over the shoulders.
  HeadPosition headPosition;
  const Vector2f relBallPos = robotPosition_->fieldToRobot(teamBallModel_->position);
  const float relativeBallAngle = atan2(relBallPos.y(), relBallPos.x());
  const float relativeBallDistanceSquared = relBallPos.squaredNorm();
  // Lock in ball if it is up to 2 meters away
  if (std::abs(relativeBallAngle) < 45 * TO_RAD && relativeBallDistanceSquared < 2.f * 2.f &&
      (teamBallModel_->seen || ballState_->found))
  {
    // Ball remains visible on the image
    headPosition =
        calculateLookAroundHeadPositions(keepTargetOnImageMaxAngle_(), relativeBallAngle);
  }
  else
  {
    headPosition = calculateLookAroundHeadPositions(45.f * TO_RAD, 0.f);
  }
  return headPosition;
}

HeadPosition HeadPositionProvider::calculateLookAroundHeadPositions(float yawMax, float angle)
{
  HeadPosition headPosition;
  // check if the angles are within the boundaries
  innerPosition_.yaw = angle;
  outerPositionLeft_.yaw = std::min(angle + yawMax, yawMax_());
  outerPositionRight_.yaw = std::max(angle - yawMax, -yawMax_());

  auto timeDiff = cycleInfo_->getTimeDiff(headMotionOutput_->timeWhenReachedTarget);
  switch (nextLookAroundState_)
  {
    case INITIAL:
      lastLookAroundState_ = INITIAL;
      nextLookAroundState_ = GOING_LEFT;
      break;
    case GOING_LEFT:
      lastLookAroundState_ = GOING_LEFT;
      headPosition = outerPositionLeft_;
      if (std::abs(headMotionOutput_->target[0] - outerPositionLeft_.yaw) <
          targetPositionTolerance_())
      {
        if (timeDiff > timeToRest_())
        {
          nextLookAroundState_ = GOING_MIDDLE;
        }
      }
      else
      {
        nextLookAroundState_ = lastLookAroundState_;
      }
      break;
    case GOING_MIDDLE:
      lastLookAroundState_ = GOING_MIDDLE;
      headPosition = innerPosition_;
      if (std::abs(headMotionOutput_->target[0] - innerPosition_.yaw) < targetPositionTolerance_())
      {
        if (timeDiff > timeToRest_())
        {
          nextLookAroundState_ = GOING_RIGHT;
        }
      }
      else
      {
        nextLookAroundState_ = lastLookAroundState_;
      }
      break;
    case GOING_RIGHT:
      lastLookAroundState_ = GOING_RIGHT;
      headPosition = outerPositionRight_;
      if (std::abs(headMotionOutput_->target[0] - outerPositionRight_.yaw) <
          targetPositionTolerance_())
      {
        if (timeDiff > timeToRest_())
        {
          nextLookAroundState_ = GOING_LEFT;
        }
      }
      else
      {
        nextLookAroundState_ = lastLookAroundState_;
      }
      break;
    default:
      nextLookAroundState_ = INITIAL;
      break;
  }
  headPosition.pitch = lookAroundPitch_();
  return headPosition;
}
