#include "Tools/Chronometer.hpp"
#include "Tools/Math/Geometry.hpp"

#include "Brain/Knowledge/LandmarkFilter/LandmarkFilter.hpp"

LandmarkFilter::LandmarkFilter(const ModuleManagerInterface& manager)
  : Module(manager)
  , ballCreatesPenaltySpotInPSO_(*this, "ballCreatesPenaltySpotInPSO", [] {})
  , bufferGoalPosts_(*this, "bufferGoalPosts", [] {})
  , maxGoalPostDistanceDeviation_(*this, "maxGoalPostDistanceDeviation", [] {})
  , maxGoalPostAge_(*this, "maxGoalPostAge", [] {})
  , goalPostAssociationRadius_(*this, "goalPostAssociationRadius", [] {})
  , maxLineLengthForCircleFraction_(*this, "maxLineLengthForCircleFraction",
                                    [this]() {
                                      maxLineLengthForCircle_ =
                                          maxLineLengthForCircleFraction_() *
                                          fieldDimensions_->fieldCenterCircleDiameter / 2.0f;
                                    })
  , minLineLengthForCircle_(*this, "minLineLengthForCircle", [] {})
  , maxLineProjectionDistanceForCircle_(*this, "maxLineProjectionDistanceForCircle", [] {})
  , minLineCountClusterable_(*this, "minLineCountClusterable", [] {})
  , minCountPointsInCluster_(*this, "minCountPointsInCluster", [] {})
  , maxDistToCircleCluster_(*this, "maxDistToCircleCluster",
                            [this] {
                              squaredMaxDistToCircleCluster_ =
                                  maxDistToCircleCluster_() * maxDistToCircleCluster_();
                            })
  , orthogonalTolerance_(*this, "orthogonalTolerance", [] {})
  , minLineLengthForCircleOrientation_(*this, "minLineLengthForCircleOrientation", [] {})
  , maxDistToCenterLineForCircleOrientation_(*this, "maxDistToCenterLineforCircleOrientation",
                                             [this] {
                                               squaredMaxDistToCenterLineForCircleOrientation_ =
                                                   maxDistToCenterLineForCircleOrientation_() *
                                                   maxDistToCenterLineForCircleOrientation_();
                                             })
  , minLineLength_(*this, "minLineLength", [] {})
  , maxLineProjectionDistance_(*this, "maxLineProjectionDistance", [] {})
  , maxLineProjectionDistanceLowNoise_(*this, "maxLineProjectionDistanceLowNoise", [] {})
  , tolerancePenaltySpotToLineDistance_(*this, "tolerancePenaltySpotToLineDistance", [] {})
  , minLineLengthForPenaltyArea_(*this, "minLineLengthForPenaltyArea", [] {})
  , maxLineExtensionForPenaltyArea_(*this, "maxLineExtensionForPenaltyArea",
                                    [this] {
                                      squaredMaxLineExtensionForPenaltyArea_ =
                                          maxLineExtensionForPenaltyArea_() *
                                          maxLineExtensionForPenaltyArea_();
                                    })
  , useLineIntersections_(*this, "useLineIntersections", [] {})
  , minIntersectionOverlap_(*this, "minIntersectionOverlap",
                            [this] {
                              squaredMinIntersectionOverlap_ =
                                  minIntersectionOverlap_() * minIntersectionOverlap_();
                            })
  , maxIntersectionDistance_(*this, "maxIntersectionDistance",
                             [this] {
                               squaredMaxIntersectionDistance_ =
                                   maxIntersectionDistance_() * maxIntersectionDistance_();
                             })
  , ballState_(*this)
  , cameraMatrix_(*this)
  , cycleInfo_(*this)
  , gameControllerState_(*this)
  , goalData_(*this)
  , lineData_(*this)
  , penaltySpotData_(*this)
  , imageData_(*this)
  , fieldDimensions_(*this)
  , motionState_(*this)
  , odometryOffset_(*this)
  , landmarkModel_(*this)
  , goalPostBuffer_()
  , optimalGoalPostDistance_(fieldDimensions_->goalInnerWidth + fieldDimensions_->goalPostDiameter)
  , maxLineLengthForCircle_()
{
  maxLineLengthForCircle_ =
      maxLineLengthForCircleFraction_() * fieldDimensions_->fieldCenterCircleDiameter / 2.0f;
  squaredMaxDistToCircleCluster_ = maxDistToCircleCluster_() * maxDistToCircleCluster_();
  squaredMaxDistToCenterLineForCircleOrientation_ =
      maxDistToCenterLineForCircleOrientation_() * maxDistToCenterLineForCircleOrientation_();
  squaredMinIntersectionOverlap_ = minIntersectionOverlap_() * minIntersectionOverlap_();
  squaredMaxIntersectionDistance_ = maxIntersectionDistance_() * maxIntersectionDistance_();
  squaredMaxLineExtensionForPenaltyArea_ =
      maxLineExtensionForPenaltyArea_() * maxLineExtensionForPenaltyArea_();
}

