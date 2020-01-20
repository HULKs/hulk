#include "BallSearchMapManager.hpp"

#include "Modules/NaoProvider.h"
#include "Tools/Chronometer.hpp"

BallSearchMapManager::BallSearchMapManager(const ModuleManagerInterface& manager)
  : Module(manager)
  , confidentBallMultiplier_(*this, "confidentBallMultiplier", [] {})
  , convolutionKernelCoreWeight_(*this, "convolutionKernelCoreWeight", [] {})
  , fovAngle_(*this, "fovAngle", [this] { fovAngle_() *= TO_RAD; })
  , maxBallAge_(*this, "maxBallAge", [] {})
  , maxBallDetectionRange_(*this, "maxBallDetectionRange",
                           [this] {
                             maxBallDetectionRangeSquared_ =
                                 maxBallDetectionRange_() * maxBallDetectionRange_();
                           })
  , minBallOutDistance_(*this, "minBallOutDistance", [] {})
  , minProbOnUpvote_(*this, "minProbOnUpvote", [] {})
  , ballState_(*this)
  , bodyPose_(*this)
  , cycleInfo_(*this)
  , fieldDimensions_(*this)
  , gameControllerState_(*this)
  , jointSensorData_(*this)
  , playerConfiguration_(*this)
  , robotPosition_(*this)
  , teamPlayers_(*this)
  , ballSearchMap_(*this)
  , fieldWidth_(fieldDimensions_->fieldWidth)
  , fieldLength_(fieldDimensions_->fieldLength)
{
  fovAngle_() *= TO_RAD; // Obviously.
  maxBallDetectionRangeSquared_ = maxBallDetectionRange_() * maxBallDetectionRange_();

  // Initialize the prob map.
  rebuildProbabilityMap();
}

void BallSearchMapManager::cycle()
{
  Chronometer time(debug(), mount_ + ".cycle_time");

  allPlayers_.clear();

  assert(convolutionKernelCoreWeight_() > 0 && "Convolution kernel was 0 or negative. "
                                               "This may cause div by 0!");

  if (gameControllerState_->gameState == GameState::PLAYING)
  {
    ownPlayer_.playerNumber = playerConfiguration_->playerNumber;
    ownPlayer_.isHULK = true;
    ownPlayer_.pose = robotPosition_->pose;
    ownPlayer_.isPoseValid = robotPosition_->valid;
    ownPlayer_.ballPosition = ballState_->position;
    ownPlayer_.timeWhenBallWasSeen = ballState_->timeWhenLastSeen;
    ownPlayer_.fallen = bodyPose_->fallen;
    ownPlayer_.penalized = gameControllerState_->penalty != Penalty::NONE;
    ownPlayer_.headYaw = jointSensorData_->angles[JOINTS::HEAD_YAW];

    allPlayers_.push_back(&ownPlayer_);

    for (const auto& player : teamPlayers_->players)
    {
      allPlayers_.push_back(&player);
    }
    std::sort(allPlayers_.begin(), allPlayers_.end(),
              [](const TeamPlayer* p1, const TeamPlayer* p2) {
                return p1->playerNumber < p2->playerNumber;
              });

    // Reset the own player's wisdom
    if (ownPlayer_.penalized)
    {
      ballSearchMap_->timestampBallSearchMapUnreliable_ = cycleInfo_->startTime;
    }

    updateMap();
  }
  else if (gameControllerState_->gameState == GameState::READY)
  {
    ballSearchMap_->timestampBallSearchMapUnreliable_ = cycleInfo_->startTime;
    resetMapForReady();
  }
}

