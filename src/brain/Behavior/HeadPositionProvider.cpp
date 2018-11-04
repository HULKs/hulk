#include "Tools/Chronometer.hpp"
#include "Tools/Kinematics/ForwardKinematics.h"
#include "Tools/Math/Angle.hpp"

#include "HeadPositionProvider.hpp"


HeadPositionProvider::HeadPositionProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , ballState_(*this)
  , teamBallModel_(*this)
  , robotPosition_(*this)
  , fieldDimensions_(*this)
  , headMotionOutput_(*this)
  , jointSensorData_(*this)
  , cycleInfo_(*this)
  , headPositionData_(*this)
  , absolutePOIs_()
  , lastLookAroundState_(INITIAL)
  , nextLookAroundState_(INITIAL)
  , timeToRest_(*this, "timeToRest", [] {})
  , yawMax_(*this, "yawMax", [this] { yawMax_() *= TO_RAD; })
  , keepTargetOnImageMaxAngle_(*this, "keepTargetOnImageMaxAngle",
                               [this] { keepTargetOnImageMaxAngle_() *= TO_RAD; })
  , targetPositionTolerance_(*this, "targetPositionTolerance", [] {})
{
  yawMax_() *= TO_RAD;
  keepTargetOnImageMaxAngle_() *= TO_RAD;

  fillInterestingLocalizationPoints();

  outerPositionLeft_ = HeadPosition();
  outerPositionRight_ = HeadPosition();
  innerPosition_ = HeadPosition();
}

void HeadPositionProvider::fillInterestingLocalizationPoints()
{
  // For now simply fill this with some manually selected points
  // the center point
  absolutePOIs_.emplace_back(0.f, 0.f, 1.f);

  // the own penalty box
  absolutePOIs_.emplace_back(-fieldDimensions_->fieldLength * 0.5f +
                                 fieldDimensions_->fieldPenaltyAreaLength,
                             fieldDimensions_->fieldPenaltyAreaWidth * 0.5, 1.f);
  absolutePOIs_.emplace_back(-fieldDimensions_->fieldLength * 0.5f +
                                 fieldDimensions_->fieldPenaltyAreaLength,
                             -fieldDimensions_->fieldPenaltyAreaWidth * 0.5, 1.f);
  // the opponents penalty box
  absolutePOIs_.emplace_back(fieldDimensions_->fieldLength * 0.5f -
                                 fieldDimensions_->fieldPenaltyAreaLength,
                             fieldDimensions_->fieldPenaltyAreaWidth * 0.5, 1.f);
  absolutePOIs_.emplace_back(fieldDimensions_->fieldLength * 0.5f -
                                 fieldDimensions_->fieldPenaltyAreaLength,
                             -fieldDimensions_->fieldPenaltyAreaWidth * 0.5, 1.f);
  // T intersection
  absolutePOIs_.emplace_back(0, fieldDimensions_->fieldWidth * 0.5f, 1.f);
  absolutePOIs_.emplace_back(0, -fieldDimensions_->fieldWidth * 0.5f, 1.f);
  // own penalty spot
  absolutePOIs_.emplace_back(
      -fieldDimensions_->fieldLength * 0.5 + fieldDimensions_->fieldPenaltyMarkerDistance, 0, 1.f);
  // opponent's penalty spot
  absolutePOIs_.emplace_back(
      fieldDimensions_->fieldLength * 0.5 - fieldDimensions_->fieldPenaltyMarkerDistance, 0, 1.f);
}

void HeadPositionProvider::createSampleHeadPositions(const int sampleSize,
                                                     const HeadPosition middleHeadPosition,
                                                     const float yawMax,
                                                     std::vector<HeadPosition>& sampleHeadPositions)
{
  sampleHeadPositions.resize(sampleSize);
  sampleHeadPositions[0] = middleHeadPosition;

  int samplesPerSide = (sampleSize - 1) / 2;
  for (int i = 1; i <= samplesPerSide; i++)
  {
    // left samples
    sampleHeadPositions[i].pitch = 0.f;
    sampleHeadPositions[i].yaw = middleHeadPosition.yaw + yawMax / samplesPerSide * i;
    // right sample
    sampleHeadPositions[i + samplesPerSide].pitch = 0.f;
    sampleHeadPositions[i + samplesPerSide].yaw =
        middleHeadPosition.yaw - yawMax / samplesPerSide * i;
  }
}

HeadPosition
HeadPositionProvider::evaluateHeadPositions(std::vector<HeadPosition>& sampleHeadPositions,
                                            HeadPosition& bestHeadPosition)
{
  HeadPosition currentHeadPosition = jointSensorData_->getHeadHeadPosition();
  int sampleSize = sampleHeadPositions.size();
  int bestHeadPositionIndex = 0;
  for (auto& absoluteInterestingPoint : absolutePOIs_)
  {
    auto relativeInterestingPoint =
        PointOfInterest(robotPosition_->fieldToRobot(absoluteInterestingPoint.position),
                        absoluteInterestingPoint.weight);

    for (auto& position : sampleHeadPositions)
    {
      calculateScore(relativeInterestingPoint, position);
    }
    calculateScore(relativeInterestingPoint, currentHeadPosition);
  }

  for (int i = 0; i < sampleSize; ++i)
  {
    if (sampleHeadPositions[i].score > sampleHeadPositions[bestHeadPositionIndex].score)
    {
      bestHeadPositionIndex = i;
    }
  }
  bestHeadPosition = sampleHeadPositions[bestHeadPositionIndex];
  return currentHeadPosition;
}