void LandmarkFilter::cycle()
{
  {
    Chronometer time(debug(), mount_ + ".cycle_time");

    if (goalData_->valid)
    {
      // Add new goal posts to buffer and eventually remove outdated ones
      updateGoalPosts();
      // Combine goal posts to goals
      assembleGoals();
    }
    if (lineData_->valid)
    {
      // Prefilter all lines
      const auto linesWithMetaData = filterLines(lineData_->lineInfos);
      // filter lines with special criteria for center circle
      const auto circleLinesWithMetaData = filterLinesForCenterCircle(lineData_->lineInfos);
      // Search for a center circle
      findCenterCircle(circleLinesWithMetaData);
      // Search for intersections
      if (useLineIntersections_())
      {
        findIntersections(linesWithMetaData);
      }
      // Search for penalty area
      if (penaltySpotData_->valid)
      {
        landmarkModel_->penaltyAreas =
            findPenaltyAreas(penaltySpotData_->penaltySpot.relativePosition, linesWithMetaData);
      }
      else if (ballCreatesPenaltySpotInPSO_() &&
               gameControllerState_->gameState == GameState::PLAYING &&
               gameControllerState_->gamePhase == GamePhase::PENALTYSHOOT &&
               ballState_->confident && !ballState_->moved &&
               cycleInfo_->getAbsoluteTimeDifference(ballState_->timeWhenLastSeen) < 0.2s)
      {

        // there was no penalty spot detected and we have a resting, confident ball that was seen
        // recently. We can assume that this ball covers the penalty spot and this can be used as a
        // penalty spot feature
        landmarkModel_->penaltyAreas = findPenaltyAreas(ballState_->position, linesWithMetaData);
      }
      // save lines and projection distances which haven't been used for landmarks yet
      saveUnusedLines(linesWithMetaData);
      // set timestamp of LandmarkModel
      landmarkModel_->timestamp = imageData_->captureTimePoint;
    }
  }
  // do debug stuff
  sendDebugImage();
  debug().update(mount_, *this);
  debug().update(mount_ + ".LandmarkModel", *landmarkModel_);
}

void LandmarkFilter::updateGoalPosts()
{
  if (!bufferGoalPosts_())
  {
    // clear buffer if buffering is switched off
    goalPostBuffer_.clear();
  }
  else
  {
    const Pose inverseOdometryOffset = odometryOffset_->odometryOffset.inverse();
    for (auto& goalPost : goalPostBuffer_)
    {
      // apply inverse odometry to the goal posts (in order to move them in relative coordinates)
      goalPost.position = inverseOdometryOffset * goalPost.position;
    }
  }
  // add new goal posts to buffer (only if new data available)
  if (goalData_->timestamp != lastTimestamp_)
  {
    lastTimestamp_ = goalData_->timestamp;
    for (auto& newGoalPostPosition : goalData_->posts)
    {
      // add new data to buffer
      GoalPost newGoalPost = GoalPost(newGoalPostPosition, cycleInfo_->startTime);
      auto goalPostIt = goalPostBuffer_.begin();
      for (; goalPostIt != goalPostBuffer_.end(); goalPostIt++)
      {
        // merge with existing goal post if within association radius and not from the same image
        if ((goalPostIt->position - newGoalPost.position).norm() < goalPostAssociationRadius_() &&
            goalPostIt->timestampLastSeen != newGoalPost.timestampLastSeen)
        {
          *goalPostIt = newGoalPost;
          break;
        }
      }
      if (goalPostIt == goalPostBuffer_.end())
      {
        goalPostBuffer_.push_front(newGoalPost);
      }
    }
  }
  // if buffering is switched on, old goal posts need to be removed from the buffer, if not the
  // buffer is cleared in the beginning of this method anyway
  if (bufferGoalPosts_())
  {
    for (auto goalPostIt = goalPostBuffer_.begin(); goalPostIt != goalPostBuffer_.end();)
    {
      if (cycleInfo_->getAbsoluteTimeDifference(goalPostIt->timestampLastSeen) > maxGoalPostAge_())
      {
        goalPostIt = goalPostBuffer_.erase(goalPostIt);
      }
      else
      {
        goalPostIt++;
      }
    }
  }
}