void BallSearchMapManager::updateMap()
{
  for (const auto& player : allPlayers_)
  {
    integrateRobotKnowledge(*player);
  }

  const auto rows = static_cast<uint8_t>(ballSearchMap_->rowsCount_);
  const auto cols = static_cast<uint8_t>(ballSearchMap_->colsCount_);
  auto& map = ballSearchMap_->probabilityMap_;
  const Vector2f absoluteOwnBall = robotPosition_->robotToField(ballState_->position);

  // Increase probability at the two throw-in positions if the ball leaves the field
  if (cycleInfo_->getTimeDiff(ballState_->timeWhenLastSeen) < 0.5f &&
      abs(absoluteOwnBall.y()) > fieldDimensions_->fieldWidth / 2.f + minBallOutDistance_())
  {
    // Project ball onto the throw in line
    const Vector2f ballProjection = Vector2f(
        absoluteOwnBall.x(),
        std::min(fieldDimensions_->fieldWidth / 2.f - fieldDimensions_->fieldThrowInLineSpacing,
                 std::max(-fieldDimensions_->fieldWidth / 2.f +
                              fieldDimensions_->fieldThrowInLineSpacing,
                          absoluteOwnBall.y())));

    ProbCell* cell;
    cell = &ballSearchMap_->cellFromPosition(Vector2f(
        std::max(ballProjection.x() - 1.f, -fieldDimensions_->fieldThrowInLineLength / 2.f),
        ballProjection.y()));
    cell->probability = std::max(minProbOnUpvote_(), cell->probability * 1.1f);

    cell = &ballSearchMap_->cellFromPosition(
        Vector2f(std::min(ballProjection.x() + 1.f, fieldDimensions_->fieldThrowInLineLength / 2.f),
                 ballProjection.y()));
    cell->probability = std::max(minProbOnUpvote_(), cell->probability * 1.1f);
  }

  // Increase probability at the free-kick position(s)
  if (gameControllerState_->setPlay == SetPlay::GOAL_FREE_KICK &&
      cycleInfo_->getTimeDiff(gameControllerState_->setPlayChanged) < 0.5f)
  {
    for (auto& row : ballSearchMap_->probabilityMap_)
    {
      for (auto& cell : row)
      {
        cell.probability = 0.f;
      }
    }

    float side = gameControllerState_->kickingTeam ? -1.f : 1.f;

    // Increase the probability at the positions the game controller told us
    ProbCell* cell;
    cell = &ballSearchMap_->cellFromPosition(Vector2f(
        side * (fieldDimensions_->fieldLength / 2.f - fieldDimensions_->fieldPenaltyMarkerDistance),
        std::copysign(fieldDimensions_->fieldPenaltyAreaWidth / 2.f, absoluteOwnBall.y())));
    cell->probability = 0.35f;
    const Vector2f p = Vector2f(cell->position.x(), cell->position.y() * -1);
    cell = &ballSearchMap_->cellFromPosition(p);
    cell->probability = 0.35f;

    // Increase probability at the positions the game controller did not told us (at leas a bit)
    cell = &ballSearchMap_->cellFromPosition({cell->position.x() * -1.f, cell->position.y()});
    cell->probability = 0.15f;
    cell = &ballSearchMap_->cellFromPosition({cell->position.x(), cell->position.y() * -1.f});
    cell->probability = 0.15f;
  }

  // Increase probability at the corner-kick position(s)
  if (gameControllerState_->setPlay == SetPlay::CORNER_KICK &&
      cycleInfo_->getTimeDiff(gameControllerState_->setPlayChanged) < 0.5f)
  {
    for (auto& row : ballSearchMap_->probabilityMap_)
    {
      for (auto& cell : row)
      {
        cell.probability = 0.f;
      }
    }

    float side = gameControllerState_->kickingTeam ? 1.f : -1.f;

    // Increase the probability at the positions the game controller told us
    ProbCell *cornerCellRight, *cornerCellLeft;
    cornerCellLeft = &ballSearchMap_->cellFromPosition(Vector2f(side * fieldDimensions_->fieldLength / 2.f, fieldDimensions_->fieldWidth / 2.f));
    cornerCellRight = &ballSearchMap_->cellFromPosition(Vector2f(side * fieldDimensions_->fieldLength / 2.f, fieldDimensions_->fieldWidth / -2.f));
    cornerCellRight->probability = 0.5f;
    cornerCellLeft->probability = 0.5f;
  }

  // set outer cells probabilities manually (to the nearest cell's probability).
  for (int x = 1; x < cols - 1; x++)
  {
    map[x][0].probability = map[x][1].probability;
    map[x][rows - 1].probability = map[x][rows - 2].probability;
  }
  for (int y = 1; y < rows - 1; y++)
  {
    map[0][y].probability = map[1][y].probability;
    map[cols - 1][y].probability = map[cols - 2][y].probability;
  }
  map[0][0].probability = map[1][1].probability;
  map[0][rows - 1].probability = map[1][rows - 2].probability;
  map[cols - 1][0].probability = map[cols - 2][1].probability;
  map[cols - 1][rows - 1].probability = map[cols - 2][rows - 2].probability;

  // backup probabilities for convolution.
  for (int x = 1; x < ballSearchMap_->colsCount_ - 1; x++)
  {
    for (int y = 1; y < ballSearchMap_->rowsCount_ - 1; y++)
    {
      map[x][y].oldProbability = map[x][y].probability;
    }
  }

  // actually convolve with [[1, 1, 1][1, x, 1][1, 1, 1]] where x is the kernel core weight (config)
  for (int x = 1; x < ballSearchMap_->colsCount_ - 1; x++)
  {
    for (int y = 1; y < ballSearchMap_->rowsCount_ - 1; y++)
    {
      float result = map[x - 1][y - 1].oldProbability + map[x][y - 1].oldProbability +
                     map[x + 1][y - 1].oldProbability + map[x - 1][y].oldProbability +
                     static_cast<float>(convolutionKernelCoreWeight_()) * map[x][y].oldProbability +
                     map[x + 1][y].oldProbability + map[x - 1][y + 1].oldProbability +
                     map[x][y + 1].oldProbability + map[x + 1][y + 1].oldProbability;

      result *= 1.f / (convolutionKernelCoreWeight_() + 8);

      // Check if we would decrease the probability with this operation. If so, do not apply the new
      // value. Reason: The prob should only be decreased if a robot is looking at this cell (and
      // no ball is found), or by normalization (then the ball was found somewhere else)
      map[x][y].probability = map[x][y].oldProbability < result ? result : map[x][y].oldProbability;
    }
  }

  // Sum all probabilities
  float weightSum = 0.f;
  for (int x = 1; x < ballSearchMap_->colsCount_ - 1; x++)
  {
    for (int y = 1; y < ballSearchMap_->rowsCount_ - 1; y++)
    {
      weightSum += map[x][y].probability;
    }
  }
  assert(weightSum > 0.f && "Weight sum was either 0 or negative.");

  // Normalize and do aging.
  for (int x = 1; x < ballSearchMap_->colsCount_ - 1; x++)
  {
    for (int y = 1; y < ballSearchMap_->rowsCount_ - 1; y++)
    {
      map[x][y].probability /= weightSum; // normalize this cell.
      map[x][y].age++;
    }
  }
}

