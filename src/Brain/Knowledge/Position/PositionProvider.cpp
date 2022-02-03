#include "Brain/Knowledge/Position/PositionProvider.hpp"
#include "Tools/Math/Angle.hpp"
#include "Tools/Math/Random.hpp"

PositionProvider::PositionProvider(const ModuleBase& module, const FieldInfo& fieldInfo,
                                   const GameControllerState& gameControllerState,
                                   const PlayerConfiguration& playerConfiguration,
                                   const LandmarkModel& landmarkModel,
                                   const FieldDimensions& fieldDimensions)
  : sigmaInitial_(module, "sigmaInitial", [] {})
  , sigmaPenalized_(module, "sigmaPenalized", [] {})
  , startAnywhereAtSidelines_(module, "startAnywhereAtSidelines", [] {})
  , maxNumberOfHypotheses_(module, "maxNumberOfHypotheses", [] {})
  , fieldInfo_(fieldInfo)
  , gameControllerState_(gameControllerState)
  , playerConfiguration_(playerConfiguration)
  , landmarkModel_(landmarkModel)
  , fieldDimensions_(fieldDimensions)
  , hypothesesCounter_(0)
{
  (void)fieldInfo_; // This variable will be used for pose computation from landmark combinations.
}

void PositionProvider::resetHypothesesCounter() const
{
  hypothesesCounter_ = 0;
}

Pose PositionProvider::getOnField() const
{
  // These are actually half of the field size.
  float fieldLength = fieldDimensions_.fieldLength * 0.5f;
  float fieldWidth = fieldDimensions_.fieldWidth * 0.5f;
  return {Random::uniformFloat(-fieldLength, fieldLength),
          Random::uniformFloat(-fieldWidth, fieldWidth),
          Random::uniformFloat(0, 2 * static_cast<float>(M_PI))};
}

Pose PositionProvider::getInitial(unsigned int& clusterHint, const bool addNoise) const
{
  Pose pose;
  // Odd player numbers (1, 3, 5) are placed at the left sideline.
  const bool leftSideline = playerConfiguration_.playerNumber % 2;
  pose.y() = (leftSideline ? 1 : -1) * fieldDimensions_.fieldWidth * 0.5f;
  pose.angle() = (leftSideline ? -1 : 1) * M_PI / 2;
  if (startAnywhereAtSidelines_())
  {
    if (addNoise)
    {
      // if we want to add noise we spread the particles all over the sideline
      pose.x() = Random::uniformFloat(-fieldDimensions_.fieldLength * 0.5f, 0);
    }
    else
    {
      // if we sample without noise we draw 8 equally distributed posees from the players side line
      const float xFraction = fieldDimensions_.fieldLength * 0.5f / (maxNumberOfHypotheses_() + 1);
      const int hypothesisID = hypothesesCounter_ % maxNumberOfHypotheses_();
      pose.x() = -xFraction * (1 + hypothesisID);
    }
  }
  else
  {
    assert(playerConfiguration_.playerNumber > 0);
    assert(playerConfiguration_.playerNumber <= playerConfiguration_.initialPoses.size());
    pose.x() = playerConfiguration_.initialPoses[playerConfiguration_.playerNumber - 1];
    clusterHint = 0;
  }
  return addNoise ? addGaussianNoise(pose, sigmaInitial_()) : pose;
}

Pose PositionProvider::getPenalized(unsigned int& clusterHint, const bool addNoise) const
{
  // By default the pose on the left border is returned.
  Pose pose(fieldDimensions_.fieldPenaltyMarkerDistance - fieldDimensions_.fieldLength * 0.5f,
            fieldDimensions_.fieldWidth * 0.5f + 0.2f, -M_PI / 2);
  // In approximately every second call the side is flipped.
  if ((hypothesesCounter_++ % 2) == 0u)
  {
    pose.y() = -pose.y();
    pose.angle() = -pose.angle();
    clusterHint = 0;
  }
  else
  {
    clusterHint = 1;
  }
  return addNoise ? addGaussianNoise(pose, sigmaPenalized_()) : pose;
}

Pose PositionProvider::getManuallyPlaced(unsigned int& clusterHint, const bool addNoise) const
{
  Pose pose;
  if (playerConfiguration_.playerNumber == 1)
  {
    // The keeper is always the player with number 1 and placed at a special pose.
    pose.x() = -fieldDimensions_.fieldLength * 0.5f;
    pose.y() = 0.f;
    pose.angle() = 0.f;
    clusterHint = 0;
  }
  else
  {
    pose.x() = -fieldDimensions_.fieldLength * 0.5f + fieldDimensions_.fieldPenaltyMarkerDistance;
    const float betweenPenaltyBoxAndFieldBorder =
        ((fieldDimensions_.fieldWidth * 0.5f) + (fieldDimensions_.fieldPenaltyAreaWidth * 0.5f)) *
        0.5f;
    pose.angle() = 0.f;
    // If the team has kickoff there is a chance to be placed in front of the center circle.
    unsigned int pos = hypothesesCounter_++ % (gameControllerState_.kickingTeam ? 5 : 4);
    switch (pos)
    {
      case 0:
        // on the penalty marker
        pose.y() = 0.f;
        break;
      case 1:
        // outside the penalty area on the height of the penalty marker
        pose.y() = betweenPenaltyBoxAndFieldBorder;
        break;
      case 2:
        // outside the penalty area on the height of the penalty marker
        pose.y() = -betweenPenaltyBoxAndFieldBorder;
        break;
      case 3:
        // in front of the penalty area
        pose.x() =
            -fieldDimensions_.fieldLength * 0.5f + fieldDimensions_.fieldPenaltyAreaLength + 0.1f;
        pose.y() = 0.f;
        break;
      case 4:
        // This branch can only be taken if the team has kickoff.
        pose.x() = -fieldDimensions_.fieldCenterCircleDiameter * 0.5f - 0.1f;
        pose.y() = 0.f;
        break;
    }
    clusterHint = pos;
  }
  return addNoise ? addGaussianNoise(pose, sigmaInitial_()) : pose;
}