void LandmarkFilter::assembleGoals()
{
  if (goalPostBuffer_.size() < 2)
  {
    return;
  }

  // Check all unique combinations of two goal posts.
  for (auto post1 = goalPostBuffer_.begin(); std::next(post1) != goalPostBuffer_.end(); post1++)
  {
    for (auto post2 = std::next(post1); post2 != goalPostBuffer_.end(); post2++)
    {
      float dist = (post1->position - post2->position).norm();
      if (std::abs(dist - optimalGoalPostDistance_) < maxGoalPostDistanceDeviation_())
      {
        if (post1->position.y() > post2->position.y())
        {
          landmarkModel_->goals.emplace_back(post1->position, post2->position);
        }
        else
        {
          landmarkModel_->goals.emplace_back(post2->position, post1->position);
        }
      }
    }
  }
}

std::vector<LineInfo> LandmarkFilter::filterLines(const std::vector<LineInfo>& linesWithMetaData)
{
  std::vector<LineInfo> filteredLineInfos;
  filteredLineInfos.reserve(linesWithMetaData.size());

  // TODO: maybe also constraint this to set
  const bool lowBodyTiltUncertainty =
      motionState_->bodyMotion == ActionCommand::Body::MotionType::STAND;
  landmarkModel_->maxLineProjectionDistance =
      lowBodyTiltUncertainty ? maxLineProjectionDistanceLowNoise_() : maxLineProjectionDistance_();

  for (const auto& lineInfo : linesWithMetaData)
  {
    // skip lines which are too short
    if (lineInfo.lineLength < minLineLength_())
    {
      continue;
    }

    // drop lines that are projected over a large distance
    if (lineInfo.projectionDistance > landmarkModel_->maxLineProjectionDistance)
    {
      continue;
    }

    // store line information of lines which passed the checks
    filteredLineInfos.emplace_back(lineInfo);
  }
  return filteredLineInfos;
}

void LandmarkFilter::findCenterCircle(const std::vector<LineInfo>& linesWithMetaData)
{
  // check if there are enough lines for clustering
  if (linesWithMetaData.size() < minLineCountClusterable_())
  {
    return;
  }

  // cluster the lines by how well they correspend with the center circle
  auto clusters = clusterLines(linesWithMetaData);
  if (clusters.empty())
  {
    return;
  }

  // find the cluster with most points in it (should be the center circle)
  auto bestCluster = clusters.begin();
  for (auto it = clusters.begin(); it != clusters.end(); ++it)
  {
    if (it->clusterCount > bestCluster->clusterCount)
    {
      bestCluster = it;
    }
  }
  // check if there are enough points in the cluster
  if (bestCluster->clusterCount < minCountPointsInCluster_())
  {
    return;
  }

  // find the orientation of the center circle by checking for a line that passes through the best
  // cluster
  bool foundOrientation = false;
  size_t circleOrientationLineID = 0;
  float circleOrientation = 0.f;
  // extracting the orientation from the set of filtered lines
  std::tie(foundOrientation, circleOrientationLineID, circleOrientation) =
      findCenterLineOrientation(lineData_->lineInfos, *bestCluster);

  // remember which lines were used
  std::vector<size_t> usedLineIds;
  usedLineIds.reserve(bestCluster->lineIds.size());
  for (auto lineId : bestCluster->lineIds)
  {
    usedLineIds.emplace_back(lineId);
  }
  if (foundOrientation)
  {
    usedLineIds.emplace_back(circleOrientationLineID);
  }

  // add center circle to landmarkModel
  landmarkModel_->centerCircles.emplace_back(bestCluster->center, foundOrientation,
                                             circleOrientation, usedLineIds);
}

std::vector<LineInfo>
LandmarkFilter::filterLinesForCenterCircle(const std::vector<LineInfo>& linesWithMetaData)
{
  std::vector<LineInfo> filteredLineInfos;
  filteredLineInfos.reserve(linesWithMetaData.size());

  // Get rid of lines which don't fit necessary requirements for centre circle
  for (size_t i = 0; i < linesWithMetaData.size(); ++i)
  {
    auto& lineInfo = linesWithMetaData[i];

    // check if the line is too short or too long
    if (lineInfo.lineLength > maxLineLengthForCircle_ ||
        lineInfo.lineLength < minLineLengthForCircle_())
    {
      continue;
    }

    // check if the line is too far away
    if (lineInfo.projectionDistance > maxLineProjectionDistanceForCircle_())
    {
      continue;
    }

    // save line and index when all checks were passed
    filteredLineInfos.emplace_back(lineInfo);
  }
  return filteredLineInfos;
}