void BallSearchMapManager::integrateRobotKnowledge(const TeamPlayer& player)
{
  // Skip player if it is penalized.
  if (player.penalized)
  {
    return;
  }

  // Skip player if they are not sure about their self localization
  if (!player.isPoseValid)
  {
    return;
  }

  // The ball age given in seconds (seconds are default).
  float ballAge = cycleInfo_->getTimeDiff(player.timeWhenBallWasSeen);

  // Vote cell up if there is a ball in it.
  if (ballAge < maxBallAge_())
  {
    ProbCell& cellWithBall = ballSearchMap_->cellFromPosition(player.pose * player.ballPosition);
    cellWithBall.probability =
        std::max(minProbOnUpvote_(), cellWithBall.probability * confidentBallMultiplier_());
    cellWithBall.age = 0;
  }
  else
  {
    // Nothing since cells will be downvoted in the next step
  }

  // Decrease probability of all cells in FOV (including the cell containing the ball if there was
  // any).
  for (int x = 1; x < ballSearchMap_->colsCount_ - 1; x++)
  {
    for (int y = 1; y < ballSearchMap_->rowsCount_ - 1; y++)
    {
      if (ballSearchMap_->isCellInFOV(player.pose, player.headYaw,
                                      ballSearchMap_->probabilityMap_[x][y],
                                      maxBallDetectionRangeSquared_, fovAngle_()))
      {
        // Reduce probability a bit.
        ballSearchMap_->probabilityMap_[x][y].probability *=
            (ballAge < maxBallAge_()) ? 0.99f : 0.98f;
        ballSearchMap_->probabilityMap_[x][y].age = 0;
      }
    }
  }
}

void BallSearchMapManager::resetMap()
{
  for (int x = 0; x < ballSearchMap_->colsCount_; ++x)
  {
    for (int y = 0; y < ballSearchMap_->rowsCount_; ++y)
    {
      ballSearchMap_->probabilityMap_[x][y].probability =
          1.f / static_cast<float>(ballSearchMap_->rowsCount_ * ballSearchMap_->colsCount_);
      ballSearchMap_->probabilityMap_[x][y].age = 0;
    }
  }
}

void BallSearchMapManager::resetMapForReady()
{
  resetMap();

  ballSearchMap_->probabilityMap_[ballSearchMap_->colsCount_ / 2][ballSearchMap_->rowsCount_ / 2]
      .probability = 0.25f;
  ballSearchMap_
      ->probabilityMap_[ballSearchMap_->colsCount_ / 2 - 1][ballSearchMap_->rowsCount_ / 2]
      .probability = 0.25f;
  ballSearchMap_
      ->probabilityMap_[ballSearchMap_->colsCount_ / 2][ballSearchMap_->rowsCount_ / 2 - 1]
      .probability = 0.25f;
  ballSearchMap_
      ->probabilityMap_[ballSearchMap_->colsCount_ / 2 - 1][ballSearchMap_->rowsCount_ / 2 - 1]
      .probability = 0.25f;
}

void BallSearchMapManager::distributeProbability(const Vector2f& p1, const Vector2f& p2,
                                                 const float totalProbability)
{
  // calculate field coordinates from the given points
  Vector2f v1 = Vector2f(std::min(p1.x(), p2.x()) * fieldDimensions_->fieldLength * 0.5,
                         std::min(p1.y(), p2.y()) * fieldDimensions_->fieldWidth * 0.5);
  Vector2f v2 = Vector2f(std::max(p1.x(), p2.x()) * fieldDimensions_->fieldLength * 0.5,
                         std::max(p1.y(), p2.y()) * fieldDimensions_->fieldWidth * 0.5);

  ProbCell cell1 = ballSearchMap_->cellFromPosition(v1);
  ProbCell cell2 = ballSearchMap_->cellFromPosition(v2);

  int cellCount =
      (cell2.indices.x() - cell1.indices.x() + 1) * (cell2.indices.y() - cell1.indices.y() + 1);
  assert(cellCount >= 1);

  for (int y = cell1.indices.y(); y < cell2.indices.y() + 1; y++)
  {
    for (int x = cell1.indices.x(); x < cell2.indices.x() + 1; x++)
    {
      ballSearchMap_->probabilityMap_[x][y].probability =
          totalProbability / static_cast<float>(cellCount);
    }
  }
}

void BallSearchMapManager::rebuildProbabilityMap()
{
  ballSearchMap_->initialize(Vector2f(fieldDimensions_->fieldLength, fieldDimensions_->fieldWidth));
}