void HeadPositionProvider::cycle()
{
  Chronometer time(debug(), mount_ + ".cycleTime");
  // Do not change order. calculateBallAndLocalizationHeadPosition() may return
  // lookAroundHeadPosition
  headPositionData_->lookAroundHeadPosition = calculateLookAroundHeadPositions();
  headPositionData_->localizationHeadPosition = calculateLocalizationHeadPosition();
  headPositionData_->ballAndLocalizationHeadPosition = calculateBallAndLocalizationHeadPosition();
  headPositionData_->trackBallHeadPosition = calculateBallTrackHeadPosition();
  headPositionData_->lookAroundBallHeadPosition = calculateLookAroundBallHeadPositions();
}

HeadPosition HeadPositionProvider::calculateBallAndLocalizationHeadPosition()
{
  if (teamBallModel_->seen || ballState_->found)
  {
    const Vector2f relBallPos = robotPosition_->fieldToRobot(teamBallModel_->position);
    const float relativeBallAngle = atan2(relBallPos.y(), relBallPos.x());
    const float relativeBallDistanceSquared = relBallPos.squaredNorm();
    if (std::abs(relativeBallAngle) < 60.f * TO_RAD ||               // within FOV
        teamBallModel_->ballType == TeamBallModel::BallType::SELF || // found ball by itself
        (std::abs(relativeBallAngle) < yawMax_() &&                  // can be seen with head motion
         relativeBallDistanceSquared < 2.f * 2.f))
    {
      const int sampleSize = 9;
      assert(sampleSize > 1 && sampleSize % 2 != 0);

      std::vector<HeadPosition> sampleHeadPositions;

      createSampleHeadPositions(sampleSize, HeadPosition(relativeBallAngle, 0.f),
                                keepTargetOnImageMaxAngle_(), sampleHeadPositions);

      HeadPosition bestHeadPosition;
      evaluateHeadPositions(sampleHeadPositions, bestHeadPosition);
      if (bestHeadPosition.score < 0.5)
      {
        return HeadPosition(relativeBallAngle, 0.f);
      }
      else
      {
        return bestHeadPosition;
      }
    }
  }
  // otherwise only look around
  return headPositionData_->lookAroundHeadPosition;
}


HeadPosition HeadPositionProvider::calculateLocalizationHeadPosition()
{
  const int sampleSize = 9;
  assert(sampleSize > 1 && sampleSize % 2 != 0);

  // Sample some head positions
  std::vector<HeadPosition> sampleHeadPositions;
  createSampleHeadPositions(sampleSize, HeadPosition(0.f, 0.f), yawMax_(), sampleHeadPositions);

  HeadPosition bestHeadPosition;
  HeadPosition currentHeadPosition = evaluateHeadPositions(sampleHeadPositions, bestHeadPosition);

  // hysteresis: bestHeadPosition needs to be 10% higher than the current position
  if (bestHeadPosition.score <= 1.1f * currentHeadPosition.score)
  {
    return currentHeadPosition;
  }
  return bestHeadPosition;
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
    headPosition = calculateLookAroundHeadPositions(45 * TO_RAD);
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
  return headPosition;
}

HeadPosition HeadPositionProvider::calculateBallTrackHeadPosition()
{
  HeadPosition headPosition;
  // look at the ball if it can be seen
  if (teamBallModel_->seen || ballState_->found)
  {
    const Vector2f relBallPos = robotPosition_->fieldToRobot(teamBallModel_->position);
    const float relativeBallAngleAbs = std::abs(atan2(relBallPos.y(), relBallPos.x()));
    const float relativeBallDistanceSquared = relBallPos.squaredNorm();
    if (relativeBallAngleAbs < 60.f * TO_RAD ||                      // within FOV
        teamBallModel_->ballType == TeamBallModel::BallType::SELF || // found ball by itself
        (relativeBallAngleAbs < yawMax_() &&                         // can be seen with head motion
         relativeBallDistanceSquared < 2.f * 2.f))
    {

      float yawBall = atan2(relBallPos.y(), relBallPos.x());
      headPosition = HeadPosition(yawBall, 15.f);
    }
  }
  // otherwise just look forward
  else
  {
    headPosition = HeadPosition();
  }
  return headPosition;
}

void HeadPositionProvider::sendDebug(HeadPosition& chosenHeadPosition)
{
  std::vector<Vector2f> pointsThatCanBeSeen;
  for (auto& absoluteInterestingPoint : absolutePOIs_)
  {
    auto relativeInterestingPoint =
        PointOfInterest(robotPosition_->fieldToRobot(absoluteInterestingPoint.position),
                        absoluteInterestingPoint.weight);
    if (calculateScore(relativeInterestingPoint, chosenHeadPosition))
    {
      pointsThatCanBeSeen.push_back(absoluteInterestingPoint.position);
    }
  }
  debug().update(mount_ + ".PointsThatCanBeSeen", pointsThatCanBeSeen);
  debug().update(mount_ + ".HeadPositionScore", chosenHeadPosition.score);
}

bool HeadPositionProvider::calculateScore(const PointOfInterest& relativePosition, HeadPosition& hp)
{
  bool isValid = false;
  float squaredDistance = relativePosition.position.squaredNorm();
  float targetAngle = std::atan2(relativePosition.position.y(), relativePosition.position.x());
  float yawDiffToTargetAngle = std::abs(targetAngle - hp.yaw);
  if (yawDiffToTargetAngle < 30 * TO_RAD && squaredDistance < (3.f * 3.f))
  {
    isValid = true;
    hp.score += relativePosition.weight * (1 - 0.25f * (yawDiffToTargetAngle / 30 * TO_RAD) -
                                           0.25f * (squaredDistance / (3.f * 3.f)) -
                                           0.5f * std::abs((hp.yaw / (120 * TO_RAD))));
  }
  return isValid;
}