std::vector<LandmarkFilter::PointCluster2D>
LandmarkFilter::clusterLines(const std::vector<LineInfo>& linesWithMetaData)
{
  std::vector<CenterPointCandidate> centerPointCandidates;
  centerPointCandidates.reserve(linesWithMetaData.size() * 2);

  for (size_t i = 0; i < linesWithMetaData.size(); ++i)
  {
    auto& lineInfo = linesWithMetaData[i];
    auto line = lineInfo.line;
    auto lineId = lineInfo.lineId;

    // calculate the center of the line
    const Vector2f lineCenter = (line->p1 + line->p2) * 0.5f;

    // calculate the othorgonal line vector:
    const Vector2f lineVector = line->p2 - line->p1;
    const float lineAngle = std::atan2(lineVector.y(), lineVector.x());
    const Vector2f relativeOrthorgonalLineRadius =
        Vector2f(lineVector.y(), -lineVector.x()).normalized() *
        fieldDimensions_->fieldCenterCircleDiameter * 0.5f;

    // calculate the relativeCircleCenter based on point with lower error
    const Vector2f relativeCircleCenterCandidate1 = lineCenter + relativeOrthorgonalLineRadius;
    const Vector2f relativeCircleCenterCandidate2 = lineCenter - relativeOrthorgonalLineRadius;
    centerPointCandidates.emplace_back(relativeCircleCenterCandidate1, lineAngle, lineId);
    centerPointCandidates.emplace_back(relativeCircleCenterCandidate2, lineAngle, lineId);
  }

  // cluster the points
  std::vector<PointCluster2D> candidateClusters;
  candidateClusters.reserve(centerPointCandidates.size());

  for (size_t it = 0; it < centerPointCandidates.size(); ++it)
  {
    auto& candidate = centerPointCandidates[it];

    bool mergedWithCluster = false;
    for (auto& cluster : candidateClusters)
    {
      // check if the candidate point is close enough to the cluster
      if ((candidate.point - cluster.center).squaredNorm() < squaredMaxDistToCircleCluster_)
      {
        // check if the candidate line is orthogonal to any inside the cluster
        bool isOrthogonal = false;
        for (auto& angle : cluster.anglesToRobotX)
        {
          auto diffAngle = Angle::angleDiff(angle, candidate.angleToRobotX + 90 * TO_RAD);
          if (diffAngle < orthogonalTolerance_())
          {
            isOrthogonal = true;
            break;
          }
        }

        if (isOrthogonal)
        {
          continue;
        }

        // add point to cluster if checks were passed
        // calculate new cluster center
        cluster.center = cluster.center * cluster.clusterCount + candidate.point;
        cluster.center /= ++cluster.clusterCount;
        cluster.anglesToRobotX.emplace_back(candidate.angleToRobotX);
        cluster.lineIds.emplace_back(candidate.lineId);

        // TODO: Reason about whether point can be part of multiple clusters
        // (currently they can)
        mergedWithCluster = true;
      }
    }

    // if the point wasn't merged with any cluster, open a new one
    if (!mergedWithCluster)
    {
      candidateClusters.emplace_back(candidate.point, 1, candidate.angleToRobotX, candidate.lineId);
    }
  }
  return candidateClusters;
}

std::tuple<bool, size_t, float>
LandmarkFilter::findCenterLineOrientation(const std::vector<LineInfo>& linesWithMetaData,
                                          const LandmarkFilter::PointCluster2D& candidateCluster)
{
  std::vector<std::pair<const Line<float>&, size_t>> linesThroughCenter;

  // search for lines that move through the center
  for (size_t i = 0; i < linesWithMetaData.size(); ++i)
  {
    auto& lineInfo = linesWithMetaData[i];
    auto& line = *(lineInfo.line);
    auto lineId = lineInfo.lineId;

    if (lineInfo.lineLength < minLineLengthForCircleOrientation_())
    {
      continue;
    }

    float squaredDistToLine = Geometry::getSquaredLineDistance(line, candidateCluster.center);
    if (squaredDistToLine > squaredMaxDistToCenterLineForCircleOrientation_)
    {
      continue;
    }

    linesThroughCenter.emplace_back(line, lineId);
  }

  if (linesThroughCenter.size() > 0)
  {
    float longestLength = 0;
    const Line<float>* longestLine = &linesThroughCenter[0].first;
    size_t longestLineId = linesThroughCenter[0].second;

    for (const auto& lineAndIdPair : linesThroughCenter)
    {
      const float lineLength = (lineAndIdPair.first.p2 - lineAndIdPair.first.p1).squaredNorm();
      if (lineLength > longestLength)
      {
        longestLength = lineLength;
        longestLine = &lineAndIdPair.first;
        longestLineId = lineAndIdPair.second;
      }
    }

    Vector2f lineVector = longestLine->p2 - longestLine->p1;
    float orientation = std::atan2(lineVector.y(), lineVector.x()) + 90 * TO_RAD;
    return {true, longestLineId, orientation};
  }
  return {false, -1, 0.f};
}