Pose PositionProvider::getPenaltyShootout(unsigned int& clusterHint,
                                          const bool multiPenaltyShootoutPositions,
                                          const bool addNoise) const
{
  Pose pose;
  const Vector2f penaltyMarkerPos(
      fieldDimensions_.fieldLength * 0.5f - fieldDimensions_.fieldPenaltyMarkerDistance, 0.f);
  // new rules 2018 - different positions where shooter could be positioned.
  const float distanceToPenaltyMarker = 1.f;
  const int numberOfPositions = 6;
  // these are the orientations the penalty shooter can be placed
  // the 0-orientation is included twice to model the actual probability distribution of the
  // orientations.
  // First two must be zero!! Don't change order!
  const std::array<float, numberOfPositions> penaltyOrientation = {{0, 0, -60, -30, 30, 60}};
  const unsigned int pos =
      multiPenaltyShootoutPositions ? hypothesesCounter_++ % numberOfPositions : 0;
  if (gameControllerState_.kickingTeam)
  {
    pose.angle() = penaltyOrientation[pos] * TO_RAD;
    pose.x() = penaltyMarkerPos.x() - distanceToPenaltyMarker * std::cos(pose.angle());
    pose.y() = penaltyMarkerPos.y() - distanceToPenaltyMarker * std::sin(pose.angle());
  }
  else
  {
    pose.x() = -fieldDimensions_.fieldLength * 0.5f;
    pose.y() = 0;
    pose.angle() = 0;
  }
  // the cluster hint of the first two positions is mapped to the same id since all particles
  // sampled from 0-orienation are considered one cluster (twice as large as all others due to
  // probability distribution)
  clusterHint = pos <= 1 ? 0 : pos - 1;
  return addNoise ? addGaussianNoise(pose, sigmaInitial_()) : pose;
}

Pose PositionProvider::getEventPose(unsigned int& clusterHint, const bool ownHalf,
                                    const bool leftHalf) const
{
  Pose pose;
  pose.x() = -0.5f * fieldDimensions_.fieldLength + fieldDimensions_.fieldPenaltyMarkerDistance;
  pose.y() = 0.5f * fieldDimensions_.fieldWidth;
  pose.angle() = -0.5f * M_PI;
  if (!ownHalf)
  {
    pose.x() *= -1.f;
  }
  if (!leftHalf)
  {
    pose.y() *= -1.f;
    pose.angle() *= -1.f;
  }
  clusterHint = 0;
  return pose;
}

bool PositionProvider::isSensorResettingAvailable() const
{
  return !landmarkModel_.goals.empty() || circleWithOrientationAvailable();
}

bool PositionProvider::circleWithOrientationAvailable() const
{
  for (auto& centerCircle : landmarkModel_.centerCircles)
  {
    if (centerCircle.hasOrientation)
    {
      return true;
    }
  }
  return false;
}

Pose PositionProvider::getSensorResetting() const
{
  if (circleWithOrientationAvailable())
  {
    for (auto& circle : landmarkModel_.centerCircles)
    {
      if (circle.hasOrientation)
      {
        Pose relativeCircleCenterObservation = Pose(circle.position, circle.orientation);
        return relativeCircleCenterObservation.inverse();
      }
    }
  }

  if (!landmarkModel_.goals.empty())
  {
    Pose resettingPose;
    // If there are multiple goals seen, each one can be used.
    const LandmarkModel::Goal& goal =
        landmarkModel_.goals[hypothesesCounter_++ % landmarkModel_.goals.size()];
    Vector2f span = goal.left - goal.right;
    Vector2f center = (goal.left + goal.right) / 2;
    // The orientation can be derived from the direction between the posts.
    // std::atan2(-span.x, span.y) is the angle of the vector (span.y, -span.x) which is span
    // rotated by 90 degrees counterclockwise.
    resettingPose.angle() = -std::atan2(-span.x(), span.y());
    // -center is the position of the robot relative to the goal center, but it has to be rotated to
    // be in the SPL reference frame.
    resettingPose.position() = resettingPose * (-center);
    // resettingPose is now relative to the center of the goal, thus its position is added.
    resettingPose.x() += fieldDimensions_.fieldLength * 0.5f;
    // This function always returns the resettingPose that assumes that the goal is the opponent
    // goal.
    return resettingPose;
  }

  assert(false);
  return Pose();
}

Pose PositionProvider::addGaussianNoise(const Pose& pose, const Vector3f& standardDeviation) const
{
  return {Random::gaussianFloat(pose.x(), standardDeviation.x()),
          Random::gaussianFloat(pose.y(), standardDeviation.y()),
          Random::gaussianFloat(pose.angle(), standardDeviation.z())};
}