std::vector<LandmarkModel::PenaltyArea>
LandmarkFilter::findPenaltyAreas(const Vector2f& relativePenaltySpotPosition,
                                 const std::vector<LineInfo>& linesWithMetaData)
{
  std::vector<LandmarkModel::PenaltyArea> penaltyAreas;
  std::vector<const LineInfo*> orientationLineCandidates;
  float orientation = 0.f;
  std::vector<size_t> usedLineIds;

  // find lines that could work with given penalty spot
  const float desiredDistance =
      fieldDimensions_->fieldPenaltyAreaLength - fieldDimensions_->fieldPenaltyMarkerDistance;
  for (auto& lineWithMetaData : linesWithMetaData)
  {
    // check the length of the line
    if (lineWithMetaData.lineLength < minLineLengthForPenaltyArea_())
    {
      continue;
    }

    // calculate distance between penalty spot and line
    const float distance =
        Geometry::distPointToLine(*lineWithMetaData.line, relativePenaltySpotPosition);

    // check if distance is within tolerance
    if (std::abs(desiredDistance - distance) > tolerancePenaltySpotToLineDistance_())
    {
      continue;
    }

    // check the length by which the line needs to be extended for intersection
    const Vector2f intersection =
        Geometry::projectPointOnLine(relativePenaltySpotPosition, *lineWithMetaData.line);

    const Vector2f vec1 = intersection - lineWithMetaData.line->p1;
    const Vector2f vec2 = intersection - lineWithMetaData.line->p2;
    // check if intersection point lies outside line segment
    if (vec1.dot(vec2) > 0)
    {
      // in here the intersection lies outside the line segment
      // now take the shorter distance and check that against the maximum length allowed
      const float squaredLength = std::min(vec1.squaredNorm(), vec2.squaredNorm());
      if (squaredLength > squaredMaxLineExtensionForPenaltyArea_)
      {
        continue;
      }
    }

    orientationLineCandidates.push_back(&lineWithMetaData);
  }

  // if theres is no or more than one candidate don't add an orientation
  if (orientationLineCandidates.size() != 1)
  {
    penaltyAreas.emplace_back(relativePenaltySpotPosition, false, orientation, usedLineIds);
    return penaltyAreas;
  }

  // calculate the orientation
  const auto* orientationLineWithMetaData = orientationLineCandidates.front();
  const Vector2f orientationVec =
      -1.f * Geometry::getPointToLineVector(relativePenaltySpotPosition,
                                            *(orientationLineWithMetaData->line));
  orientation = std::atan2(orientationVec.y(), orientationVec.x());
  // store the id of the used line so that we don't use it for any other updates
  usedLineIds.push_back(orientationLineWithMetaData->lineId);

  penaltyAreas.emplace_back(relativePenaltySpotPosition, true, orientation, usedLineIds);
  return penaltyAreas;
}

void LandmarkFilter::findIntersections(const std::vector<LineInfo>& linesWithMetaData)
{
  auto orthogonalLinePairs = findOrthogonalLines(linesWithMetaData);

  landmarkModel_->intersections = constructIntersections(orthogonalLinePairs);
}

std::vector<std::pair<const LineInfo&, const LineInfo&>>
LandmarkFilter::findOrthogonalLines(const std::vector<LineInfo>& linesWithMetaData)
{
  // find Orthogonal lines
  std::vector<std::pair<const LineInfo&, const LineInfo&>> orthogonalLines;
  orthogonalLines.reserve(linesWithMetaData.size() * 2);

  for (auto it1 = linesWithMetaData.begin();
       linesWithMetaData.size() > 0 && it1 != linesWithMetaData.end() - 1; ++it1)
  {
    auto& line1 = *(it1->line);

    for (auto it2 = it1 + 1; it2 != linesWithMetaData.end(); ++it2)
    {
      auto& line2 = *(it2->line);

      // calculate angle between lines
      float angle = 0.f;
      bool valid = Geometry::getAngleBetween(line1, line2, angle);
      if (valid)
      {
        // check if the lines are orthogonal
        auto diffAngle = Angle::angleDiff(angle, 90 * TO_RAD);
        if (diffAngle < orthogonalTolerance_())
        {
          orthogonalLines.emplace_back(*it1, *it2);
        }
      }
    }
  }

  return orthogonalLines;
}

std::vector<LandmarkModel::Intersection> LandmarkFilter::constructIntersections(
    const std::vector<std::pair<const LineInfo&, const LineInfo&>> orthogonalLinePairs)
{
  using IntersectionType = LandmarkModel::Intersection::IntersectionType;

  std::vector<LandmarkModel::Intersection> intersections;
  intersections.reserve(orthogonalLinePairs.size());

  Vector2f line1vec1 = {0.f, 0.f};
  Vector2f line1vec2 = {0.f, 0.f};
  Vector2f line2vec1 = {0.f, 0.f};
  Vector2f line2vec2 = {0.f, 0.f};
  float dotProductLine1 = 0.f;
  float dotProductLine2 = 0.f;

  for (auto& linePair : orthogonalLinePairs)
  {
    LandmarkModel::Intersection intersection;
    auto& line1 = *linePair.first.line;
    auto& line2 = *linePair.second.line;

    // Find the point of intersection
    bool valid = Geometry::getIntersection(line1, line2, intersection.position);

    // Check which type of intersection it is
    if (valid)
    {
      line1vec1 = intersection.position - line1.p1;
      line1vec2 = intersection.position - line1.p2;
      line2vec1 = intersection.position - line2.p1;
      line2vec2 = intersection.position - line2.p2;

      dotProductLine1 = line1vec1.dot(line1vec2);
      dotProductLine2 = line2vec1.dot(line2vec2);

      // if dotProductLine is negative the intersection point lies on the line
      intersection.intersectionOnLine1 = (dotProductLine1 < 0.f);
      intersection.intersectionOnLine2 = (dotProductLine2 < 0.f);

      // define the type of intersection
      if (intersection.intersectionOnLine1 && intersection.intersectionOnLine2)
      {
        intersection.intersectionType = IntersectionType::XINTERSECTION;
      }
      else if (intersection.intersectionOnLine1 || intersection.intersectionOnLine2)
      {
        intersection.intersectionType = IntersectionType::TINTERSECTION;
      }
      else
      {
        intersection.intersectionType = IntersectionType::LINTERSECTION;
      }

      // save used lineIds
      intersection.usedLineIds = {linePair.first.lineId, linePair.second.lineId};

      // check if the intersection fulfills all requirements
      bool intersectionOk = checkIntersection(intersection);

      // save the intersection
      if (intersectionOk)
      {
        // check the orientation of the intersection
        std::tie(intersection.hasOrientation, intersection.orientation) =
            findIntersectionOrientation(intersection);

        intersections.push_back(intersection);
      }
    }
  }

  return intersections;
}

bool LandmarkFilter::checkIntersection(LandmarkModel::Intersection& intersection)
{
  using IntersectionType = LandmarkModel::Intersection::IntersectionType;

  auto& line1 = lineData_->lines[intersection.usedLineIds.front()];
  auto& line2 = lineData_->lines[intersection.usedLineIds.back()];
  auto& intersectionPoint = intersection.position;

  float minDistSquaredLine1 = std::min((intersectionPoint - line1.p1).squaredNorm(),
                                       (intersectionPoint - line1.p2).squaredNorm());
  float minDistSquaredLine2 = std::min((intersectionPoint - line2.p1).squaredNorm(),
                                       (intersectionPoint - line2.p2).squaredNorm());

  // check if the there is enough overlap for an X intersection
  if (intersection.intersectionType == IntersectionType::XINTERSECTION)
  {
    // degrade to T intersection if necessary
    if (minDistSquaredLine1 < squaredMinIntersectionOverlap_)
    {
      intersection.intersectionType = IntersectionType::TINTERSECTION;
      intersection.intersectionOnLine1 = false;
    }
    else if (minDistSquaredLine2 < squaredMinIntersectionOverlap_)
    {
      intersection.intersectionType = IntersectionType::TINTERSECTION;
      intersection.intersectionOnLine2 = false;
    }
  }

  // check if there is enough overlap for a T section
  if (intersection.intersectionType == IntersectionType::TINTERSECTION)
  {
    if (intersection.intersectionOnLine1)
    {
      // degrade to L intersection
      if (minDistSquaredLine1 < squaredMinIntersectionOverlap_)
      {
        intersection.intersectionType = IntersectionType::LINTERSECTION;
        intersection.intersectionOnLine1 = false;
      }
    }
    else if (intersection.intersectionOnLine2)
    {
      // degrade to L intersection
      if (minDistSquaredLine2 < squaredMinIntersectionOverlap_)
      {
        intersection.intersectionType = IntersectionType::LINTERSECTION;
        intersection.intersectionOnLine2 = false;
      }
    }
  }

  // check the length between the line ends and the intersection point
  if (!intersection.intersectionOnLine1)
  {
    if (minDistSquaredLine1 > squaredMaxIntersectionDistance_)
    {
      return false;
    }
  }
  if (!intersection.intersectionOnLine2)
  {
    if (minDistSquaredLine2 > squaredMaxIntersectionDistance_)
    {
      return false;
    }
  }

  return true;
}

std::tuple<bool, float>
LandmarkFilter::findIntersectionOrientation(const LandmarkModel::Intersection& intersection)
{
  using IntersectionType = LandmarkModel::Intersection::IntersectionType;

  Vector2f orientationVec(0.f, 0.f);
  float orientation = 0.f;
  bool hasOrientation = false;

  auto& line1 = lineData_->lines[intersection.usedLineIds.front()];
  auto& line2 = lineData_->lines[intersection.usedLineIds.back()];

  switch (intersection.intersectionType)
  {
    // not possible to define an orientation
    case IntersectionType::XINTERSECTION:
      break;

    /* orientation defined by the lower line of the T
     *   ------
     *     |
     *     |
     *     | <- orientation vector
     */
    case IntersectionType::TINTERSECTION:
      if (intersection.intersectionOnLine1)
      {
        if ((intersection.position - line2.p1).squaredNorm() >
            (intersection.position - line2.p2).squaredNorm())
        {
          orientationVec = line2.p1 - line2.p2;
        }
        else
        {
          orientationVec = line2.p2 - line2.p1;
        }
      }
      else if (intersection.intersectionOnLine2)
      {
        if ((intersection.position - line1.p1).squaredNorm() >
            (intersection.position - line1.p2).squaredNorm())
        {
          orientationVec = line1.p1 - line1.p2;
        }
        else
        {
          orientationVec = line1.p2 - line1.p1;
        }
      }
      orientation = std::atan2(orientationVec.y(), orientationVec.x());
      hasOrientation = true;
      break;

    /* orientation defined by a vector 45 degs between the two legs of intersection
     *   -----
     *  |\
     *  | \
     *  |  \<- orientation vector
     */
    case IntersectionType::LINTERSECTION: {
      // calculate line vectors so they point away from the intersection
      Vector2f line1Vec;
      Vector2f line2Vec;
      if ((intersection.position - line1.p1).squaredNorm() >
          (intersection.position - line1.p2).squaredNorm())
      {
        line1Vec = line1.p1 - line1.p2;
      }
      else
      {
        line1Vec = line1.p2 - line1.p1;
      }

      if ((intersection.position - line2.p1).squaredNorm() >
          (intersection.position - line2.p2).squaredNorm())
      {
        line2Vec = line2.p1 - line2.p2;
      }
      else
      {
        line2Vec = line2.p2 - line2.p1;
      }

      orientationVec = line1Vec.normalized() + line2Vec.normalized();
      orientation = std::atan2(orientationVec.y(), orientationVec.x());
      hasOrientation = true;
      break;
    }
    case IntersectionType::UNDEFINED:
      break;
    default:
      break;
  }

  return {hasOrientation, orientation};
}

void LandmarkFilter::saveUnusedLines(const std::vector<LineInfo>& linesWithMetaData)
{
  std::vector<size_t> lineIdsUsedByLandmarks;

  // collect lineIds used by center circle
  for (const auto& centerCircle : landmarkModel_->centerCircles)
  {
    lineIdsUsedByLandmarks.insert(lineIdsUsedByLandmarks.end(), centerCircle.usedLineIds.begin(),
                                  centerCircle.usedLineIds.end());
  }
  // collect lineIds used by penalty areas
  for (const auto& penaltyArea : landmarkModel_->penaltyAreas)
  {
    lineIdsUsedByLandmarks.insert(lineIdsUsedByLandmarks.end(), penaltyArea.usedLineIds.begin(),
                                  penaltyArea.usedLineIds.end());
  }
  // collect lineIds used by intersections
  for (auto& intersection : landmarkModel_->intersections)
  {
    lineIdsUsedByLandmarks.insert(lineIdsUsedByLandmarks.end(), intersection.usedLineIds.begin(),
                                  intersection.usedLineIds.end());
  }

  unsigned int filteredLineId = 0;
  for (size_t i = 0; i < linesWithMetaData.size(); ++i)
  {
    const auto& lineInfo = linesWithMetaData[i];

    // check whether the line is being used in other landmarks
    if (std::find(lineIdsUsedByLandmarks.begin(), lineIdsUsedByLandmarks.end(), i) !=
        lineIdsUsedByLandmarks.end())
    {
      continue;
    }

    // copy the lines and projectionDistances that haven't been used to landmarkModel
    landmarkModel_->filteredLines.emplace_back(*(lineInfo.line));
    auto& line = landmarkModel_->filteredLines[filteredLineId];
    landmarkModel_->filteredLineInfos.emplace_back(line, lineInfo.projectionDistance,
                                                   lineInfo.lineLength, filteredLineId);
    ++filteredLineId;
  }
}

void LandmarkFilter::sendDebugImage()
{
  if (debug().isSubscribed(mount_ + "." + imageData_->identification + "_image"))
  {
    Image image(imageData_->image422.to444Image());

    // draw center circle
    for (auto& centerCircle : landmarkModel_->centerCircles)
    {
      if (centerCircle.hasOrientation)
      {
        const auto& lineThroughMiddle = lineData_->lines[centerCircle.usedLineIds.back()];
        const std::optional<Vector2i> pixelCoordsLineP1 =
            cameraMatrix_->robotToPixel(lineThroughMiddle.p1);
        const std::optional<Vector2i> pixelCoordsLineP2 =
            cameraMatrix_->robotToPixel(lineThroughMiddle.p2);
        if (!pixelCoordsLineP1.has_value() || !pixelCoordsLineP2.has_value())
        {
          break;
        }
        image.drawLine(Image422::get444From422Vector(pixelCoordsLineP1.value()),
                       Image422::get444From422Vector(pixelCoordsLineP2.value()), Color::RED);
      }
      const std::optional<Vector2i> pixelCoordsCenter =
          cameraMatrix_->robotToPixel(centerCircle.position);
      if (!pixelCoordsCenter.has_value())
      {
        break;
      }
      image.drawCross(Image422::get444From422Vector(pixelCoordsCenter.value()), 15, Color::BLUE);
    }

    for (const auto& penaltyArea : landmarkModel_->penaltyAreas)
    {
      if (penaltyArea.hasOrientation)
      {
        const auto& orientationLine = lineData_->lines[penaltyArea.usedLineIds.back()];
        const std::optional<Vector2i> pixelCoordsLineP1 =
            cameraMatrix_->robotToPixel(orientationLine.p1);
        const std::optional<Vector2i> pixelCoordsLineP2 =
            cameraMatrix_->robotToPixel(orientationLine.p2);
        if (!pixelCoordsLineP1.has_value() || !pixelCoordsLineP2.has_value())
        {
          break;
        }
        image.drawLine(Image422::get444From422Vector(pixelCoordsLineP1.value()),
                       Image422::get444From422Vector(pixelCoordsLineP2.value()), Color::RED);
      }
      const std::optional<Vector2i> pixelCoordsCenter =
          cameraMatrix_->robotToPixel(penaltyArea.position);
      if (!pixelCoordsCenter.has_value())
      {
        break;
      }
      image.drawCross(Image422::get444From422Vector(pixelCoordsCenter.value()), 15, Color::BLUE);
    }

    // draw intersections
    for (auto& intersection : landmarkModel_->intersections)
    {
      auto& line1 = lineData_->lines[intersection.usedLineIds.front()];
      auto& line2 = lineData_->lines[intersection.usedLineIds.back()];
      auto color = Color::BLACK;

      switch (intersection.intersectionType)
      {
        case LandmarkModel::Intersection::IntersectionType::LINTERSECTION:
          color = Color::BLUE;
          break;
        case LandmarkModel::Intersection::IntersectionType::XINTERSECTION:
          color = Color::RED;
          break;
        case LandmarkModel::Intersection::IntersectionType::TINTERSECTION:
          color = Color::ORANGE;
          break;
        default:
          break;
      }

      const std::optional<Vector2i> pixelCoordsLineP1 = cameraMatrix_->robotToPixel(line1.p1);
      const std::optional<Vector2i> pixelCoordsLineP2 = cameraMatrix_->robotToPixel(line1.p2);
      if (!pixelCoordsLineP1.has_value() || !pixelCoordsLineP2.has_value())
      {
        break;
      }
      image.drawLine(Image422::get444From422Vector(pixelCoordsLineP1.value()),
                     Image422::get444From422Vector(pixelCoordsLineP2.value()), color);

      const std::optional<Vector2i> pixelCoordsLine2P1 = cameraMatrix_->robotToPixel(line2.p1);
      const std::optional<Vector2i> pixelCoordsLine2P2 = cameraMatrix_->robotToPixel(line2.p2);
      if (!pixelCoordsLine2P1.has_value() || !pixelCoordsLine2P2.has_value())
      {
        break;
      }
      image.drawLine(Image422::get444From422Vector(pixelCoordsLine2P1.value()),
                     Image422::get444From422Vector(pixelCoordsLine2P2.value()), color);
    }
    debug().sendImage(mount_ + "." + imageData_->identification + "_image", image);
  }
}

void LandmarkFilter::toValue(Uni::Value& value) const
{
  value = Uni::Value(Uni::ValueType::OBJECT);
  value["goalPosts"] << goalPostBuffer_;
}
